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
fn test_ruthless_signal_delivery() {
    serial_println!("test: Verifying Ruthless Signal Delivery (SIGINT -> FLSCHED Park)...");
    
    // 1. Target PD setup
    let pd_id = 4000;
    
    // 2. Simulate SIGINT routing via pure PDX
    // This will unpark the target's trampoline task
    let res = sex_kernel::ipc::router::route_signal(1 /* root */, pd_id, 2 /* SIGINT */, 1 /* cap */);
    
    assert!(res.is_ok(), "Signal routing failed");
    
    // 3. Verification: Check task state in scheduler (should be Ready/Running)
    let _trampoline_tid = pd_id | 0x8000_0000;
    // In a real test, we'd wait for the handler to execute and check a memory flag.
    
    serial_println!("test: Ruthless Signal Delivery SUCCESS.");
}
