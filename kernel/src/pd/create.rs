use crate::capability::ProtectionDomain;
use crate::ipc::DOMAIN_REGISTRY;
use alloc::sync::Arc;
use crate::loader::elf::ElfLoader;
use crate::serial_println;

/// create_protection_domain: High-level PD lifecycle management.
pub fn create_protection_domain(elf_path: &str) -> Result<u32, &'static str> {
    serial_println!("pd: Creating domain for {}...", elf_path);

    // 1. Allocate a unique PD ID and PKU key
    let pd_id = 4000 + (x86_64::instructions::random::rdseed().unwrap_or(0) as u32 % 1000);
    let pku_key = (pd_id % 15) as u8 + 1;
    let new_pd = Arc::new(ProtectionDomain::new(pd_id, pku_key));

    // 2. Load ELF (Simplified for Phase 8, normally via sexvfs PDX)
    // We assume the binary is already in a buffer for this prototype
    let binary_data = [0u8; 1024]; // Placeholder
    let entry = ElfLoader::load_elf(&binary_data, pku_key)?;

    // 3. PD Init (Control ring, signal ring, trampoline)
    crate::servers::sexc::pd_init(new_pd.clone());

    // 4. Register root capabilities
    crate::capabilities::engine::CapEngine::grant_initial_rights(&new_pd);

    // 5. Insert into registry
    DOMAIN_REGISTRY.write().insert(pd_id, new_pd.clone());

    // 6. Create initial Task and add to scheduler
    let stack_top = 0x_7000_0000_0000 + (pd_id as u64 * 0x1000_0000);
    let task = crate::scheduler::Task {
        id: pd_id,
        context: crate::scheduler::TaskContext::new(entry.as_u64(), stack_top, new_pd, true),
        state: crate::scheduler::TaskState::Ready,
        signal_ring: Arc::new(crate::ipc_ring::RingBuffer::new()),
    };
    crate::scheduler::balanced_spawn(task);

    Ok(pd_id)
}
