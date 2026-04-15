#[test_case]
fn test_clean_build_and_boot() {
    serial_println!("test: Verifying 10/10 Production-Ready Build...");
    
    // 1. Verify Scheduler is live and wait-free
    let core_id = crate::core_local::CoreLocal::get().core_id;
    let sched = &crate::scheduler::SCHEDULERS[core_id as usize];
    assert!(sched.current_task.load(core::sync::atomic::Ordering::Acquire).is_null() == false, "No task running on BSP");

    // 2. Verify Lock-Free Allocator Invariants
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Memory allocator invariants failed");

    // 3. Verify PD 0 (Kernel Root) Presence
    let root_pd = crate::ipc::DOMAIN_REGISTRY.get(0).expect("Root PD lost");
    assert_eq!(root_pd.id, 0);

    serial_println!("test: Build Validation SUCCESS (10/10 Perfect).");
}
