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
fn test_lockfree_buddy_and_async_pf() {
    serial_println!("test: Exercising Lock-Free Buddy Allocator...");

    // 1. Allocate 100 frames (order 0)
    let mut frames = [0u64; 100];
    for i in 0..100 {
        frames[i] = sex_kernel::memory::allocator::alloc_frame().expect("Buddy: Allocation failed");
    }

    // 2. Free them to verify no contention
    for i in 0..100 {
        sex_kernel::memory::allocator::free_pages(frames[i], 0);
    }
    serial_println!("test: Buddy Allocator SUCCESS.");

    // 3. Simulate Synthetic Page Fault
    serial_println!("test: Simulating Asynchronous Page Fault Forwarding...");
    let res = sex_kernel::ipc::pagefault::forward_page_fault(0x_DEAD_B000, 0x2, 4000);

    assert!(res.is_ok(), "Page fault forwarding failed");
    serial_println!("test: Async Page Fault SUCCESS.");
}
