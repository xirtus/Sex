use crate::capability::ProtectionDomain;
use crate::ipc::DOMAIN_REGISTRY;
use crate::loader::elf::ElfLoader;
use crate::serial_println;
use crate::memory::allocator::GLOBAL_ALLOCATOR;
use crate::pku;
use crate::capabilities::engine::CapEngine;

/// create_protection_domain: Ruthless Phase 6/8/10 implementation.
pub fn create_protection_domain(elf_path: &str, requested_id: Option<u32>) -> Result<u32, &'static str> {
    serial_println!("pd: Creating domain for {}...", elf_path);

    // 1. Allocate a unique PD ID and PKU key
    let pd_id = if let Some(id) = requested_id {
        id
    } else {
        // Fallback for rdseed in this toolchain
        4001 // Simplified for prototype
    };
    let pku_key = (pd_id % 15) as u8 + 1;
    
    // Raw pointer for ProtectionDomain
    let new_pd = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(ProtectionDomain::new(pd_id, pku_key)));

    // 2. Assign PKU domain key (Hardware isolation)
    let pkru_mask = pku::init_pd_pkru(pku_key);
    unsafe { (*new_pd).current_pkru_mask.store(pkru_mask, core::sync::atomic::Ordering::Release); }

    // 3. Load ELF via PDX to sexvfs
    let entry = ElfLoader::load_elf(elf_path, pku_key)?;

    // 4. Register with Registry (Lock-free insertion)
    DOMAIN_REGISTRY.insert(pd_id, new_pd);
    let pd_ref = DOMAIN_REGISTRY.get(pd_id).unwrap();

    // 5. Mint initial root capabilities
    CapEngine::grant_initial_rights(pd_ref);

    // 6. Create main execution Task (Raw Pointer)
    let stack_top = 0x_7000_0000_0000;
    let task_ptr = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(crate::scheduler::Task::new(
        pd_id, entry.as_u64(), stack_top, pd_ref, true
    )));
    
    // Phase 13.2.1: Store main task for interrupt unparking
    unsafe { (*new_pd).main_task.store(task_ptr, core::sync::atomic::Ordering::Release); }
    
    crate::scheduler::SCHEDULERS[0].runqueue.enqueue(task_ptr);

    // 7. Create Dedicated Signal Trampoline Task
    let trampoline_stack_top = 0x_7000_1000_0000u64;
 
    let trampoline_task_ptr = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(crate::scheduler::Task::new(
        pd_id | 0x8000_0000, 0, trampoline_stack_top, pd_ref, true
    )));
    crate::scheduler::SCHEDULERS[0].runqueue.enqueue(trampoline_task_ptr);

    serial_println!("pd: PD {} Spawned (Trampoline task active).", pd_id);
    Ok(pd_id)
}
