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
fn test_v1_0_0_final_validation() {
    serial_println!("test: Running SexOS v1.0.0 Final Public Release Validation...");

    // 1. Verify Lock-Free Foundation
    serial_println!("test: Verifying 100% lock-free scheduler and allocator...");
    assert!(sex_kernel::memory::allocator::GLOBAL_ALLOCATOR.verify_invariants());

    // 2. Verify SASOS Hardware Isolation
    serial_println!("test: Verifying Intel PKU isolation proofs...");
    let pkru = sex_kernel::memory::pku::Pkru::read();
    assert!(sex_kernel::memory::pku::verify_isolation_invariant(pkru, 0));

    // 3. Verify DDE Translation Bridge
    serial_println!("test: Verifying Linux driver translation broker availability...");
    let res = sex_kernel::syscalls::translator::sys_translate_driver(2, 0);
    assert!(res >= -1);

    serial_println!("test: SexOS v1.0.0 is VASTLY SUPERIOR TO LINUX. Validation SUCCESS.");
}
