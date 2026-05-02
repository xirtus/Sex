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
const OP_USB_MOUSE_REPORT: u64 = 0x260;
// USB physical capture mode: keep synthetic drag proof code available, but
// disable repeated injection so physical USB movement logs are not polluted.
const USB_PROOF_DISABLE_SYNTH_DRAG: bool = true;

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
    let mut drag_proof_stage: u8 = 0;

    loop {
        // 0. Local USB->sexinput PDX proof path (no shell routing in this phase).
        if let Some(req) = pdx_try_listen_raw(0) {
            serial_println!("[sexinput.usb_mouse.recv] type={:#x}", req.type_id);
            if req.type_id == OP_USB_MOUSE_REPORT {
                let buttons = req.arg1 as u8;
                let packed = req.arg2;
                let dx = (packed as u8) as i8;
                let dy = ((packed >> 8) as u8) as i8;
                let wheel = ((packed >> 16) as u8) as i8;
                serial_println!(
                    "[sexinput.usb_mouse.decode.ok] buttons={:#x} dx={} dy={} wheel={}",
                    buttons,
                    dx,
                    dy,
                    wheel
                );

                serial_println!("[sexinput.usb_mouse.normalize.start]");
                let report = HidPointerRawReport {
                    dx: dx as i16,
                    dy: dy as i16,
                    buttons,
                    wheel,
                };
                let mut normalized_events: [(u64, u64, u64); 4] = [(0, 0, 0); 4];
                let mut norm_count: usize = 0;
                normalize_pointer_report_v1(
                    report,
                    unsafe { &mut *core::ptr::addr_of_mut!(LAST_BUTTONS) },
                    |arg0, arg1, arg2| {
                        if norm_count < normalized_events.len() {
                            normalized_events[norm_count] = (arg0, arg1, arg2);
                            norm_count += 1;
                        }
                    },
                );
                serial_println!("[sexinput.usb_mouse.normalize.ok]");

                serial_println!("[sexinput.usb_mouse.shell_send.start]");
                // Proof tap for shell-side decode logging.
                let proof_send = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, buttons as u64, packed);
                let mut send_err: u64 = match proof_send {
                    Ok(_) => 0,
                    Err(err) => err,
                };

                for i in 0..norm_count {
                    let (arg0, arg1, arg2) = normalized_events[i];
                    if let Err(err) = pdx_call_checked(SLOT_SHELL, OP_HID_EVENT, arg0, arg1, arg2) {
                        if send_err == 0 {
                            send_err = err;
                        }
                    }
                }
                if send_err == 0 {
                    serial_println!("[sexinput.usb_mouse.shell_send.ok]");
                } else {
                    serial_println!("[sexinput.usb_mouse.shell_send.fail] err={}", send_err);
                }
            }
        }

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

        // 3. Deterministic synthetic drag proof (bounded):
        //    EV_ABS anchor, then BTN down -> REL move -> BTN up via normalizer.
        tick = tick.wrapping_add(1);
        if !USB_PROOF_DISABLE_SYNTH_DRAG && tick % 120 == 0 {
            let report = match drag_proof_stage {
                0 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 200, 200, EV_ABS);
                    serial_println!("[sexinput.drag_proof.start]");
                    HidPointerRawReport { dx: 0, dy: 0, buttons: 0x01, wheel: 0 } // left down edge
                }
                1 => HidPointerRawReport { dx: 6, dy: 4, buttons: 0x01, wheel: 0 }, // drag move
                _ => HidPointerRawReport { dx: 0, dy: 0, buttons: 0x00, wheel: 0 }, // left up edge
            };

            normalize_pointer_report_v1(report, unsafe { &mut *core::ptr::addr_of_mut!(LAST_BUTTONS) }, |arg0, arg1, arg2| {
                pdx_call(SLOT_SHELL, OP_HID_EVENT, arg0, arg1, arg2);
                if arg2 == EV_BTN && arg0 == 1 && arg1 == 1 {
                    serial_println!("[sexinput.drag_proof.down]");
                } else if arg2 == EV_REL {
                    serial_println!("[sexinput.drag_proof.move]");
                } else if arg2 == EV_BTN && arg0 == 1 && arg1 == 0 {
                    serial_println!("[sexinput.drag_proof.up]");
                }
            });

            drag_proof_stage = (drag_proof_stage + 1) % 3;
        }

        sys_yield();
    }
}
