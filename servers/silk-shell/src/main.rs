#![no_std]
#![no_main]

use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_listen, serial_println,
    SLOT_DISPLAY,
    OP_WINDOW_CREATE, OP_WINDOW_PAINT, OP_SET_BG, OP_RENDER_BAR,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[silk-shell] PD3 start — issuing OP_WINDOW_CREATE to SLOT_DISPLAY={}", SLOT_DISPLAY);

    // Background surface (full screen, z=0)
    let _bg_id = pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, 0, 0, 0);
    serial_println!("[silk-shell] bg window id={}", _bg_id);

    pdx_call(SLOT_DISPLAY, OP_SET_BG, 0x1A1A2E, 0, 0);

    // Silkbar panel (full width, 32px tall, z=1)
    let panel_id = pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, 0, 32, 1);
    serial_println!("[silk-shell] panel_id={}", panel_id);

    // First render — this lights up the screen
    pdx_call(SLOT_DISPLAY, OP_RENDER_BAR, panel_id, 0, 0);
    pdx_call(SLOT_DISPLAY, OP_WINDOW_PAINT, panel_id, 0, 0);
    serial_println!("[silk-shell] initial render sent");

    loop {
        let msg = pdx_listen();
        if msg.type_id == 0x202 { // HIDEvent
            let code  = msg.arg0;
            let value = msg.arg1;
            if code == 1 && value == 1 {
                // toggle launcher — stub for Phase 25
                pdx_call(SLOT_DISPLAY, OP_WINDOW_PAINT, panel_id, 0, 0);
            }
        }
    }
}
