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

pub fn load_elf_for_pd(elf_data: &[u8], vas: &mut GlobalVas, pku_key: u8, load_base: VirtAddr) -> Result<VirtAddr, &'static str> {
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
            let segment_vaddr = load_base + ph.p_vaddr;
            serial_println!("ELF: Loading segment: vaddr={:#x}, memsz={:#x} (Key: {})", 
                segment_vaddr.as_u64(), ph.p_memsz, pku_key);

            let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
            
            serial_println!("   → Mapping range...");
            vas.map_pku_range(segment_vaddr, ph.p_memsz, flags, pku_key)?;
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

            if (ph.p_flags & PF_W) == 0 {
                 serial_println!("   → Setting to Read-Only...");
            }
        }
    }

    Ok(load_base + header.entry)
}
