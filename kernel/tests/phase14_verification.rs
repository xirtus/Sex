#[test_case]
fn test_phase14_hardened_invariants() {
    serial_println!("test: Starting Phase 14 Hardened Invariants Validation...");

    // 1. Verify Sharded Allocator Invariants
    serial_println!("test: Verifying sharded buddy allocator invariants...");
    assert!(crate::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Allocator invariant failure");

    // 2. Verify PKU Isolation Proofs
    serial_println!("test: Verifying PKU hardware isolation proofs...");
    let pkru = crate::memory::pku::Pkru::read();
    // Rule: PD 0 (Kernel) must always have access to its own key
    assert!(crate::memory::pku::verify_isolation_invariant(pkru, 0), "Kernel isolation invariant violated");

    // 3. Verify Capability Ownership tracking
    serial_println!("test: Verifying capability ownership invariants...");
    let current_pd = crate::core_local::CoreLocal::get().current_pd_ref();
    // VFS cap (id 1) must track its source
    assert!(current_pd.verify_ownership(1), "Capability ownership tracking lost");

    serial_println!("test: Phase 14 Hardening SUCCESS.");
}
