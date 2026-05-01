#![no_std]
#![no_main]

use sex_pdx::*;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { sys_yield(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_rt::heap_init();
    serial_println!("[sexinput] Normalizer Starting...");

    unsafe {
        sys_set_state(SVC_STATE_LISTENING);
    }

    let mut tick: u64 = 0;

    loop {
        // 1. Non-blocking poll for raw scancodes from kernel (SLOT_INPUT = 3)
        //    Non-blocking allows synthetic producer to run on idle ticks.
        if let Some(req) = pdx_try_listen_raw(SLOT_INPUT) {
            // Kernel RawInput is type 0x201, arg0 = scancode
            if req.type_id == 0x201 {
                let scancode = req.arg0;
                // serial_println!("[sexinput] Raw scancode: {:#x}", scancode);

                // 2. Normalize and forward to silk-shell (SLOT_SHELL = 6)
                // Typed event via 0x202: arg0=code(break-bit stripped), arg1=1(press)/0(release), arg2=EV_KEY
                let value = if scancode & 0x80 == 0 { 1 } else { 0 };
                let code = (scancode & 0x7F) as u64;
                pdx_call(SLOT_SHELL, 0x202, code, value, EV_KEY);
            }
        }

        // 3. Synthetic pointer event producer (transport proof, not product)
        //    Emits bounded EV_REL + occasional EV_BTN to validate the typed event path
        //    through silk-shell's existing POINTER_EVENT_NORMALIZATION_V1 consumer.
        //    Cadence cap: 1 burst per 120 ticks, each burst ≤ 1 EV_REL + optional click.
        tick = tick.wrapping_add(1);
        if tick % 120 == 0 {
            // Small motion (will accumulate across bursts)
            pdx_call(SLOT_SHELL, 0x202, 5, 3, EV_REL);

            // Click sequence: press every 480 ticks, release 240 ticks later
            if tick % 480 == 0 {
                pdx_call(SLOT_SHELL, 0x202, 1, 1, EV_BTN);
                serial_println!("[sexinput] Synthetic pointer click press");
            } else if tick % 480 == 240 {
                pdx_call(SLOT_SHELL, 0x202, 1, 0, EV_BTN);
            }
        }

        sys_yield();
    }
}
