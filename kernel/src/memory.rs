use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB, Page, PageTableFlags, frame::PhysFrameRangeInclusive, page_table::PageTableEntry},
    PhysAddr, VirtAddr,
};
use spin::Mutex;
use lazy_static::lazy_static;

pub mod allocator;
pub mod pku;

/// The Global Virtual Address Space container.
pub struct GlobalVas {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BitmapFrameAllocator,
    pub phys_mem_offset: VirtAddr,
}

unsafe impl Send for GlobalVas {}
unsafe impl Sync for GlobalVas {}

impl GlobalVas {
    pub fn map_pku_range(&mut self, vaddr: VirtAddr, size: u64, flags: PageTableFlags, pku_key: u8) -> Result<(), &'static str> {
        let page_range = {
            let start_page = Page::containing_address(vaddr);
            let end_page = Page::containing_address(vaddr + size - 1u64);
            Page::range_inclusive(start_page, end_page)
        };

        for page in page_range {
            let frame = self.frame_allocator.allocate_frame().ok_or("OOM")?;
            unsafe {
                self.mapper.map_to(page, frame, flags, &mut self.frame_allocator).map_err(|_| "Map failed")?.flush();
                update_page_pkey(page, pku_key, self.phys_mem_offset);
            }
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref GLOBAL_VAS: Mutex<Option<GlobalVas>> = Mutex::new(None);
}

/// Initialize a new OffsetPageTable.
pub unsafe fn init_sexting(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub fn update_page_pkey(page: Page, pku_key: u8, physical_memory_offset: VirtAddr) {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        let p4_entry = &level_4_table[page.p4_index()];
        let p3_table_phys = p4_entry.frame().unwrap().start_address();
        let p3_table: &mut PageTable = &mut *((physical_memory_offset + p3_table_phys.as_u64()).as_mut_ptr());
        let p3_entry = &p3_table[page.p3_index()];
        let p2_table_phys = p3_entry.frame().unwrap().start_address();
        let p2_table: &mut PageTable = &mut *((physical_memory_offset + p2_table_phys.as_u64()).as_mut_ptr());
        let p2_entry = &p2_table[page.p2_index()];
        let p1_table_phys = p2_entry.frame().unwrap().start_address();
        let p1_table: &mut PageTable = &mut *((physical_memory_offset + p1_table_phys.as_u64()).as_mut_ptr());
        let entry = &mut p1_table[page.p1_index()];
        
        let mut entry_bits = *(entry as *const _ as *const u64);
        entry_bits &= !(0xF << 59);
        entry_bits |= (pku_key as u64 & 0xF) << 59;
        
        *(entry as *mut _ as *mut u64) = entry_bits;
        core::arch::asm!("invlpg [{}]", in(reg) page.start_address().as_u64());
    }
}

pub struct BitmapFrameAllocator {
    inner: BootInfoFrameAllocator,
}

unsafe impl Send for BitmapFrameAllocator {}
unsafe impl Sync for BitmapFrameAllocator {}

impl BitmapFrameAllocator {
    pub unsafe fn init(memory_regions: &'static MemoryRegions, _offset: VirtAddr) -> Self {
        Self {
            inner: BootInfoFrameAllocator::init(memory_regions),
        }
    }

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrameRangeInclusive<Size4KiB>> {
        let start = self.allocate_frame()?;
        for _ in 1..count {
            self.allocate_frame()?;
        }
        let end = PhysFrame::containing_address(start.start_address() + (count as u64 - 1) * 4096);
        Some(PhysFrame::range_inclusive(start, end))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.inner.allocate_frame()
    }
}

pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_regions.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub struct DummyFrameAllocator {
    pub last_allocated: usize,
}

impl DummyFrameAllocator {
    pub fn new() -> Self {
        Self { last_allocated: 0 }
    }
}

unsafe impl FrameAllocator<Size4KiB> for DummyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        for idx in (self.last_allocated + 1)..1000000 {
            if idx > 1024 {
                self.last_allocated = idx;
                let phys_addr = PhysAddr::new((idx as u64) * 4096);
                return Some(PhysFrame::containing_address(phys_addr));
            }
        }
        None
    }
}
