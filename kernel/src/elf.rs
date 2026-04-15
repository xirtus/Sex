use crate::memory::GlobalVas;
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

pub fn load_elf_for_pd(elf_data: &[u8], vas: &mut GlobalVas, pku_key: u8) -> Result<VirtAddr, &'static str> {
    // 1. Validate ELF magic
    let header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };
    if header.magic != [0x7f, b'E', b'L', b'F'] {
        return Err("ELF: Invalid magic number");
    }

    serial_println!("ELF: Valid header. Entry point: {:#x}", header.entry);

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
            serial_println!("ELF: Loading segment: vaddr={:#x}, memsz={:#x} (Key: {})", 
                ph.p_vaddr, ph.p_memsz, pku_key);

            // 3. Determine PageTableFlags
            let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
            if (ph.p_flags & PF_W) != 0 {
                flags |= PageTableFlags::WRITABLE;
            }
            if (ph.p_flags & PF_X) == 0 {
                // flags |= PageTableFlags::NO_EXECUTE; 
                // Note: On x86_64, NX bit is bit 63 of the page table entry.
            }

            // 4. Map the segment into the Global VAS with the PD's PKU key
            let vaddr = VirtAddr::new(ph.p_vaddr);
            vas.map_pku_range(vaddr, ph.p_memsz, flags, pku_key)?;

            // 5. Copy the segment data
            // In a SASOS, we copy directly into the virtual address.
            let dest = vaddr.as_mut_ptr::<u8>();
            let src_offset = ph.p_offset as usize;
            let src = &elf_data[src_offset..src_offset + ph.p_filesz as usize];
            
            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dest, ph.p_filesz as usize);
                // Zero out the remaining memsz (bss)
                if ph.p_memsz > ph.p_filesz {
                    core::ptr::write_bytes(dest.add(ph.p_filesz as usize), 0, (ph.p_memsz - ph.p_filesz) as usize);
                }
            }
        }
    }

    Ok(VirtAddr::new(header.entry))
}
