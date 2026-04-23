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
fn test_dynamic_translator_loading() {
    serial_println!("test: Verifying Dynamic Translators (sexnode discovery)...");

    // 1. Allocate buffer for non-native binary code
    let code_vaddr = sex_kernel::memory::allocator::alloc_frame().expect("Test: buffer OOM");

    // 2. Perform translation call via PDX to standalone sexnode
    // This simulates the execve path for a non-native binary
    let entry = sex_kernel::syscalls::translator::sys_translate_and_exec(
        1,            // Translator capability ID
        0x_4000_0000, // Simulated path pointer
        code_vaddr,
        4096,
    );

    assert!(entry > 0, "Binary translation failed to return entry point");
    assert_eq!(entry, 0x_4000_1000, "Translated entry point mismatch");

    serial_println!("test: Dynamic Translator Loading SUCCESS.");
}
