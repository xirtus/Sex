use crate::serial_println;
use crate::memory::GlobalVas;
use crate::pd::create::create_protection_domain;
use x86_64::VirtAddr;

const MAGIC: &[u8; 8] = b"SEXPAC01";

#[repr(C, packed)]
struct PackHeader {
    magic: [u8; 8],
    name: [u8; 32],
    size: u64,
    hash: [u8; 32],
}

pub fn bootstrap_initrd(ramdisk_addr: VirtAddr, ramdisk_len: u64, _vas: &mut GlobalVas) -> Result<(), &'static str> {
    serial_println!("INITRD: Locating SAS artifacts at {:?} (len: {})", ramdisk_addr, ramdisk_len);
    
    let mut offset = 0;
    while offset < ramdisk_len {
        let header_ptr = (ramdisk_addr.as_u64() + offset) as *const PackHeader;
        let header = unsafe { &*header_ptr };
        
        if &header.magic != MAGIC {
            break;
        }
        
        let name = core::str::from_utf8(&header.name).unwrap_or("unknown").trim_matches('\0');
        let size = header.size;
        
        serial_println!("INITRD: Found PD artifact '{}' ({} bytes)", name, size);
        
        // Handle specific critical PDs
        if name == "sexit" || name == "sext" {
            // Simplified for prototype: we already spawn these in init.rs
            // In a real system, we'd load them from here.
        }
        
        // Move to next entry (aligned to 4KB as per sexpac.py)
        offset += 80 + size;
        offset = (offset + 4095) & !4095;
    }
    
    Ok(())
}
