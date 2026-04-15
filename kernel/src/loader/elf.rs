use crate::memory::allocator::GLOBAL_ALLOCATOR;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};
use crate::serial_println;

pub struct ElfLoader;

impl ElfLoader {
    /// Loads a 64-bit ELF from a buffer into the Global SAS.
    pub fn load_elf(data: &[u8], pku_key: u8) -> Result<VirtAddr, &'static str> {
        if data.len() < 64 || &data[0..4] != b"\x7fELF" {
            return Err("loader: invalid ELF header");
        }

        let entry = unsafe { *(data.as_ptr().add(24) as *const u64) };
        let phoff = unsafe { *(data.as_ptr().add(32) as *const u64) };
        let phnum = unsafe { *(data.as_ptr().add(56) as *const u16) };

        serial_println!("loader: Loading ELF with {} segments, entry at {:#x}", phnum, entry);

        for i in 0..phnum {
            let offset = phoff as usize + (i as usize * 56);
            let p_type = unsafe { *(data.as_ptr().add(offset) as *const u32) };
            
            if p_type == 1 { // PT_LOAD
                let p_offset = unsafe { *(data.as_ptr().add(offset + 8) as *const u64) };
                let p_vaddr = unsafe { *(data.as_ptr().add(offset + 16) as *const u64) };
                let p_filesz = unsafe { *(data.as_ptr().add(offset + 32) as *const u64) };
                let p_memsz = unsafe { *(data.as_ptr().add(offset + 40) as *const u64) };
                let p_flags = unsafe { *(data.as_ptr().add(offset + 4) as *const u32) };

                let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
                if p_flags & 2 != 0 { flags |= PageTableFlags::WRITABLE; }
                if p_flags & 1 == 0 { flags |= PageTableFlags::NO_EXECUTE; }

                let mut gvas = crate::memory::GLOBAL_VAS.lock();
                if let Some(ref mut vas) = *gvas {
                    let mut allocator = GLOBAL_ALLOCATOR.lock();
                    for j in (0..p_memsz).step_by(4096) {
                        if let Some(_phys) = allocator.alloc(0) {
                            vas.map_pku_range(VirtAddr::new(p_vaddr + j), 4096, flags, pku_key)?;
                        }
                    }
                    
                    // Copy segment data from buffer to SAS
                    unsafe {
                        let src = data.as_ptr().add(p_offset as usize);
                        let dst = p_vaddr as *mut u8;
                        core::ptr::copy_nonoverlapping(src, dst, p_filesz as usize);
                        
                        // Zero out BSS
                        if p_memsz > p_filesz {
                            core::ptr::write_bytes(dst.add(p_filesz as usize), 0, (p_memsz - p_filesz) as usize);
                        }
                    }
                }
            }
        }
        Ok(VirtAddr::new(entry))
    }
}
