use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB, Page, PageTableFlags},
    PhysAddr, VirtAddr,
};
use spin::Mutex;

/// Initialize a new OffsetPageTable.
pub unsafe fn init_sexting(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_VAS: Mutex<Option<GlobalVas>> = Mutex::new(None);
}

/// The Global Virtual Address Space Manager.
/// In SASOS, there is only one of these for the entire system.
pub struct GlobalVas {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BitmapFrameAllocator,
}

impl GlobalVas {
    /// Maps a virtual address range to physical frames.
    /// This is a Phase 1 primitive.
    pub fn map_range(&mut self, start: VirtAddr, size: u64, flags: PageTableFlags) -> Result<(), &'static str> {
        self.map_pku_range(start, size, flags, 0)
    }

    pub fn map_pku_range(&mut self, start: VirtAddr, size: u64, flags: PageTableFlags, pku_key: u8) -> Result<(), &'static str> {
        let page_range = {
            let start_page = Page::containing_address(start);
            let end_page = Page::containing_address(start + size - 1u64);
            Page::range_inclusive(start_page, end_page)
        };

        for page in page_range {
            let frame = self.frame_allocator.allocate_frame()
                .ok_or("VAS: Frame allocation failed")?;
            unsafe {
                let _flush = self.mapper.map_to(page, frame, flags, &mut self.frame_allocator)
                    .map_err(|_| "VAS: Mapping failed")?;
                
                // Manual PKU bit manipulation if needed (Bits 59-62)
                // For this prototype, we'll assume the mapper handles standard flags.
                // To set PKU bits, we'd need to traverse the page tables manually or use a custom mapper.
                
                _flush.flush();
            }
        }
        Ok(())
    }
}

pub struct BitmapFrameAllocator {
    bitmap: &'static mut [u8],
    max_frame: usize,
    last_allocated: usize,
}

impl BitmapFrameAllocator {
    /// Initialize a new BitmapFrameAllocator from the passed memory map.
    /// 
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid and that the physical memory offset is correct.
    pub unsafe fn init(memory_map: &'static MemoryRegions, physical_memory_offset: VirtAddr) -> Self {
        // 1. Find max physical address to determine bitmap size
        let max_addr = memory_map.iter().map(|r| r.end).max().unwrap_or(0);
        let max_frame = (max_addr / 4096) as usize;
        let bitmap_size = (max_frame + 7) / 8;

        // 2. Find a usable memory region large enough to store the bitmap
        // We look for a region that isn't at the very beginning of memory to avoid BIOS/bootloader structures
        let bitmap_region = memory_map.iter()
            .find(|r| r.kind == MemoryRegionKind::Usable && (r.end - r.start) as usize >= bitmap_size)
            .expect("Memory: No usable region found for physical memory bitmap");

        let bitmap_phys_addr = PhysAddr::new(bitmap_region.start);
        let bitmap_virt_addr = physical_memory_offset + bitmap_phys_addr.as_u64();
        let bitmap_ptr = bitmap_virt_addr.as_mut_ptr::<u8>();
        
        // Initialize the bitmap slice
        let bitmap = core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size);
        
        // Start with all frames marked as "used" (1)
        bitmap.fill(0xFF);

        let mut allocator = BitmapFrameAllocator {
            bitmap,
            max_frame,
            last_allocated: 0,
        };

        // 3. Mark usable frames as "free" (0)
        for region in memory_map.iter().filter(|r| r.kind == MemoryRegionKind::Usable) {
            for addr in (region.start..region.end).step_by(4096) {
                allocator.free_frame_internal(PhysFrame::containing_address(PhysAddr::new(addr)));
            }
        }

        // 4. Mark the frames used by the bitmap itself as "used"
        let bitmap_start_frame = PhysFrame::containing_address(bitmap_phys_addr);
        let bitmap_end_frame = PhysFrame::containing_address(PhysAddr::new(bitmap_phys_addr.as_u64() + bitmap_size as u64 - 1));
        for frame in PhysFrame::range_inclusive(bitmap_start_frame, bitmap_end_frame) {
            allocator.mark_used_internal(frame);
        }

        allocator
    }

    fn free_frame_internal(&mut self, frame: PhysFrame) {
        let frame_idx = (frame.start_address().as_u64() / 4096) as usize;
        if frame_idx < self.max_frame {
            self.bitmap[frame_idx / 8] &= !(1 << (frame_idx % 8));
        }
    }

    fn mark_used_internal(&mut self, frame: PhysFrame) {
        let frame_idx = (frame.start_address().as_u64() / 4096) as usize;
        if frame_idx < self.max_frame {
            self.bitmap[frame_idx / 8] |= 1 << (frame_idx % 8);
        }
    }

    /// Allocates multiple contiguous frames.
    pub fn allocate_contiguous(&mut self, num_frames: usize) -> Option<PhysFrame> {
        for i in 0..self.max_frame {
            let start_idx = (self.last_allocated + i) % self.max_frame;
            
            // Check if there are enough contiguous bits
            if start_idx + num_frames > self.max_frame { continue; }

            let mut found = true;
            for j in 0..num_frames {
                let idx = start_idx + j;
                if self.bitmap[idx / 8] & (1 << (idx % 8)) != 0 {
                    found = false;
                    break;
                }
            }

            if found {
                // Mark all as used
                for j in 0..num_frames {
                    let idx = start_idx + j;
                    self.bitmap[idx / 8] |= 1 << (idx % 8);
                }
                self.last_allocated = start_idx + num_frames;
                return Some(PhysFrame::containing_address(PhysAddr::new((start_idx as u64) * 4096)));
            }
        }
        None
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Linear search for a free bit, starting from last_allocated for better distribution
        for i in 0..self.max_frame {
            let idx = (self.last_allocated + i) % self.max_frame;
            let byte_idx = idx / 8;
            let bit_idx = idx % 8;

            if self.bitmap[byte_idx] & (1 << bit_idx) == 0 {
                // Mark as used
                self.bitmap[byte_idx] |= 1 << bit_idx;
                self.last_allocated = idx;
                
                let phys_addr = PhysAddr::new((idx as u64) * 4096);
                return Some(PhysFrame::containing_address(phys_addr));
            }
        }
        None
    }
}
