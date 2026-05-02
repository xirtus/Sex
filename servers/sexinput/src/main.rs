#![no_std]
#![no_main]

use sex_pdx::*;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { sys_yield(); }
}

// ── Pointer state for HID report normalizer ──
static mut LAST_BUTTONS: u8 = 0;
const OP_HID_EVENT: u64 = 0x202;

#[derive(Copy, Clone)]
struct HidPointerRawReport {
    dx: i16,
    dy: i16,
    buttons: u8,
    wheel: i8,
}

/// Parse a boot-mouse-style 3-byte report, detect button edge transitions,
/// and emit normalized EV_REL/EV_BTN events via the callback.
///
/// Transport-agnostic: callable from synthetic producer or future USB HID source.
/// Returns the number of events emitted (0-4: 1 REL + up to 3 button edges).
fn normalize_pointer_report_v1(
    report: HidPointerRawReport,
    last_buttons: &mut u8,
    mut emit: impl FnMut(u64, u64, u64),
) -> usize {
    let mut count = 0;
    let buttons = report.buttons & 0x07;       // mask to valid button bits
    let changed = buttons ^ *last_buttons; // XOR edge detection
    let mut bit = 1u8;
    let mut btn_id = 1u8;

    // Emit BTN events for each changed button (bit0=left, bit1=right, bit2=middle)
    while bit <= 0x04 {
        if changed & bit != 0 {
            let pressed = buttons & bit != 0;
            emit(btn_id as u64, if pressed { 1 } else { 0 }, EV_BTN);
            count += 1;
        }
        bit <<= 1;
        btn_id += 1;
    }
    *last_buttons = buttons;

    // Keep wire encoding byte-for-byte: shell decodes as msg.argN as i32.
    let dx = report.dx as i32;
    let dy = report.dy as i32;
    if dx != 0 || dy != 0 {
        emit(dx as u64, dy as u64, EV_REL);
        count += 1;
    }

    // V1 keeps wheel in the raw report model but does not emit wheel events yet.
    let _ = report.wheel;

    count
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
                pdx_call(SLOT_SHELL, OP_HID_EVENT, code, value, EV_KEY);
            }
        }

        // 3. Synthetic pointer event producer (transport proof, not product)
        //    Emits bounded EV_REL + occasional EV_BTN to validate the typed event path
        //    through silk-shell's existing POINTER_EVENT_NORMALIZATION_V1 consumer.
        //    Cadence cap: 1 burst per 120 ticks. Routes through normalize_pointer_report_v1()
        //    normalizer to prove transport-agnostic contract.
        tick = tick.wrapping_add(1);
        if tick % 120 == 0 {
            let buttons = if tick % 480 == 0 { 0x01 } else { 0x00 };
            let report = HidPointerRawReport { dx: 5, dy: 3, buttons, wheel: 0 };
            normalize_pointer_report_v1(report, unsafe { &mut *core::ptr::addr_of_mut!(LAST_BUTTONS) }, |arg0, arg1, arg2| {
                pdx_call(SLOT_SHELL, OP_HID_EVENT, arg0, arg1, arg2);
                if arg2 == EV_BTN && arg1 == 1 {
                    serial_println!("[sexinput] Synthetic pointer click press");
                }
            });
        }

        sys_yield();
    }
}
