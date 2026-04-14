use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB, Page, PageTableFlags},
    PhysAddr, VirtAddr,
};
use spin::Mutex;

/// Initialize a new OffsetPageTable.
pub unsafe fn init_paging(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
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

/// The Global Virtual Address Space Manager.
/// In SASOS, there is only one of these for the entire system.
pub struct GlobalVas {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}

impl GlobalVas {
    /// Maps a virtual address range to physical frames.
    /// This is a Phase 1 primitive.
    pub fn map_range(&mut self, start: VirtAddr, size: u64, flags: PageTableFlags) -> Result<(), &'static str> {
        let page_range = {
            let start_page = Page::containing_address(start);
            let end_page = Page::containing_address(start + size - 1u64);
            Page::range_inclusive(start_page, end_page)
        };

        for page in page_range {
            let frame = self.frame_allocator.allocate_frame()
                .ok_or("VAS: Frame allocation failed")?;
            unsafe {
                self.mapper.map_to(page, frame, flags, &mut self.frame_allocator)
                    .map_err(|_| "VAS: Mapping failed")?
                    .flush();
            }
        }
        Ok(())
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
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
