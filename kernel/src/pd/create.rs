use crate::capability::ProtectionDomain;
use crate::ipc::DOMAIN_REGISTRY;
use alloc::sync::Arc;
use crate::loader::elf::ElfLoader;
use crate::serial_println;
use crate::memory::allocator::GLOBAL_ALLOCATOR;
use crate::memory::pku;
use crate::capabilities::engine::CapEngine;

/// create_protection_domain: High-level PD lifecycle management.
pub fn create_protection_domain(elf_path: &str) -> Result<u32, &'static str> {
    serial_println!("pd: Creating domain for {}...", elf_path);

    // 1. Allocate a unique PD ID and PKU key
    let pd_id = 4000 + (x86_64::instructions::random::rdseed().unwrap_or(0) as u32 % 1000);
    let pku_key = (pd_id % 15) as u8 + 1;
    let new_pd = Arc::new(ProtectionDomain::new(pd_id, pku_key));

    // 2. Assign PKU domain key (Hardware isolation)
    let pkru_mask = pku::init_pd_pkru(pku_key);
    new_pd.current_pkru_mask.store(pkru_mask, core::sync::atomic::Ordering::Release);

    // 3. Load ELF via PDX to sexvfs
    let entry = ElfLoader::load_elf(elf_path, pku_key)?;

    // 4. Mint initial root capabilities (Signal, VFS, etc.) in RCU Table
    CapEngine::grant_initial_rights(&new_pd);

    // 5. Register with Registry (Lock-free insertion)
    DOMAIN_REGISTRY.insert(pd_id, new_pd.clone());

    // 6. Create main execution Task
    let stack_top = 0x_7000_0000_0000;
    let task = Box::into_raw(Box::new(crate::scheduler::Task::new(
        pd_id, entry.as_u64(), stack_top, new_pd.clone(), true
    )));
    crate::scheduler::SCHEDULERS[0].runqueue.enqueue(task);

    // 7. Create Dedicated Signal Trampoline Task (Phase 6 Polish)
    // Dedicated stack for signals to prevent kernel stack touch
    let trampoline_stack_top = 0x_7000_1000_0000; 
    let trampoline_task = Box::into_raw(Box::new(crate::scheduler::Task::new(
        pd_id | 0x8000_0000, 0 /* sexc_trampoline_handler entry */, trampoline_stack_top, new_pd.clone(), true
    )));
    crate::scheduler::SCHEDULERS[0].runqueue.enqueue(trampoline_task);

    serial_println!("pd: Spawning PD {} (PKU Key {}) -> entry {:#x}", pd_id, pku_key, entry.as_u64());
    Ok(pd_id)
}
