use crate::capability::ProtectionDomain;
use crate::ipc::DOMAIN_REGISTRY;
use alloc::sync::Arc;
use crate::loader::elf::ElfLoader;
use crate::serial_println;
use crate::memory::allocator::GLOBAL_ALLOCATOR;
use crate::memory::pku;

/// create_protection_domain: High-level PD lifecycle management.
/// Phase 7: Now backed by lock-free buddy and PKU initialization.
pub fn create_protection_domain(elf_path: &str) -> Result<u32, &'static str> {
    serial_println!("pd: Creating domain for {}...", elf_path);

    // 1. Allocate a unique PD ID and PKU key
    let pd_id = 4000 + (x86_64::instructions::random::rdseed().unwrap_or(0) as u32 % 1000);
    let pku_key = (pd_id % 15) as u8 + 1;
    let new_pd = Arc::new(ProtectionDomain::new(pd_id, pku_key));

    // 2. Allocate initial memory from lock-free buddy
    // For this prototype, we allocate 16 KiB (order 2) for stack + text
    let initial_phys = GLOBAL_ALLOCATOR.alloc(2).ok_or("pd: OOM during spawn")?;
    
    // 3. PD PKU Initialization
    let pkru_mask = pku::init_pd_pkru(pku_key);
    new_pd.current_pkru_mask.store(pkru_mask, core::sync::atomic::Ordering::Release);

    // 4. Load ELF (Normally via sexvfs, here using simulated buffer)
    let binary_data = [0u8; 1024]; 
    let entry = ElfLoader::load_elf(&binary_data, pku_key)?;

    // 5. Register with registry (Lock-free insertion)
    DOMAIN_REGISTRY.insert(pd_id, new_pd.clone());

    // 6. Create initial Task and add to scheduler
    let stack_top = 0x_7000_0000_0000 + (pd_id as u64 * 0x1000_0000);
    let task = Box::into_raw(Box::new(crate::scheduler::Task::new(
        pd_id, entry.as_u64(), stack_top, new_pd.clone(), true
    )));
    
    crate::scheduler::SCHEDULERS[0].runqueue.enqueue(task);

    serial_println!("pd: Spawning PD {} (PKU Key {}) at Phys {:#x}", pd_id, pku_key, initial_phys);
    Ok(pd_id)
}
