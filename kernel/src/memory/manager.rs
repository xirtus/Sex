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

    pub fn map_physical_range(&mut self, va: VirtAddr, pa: u64, size: u64, flags: PageTableFlags, pkey: u8) -> Result<(), &'static str> {
        use x86_64::structures::paging::{Page, Mapper};
        let start_page = Page::<Size4KiB>::containing_address(va);
        let end_page   = Page::<Size4KiB>::containing_address(va + size - 1u64);
        let mut phys = pa & !0xFFF;
        for page in Page::range_inclusive(start_page, end_page) {
            let frame = PhysFrame::containing_address(PhysAddr::new(phys));
            unsafe {
                self.mapper.map_to(page, frame, flags, &mut self.frame_allocator)
                    .map_err(|_| "FB map failed")?.flush();
                pku::tag_virtual_address(page.start_address().as_u64(), pkey);
            }
            phys += 4096;
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref GLOBAL_VAS: Mutex<Option<GlobalVas>> = Mutex::new(None);
}

pub fn init(memmap: &'static limine::request::MemmapResponse, hhdm_offset: u64) {
    crate::serial_println!("allocator.init.begin");
    let level_4_table = unsafe { active_level_4_table(VirtAddr::new(hhdm_offset)) };
    let mut mapper = unsafe { OffsetPageTable::new(level_4_table, VirtAddr::new(hhdm_offset)) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::new(memmap) };

    // Initialize the Rust heap (linked_list_allocator). Must happen before any Box/Vec.
    // frame_allocator.next advances past the heap pages, so GLOBAL_VAS gets a valid cursor.
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .unwrap_or_else(|_| {
            loop { core::hint::spin_loop(); }
        });

    let entries = memmap.entries();
    crate::serial_println!("allocator.memory_regions.count={}", entries.len());

    // Size metadata against highest *usable* physical address only.
    // Including non-usable high regions can over-allocate metadata and prevent allocator bootstrap.
    let mut max_usable_phys_addr = 0u64;
    for entry in entries.iter().filter(|e| e.type_ == 0) {
        let end = entry.base + entry.length;
        if end > max_usable_phys_addr {
            max_usable_phys_addr = end;
        }
    }
    let total_pages = (max_usable_phys_addr + 4095) / 4096;
    let metadata_bytes = total_pages * core::mem::size_of::<crate::memory::allocator::PageMetadata>() as u64;
    let metadata_pages = (metadata_bytes + 4095) / 4096;

    // Seed GLOBAL_ALLOCATOR (LockFreeBuddyAllocator) with remaining usable physical regions.
    // The bump allocator consumed heap_pages frames sequentially from usable regions.
    // Track how many frames were consumed and skip them when feeding the buddy allocator
    // so physical frames used for the heap aren't double-allocated.
    let heap_pages = (crate::HEAP_SIZE as u64) / 4096;
    let mut consumed = 0u64;
    let mut metadata_allocated = false;

    for entry in entries.iter().filter(|e| e.type_ == 0) {
        let region_pages = entry.length / 4096;
        let mut region_base = entry.base;
        let mut region_len = entry.length;

        if consumed < heap_pages {
            if consumed + region_pages <= heap_pages {
                // Entire region consumed by heap mapping.
                consumed += region_pages;
                continue;
            } else {
                // Partial overlap: skip the consumed portion.
                let skip_pages = heap_pages - consumed;
                region_base += skip_pages * 4096;
                region_len -= skip_pages * 4096;
                consumed = heap_pages; // saturate
            }
        }

        // Carve metadata array from the first available usable region after heap
        if !metadata_allocated && region_len >= metadata_pages * 4096 {
            let metadata_phys = region_base;
            let metadata_vaddr = hhdm_offset + metadata_phys;
            unsafe {
                crate::memory::allocator::GLOBAL_ALLOCATOR.init_metadata(metadata_vaddr, total_pages);
            }
            region_base += metadata_pages * 4096;
            region_len -= metadata_pages * 4096;
            metadata_allocated = true;
        }

        if region_len > 0 {
            unsafe { crate::memory::allocator::GLOBAL_ALLOCATOR.add_memory_region(region_base, region_len) };
        }
    }

    if !metadata_allocated {
        crate::serial_println!(
            "allocator.init.error metadata_not_allocated metadata_pages={} total_pages={}",
            metadata_pages,
            total_pages
        );
    }
    let usable_frames_total = crate::memory::allocator::debug_global_free_frames();
    crate::serial_println!("allocator.usable_frames.total={}", usable_frames_total);
    crate::serial_println!("allocator.init.done");

    *GLOBAL_VAS.lock() = Some(GlobalVas { mapper, frame_allocator });
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

pub fn read_pte_flags(va: VirtAddr) -> Result<u64, &'static str> {
    use x86_64::registers::control::Cr3;
    let hhdm = crate::HHDM_REQUEST
        .response()
        .ok_or("HHDM missing")?
        .offset;

    let (cr3_frame, _) = Cr3::read();
    let pml4 = (cr3_frame.start_address().as_u64() + hhdm) as *const u64;

    let pml4_i = ((va.as_u64() >> 39) & 0x1ff) as usize;
    let pdpt_i = ((va.as_u64() >> 30) & 0x1ff) as usize;
    let pd_i = ((va.as_u64() >> 21) & 0x1ff) as usize;
    let pt_i = ((va.as_u64() >> 12) & 0x1ff) as usize;

    unsafe {
        let pml4e = *pml4.add(pml4_i);
        if (pml4e & 1) == 0 {
            return Err("pml4e not present");
        }
        let pdpt = ((pml4e & 0x000f_ffff_ffff_f000) + hhdm) as *const u64;
        let pdpte = *pdpt.add(pdpt_i);
        if (pdpte & 1) == 0 {
            return Err("pdpte not present");
        }
        if (pdpte & (1 << 7)) != 0 {
            return Ok(pdpte);
        }
        let pd = ((pdpte & 0x000f_ffff_ffff_f000) + hhdm) as *const u64;
        let pde = *pd.add(pd_i);
        if (pde & 1) == 0 {
            return Err("pde not present");
        }
        if (pde & (1 << 7)) != 0 {
            return Ok(pde);
        }
        let pt = ((pde & 0x000f_ffff_ffff_f000) + hhdm) as *const u64;
        let pte = *pt.add(pt_i);
        if (pte & 1) == 0 {
            return Err("pte not present");
        }
        Ok(pte)
    }
}

pub fn log_page_walk(va: VirtAddr, tag: &str) {
    let hhdm = match crate::HHDM_REQUEST.response() {
        Some(v) => v.offset,
        None => {
            crate::serial_println!("walk:{} va={:#x} hhdm=missing", tag, va.as_u64());
            return;
        }
    };

    use x86_64::registers::control::Cr3;
    let (cr3_frame, _) = Cr3::read();
    let pml4 = (cr3_frame.start_address().as_u64() + hhdm) as *const u64;

    let pml4_i = ((va.as_u64() >> 39) & 0x1ff) as usize;
    let pdpt_i = ((va.as_u64() >> 30) & 0x1ff) as usize;
    let pd_i = ((va.as_u64() >> 21) & 0x1ff) as usize;
    let pt_i = ((va.as_u64() >> 12) & 0x1ff) as usize;

    #[inline]
    fn bits(raw: u64) -> (bool, bool, bool, bool, u64) {
        let p = (raw & 1) != 0;
        let w = (raw & (1 << 1)) != 0;
        let u = (raw & (1 << 2)) != 0;
        let nx = (raw & (1u64 << 63)) != 0;
        let addr = raw & 0x000f_ffff_ffff_f000;
        (p, w, u, nx, addr)
    }

    unsafe {
        let pml4e = *pml4.add(pml4_i);
        let (p, w, u, nx, addr) = bits(pml4e);
        crate::serial_println!(
            "walk:{} PML4E raw={:#x} addr={:#x} p={} w={} u={} nx={}",
            tag, pml4e, addr, p, w, u, nx
        );
        if !p {
            return;
        }
        let pdpt = (addr + hhdm) as *const u64;
        let pdpe = *pdpt.add(pdpt_i);
        let (p, w, u, nx, addr) = bits(pdpe);
        crate::serial_println!(
            "walk:{} PDPE  raw={:#x} addr={:#x} p={} w={} u={} nx={}",
            tag, pdpe, addr, p, w, u, nx
        );
        if !p {
            return;
        }
        if (pdpe & (1 << 7)) != 0 {
            return;
        }
        let pd = (addr + hhdm) as *const u64;
        let pde = *pd.add(pd_i);
        let (p, w, u, nx, addr) = bits(pde);
        crate::serial_println!(
            "walk:{} PDE   raw={:#x} addr={:#x} p={} w={} u={} nx={}",
            tag, pde, addr, p, w, u, nx
        );
        if !p {
            return;
        }
        if (pde & (1 << 7)) != 0 {
            return;
        }
        let pt = (addr + hhdm) as *const u64;
        let pte = *pt.add(pt_i);
        let (p, w, u, nx, addr) = bits(pte);
        crate::serial_println!(
            "walk:{} PTE   raw={:#x} addr={:#x} p={} w={} u={} nx={}",
            tag, pte, addr, p, w, u, nx
        );
    }
}
