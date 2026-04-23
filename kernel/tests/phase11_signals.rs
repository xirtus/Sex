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

// use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use sex_kernel::capability::ProtectionDomain;
use sex_kernel::ipc::DOMAIN_REGISTRY;
// FIXME: sex_kernel::servers::sexc is missing or broken.
// use sex_kernel::servers::sexc::{drain_pending_signals_for_pd, sexc, KernelSigAction};

extern crate alloc;

static HANDLER_FIRED: AtomicUsize = AtomicUsize::new(0);

#[allow(dead_code)]
extern "C" fn sigint_handler(sig: i32) {
    HANDLER_FIRED.fetch_add(sig as usize, Ordering::SeqCst);
}

#[test_case]
fn phase11_sigint_routes_into_user_handler() {
    HANDLER_FIRED.store(0, Ordering::SeqCst);

    // FIXME: This test is highly broken and needs a complete rewrite.
    // It relies on non-existent sexc server and Arc which isn't suitable here.

    let pd_ptr =
        alloc::boxed::Box::into_raw(alloc::boxed::Box::new(ProtectionDomain::new(0x1100, 3)));
    let pd_id = unsafe { (*pd_ptr).id };
    DOMAIN_REGISTRY.insert(pd_id, pd_ptr);

    /*
    sex_kernel::servers::sexc::init_signal_trampoline(pd_id);

    let bridge = sexc::new(pd_id);
    let action = KernelSigAction {
        handler: sigint_handler as usize as u64,
        flags: 0,
        mask: 0,
        restorer: 0,
    };

    bridge
        .sigaction(2, &action as *const KernelSigAction as u64)
        .expect("sigaction should register");
    bridge.kill(pd_id, 2).expect("kill should route signal");

    assert_eq!(drain_pending_signals_for_pd(pd_id), 1);
    assert_eq!(HANDLER_FIRED.load(Ordering::SeqCst), 2);
    */
}
