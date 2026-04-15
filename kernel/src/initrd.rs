use crate::serial_println;
use crate::memory::GlobalVas;
use crate::capability::ProtectionDomain;
use crate::ipc::DOMAIN_REGISTRY;
use crate::scheduler::{Task, TaskContext, TaskState, balanced_spawn};
use alloc::sync::Arc;
use x86_64::VirtAddr;

const MAGIC: &[u8; 8] = b"SEXPAC01";

#[repr(C, packed)]
struct PackHeader {
    magic: [u8; 8],
    name: [u8; 32],
    size: u64,
    hash: [u8; 32],
}

pub fn bootstrap_initrd(ramdisk_addr: VirtAddr, ramdisk_len: u64, vas: &mut GlobalVas) -> Result<(), &'static str> {
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
        let data_ptr = (ramdisk_addr.as_u64() + offset + 80) as *const u8;
        let data = unsafe { core::slice::from_raw_parts(data_ptr, size as usize) };
        
        serial_println!("INITRD: Found PD artifact '{}' ({} bytes)", name, size);
        
        // Handle specific critical PDs
        if name == "sexit" || name == "sext" {
            spawn_bundled_pd(name, data, vas)?;
        }
        
        // Move to next entry (aligned to 4KB as per sexpac.py)
        offset += 80 + size;
        offset = (offset + 4095) & !4095;
    }
    
    Ok(())
}

fn spawn_bundled_pd(name: &str, elf_data: &[u8], vas: &mut GlobalVas) -> Result<(), &'static str> {
    // 1. Assign static PD IDs and Keys for infrastructure
    let (pd_id, pku_key) = match name {
        "sexit" => (1, 1),
        "sext" => (600, 2),
        _ => (2000, 10),
    };
    
    serial_println!("INITRD: Bootstrapping {} (PD {}, Key {})", name, pd_id, pku_key);
    
    // 2. Create Protection Domain
    let pd = Arc::new(ProtectionDomain::new(pd_id, pku_key));
    DOMAIN_REGISTRY.write().insert(pd.id, pd.clone());
    crate::servers::sexc::init_signal_trampoline(pd.id);
    
    // 3. Load ELF into Global SAS
    let entry_point = crate::elf::load_elf_for_pd(elf_data, vas, pku_key)?;
    
    // 4. Create Task and Spawn
    let stack_top = 0x_7000_0000_0000 + (pd_id as u64 * 0x1000_0000);
    
    // Map stack
    vas.map_pku_range(
        VirtAddr::new(stack_top - 64*1024),
        64*1024,
        x86_64::structures::paging::PageTableFlags::PRESENT |
        x86_64::structures::paging::PageTableFlags::WRITABLE |
        x86_64::structures::paging::PageTableFlags::USER_ACCESSIBLE,
        pku_key
    )?;
    
    let signal_ring = pd.signal_ring.clone();
    let task = Task {
        id: pd_id,
        context: TaskContext::new(entry_point.as_u64(), stack_top, pd, true),
        state: TaskState::Ready,
        signal_ring,
    };
    
    balanced_spawn(task);
    Ok(())
}
