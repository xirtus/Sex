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
fn test_sexfiles_io_pdx() {
    serial_println!("test: Verifying standalone sexfiles I/O (PDX path)...");

    // 1. Allocate 4KiB buffer for I/O
    let buffer = sex_kernel::memory::allocator::alloc_frame().expect("Test: buffer OOM");

    // 2. Perform sys_write via kernel bridge
    // This will route via PDX to standalone sexfiles PD
    let write_res = sex_kernel::syscalls::fs::sys_write(1 /* stdout/file */, buffer, 4096);
    assert_eq!(write_res, 4096, "sys_write failed to return correct size");

    // 3. Perform sys_read via kernel bridge
    let read_res = sex_kernel::syscalls::fs::sys_read(1, buffer, 4096);
    assert_eq!(read_res, 4096, "sys_read failed to return correct size");

    serial_println!("test: sexfiles PDX I/O SUCCESS.");
}
