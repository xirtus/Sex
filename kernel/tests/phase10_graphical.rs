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
fn test_graphical_glyph_render() {
    serial_println!("test: Verifying Phase 10 Graphical Plumbing (Glyph Render)...");
    
    // 1. Allocate lent buffer for glyph data
    let _buffer = sex_kernel::memory::allocator::alloc_frame().expect("Test: OOM");
    
    // 2. Perform GpuCall via PDX to standalone sexdisplay
    // Simulation: PD 500 is sexdisplay
    let msg = sex_kernel::ipc::messages::MessageType::GpuCall {
        command: 1, // RENDER_GLYPH
        buffer_cap: 1, // Simulated cap to the allocated frame
        width: 8,
        height: 16,
    };
    
    // Simulation: Instead of direct DOMAIN_REGISTRY lookup, a real app uses its capability table.
    // For test purposes, assume capability slot 5 points to sexdisplay (granted at boot).
    let res_ptr = sex_kernel::ipc::safe_pdx_call(5, &msg as *const _ as u64).unwrap();
    
    let reply = unsafe { *(res_ptr as *const sex_kernel::ipc::messages::MessageType) };
    match reply {
        sex_kernel::ipc::messages::MessageType::GpuReply { status } => {
            assert_eq!(status, 0, "Glyph render failed");
        },
        _ => panic!("Expected GpuReply"),
    }
    
    serial_println!("test: Graphical Glyph Render SUCCESS.");
}
