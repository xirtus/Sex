use crate::memory::manager::GlobalVas;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};
use crate::serial_println;

/// Minimal ELF64 Header structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub osabi: u8,
    pub abiversion: u8,
    pub pad: [u8; 7],
    pub elf_type: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

/// Minimal ELF64 Program Header structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

fn load_bias(elf_data: &[u8]) -> Result<u64, &'static str> {
    let header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };
    let ph_start = header.phoff as usize;
    let ph_count = header.phnum as usize;
    let ph_size = header.phentsize as usize;
    let mut min_vaddr = u64::MAX;

    for i in 0..ph_count {
        let ph_ptr = unsafe {
            let offset = ph_start + (i * ph_size);
            elf_data.as_ptr().add(offset) as *const ProgramHeader
        };
        let ph = unsafe { &*ph_ptr };
        if ph.p_type == PT_LOAD && ph.p_memsz > 0 {
            if ph.p_vaddr < min_vaddr {
                min_vaddr = ph.p_vaddr;
            }
        }
    }

    if min_vaddr == u64::MAX {
        return Err("ELF: no PT_LOAD segments");
    }
    Ok(min_vaddr)
}

pub fn load_elf_for_pd(elf_data: &[u8], vas: &mut GlobalVas, pku_key: u8, load_base: VirtAddr) -> Result<VirtAddr, &'static str> {
    // 1. Validate ELF magic
    let header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };
    if header.magic != [0x7f, b'E', b'L', b'F'] {
        return Err("ELF: Invalid magic number");
    }

    let min_vaddr = load_bias(elf_data)?;
    serial_println!(
        "ELF: Valid header. Entry point: {:#x} min_vaddr={:#x}",
        header.entry,
        min_vaddr
    );

    // 2. Iterate through program headers
    let ph_start = header.phoff as usize;
    let ph_count = header.phnum as usize;
    let ph_size = header.phentsize as usize;

    for i in 0..ph_count {
        let ph_ptr = unsafe {
            let offset = ph_start + (i * ph_size);
            elf_data.as_ptr().add(offset) as *const ProgramHeader
        };
        let ph = unsafe { &*ph_ptr };

        if ph.p_type == PT_LOAD {
            let relocated = ph.p_vaddr.saturating_sub(min_vaddr);
            let segment_vaddr = load_base + relocated;
            serial_println!("ELF: Loading segment: vaddr={:#x}, memsz={:#x} (Key: {})", 
                segment_vaddr.as_u64(), ph.p_memsz, pku_key);

            // Stage-1 mapping for kernel copy path: supervisor writable.
            // Keep USER at stage-1 so upper-level page-table entries are user-capable.
            // x86 requires U/S permission on every level for CPL3 fetch/access.
            let map_flags =
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            // Final user-visible segment policy.
            let mut final_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
            if (ph.p_flags & PF_W) != 0 {
                final_flags |= PageTableFlags::WRITABLE;
            }
            if (ph.p_flags & PF_X) == 0 {
                final_flags |= PageTableFlags::NO_EXECUTE;
            }
            let page_count = (ph.p_memsz + 4095) / 4096;
            serial_println!(
                "ELF: PT_LOAD idx={} vstart={:#x} vend={:#x} filesz={:#x} memsz={:#x} flags=R{}W{}X{} pages={}",
                i,
                segment_vaddr.as_u64(),
                segment_vaddr.as_u64().saturating_add(ph.p_memsz),
                ph.p_filesz,
                ph.p_memsz,
                if (ph.p_flags & PF_R) != 0 { "+" } else { "-" },
                if (ph.p_flags & PF_W) != 0 { "+" } else { "-" },
                if (ph.p_flags & PF_X) != 0 { "+" } else { "-" },
                page_count
            );
            serial_println!(
                "loader.map relocated old_va={:#x} -> new_va={:#x} flags={:?}",
                ph.p_vaddr,
                segment_vaddr.as_u64(),
                final_flags
            );
            
            serial_println!("   → Mapping range...");
            vas.map_pku_range(segment_vaddr, ph.p_memsz, map_flags, pku_key)?;
            serial_println!("   → Mapping complete. Copying data to {:#x}...", segment_vaddr.as_u64());

            let dest = segment_vaddr.as_mut_ptr::<u8>();
            let src_offset = ph.p_offset as usize;
            let src = &elf_data[src_offset..src_offset + ph.p_filesz as usize];
            
            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dest, ph.p_filesz as usize);
                serial_println!("   → Copy complete.");
                if ph.p_memsz > ph.p_filesz {
                    serial_println!("   → Zeroing BSS...");
                    core::ptr::write_bytes(dest.add(ph.p_filesz as usize), 0, (ph.p_memsz - ph.p_filesz) as usize);
                    serial_println!("   → BSS zeroed.");
                }
            }

            {
                use x86_64::structures::paging::{Mapper, Page, Size4KiB};
                let start_page = Page::<Size4KiB>::containing_address(segment_vaddr);
                let end_page = Page::<Size4KiB>::containing_address(
                    segment_vaddr + ph.p_memsz.saturating_sub(1u64),
                );
                for page in Page::range_inclusive(start_page, end_page) {
                    unsafe {
                        vas.mapper
                            .update_flags(page, final_flags)
                            .map_err(|_| "ELF: update_flags failed")?
                            .flush();
                    }
                }
                serial_println!("   → Finalized segment flags={:?}", final_flags);
            }

            if segment_vaddr.as_u64() <= 0x4000_0000
                && 0x4000_0000 < segment_vaddr.as_u64().saturating_add(ph.p_memsz)
            {
                match crate::memory::manager::read_pte_flags(VirtAddr::new(0x4000_0000)) {
                    Ok(pte) => {
                        serial_println!("ELF: final PTE @0x40000000 = {:#x}", pte);
                    }
                    Err(e) => {
                        serial_println!("ELF: final PTE @0x40000000 unavailable: {}", e);
                    }
                }
            }
        }
    }

    let relocated_entry = header.entry.saturating_sub(min_vaddr);
    Ok(load_base + relocated_entry)
}
