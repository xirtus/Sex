use crate::syscalls::spawn::sys_spawn_pd;
use crate::ipc::DOMAIN_REGISTRY;

#[test]
fn test_elf_pd_spawn() {
    // Phase 8: Verify ELF loading and PD spawning
    let path = "/bin/ash\0".as_ptr();
    let pd_id = sys_spawn_pd(path);
    
    assert!(pd_id >= 4000, "PD spawning failed");
    
    let registry = DOMAIN_REGISTRY.read();
    let pd = registry.get(&(pd_id as u32)).expect("PD not found in registry");
    
    // Check if trampoline is running (mocked by checking sexc_state)
    assert!(pd.sexc_state.lock().is_some(), "Trampoline state not initialized");
}
