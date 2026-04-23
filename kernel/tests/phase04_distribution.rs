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
fn test_remote_pdx_routing() {
    serial_println!("test: Verifying Distribution (Remote PDX Proxy)...");

    // 1. Create a socket via PDX to standalone sexnet
    let socket_cap = sex_kernel::syscalls::net::sys_socket(
        1, /* net_cap_id */
        2, /* AF_INET */
        1, /* SOCK_STREAM */
        0,
    );
    assert!(socket_cap > 0, "Socket creation failed");

    // 2. Allocate packet buffer (lent-memory source)
    let buffer = sex_kernel::memory::allocator::alloc_frame().expect("Test: buffer OOM");

    // 3. Send 1KiB packet via lent-memory PDX path
    // This routes to standalone sexnet which acts as the distribution proxy
    let send_res = sex_kernel::syscalls::net::sys_send(socket_cap as u32, buffer, 1024);
    assert_eq!(send_res, 1024, "Send result size mismatch");

    serial_println!("test: Distribution Remote PDX SUCCESS.");
}
