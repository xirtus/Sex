//! kaleidoscope — Silk browser entry.
//! No std. PDX-native.

#![no_std]
#![no_main]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Create window via SexCompositor PDX
    // 2. Initialize Servo WebRender surface
    // 3. Enter event loop
    loop {}
}
