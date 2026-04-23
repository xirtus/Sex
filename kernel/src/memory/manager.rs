use x86_64::{PhysAddr, VirtAddr, structures::paging::{PageTable, OffsetPageTable, PageTableFlags, FrameAllocator, Size4KiB, PhysFrame}};
use crate::pku;
use super::allocator;
use spin::Mutex;
use lazy_static::lazy_static;

pub struct BootInfoFrameAllocator {
    memory_map: &'static limine::request::MemmapResponse,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn new(memory_map: &'static limine::request::MemmapResponse) -> Self {
        BootInfoFrameAllocator { memory_map, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.entries();
        regions.iter()
            .filter(|r| r.type_ == 0) // 0 = LIMINE_MEMMAP_USABLE
            .map(|r| r.base..(r.base + r.length))
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub struct GlobalVas {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}

impl GlobalVas {
    pub fn map_pku_range(&mut self, va: VirtAddr, size: u64, flags: PageTableFlags, pkey: u8) -> Result<(), &'static str> {
        let start_page = x86_64::structures::paging::Page::<Size4KiB>::containing_address(va);
        let end_page = x86_64::structures::paging::Page::<Size4KiB>::containing_address(va + size - 1u64);
        
        for page in x86_64::structures::paging::Page::range_inclusive(start_page, end_page) {
            let frame = self.frame_allocator.allocate_frame().ok_or("Out of memory")?;
            unsafe {
                use x86_64::structures::paging::Mapper;
                self.mapper.map_to(page, frame, flags, &mut self.frame_allocator)
                    .map_err(|_| "Mapping failed")?.flush();
                pku::tag_virtual_address(page.start_address().as_u64(), pkey);
            }
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref GLOBAL_VAS: Mutex<Option<GlobalVas>> = Mutex::new(None);
}

pub fn init(memmap: &'static limine::request::MemmapResponse, hhdm_offset: u64) {
    let level_4_table = unsafe { active_level_4_table(VirtAddr::new(hhdm_offset)) };
    let mut mapper = unsafe { OffsetPageTable::new(level_4_table, VirtAddr::new(hhdm_offset)) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::new(memmap) };

    // Initialize the Rust heap (linked_list_allocator). Must happen before any Box/Vec.
    // frame_allocator.next advances past the heap pages, so GLOBAL_VAS gets a valid cursor.
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .unwrap_or_else(|_| {
            loop { core::hint::spin_loop(); }
        });

    // Seed GLOBAL_ALLOCATOR (LockFreeBuddyAllocator) with remaining usable physical regions.
    // The bump allocator consumed heap_pages frames sequentially from usable regions.
    // Track how many frames were consumed and skip them when feeding the buddy allocator
    // so physical frames used for the heap aren't double-allocated.
    let heap_pages = (crate::HEAP_SIZE as u64) / 4096;
    let mut consumed = 0u64;
    for entry in memmap.entries().iter().filter(|e| e.type_ == 0) {
        let region_pages = entry.length / 4096;
        if consumed >= heap_pages {
            // All heap pages already skipped; add this full region.
            unsafe { crate::memory::allocator::GLOBAL_ALLOCATOR.add_memory_region(entry.base, entry.length) };
        } else if consumed + region_pages > heap_pages {
            // Partial overlap: skip the consumed portion, add the rest.
            let skip_pages = heap_pages - consumed;
            let base = entry.base + skip_pages * 4096;
            let len  = entry.length - skip_pages * 4096;
            unsafe { crate::memory::allocator::GLOBAL_ALLOCATOR.add_memory_region(base, len) };
            consumed = heap_pages; // saturate
        } else {
            // Entire region consumed by heap mapping.
            consumed += region_pages;
        }
    }

    *GLOBAL_VAS.lock() = Some(GlobalVas { mapper, frame_allocator });

    // Phase 3: Carve out a hardcoded shared memory allocation at 0x4000_0000
    // Tag the physical pages for dual PKEY access (PKEY 1 and PKEY 3)
    if let Some(vas) = GLOBAL_VAS.lock().as_mut() {
        let va = x86_64::VirtAddr::new(0x4000_0000);
        let size = 1920 * 1080 * 4; // 8MB buffer
        let flags = x86_64::structures::paging::PageTableFlags::PRESENT 
                  | x86_64::structures::paging::PageTableFlags::WRITABLE 
                  | x86_64::structures::paging::PageTableFlags::USER_ACCESSIBLE;
        let _ = vas.map_pku_range(va, size as u64, flags, 0); // PKEY 0 allows dual access
    }
}

pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

pub fn update_page_pkey(va: VirtAddr, pkey: u8) {
    unsafe { pku::tag_virtual_address(va.as_u64(), pkey); }
}
