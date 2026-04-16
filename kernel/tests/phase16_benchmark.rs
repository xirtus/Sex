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
fn test_maturity_benchmarks() {
    serial_println!("test: Running Phase 16 Maturity Benchmarks...");

    // 1. Run the kernel-side benchmarking suite
    sex_kernel::benchmark::run_maturity_benchmarks();

    // 2. Verify wait-free execution of a PDX loop
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    for _ in 0..100 {
        // sys_getpid simulation via sexc (Cap Slot 3)
        let _ = sex_kernel::ipc::safe_pdx_call(3, 0);
    }
    let end = unsafe { core::arch::x86_64::_rdtsc() };
    let total = end - start;
    
    assert!(total < 500_000, "PDX latency exceeds wait-free threshold");
    
    serial_println!("test: Userspace Maturity & Benchmarking SUCCESS.");
}
