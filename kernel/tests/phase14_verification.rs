#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(sex_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use sex_kernel::serial_println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("TEST FAILED: {}", info);
    sex_kernel::exit_qemu(sex_kernel::QemuExitCode::Failed);
    loop {}
}

#[test_case]
fn test_phase14_hardened_invariants() {
    serial_println!("test: Starting Phase 14 Hardened Invariants Validation...");

    // 1. Verify Sharded Allocator Invariants
    serial_println!("test: Verifying sharded buddy allocator invariants...");
    assert!(sex_kernel::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants(), "Allocator invariant failure");

    // 2. Verify PKU Isolation Proofs
    serial_println!("test: Verifying PKU hardware isolation proofs...");
    let pkru = sex_kernel::memory::pku::Pkru::read();
    // Rule: PD 0 (Kernel) must always have access to its own key
    assert!(sex_kernel::memory::pku::verify_isolation_invariant(pkru, 0), "Kernel isolation invariant violated");

    // 3. Verify Capability Ownership tracking
    serial_println!("test: Verifying capability ownership invariants...");
    let current_pd = sex_kernel::core_local::CoreLocal::get().current_pd_ref();
    // VFS cap (id 1) must track its source
    assert!(current_pd.verify_ownership(1), "Capability ownership tracking lost");

    serial_println!("test: Phase 14 Hardening SUCCESS.");
}
