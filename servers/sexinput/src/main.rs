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
// Enable synthetic drag proof via HID_EVENT path.
// Proves sexinput→shell drag-window behavior without physical USB device.
// Positions cursor at (200,200) over SURFACE_ID_APP, fires left-click, drag move, release.
const USB_PROOF_DISABLE_SYNTH_DRAG: bool = false;
// One-shot synthetic click proof via OP_USB_MOUSE_REPORT path.
// Proves sexinput→shell click_focus chain without physical USB device.
// Positions cursor over SURFACE_ID_LINEN (900,500,300x150), fires left click.
// OFF by default — set false to re-run click-focus proof.
const USB_PROOF_DISABLE_SYNTH_CLICK: bool = false;
// SilkBar panel click proof — fires synthetic clicks on bar elements
// (launcher, workspace, status chip, clock) to prove hit-test dispatch.
// ON by default for this mission.
const USB_PROOF_DISABLE_SYNTH_SILKBAR_CLICK: bool = false;
// One-shot gate: set true after synthetic drag proof stages 0→1→2 complete.
// Prevents the drag proof from wrapping and replaying endlessly every 120 ticks.
static mut SYNTHETIC_DRAG_PROOF_DONE: bool = false;

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
    let mut synth_click_stage: u8 = 0;
    let mut silkbar_click_stage: u8 = 0;

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
        //    One-shot: after stages 0→1→2 complete, SYNTHETIC_DRAG_PROOF_DONE
        //    prevents further replay. Without this gate, drag_proof_stage wraps
        //    with % 3 forever, causing endless drag.start/move/end cycles every
        //    120 ticks that flood the shell and starve visual updates.
        tick = tick.wrapping_add(1);
        if !USB_PROOF_DISABLE_SYNTH_DRAG && !unsafe { SYNTHETIC_DRAG_PROOF_DONE } && tick % 120 == 0 {
            let report = match drag_proof_stage {
                0 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 200, 200, EV_ABS);
                    serial_println!("[sexinput.drag_proof.start]");
                    HidPointerRawReport { dx: 0, dy: 0, buttons: 0x01, wheel: 0 } // left down edge
                }
                1 => HidPointerRawReport { dx: 6, dy: 4, buttons: 0x01, wheel: 0 }, // drag move
                _ => {
                    // Stage 2 (final): button up. Set DONE to prevent wrap/replay.
                    unsafe { SYNTHETIC_DRAG_PROOF_DONE = true; }
                    serial_println!("[sexinput.drag_proof.done]");
                    HidPointerRawReport { dx: 0, dy: 0, buttons: 0x00, wheel: 0 } // left up edge
                }
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

        // 4. Synthetic SilkBar click proof via HID_EVENT path.
        //    Fires clicks on launcher, workspace, status chip, clock.
        //    Resets CLICK_ACTIVE before each click to avoid drag-proof interference.
        if !USB_PROOF_DISABLE_SYNTH_SILKBAR_CLICK {
            match silkbar_click_stage {
                // Reset CLICK_ACTIVE from drag proof (stage 0 at tick 0 sets left held)
                0 if tick == 2 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 1;
                }
                // Click launcher at (100, 25)
                1 if tick == 3 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 100, 25, EV_ABS);
                    silkbar_click_stage = 2;
                }
                2 if tick == 4 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=launcher");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 3;
                }
                3 if tick == 5 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 4;
                }
                // Click workspace 2 (index=2) at (635, 25) — SwitchWorkspace(3), ws_idx=2
                4 if tick == 7 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 635, 25, EV_ABS);
                    silkbar_click_stage = 5;
                }
                5 if tick == 8 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=workspace index=3");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 6;
                }
                6 if tick == 9 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 7;
                }
                // Click status chip at (940, 25)
                7 if tick == 11 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 940, 25, EV_ABS);
                    silkbar_click_stage = 8;
                }
                8 if tick == 12 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=status");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 9;
                }
                9 if tick == 13 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 10;
                }
                // Click clock at (1100, 25)
                10 if tick == 15 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1100, 25, EV_ABS);
                    silkbar_click_stage = 11;
                }
                11 if tick == 16 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=clock");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 12;
                }
                12 if tick == 17 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 13;
                }
                // Close launcher panel (click launcher button again to dismiss)
                13 if tick == 19 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 100, 25, EV_ABS);
                    silkbar_click_stage = 14;
                }
                14 if tick == 20 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=launcher_close");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 15;
                }
                15 if tick == 21 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 16;
                }
                // Close status panel (second click on status chip at 940, 25)
                16 if tick == 23 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 940, 25, EV_ABS);
                    silkbar_click_stage = 17;
                }
                17 if tick == 24 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=status_close");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 18;
                }
                18 if tick == 25 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 19;
                }
                // Close clock panel (second click on clock at 1100, 25)
                19 if tick == 27 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1100, 25, EV_ABS);
                    silkbar_click_stage = 20;
                }
                20 if tick == 28 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=clock_close");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 21;
                }
                21 if tick == 29 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 22;
                }
                // Click bell at (1025, 25) — centered in CHIP_X_BELL=1020..1038
                22 if tick == 31 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1025, 25, EV_ABS);
                    silkbar_click_stage = 23;
                }
                23 if tick == 32 => {
                    serial_println!("[sexinput.synthetic.silkbar_click] target=bell");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 1, EV_BTN);
                    silkbar_click_stage = 24;
                }
                24 if tick == 33 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 1, 0, EV_BTN);
                    silkbar_click_stage = 25;
                }
                _ => {}
            }
        }

        // 5. One-shot synthetic click proof via USB mouse path.
        //    Moves cursor to (940,560) ∈ SURFACE_ID_LINEN, then clicks.
        //    Proves sexinput→shell click_focus chain.
        if !USB_PROOF_DISABLE_SYNTH_CLICK {
            // packed_axes = dx_u8 | (dy_u8 << 8)
            match synth_click_stage {
                0 if tick == 10 => {
                    // Init cursor (POINTER_USB_STATE_INIT → 640,400)
                    serial_println!("[sexinput.synthetic.click_focus.start]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, 0u64);
                    synth_click_stage = 1;
                }
                1 if tick == 11 => {
                    // Move +127,+100 → cursor (767,500)
                    let packed: u64 = (127u8 as u64) | ((100u8 as u64) << 8);
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, packed);
                    synth_click_stage = 2;
                }
                2 if tick == 12 => {
                    // Move +127,+60 → cursor (894,560)
                    let packed: u64 = (127u8 as u64) | ((60u8 as u64) << 8);
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, packed);
                    synth_click_stage = 3;
                }
                3 if tick == 13 => {
                    // Move +46,+0 → cursor (940,560) ∈ LINEN [900,1200)×[500,650)
                    let packed: u64 = 46u8 as u64;
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, packed);
                    synth_click_stage = 4;
                }
                4 if tick == 14 => {
                    // Button down — triggers click_focus hit on SURFACE_ID_LINEN
                    serial_println!("[sexinput.synthetic.click_focus.down]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0x01u64, 0u64);
                    synth_click_stage = 5;
                }
                5 if tick == 15 => {
                    // Button up — resets CLICK_ACTIVE
                    serial_println!("[sexinput.synthetic.click_focus.up]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, 0u64);
                    synth_click_stage = 6;
                }
                _ => {}
            }
        }

        sys_yield();
    }
}
