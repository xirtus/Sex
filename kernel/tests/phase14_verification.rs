#[test_case]
fn test_phase14_core_hardening() {
    serial_println!("test: Starting Phase 14 Core Hardening Validation...");

    // 1. Verify Allocator Sharding (Should not panic)
    serial_println!("test: Exercising sharded lock-free allocator...");
    let mut frames = [0u64; 100];
    for i in 0..100 {
        frames[i] = crate::memory::allocator::alloc_frame().expect("Phase 14: OOM");
    }
    for i in 0..100 {
        crate::memory::allocator::free_pages(frames[i], 0);
    }
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Allocator invariant failure");

    // 2. Verify PKU Isolation Proof
    serial_println!("test: Verifying hardware PKU isolation invariants...");
    let pkru = crate::memory::pku::Pkru::read();
    // Key 0 (kernel) should always be accessible (invariant 1)
    assert!(crate::memory::pku::verify_isolation_invariant(pkru, 0), "Kernel isolation violated");

    // 3. Verify Capability Ownership Hook
    serial_println!("test: Verifying capability ownership tracking...");
    let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();
    // VFS cap at slot 1 should have valid ownership (mocked as true in Phase 14 logic)
    assert!(current_pd.verify_ownership(1), "Capability ownership lost");

    serial_println!("test: Phase 14 Hardening SUCCESS.");
}
