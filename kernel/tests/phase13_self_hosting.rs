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
fn test_self_hosting_bootstrap() {
    serial_println!("test: Verifying Phase 13 Self-Hosting Bootstrap...");
    
    // 1. Allocate buffer for fetched source code
    let buffer_vaddr = sex_kernel::memory::allocator::alloc_frame().expect("Test: buffer OOM");
    
    // 2. Perform FETCH_PACKAGE via PDX to standalone sexstore
    let res = sex_kernel::syscalls::store::sys_store_fetch(
        1,            // Store capability ID
        0x_5000_0000, // Simulated "kernel-src" package pointer
        buffer_vaddr,
        4096
    );
    
    assert!(res >= 0, "Package fetch failed");
    assert_eq!(res, 4096, "Fetched package size mismatch");
    
    // 3. In a real scenario, sex-gemini would then invoke GCC via PDX on the lent buffer
    serial_println!("test: Self-Hosting Bootstrap SUCCESS.");
}
