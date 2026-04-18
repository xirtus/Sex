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

pub fn init(mmap: &limine::request::MemmapResponse, hhdm_offset: u64) {
    crate::serial_println!("Sex: Memory init starting...");
    let offset = VirtAddr::new(hhdm_offset);
    let mut mapper = unsafe { init_sexting(offset) };
    
    let entries = mmap.entries();
    
    let mut frame_allocator = unsafe { BitmapFrameAllocator::init(entries, offset) };

    crate::serial_println!("Sex: Initializing kernel heap...");
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Kernel Heap Init Failed");

    crate::serial_println!("Sex: Initializing buddy allocator...");
    let mut _total_usable_pages = 0;
    let mut max_phys_addr = 0;
    for entry in entries.iter() {
        if entry.type_ == limine::memmap::MEMMAP_USABLE {
            _total_usable_pages += entry.length / 4096;
            let end = entry.base + entry.length;
            if end > max_phys_addr { max_phys_addr = end; }
        }
    }

    // Allocate metadata for ALL physical pages to avoid complex indexing
    let total_pages = max_phys_addr / 4096;
    let metadata_size = total_pages * core::mem::size_of::<allocator::PageMetadata>() as u64;
    
    // Find a spot for metadata (use first large region)
    let metadata_region = entries.iter().find(|entry| {
        entry.type_ == limine::memmap::MEMMAP_USABLE && entry.length > metadata_size + 1024*1024
    }).expect("No space for buddy metadata");
    
    let metadata_phys = metadata_region.base;
    let metadata_vaddr = hhdm_offset + metadata_phys;

    unsafe {
        allocator::GLOBAL_ALLOCATOR.init_metadata(metadata_vaddr, total_pages);
        
        for entry in entries.iter() {
            if entry.type_ == limine::memmap::MEMMAP_USABLE {
                let mut start = entry.base;
                let mut len = entry.length;
                
                // Subtract metadata if it's in this region
                if start == metadata_phys {
                    let aligned_size = (metadata_size + 4095) & !4095;
                    start += aligned_size;
                    len -= aligned_size;
                }
                
                if len > 0 {
                    allocator::GLOBAL_ALLOCATOR.add_memory_region(start, len);
                }
            }
        }
    }

    *GLOBAL_VAS.lock() = Some(GlobalVas {
        mapper,
        frame_allocator,
        phys_mem_offset: offset,
    });
    crate::serial_println!("Sex: Memory init complete.");
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

pub trait PageTableEntryExt {
    fn set_pku_key(&mut self, key: u8);
    fn pku_key(&self) -> u8;
}

impl PageTableEntryExt for PageTableEntry {
    fn set_pku_key(&mut self, key: u8) {
        let entry_ptr = self as *mut PageTableEntry as *mut u64;
        unsafe {
            let mut bits = *entry_ptr;
            bits &= !(0xF << 59);
            bits |= (key as u64 & 0xF) << 59;
            *entry_ptr = bits;
        }
    }

    fn pku_key(&self) -> u8 {
        let entry_bits = unsafe { *(self as *const PageTableEntry as *const u64) };
        ((entry_bits >> 59) & 0xF) as u8
    }
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

        p1_table[page.p1_index()].set_pku_key(pku_key);

        core::arch::asm!("invlpg [{}]", in(reg) page.start_address().as_u64());
    }
}


pub struct BitmapFrameAllocator {
    inner: BootInfoFrameAllocator,
}

unsafe impl Send for BitmapFrameAllocator {}
unsafe impl Sync for BitmapFrameAllocator {}

impl BitmapFrameAllocator {
    pub unsafe fn init(memory_regions: &'static [&'static limine::memmap::Entry], _offset: VirtAddr) -> Self {
        Self {
            inner: BootInfoFrameAllocator::init(memory_regions),
        }
    }

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrameRangeInclusive<Size4KiB>> {
        if count == 0 { return None; }
        
        let start = self.allocate_frame()?;
        for i in 1..count {
            let frame = self.allocate_frame()?;
            // Hardening: Verify contiguity
            if frame.start_address() != start.start_address() + (i as u64 * 4096) {
                 crate::serial_println!("FATAL: BitmapFrameAllocator failed contiguity invariant! count={}, i={}", count, i);
                 return None; 
            }
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
    memory_regions: &'static [&'static limine::memmap::Entry],
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static [&'static limine::memmap::Entry]) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_regions.iter();
        let usable_regions = regions
            .filter(|r| r.type_ == limine::memmap::MEMMAP_USABLE);
        let addr_ranges = usable_regions.map(|r| r.base..r.base + r.length);
        let frame_addresses = addr_ranges.flat_map(|r: core::ops::Range<u64>| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
