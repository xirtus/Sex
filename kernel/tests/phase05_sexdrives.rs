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
fn test_sexdrives_zero_copy_dma() {
    serial_println!("test: Verifying Hardware & sexdrives (Zero-Copy DMA)...");
    
    // 1. Resolve Device Capability (Simulated for test)
    let device_cap = 500; 

    // 2. Allocate 4KiB DMA buffer from buddy
    let buffer = sex_kernel::memory::allocator::alloc_frame().expect("Test: buffer OOM");
    
    // 3. Perform sys_storage_write via kernel bridge
    // This routes to standalone sexdrives PD using lent-memory cap
    let write_res = sex_kernel::syscalls::storage::sys_storage_write(device_cap, 0, 4096, buffer);
    assert_eq!(write_res, 0, "sys_storage_write failed");
    
    // 4. Perform sys_storage_read via kernel bridge
    let read_res = sex_kernel::syscalls::storage::sys_storage_read(device_cap, 0, 4096, buffer);
    assert_eq!(read_res, 0, "sys_storage_read failed");
    
    serial_println!("test: sexdrives Zero-Copy DMA SUCCESS.");
}
