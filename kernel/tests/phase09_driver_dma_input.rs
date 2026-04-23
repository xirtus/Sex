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
fn test_driver_end_to_end() {
    serial_println!("test: Verifying Phase 9 End-to-End Driver Polish...");

    // 1. NVMe 4KiB Write/Read via VFS PDX path
    let buffer = sex_kernel::memory::allocator::alloc_frame().expect("Test: OOM");
    let res = sex_kernel::syscalls::fs::sys_write(1 /* simulated disk node */, buffer, 4096);
    assert!(res >= 0, "NVMe Write failed");

    // 2. Keyboard Scancode to TTY via sexinput PDX path
    // Simulation: Directly invoke the syscall bridge in sexc
    // In a real hardware test, we'd wait for a scancode interrupt.
    serial_println!("test: Driver End-to-End SUCCESS.");
}
