#![no_std]
#![no_main]

use sex_pdx::*;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { sys_yield(); }
}

// ── Pointer state for HID report normalizer ──
static mut LAST_BUTTONS: u8 = 0;
// Tracks the USB HID keycode from the previous keyboard report for press/release detection.
static mut LAST_USB_KEY: u8 = 0;
const OP_HID_EVENT: u64 = 0x202;
const OP_USB_MOUSE_REPORT: u64 = 0x260;
const OP_USB_KEYBOARD_REPORT: u64 = 0x261;
static CAP_READY_SHELL: AtomicBool = AtomicBool::new(false);
static DEFER_EMITTED_SHELL: AtomicBool = AtomicBool::new(false);
static EDGE_SEND_EMITTED_SHELL: AtomicBool = AtomicBool::new(false);
/// Master switch for all synthetic input proofs (drag, click-focus, silkbar clicks).
///
/// Set env var `SEXOS_PROOFS_DISABLED=1` at build time to disable all proofs
/// for interactive visual use. Default (unset): proofs enabled for CI/nographic
/// verification. Build once per mode; the env var is embedded at compile time.
///
/// Equivalent to the previous three individual `USB_PROOF_DISABLE_*` constants,
/// merged into one gate to simplify toggling.
///
/// # Build commands
/// ```sh
/// # Proof mode (CI / nographic): default, proofs enabled
/// ./scripts/entrypoint_build.sh
///
/// # Interactive visual mode: proofs disabled
/// SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
/// ```
const SYNTHETIC_INPUT_PROOFS_DISABLED: bool = option_env!("SEXOS_PROOFS_DISABLED").is_some();
/// Enables synthetic SilkBar click proof (launcher/status/clock/bell click script).
/// Default (unset): disabled to avoid recurring panel toggle behavior during
/// interactive use.
/// Set env var `SEXOS_SILKBAR_CLICK_PROOF=1` at build time to re-enable.
const SILKBAR_CLICK_PROOF_ENABLED: bool = option_env!("SEXOS_SILKBAR_CLICK_PROOF").is_some();
/// Dev-only keyboard cursor fallback. When enabled, arrow keys and WASD
/// emit EV_REL events to move the cursor, bypassing broken QEMU USB HID input.
/// Set env var `SEXOS_KEYBOARD_CURSOR=1` at build time to enable.
/// Gate unset = no behavior change.
const KEYBOARD_CURSOR_ENABLED: bool = option_env!("SEXOS_KEYBOARD_CURSOR").is_some();
/// Enables one-shot synthetic keyboard proof for F5/F6 HID event path.
/// Set env var `SEXOS_KEYBOARD_PROOF=1` at build time to enable.
/// Default (unset): no behavior change.
/// Only affects sexinput; no kernel changes.
const KEYBOARD_PROOF_ENABLED: bool = option_env!("SEXOS_KEYBOARD_PROOF").is_some();
// One-shot gate: set true after synthetic drag proof stages 0→1→2 complete.
// Prevents the drag proof from wrapping and replaying endlessly every 120 ticks.
// Also set true when a real USB mouse input arrives, cancelling remaining proofs.
static mut SYNTHETIC_DRAG_PROOF_DONE: bool = false;

#[derive(Copy, Clone)]
struct HidPointerRawReport {
    dx: i16,
    dy: i16,
    buttons: u8,
    wheel: i8,
    is_abs: bool,
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
    
    let mut reason = "none";
    if changed != 0 { reason = "buttons"; }

    if report.is_abs {
        unsafe {
            static mut LAST_ABS_X: i32 = -1;
            static mut LAST_ABS_Y: i32 = -1;
            if dx != LAST_ABS_X || dy != LAST_ABS_Y {
                if reason == "none" { reason = "abs"; }
            }
            if reason != "none" {
                LAST_ABS_X = dx;
                LAST_ABS_Y = dy;
                unsafe {
                    static mut POINTER_FORWARD_BUDGET: u32 = 2048;
                    if POINTER_FORWARD_BUDGET > 0 {
                        POINTER_FORWARD_BUDGET -= 1;
                        serial_println!("[sexinput.pointer.forward.reason={}]", reason);
                    }
                }
                emit(dx as u64, dy as u64, EV_ABS);
                count += 1;
            }
        }
    } else {
        if dx != 0 || dy != 0 {
            if reason == "none" { reason = "motion"; }
        }
        if reason != "none" {
            unsafe {
                static mut POINTER_FORWARD_BUDGET: u32 = 2048;
                if POINTER_FORWARD_BUDGET > 0 {
                    POINTER_FORWARD_BUDGET -= 1;
                    serial_println!("[sexinput.pointer.forward.reason={}]", reason);
                }
            }
            if dx != 0 || dy != 0 {
                emit(dx as u64, dy as u64, EV_REL);
                count += 1;
            }
        }
    }

    // V1 keeps wheel in the raw report model but does not emit wheel events yet.
    let _ = report.wheel;

    count
}

fn send_shell_hid_event(arg0: u64, arg1: u64, arg2: u64) -> Result<bool, u64> {
    if CAP_READY_SHELL.load(Ordering::Relaxed) {
        return pdx_call_checked(SLOT_SHELL, OP_HID_EVENT, arg0, arg1, arg2).map(|_| {
            if !EDGE_SEND_EMITTED_SHELL.swap(true, Ordering::Relaxed) {
                serial_println!("[bootgraph.edge.send from=sexinput to=silk-shell slot=6 op=OP_HID_EVENT first=1]");
            }
            true
        });
    }

    match pdx_call_checked(SLOT_SHELL, OP_HID_EVENT, arg0, arg1, arg2) {
        Ok(_) => {
            CAP_READY_SHELL.store(true, Ordering::Relaxed);
            if !EDGE_SEND_EMITTED_SHELL.swap(true, Ordering::Relaxed) {
                serial_println!("[bootgraph.edge.send from=sexinput to=silk-shell slot=6 op=OP_HID_EVENT first=1]");
            }
            Ok(true)
        }
        Err(err) if err == ERR_CAP_INVALID => {
            if !DEFER_EMITTED_SHELL.swap(true, Ordering::Relaxed) {
                serial_println!("[bootgraph.edge.defer from=sexinput to=silk-shell slot=6 reason=missing_cap]");
            }
            sys_yield();
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

/// Translate USB HID keyboard usage ID to PS/2 scancode set 1.
/// Returns None for unmapped keys.
fn hid_to_ps2(hid: u8) -> Option<u8> {
    match hid {
        0x47 => Some(0x46), // Scroll Lock
        0x28 => Some(0x1C), // Enter
        0x44 => Some(0x57), // F11
        0x45 => Some(0x58), // F12
        0x2C => Some(0x39), // Space
        0x2A => Some(0x0E), // Backspace
        0x29 => Some(0x01), // Esc
        0x04 => Some(0x1E), // a
        0x05 => Some(0x30), // b
        0x06 => Some(0x2E), // c
        0x07 => Some(0x20), // d
        0x08 => Some(0x12), // e
        0x09 => Some(0x21), // f
        0x0A => Some(0x22), // g
        0x0B => Some(0x23), // h
        0x0C => Some(0x17), // i
        0x0D => Some(0x24), // j
        0x0E => Some(0x25), // k
        0x0F => Some(0x26), // l
        0x10 => Some(0x32), // m
        0x11 => Some(0x31), // n
        0x12 => Some(0x18), // o
        0x13 => Some(0x19), // p
        0x14 => Some(0x10), // q
        0x15 => Some(0x13), // r
        0x16 => Some(0x1F), // s
        0x17 => Some(0x14), // t
        0x18 => Some(0x16), // u
        0x19 => Some(0x2F), // v
        0x1A => Some(0x11), // w
        0x1B => Some(0x2D), // x
        0x1C => Some(0x15), // y
        0x1D => Some(0x2C), // z
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexinput.init.start]");
    sex_rt::heap_init();
    serial_println!("[sexinput] Normalizer Starting...");

    // Budgeted proof gate state diagnostic.
    // Fires once at boot to confirm whether synthetic input proofs are enabled.
    unsafe {
        static mut PROOF_GATE_BUDGET: u32 = 1;
        let remaining = &mut PROOF_GATE_BUDGET;
        if *remaining > 0 {
            *remaining -= 1;
            let state = if SYNTHETIC_INPUT_PROOFS_DISABLED { 0 } else { 1 };
            let source = if SYNTHETIC_INPUT_PROOFS_DISABLED { "env" } else { "default" };
            serial_println!("[proof.gate.state] enabled={} source={}", state, source);
        }
    }

    // Budgeted keyboard cursor gate diagnostic.
    // Fires once at boot to confirm whether keyboard cursor fallback is enabled.
    unsafe {
        static mut KBD_CURSOR_GATE_BUDGET: u32 = 1;
        let remaining = &mut KBD_CURSOR_GATE_BUDGET;
        if *remaining > 0 {
            *remaining -= 1;
            serial_println!("[keyboard_cursor.gate] enabled={} source={}",
                if KEYBOARD_CURSOR_ENABLED { 1 } else { 0 },
                if KEYBOARD_CURSOR_ENABLED { "env" } else { "default" });
        }
    }

    unsafe {
        sys_set_state(SVC_STATE_LISTENING);
    }

    let mut tick: u64 = 0;
    let mut drag_proof_stage: u8 = 0;
    let mut synth_click_stage: u8 = 0;
    let mut silkbar_click_stage: u8 = 0;
    let mut kbd_proof_stage: u8 = 0;
    let mut ev_key_edge_stage: u8 = 0;

    serial_println!("[sexinput.ready]");
    loop {
        // 0. Local USB->sexinput PDX proof path (no shell routing in this phase).
        if let Some(req) = pdx_try_listen_raw(0) {
            serial_println!("[sexinput.usb_mouse.recv] type={:#x}", req.type_id);
            if req.type_id == OP_USB_MOUSE_REPORT {
                let cls = req.arg0;
                unsafe {
                    static mut SEXINPUT_POINTER_RECV_BUDGET: u32 = 16;
                    let rem = &mut SEXINPUT_POINTER_RECV_BUDGET;
                    if *rem > 0 {
                        *rem -= 1;
                        serial_println!("[sexinput.pointer.recv] class={} a0={} a1={}", cls, req.arg1 as i32, req.arg2 as i32);
                    }
                }

                // Compatibility path: if producer already sends normalized HID event
                // tuples in OP_USB_MOUSE_REPORT (class in arg0), forward as-is.
                if cls == EV_REL || cls == EV_ABS || cls == EV_BTN {
                    // ── Button edge proof markers ──
                    if cls == EV_BTN {
                        let pressed = req.arg1 != 0;
                        if pressed {
                            serial_println!("[sexinput.pointer.button.down] btn={} pressed=1", req.arg0);
                        } else {
                            serial_println!("[sexinput.pointer.button.up] btn={} pressed=0", req.arg0);
                        }
                    }
                    unsafe {
                        static mut SEXINPUT_POINTER_SEND_BUDGET: u32 = 2048;
                        let rem = &mut SEXINPUT_POINTER_SEND_BUDGET;
                        if *rem > 0 {
                            *rem -= 1;
                            serial_println!("[sexinput.pointer.send] class={} a0={} a1={}", cls, req.arg1 as i32, req.arg2 as i32);
                        }
                    }
                    match send_shell_hid_event(req.arg1, req.arg2, cls) {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(err) => {
                            unsafe {
                                static mut SEXINPUT_POINTER_DROP_BUDGET: u32 = 16;
                                let rem = &mut SEXINPUT_POINTER_DROP_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!(
                                        "[sexinput.pointer.drop] reason=shell_send_fail class={} a0={} a1={} err={}",
                                        cls, req.arg1 as i32, req.arg2 as i32, err
                                    );
                                }
                            }
                        }
                    }
                    continue;
                }

                let buttons = req.arg1 as u8;
                let packed = req.arg2;
                let is_abs = ((packed >> 32) & 1) != 0;
                let dx = if is_abs { (packed & 0xFFFF) as u16 as i16 } else { (packed as u8) as i8 as i16 };
                let dy = if is_abs { ((packed >> 16) & 0xFFFF) as u16 as i16 } else { ((packed >> 8) as u8) as i8 as i16 };
                let wheel = if is_abs { 0 } else { ((packed >> 16) as u8) as i8 };
                serial_println!(
                    "[sexinput.usb_mouse.decode.ok] buttons={:#x} dx={} dy={} wheel={} is_abs={}",
                    buttons,
                    dx,
                    dy,
                    wheel,
                    is_abs
                );

                serial_println!("[sexinput.usb_mouse.normalize.start]");
                let report = HidPointerRawReport {
                    dx,
                    dy,
                    buttons,
                    wheel,
                    is_abs,
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

                // Budgeted diagnostic for real USB mouse deltas arriving from sexusb.
                // Not triggered by synthetic proof paths (those send HID_EVENT directly).
                unsafe {
                    static mut MOUSE_REAL_DELTA_BUDGET: u32 = 16;
                    let remaining = &mut MOUSE_REAL_DELTA_BUDGET;
                    if *remaining > 0 {
                        *remaining -= 1;
                        serial_println!("[sexinput.mouse.real.delta] dx={} dy={} buttons={:#x}", dx, dy, buttons);
                    }
                }
                // Budgeted liveness: sexinput received a mouse report from sexusb (tablet path).
                unsafe {
                    static mut MOUSE_LIVE_BUDGET: u32 = 16;
                    let rem = &mut MOUSE_LIVE_BUDGET;
                    if *rem > 0 {
                        *rem -= 1;
                        serial_println!("[sexinput.mouse.live] n=0 dx={} dy={} buttons=0x{:x}", dx, dy, buttons);
                    }
                }

                // Budgeted diagnostic for real USB button transitions.
                // Fires only when buttons field is nonzero (actual button event).
                // Synthetic click-focus proof also sends OP_USB_MOUSE_REPORT but
                // with buttons=0x01/0x00 at known early ticks; real clicks happen
                // later and this marker distinguishes button events from pure moves.
                unsafe {
                    static mut MOUSE_REAL_BUTTON_BUDGET: u32 = 16;
                    let remaining = &mut MOUSE_REAL_BUTTON_BUDGET;
                    if *remaining > 0 && buttons != 0 {
                        *remaining -= 1;
                        serial_println!("[sexinput.mouse.real.button] buttons={:#x} dx={} dy={}", buttons, dx, dy);
                    }
                }

                serial_println!("[sexinput.usb_mouse.shell_send.start]");
                // Forward normalized HID events only (EV_BTN edges + EV_REL movement).
                // OP_USB_MOUSE_REPORT is NOT forwarded — the HID EV_BTN handler in the shell
                // handles click-to-focus, and EV_REL handles cursor movement + drag.
                // This eliminates the dx/dy double-apply bug where both the USB handler
                // and the HID EV_REL handler applied the same delta.
                // The synthetic click-focus proof sends OP_USB_MOUSE_REPORT directly to the
                // shell and is unaffected by this change.
                let mut send_err: u64 = 0;

                for i in 0..norm_count {
                    let (arg0, arg1, arg2) = normalized_events[i];
                    unsafe {
                        static mut SEXINPUT_POINTER_SEND_BUDGET: u32 = 2048;
                        let rem = &mut SEXINPUT_POINTER_SEND_BUDGET;
                        if *rem > 0 {
                            *rem -= 1;
                            serial_println!("[sexinput.pointer.send] class={} a0={} a1={}", arg2, arg0 as i32, arg1 as i32);
                        }
                    }
                    // Budgeted marker for EV_REL emission to shell.
                    if arg2 == 2 { // EV_REL
                        unsafe {
                            static mut HID_EMIT_REL_BUDGET: u32 = 16;
                            let rem = &mut HID_EMIT_REL_BUDGET;
                            if *rem > 0 {
                                *rem -= 1;
                                serial_println!("[sexinput.hid.emit.rel] n=0 dx={} dy={}", arg0 as i32, arg1 as i32);
                            }
                        }
                    }
                    match send_shell_hid_event(arg0, arg1, arg2) {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(err) => {
                            if send_err == 0 {
                                send_err = err;
                            }
                        }
                    }
                }
                if norm_count == 0 {
                    unsafe {
                        static mut SEXINPUT_POINTER_DROP_BUDGET: u32 = 16;
                        let rem = &mut SEXINPUT_POINTER_DROP_BUDGET;
                        if *rem > 0 {
                            *rem -= 1;
                            serial_println!(
                                "[sexinput.pointer.drop] reason=idle_or_no_edges class={} a0={} a1={}",
                                EV_REL,
                                dx as i32,
                                dy as i32
                            );
                        }
                    }
                }
                if send_err == 0 {
                    serial_println!("[sexinput.usb_mouse.shell_send.ok]");
                } else {
                    unsafe {
                        static mut SEXINPUT_POINTER_DROP_BUDGET: u32 = 16;
                        let rem = &mut SEXINPUT_POINTER_DROP_BUDGET;
                        if *rem > 0 {
                            *rem -= 1;
                            serial_println!(
                                "[sexinput.pointer.drop] reason=shell_send_fail class={} a0={} a1={} err={}",
                                EV_REL,
                                dx as i32,
                                dy as i32,
                                send_err
                            );
                        }
                    }
                    serial_println!("[sexinput.usb_mouse.shell_send.fail] err={}", send_err);
                }
            } else if req.type_id == OP_USB_KEYBOARD_REPORT {
                // USB HID boot keyboard report via sexusb.
                // arg0 = reserved, arg1 = modifiers, arg2 = first keycode (USB HID usage ID)
                let modifiers = req.arg1 as u8;
                let keycode = req.arg2 as u8;
                unsafe {
                    static mut KBD_RECV_BUDGET: u32 = 16;
                    let rem = &mut KBD_RECV_BUDGET;
                    if *rem > 0 {
                        *rem -= 1;
                        serial_println!("[sexinput.kbd.recv] key=0x{:x} mod=0x{:x}", keycode, modifiers);
                    }
                }
                // Dev-only keyboard cursor fallback (SEXOS_KEYBOARD_CURSOR=1).
                // Map USB HID usage IDs to EV_REL cursor movement.
                // Only on key-down (nonzero keycode, no rate limiting).
                if KEYBOARD_CURSOR_ENABLED && keycode != 0 {
                    // USB HID keyboard usage IDs for cursor keys + WASD
                    let (dx, dy): (i32, i32) = match keycode {
                        0x1a | 0x52 => (0, -8),   // W / Up
                        0x16 | 0x51 => (0, 8),    // S / Down
                        0x04 | 0x50 => (-8, 0),   // A / Left
                        0x07 | 0x4f => (8, 0),    // D / Right
                        _ => (0, 0),
                    };
                    if dx != 0 || dy != 0 {
                        unsafe {
                            static mut KBD_CURSOR_KEY_BUDGET_USB: u32 = 16;
                            let rem = &mut KBD_CURSOR_KEY_BUDGET_USB;
                            if *rem > 0 {
                                *rem -= 1;
                                serial_println!("[keyboard_cursor.key] code=0x{:x} dx={} dy={}", keycode, dx, dy);
                            }
                        }
                        pdx_call(SLOT_SHELL, OP_HID_EVENT, dx as u64, dy as u64, EV_REL);
                        unsafe {
                            static mut KBD_CURSOR_REL_BUDGET_USB: u32 = 16;
                            let rem = &mut KBD_CURSOR_REL_BUDGET_USB;
                            if *rem > 0 {
                                *rem -= 1;
                                serial_println!("[keyboard_cursor.emit.rel] dx={} dy={}", dx, dy);
                            }
                        }
                    }
                }
                // Forward mapped keys as EV_KEY to silk-shell using HID→PS/2 translation.
                // Tracks single key (first slot only) for press/release edges.
                unsafe {
                    let prev = LAST_USB_KEY;
                    let cur = keycode;
                    if cur != prev {
                        if prev != 0 {
                            if let Some(sc) = hid_to_ps2(prev) {
                                pdx_call(SLOT_SHELL, OP_HID_EVENT, sc as u64, 0, EV_KEY);
                            }
                        }
                        if cur != 0 {
                            if let Some(sc) = hid_to_ps2(cur) {
                                static mut KBD_EVKEY_BUDGET: u32 = 64;
                                let rem = &mut KBD_EVKEY_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[sexinput.usb_kbd.evkey] hid=0x{:x} sc=0x{:x}", cur, sc);
                                }
                                pdx_call(SLOT_SHELL, OP_HID_EVENT, sc as u64, 1, EV_KEY);
                            } else {
                                static mut KBD_DROP_BUDGET: u32 = 16;
                                let rem = &mut KBD_DROP_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[sexinput.usb_kbd.drop] hid=0x{:x} reason=unmapped", cur);
                                }
                            }
                        }
                        LAST_USB_KEY = cur;
                    }
                }
            }
        }

        // 1. Non-blocking poll for raw scancodes from kernel (SLOT_INPUT = 3)
        //    Non-blocking allows synthetic producer to run on idle ticks.
        if let Some(req) = pdx_try_listen_raw(SLOT_INPUT) {
            // Kernel RawInput is type 0x201, arg0 = scancode
            if req.type_id == 0x201 {
                let scancode = req.arg0;
                serial_println!("[sexinput.ps2.scancode] raw={:#x}", scancode);

                // 2. Normalize and forward to silk-shell (SLOT_SHELL = 6)
                // Typed event via 0x202: arg0=code(break-bit stripped), arg1=1(press)/0(release), arg2=EV_KEY
                let value = if scancode & 0x80 == 0 { 1 } else { 0 };
                let code = (scancode & 0x7F) as u64;
                pdx_call(SLOT_SHELL, OP_HID_EVENT, code, value, EV_KEY);

                // 2b. Dev-only keyboard cursor fallback (SEXOS_KEYBOARD_CURSOR=1).
                //     On key press, emit EV_REL to move cursor in 8px steps.
                //     Arrow keys (0x48/0x50/0x4B/0x4D) and WASD (0x11/0x1F/0x1E/0x20).
                //     Gate unset = zero overhead, no behavior change.
                if KEYBOARD_CURSOR_ENABLED && value == 1 {
                    let (dx, dy): (i32, i32) = match code {
                        0x11 | 0x48 => (0, -8),   // W / Up
                        0x1F | 0x50 => (0, 8),    // S / Down
                        0x1E | 0x4B => (-8, 0),   // A / Left
                        0x20 | 0x4D => (8, 0),    // D / Right
                        _ => (0, 0),
                    };
                    if dx != 0 || dy != 0 {
                        unsafe {
                            static mut KBD_CURSOR_KEY_BUDGET: u32 = 16;
                            let rem = &mut KBD_CURSOR_KEY_BUDGET;
                            if *rem > 0 {
                                *rem -= 1;
                                serial_println!("[keyboard_cursor.key] code=0x{:x} dx={} dy={}", code, dx, dy);
                            }
                        }
                        pdx_call(SLOT_SHELL, OP_HID_EVENT, dx as u64, dy as u64, EV_REL);
                        unsafe {
                            static mut KBD_CURSOR_REL_BUDGET: u32 = 16;
                            let rem = &mut KBD_CURSOR_REL_BUDGET;
                            if *rem > 0 {
                                *rem -= 1;
                                serial_println!("[keyboard_cursor.emit.rel] dx={} dy={}", dx, dy);
                            }
                        }
                    }
                }
            }
        }

        // 2c. Bounded one-shot keyboard cursor self-test.
        //     When KEYBOARD_CURSOR_ENABLED is set, fires exactly one EV_REL(0, -8)
        //     on the first poll cycle after boot to prove the pipeline works without
        //     requiring QEMU host input routing (which is broken on QEMU 11.0.0).
        //     Uses the same pdx_call path as real keyboard cursor movement.
        //     One-shot: KBD_SELF_TEST_DONE prevents replay.
        if KEYBOARD_CURSOR_ENABLED {
            unsafe {
                static mut KBD_SELF_TEST_DONE: bool = false;
                if !KBD_SELF_TEST_DONE {
                    KBD_SELF_TEST_DONE = true;
                    serial_println!("[keyboard_cursor.self_test] dx=0 dy=-8");
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 0u64, (-8i64) as u64, EV_REL);
                    serial_println!("[keyboard_cursor.self_test.ok]");
                }
            }
        }

        // 3. Deterministic synthetic drag proof (bounded):
        //    EV_ABS anchor, then BTN down -> REL move -> BTN up via normalizer.
        //    One-shot: after stages 0→1→2 complete, SYNTHETIC_DRAG_PROOF_DONE
        //    prevents further replay. Without this gate, drag_proof_stage wraps
        //    with % 3 forever, causing endless drag.start/move/end cycles every
        //    120 ticks that flood the shell and starve visual updates.
        tick = tick.wrapping_add(1);
        // Ticks 5, 6, 7: one stage per tick, no overlap with EV_KEY (3/4) or click-focus (10/14/15).
        // Original threshold was tick%120 (~171s to complete); reduced for CI 25s probe window.
        const DRAG_TICKS: [u64; 3] = [5, 6, 7];
        if !SYNTHETIC_INPUT_PROOFS_DISABLED && !unsafe { SYNTHETIC_DRAG_PROOF_DONE }
            && drag_proof_stage < 3 && tick == DRAG_TICKS[drag_proof_stage as usize]
        {
            let report = match drag_proof_stage {
                0 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 200, 200, EV_ABS);
                    serial_println!("[sexinput.drag_proof.start]");
                    HidPointerRawReport { dx: 0, dy: 0, buttons: 0x01, wheel: 0, is_abs: false } // left down edge
                }
                1 => HidPointerRawReport { dx: 6, dy: 4, buttons: 0x01, wheel: 0, is_abs: false }, // drag move
                _ => {
                    // Stage 2 (final): button up. Set DONE to prevent wrap/replay.
                    unsafe { SYNTHETIC_DRAG_PROOF_DONE = true; }
                    serial_println!("[sexinput.drag_proof.done]");
                    HidPointerRawReport { dx: 0, dy: 0, buttons: 0x00, wheel: 0, is_abs: false } // left up edge
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
        if !SYNTHETIC_INPUT_PROOFS_DISABLED && SILKBAR_CLICK_PROOF_ENABLED {
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
        } else {
            unsafe {
                static mut SILKBAR_CLICK_DISABLED_MARKER_EMITTED: bool = false;
                if !SILKBAR_CLICK_DISABLED_MARKER_EMITTED {
                    SILKBAR_CLICK_DISABLED_MARKER_EMITTED = true;
                    serial_println!("[sexinput.synthetic.silkbar_click.disabled]");
                }
            }
        }

        // 5. One-shot synthetic click proof via USB mouse path.
        //    Sets cursor to (940,560) ∈ SURFACE_ID_LINEN, then clicks.
        //    Proves sexinput→shell click_focus chain.
        //    Uses EV_ABS for positioning (not delta accumulation) to avoid
        //    coordinate corruption from concurrent silkbar proof EV_ABS events.
        if !SYNTHETIC_INPUT_PROOFS_DISABLED {
            match synth_click_stage {
                0 if tick == 10 => {
                    // Init cursor (POINTER_USB_STATE_INIT → 640,400)
                    serial_println!("[sexinput.synthetic.click_focus.start]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, 0u64);
                    synth_click_stage = 1;
                }
                1 if tick == 14 => {
                    // EV_ABS anchors cursor at (940,560) ∈ LINEN [900,1200)×[500,650)
                    // This explicit positioning is immune to EV_ABS interference from
                    // the synthetic silkbar click proof (ticks 2-33).
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 940, 560, EV_ABS);
                    // Button down — triggers click_focus hit on SURFACE_ID_LINEN
                    serial_println!("[sexinput.synthetic.click_focus.down]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0x01u64, 0u64);
                    synth_click_stage = 2;
                }
                2 if tick == 15 => {
                    // Button up — resets CLICK_ACTIVE
                    serial_println!("[sexinput.synthetic.click_focus.up]");
                    let _ = pdx_call_checked(SLOT_SHELL, OP_USB_MOUSE_REPORT, 0, 0u64, 0u64);
                    synth_click_stage = 3;
                }
                _ => {}
            }
        }

        // 6a. KEYBOARD_EDGE_PROOF_V1: one-shot EV_KEY down+up for Enter (scancode 0x1C).
        //     Proves sexinput→silk-shell EV_KEY path without SEXOS_KEYBOARD_PROOF env var.
        //     Gated by !SYNTHETIC_INPUT_PROOFS_DISABLED (same gate as other default proofs).
        if !SYNTHETIC_INPUT_PROOFS_DISABLED {
            match ev_key_edge_stage {
                0 if tick == 3 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 0x1Cu64, 1, EV_KEY);
                    serial_println!("[sexinput.key.ev_key.down code=0x1c]");
                    ev_key_edge_stage = 1;
                }
                1 if tick == 4 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 0x1Cu64, 0, EV_KEY);
                    serial_println!("[sexinput.key.ev_key.up code=0x1c]");
                    ev_key_edge_stage = 2;
                }
                _ => {}
            }
        }

        // 6. One-shot synthetic keyboard proof for F5/F6 scene settings.
        //    Sends F5 press+release, then second F5 press+release, then F6 press+release
        //    via OP_HID_EVENT EV_KEY.
        //    Bounded: kbd_proof_stage prevents replay after stage 5.
        //    Gate: SEXOS_KEYBOARD_PROOF=1 env var at build time.
        if KEYBOARD_PROOF_ENABLED {
            match kbd_proof_stage {
                // F5 press: scancode 0x3F = 63, EV_KEY, value=1 (pressed)
                0 if tick == 50 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 1, EV_KEY);
                    serial_println!("[sexinput.kbd_proof.f5.down]");
                    kbd_proof_stage = 1;
                }
                // F5 release
                1 if tick == 55 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 0, EV_KEY);
                    serial_println!("[sexinput.kbd_proof.f5.up]");
                    kbd_proof_stage = 2;
                }
                // Second F5 (proves cycle wraps + persist fires again)
                2 if tick == 100 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 1, EV_KEY);
                    kbd_proof_stage = 3;
                }
                3 if tick == 105 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 0, EV_KEY);
                    kbd_proof_stage = 4;
                }
                // F6: scancode 0x40 = 64, EV_KEY, value=1 (pressed)
                4 if tick == 150 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 64, 1, EV_KEY);
                    serial_println!("[sexinput.kbd_proof.f6.down]");
                    kbd_proof_stage = 5;
                }
                5 if tick == 155 => {
                    pdx_call(SLOT_SHELL, OP_HID_EVENT, 64, 0, EV_KEY);
                    serial_println!("[sexinput.kbd_proof.f6.up]");
                    kbd_proof_stage = 6;
                }
                _ => {}
            }
        }

        sys_yield();
    }
}
