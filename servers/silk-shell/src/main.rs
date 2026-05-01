#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_try_listen, pdx_reply, sys_yield, sys_set_state, serial_println, WindowDescriptor,
    SLOT_DISPLAY, SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, OP_SILKBAR_FOCUS_STATE,
    SVC_STATE_LISTENING, ERR_CAP_INVALID,
};

// Local Opcodes
pub const OP_DISPLAY_SET_SNAPSHOT: u64 = 0x15;
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;
pub const OP_HID_EVENT: u64 = 0x202;
pub const OP_SURFACE_UPDATE: u64 = 0xEB;
pub const SURFACE_ID_APP: u64 = 100;
pub const SURFACE_ID_STATIC: u64 = 101;
pub const OP_SURFACE_DESTROY: u64 = 0xEE;

// ── Policy Model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    FocusToggle,
    Focus100, Focus101,
    DestroyFocused,
    RecreateFocused,
    ResetAll,
    SnapLeft, SnapRight, Maximize, Center,
    SnapHome, SnapEnd,
    ShrinkWidth, GrowWidth, ShrinkHeight, GrowHeight,
    LegacyFocusToggle,
}

struct DesktopPolicy {
    width: i32,
    height: i32,
    bar_height: i32,
    move_step: i32,
    resize_step: u32,
    min_width: u32,
    min_height: u32,
    boot_rect_100: (i32, i32, u32, u32),
    boot_rect_101: (i32, i32, u32, u32),
}

const P: DesktopPolicy = DesktopPolicy {
    width: 1280,
    height: 720,
    bar_height: 50,
    move_step: 10,
    resize_step: 20,
    min_width: 120,
    min_height: 80,
    boot_rect_100: (100, 100, 800, 500),
    boot_rect_101: (180, 160, 500, 300),
};

fn scancode_to_action(scancode: u8) -> Option<SurfaceAction> {
    match scancode {
        0x0F => Some(SurfaceAction::FocusToggle),
        0x3C => Some(SurfaceAction::DestroyFocused),
        0x02 => Some(SurfaceAction::Focus100),
        0x03 => Some(SurfaceAction::Focus101),
        0x3D => Some(SurfaceAction::RecreateFocused),
        0x13 => Some(SurfaceAction::ResetAll),
        0x26 => Some(SurfaceAction::SnapLeft),
        0x27 => Some(SurfaceAction::SnapRight),
        0x32 => Some(SurfaceAction::Maximize),
        0x2E => Some(SurfaceAction::Center),
        0x1A => Some(SurfaceAction::ShrinkWidth),
        0x1B => Some(SurfaceAction::GrowWidth),
        0x0C => Some(SurfaceAction::ShrinkHeight),
        0x0D => Some(SurfaceAction::GrowHeight),
        0x3B => Some(SurfaceAction::LegacyFocusToggle),
        0x47 => Some(SurfaceAction::SnapHome),
        0x4F => Some(SurfaceAction::SnapEnd),
        0x4B => Some(SurfaceAction::MoveLeft),
        0x4D => Some(SurfaceAction::MoveRight),
        0x48 => Some(SurfaceAction::MoveUp),
        0x50 => Some(SurfaceAction::MoveDown),
        _ => None,
    }
}

fn layout_left() -> (i32, i32, u32, u32) {
    (0, P.bar_height, (P.width as u32) / 2, (P.height - P.bar_height) as u32)
}

fn layout_right() -> (i32, i32, u32, u32) {
    (P.width / 2, P.bar_height, (P.width as u32) / 2, (P.height - P.bar_height) as u32)
}

fn layout_maximize() -> (i32, i32, u32, u32) {
    (0, P.bar_height, P.width as u32, (P.height - P.bar_height) as u32)
}

/// Clamp surface position to stay within content area.
/// Uses saturating arithmetic so policy drift never panics.
fn clamp_position(x: i32, y: i32, w: u32, h: u32) -> (i32, i32) {
    let max_x = (P.width as u32).saturating_sub(w) as i32;
    let max_y = (P.height as u32).saturating_sub(h).max(P.bar_height as u32) as i32;
    (x.clamp(0, max_x), y.clamp(P.bar_height, max_y))
}

/// Bottom-right edge position for SnapEnd.
/// Uses saturating arithmetic so policy drift never panics.
fn snap_end_pos(w: u32, h: u32) -> (i32, i32) {
    let x = (P.width as u32).saturating_sub(w) as i32;
    let y = (P.height as u32).saturating_sub(h) as i32;
    (x.max(0), y.max(P.bar_height))
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("{}", info);
    loop { sys_yield(); }
}

struct WindowState {
    desc: WindowDescriptor,
}

static mut WINDOWS: Vec<WindowState> = Vec::new();
static mut FOCUS_ID: u64 = 0;
static mut FOCUSED_SURFACE_ID: u64 = SURFACE_ID_APP;
static mut SURFACE_100_ALIVE: bool = true;
static mut SURFACE_101_ALIVE: bool = true;
static mut SURFACE_101_X: i32 = 180;
static mut SURFACE_101_Y: i32 = 160;
static mut SURFACE_100_W: u32 = 800;
static mut SURFACE_100_H: u32 = 500;
static mut SURFACE_101_W: u32 = 500;
static mut SURFACE_101_H: u32 = 300;
static mut SNAPSHOT: [WindowDescriptor; 16] = [
    WindowDescriptor { window_id: 0, buffer_handle: 0, x: 0, y: 0, width: 0, height: 0, z_index: 0, focus_state: 0 }; 16
];

fn clamp_surface_size(x: i32, y: i32, w: u32, h: u32) -> (u32, u32) {
    let max_w = (P.width - x).max(P.min_width as i32) as u32;
    let max_h = (P.height - y).max(P.min_height as i32) as u32;
    (w.min(max_w).max(P.min_width), h.min(max_h).max(P.min_height))
}

fn emit_snapshot() {
    unsafe {
        let mut len = 0;
        // Authorities Z-order sorting here: Focused window always on top (last in array)
        let focus_id = FOCUS_ID;
        
        let mut sorted_windows: Vec<usize> = (0..WINDOWS.iter().len()).collect();
        // Simple sort: focus_id window goes to the end
        sorted_windows.sort_by(|&a, &b| {
            if WINDOWS[a].desc.window_id == focus_id { core::cmp::Ordering::Greater }
            else if WINDOWS[b].desc.window_id == focus_id { core::cmp::Ordering::Less }
            else { core::cmp::Ordering::Equal }
        });

        for (i, &idx) in sorted_windows.iter().enumerate() {
            if i >= 16 { break; }
            let w = &WINDOWS[idx];
            SNAPSHOT[i] = w.desc;
            SNAPSHOT[i].z_index = i as u32;
            SNAPSHOT[i].focus_state = if w.desc.window_id == focus_id { 1 } else { 0 };
            len += 1;
        }

        // Emit to sexdisplay (SLOT 5)
        pdx_call(SLOT_DISPLAY, OP_DISPLAY_SET_SNAPSHOT, SNAPSHOT.as_ptr() as u64, len as u64, 0);

        // Surface 100 position update
        if WINDOWS.len() > 1 && SURFACE_100_ALIVE {
            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_APP, WINDOWS[1].desc.x as u64, WINDOWS[1].desc.y as u64);
        }
        // Surface 101 position update (static tracked position)
        if SURFACE_101_ALIVE {
            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_STATIC, SURFACE_101_X as u64, SURFACE_101_Y as u64);
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_rt::heap_init();
    serial_println!("[silk-shell] Authority Starting...");

    unsafe {
        WINDOWS = Vec::with_capacity(16);
        
        // Create background window (id=1)
        WINDOWS.push(WindowState {
            desc: WindowDescriptor {
                window_id: 1,
                buffer_handle: 0, // Placeholder
                x: 0, y: 0, width: 1280, height: 720,
                z_index: 0, focus_state: 0,
            }
        });
        FOCUS_ID = 1;

        sys_set_state(SVC_STATE_LISTENING);
    }
    serial_println!("[silk-shell] AUTHORITATIVE WM LISTENING (PDX SLOT 6)");

    // Stage 2B: advertise workspace 0 active to SilkBar
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, 0, 0, 0);
    // Stage 2C: one-time focus advertisement (shell)
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, 0, 0);
    serial_println!("[silk-shell] Boot workspace advertisement sent to SilkBar");

    // Stage: boot-time safe inline surface create (0xEC — client-supplied id)
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (100u64 << 32) | 100u64, (500u64 << 32) | 800u64);
    serial_println!("[silk-shell] Boot 0xEC surface 100 create sent to sexdisplay");
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (160u64 << 32) | 180u64, (300u64 << 32) | 500u64);
    serial_println!("[silk-shell] Boot 0xEC surface 101 create sent to sexdisplay");

    // Initialize focus on surface 100 (syncs sexdisplay z-order + color)
    pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
    serial_println!("[silk-shell] Boot focus set to surface 100");

    loop {
        let mut mutated = false;

        while let Some(msg) = pdx_try_listen() {
            match msg.type_id {
                OP_SHELL_BIND_BUFFER => {
                    let buffer_handle = msg.arg0;
                    serial_println!("[silk-shell] Binding buffer {:#x} to sexdrive window", buffer_handle);

                    unsafe {
                        let mut found = false;
                        for w in WINDOWS.iter_mut() {
                            if w.desc.window_id == 2 {
                                w.desc.buffer_handle = buffer_handle;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            WINDOWS.push(WindowState {
                                desc: WindowDescriptor {
                                    window_id: 2,
                                    buffer_handle,
                                    x: 100, y: 100, width: 1024, height: 768,
                                    z_index: 1, focus_state: 1,
                                }
                            });
                            FOCUS_ID = 2;
                        }
                    }
                    mutated = true;
                    pdx_reply(0);
                }
                OP_HID_EVENT => {
                    let scancode = msg.arg0 as u8;
                    let value = msg.arg1; // 1=pressed, 0=released

                    unsafe {
                        // ── Make-code dispatch via policy lookup ──────────────
                        if value == 1 {
                            if let Some(action) = scancode_to_action(scancode) {
                                match action {
                                    SurfaceAction::FocusToggle => {
                                        let current = FOCUSED_SURFACE_ID;
                                        if current == SURFACE_ID_APP && SURFACE_101_ALIVE {
                                            FOCUSED_SURFACE_ID = SURFACE_ID_STATIC;
                                            pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_STATIC, 0, 0);
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        } else if current == SURFACE_ID_STATIC && SURFACE_100_ALIVE {
                                            FOCUSED_SURFACE_ID = SURFACE_ID_APP;
                                            pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        }
                                    }

                                    SurfaceAction::DestroyFocused => {
                                        let target = FOCUSED_SURFACE_ID;
                                        let mut destroyed = false;
                                        if target == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            SURFACE_100_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 100");
                                        } else if target == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            SURFACE_101_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 101");
                                        }
                                        if destroyed {
                                            if target == SURFACE_ID_APP && SURFACE_101_ALIVE {
                                                FOCUSED_SURFACE_ID = SURFACE_ID_STATIC;
                                                pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_STATIC, 0, 0);
                                                serial_println!("[silk-shell] Auto-switched focus to surface 101");
                                            } else if target == SURFACE_ID_STATIC && SURFACE_100_ALIVE {
                                                FOCUSED_SURFACE_ID = SURFACE_ID_APP;
                                                pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
                                                serial_println!("[silk-shell] Auto-switched focus to surface 100");
                                            }
                                            mutated = true;
                                        }
                                    }

                                    SurfaceAction::Focus100 => {
                                        if SURFACE_100_ALIVE {
                                            FOCUSED_SURFACE_ID = SURFACE_ID_APP;
                                            pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 100");
                                        }
                                    }

                                    SurfaceAction::Focus101 => {
                                        if SURFACE_101_ALIVE {
                                            FOCUSED_SURFACE_ID = SURFACE_ID_STATIC;
                                            pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_STATIC, 0, 0);
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 101");
                                        }
                                    }

                                    SurfaceAction::RecreateFocused => {
                                        let (rx, ry, rw, rh) = P.boot_rect_100;
                                        if FOCUSED_SURFACE_ID == SURFACE_ID_APP && !SURFACE_100_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_100_ALIVE = true;
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            SURFACE_100_W = rw; SURFACE_100_H = rh;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 100");
                                        } else if FOCUSED_SURFACE_ID == SURFACE_ID_STATIC && !SURFACE_101_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_101;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_101_ALIVE = true;
                                            SURFACE_101_X = rx; SURFACE_101_Y = ry;
                                            SURFACE_101_W = rw; SURFACE_101_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 101");
                                        }
                                        else if FOCUSED_SURFACE_ID == 0 && !SURFACE_100_ALIVE && !SURFACE_101_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_100_ALIVE = true;
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            SURFACE_100_W = rw; SURFACE_100_H = rh;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            FOCUSED_SURFACE_ID = SURFACE_ID_APP;
                                            pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 100 (fallback)");
                                        }
                                    }

                                    SurfaceAction::ResetAll => {
                                        let (rx, ry, rw, rh) = P.boot_rect_100;
                                        SURFACE_100_ALIVE = true;
                                        WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                        SURFACE_100_W = rw; SURFACE_100_H = rh;
                                        WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;

                                        let (rx2, ry2, rw2, rh2) = P.boot_rect_101;
                                        SURFACE_101_ALIVE = true;
                                        SURFACE_101_X = rx2; SURFACE_101_Y = ry2;
                                        SURFACE_101_W = rw2; SURFACE_101_H = rh2;

                                        FOCUSED_SURFACE_ID = SURFACE_ID_APP;

                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry2 as u64) << 32 | rx2 as u64, (rh2 as u64) << 32 | rw2 as u64);
                                        pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);

                                        mutated = true;
                                        serial_println!("[silk-shell] Reset both surfaces to boot state");
                                    }

                                    SurfaceAction::SnapLeft => {
                                        let (rx, ry, rw, rh) = layout_left();
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 snapped to left half");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_101_X = rx; SURFACE_101_Y = ry;
                                            SURFACE_101_W = rw; SURFACE_101_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 snapped to left half");
                                        }
                                    }

                                    SurfaceAction::SnapRight => {
                                        let (rx, ry, rw, rh) = layout_right();
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 snapped to right half");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_101_X = rx; SURFACE_101_Y = ry;
                                            SURFACE_101_W = rw; SURFACE_101_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 snapped to right half");
                                        }
                                    }

                                    SurfaceAction::Maximize => {
                                        let (rx, ry, rw, rh) = layout_maximize();
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 maximized");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_101_X = rx; SURFACE_101_Y = ry;
                                            SURFACE_101_W = rw; SURFACE_101_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 maximized");
                                        }
                                    }

                                    SurfaceAction::Center => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_100;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 centered");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_101;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_101_X = rx; SURFACE_101_Y = ry;
                                            SURFACE_101_W = rw; SURFACE_101_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 centered");
                                        }
                                    }

                                    SurfaceAction::SnapHome => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            WINDOWS[1].desc.x = 0; WINDOWS[1].desc.y = P.bar_height;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 snapped home");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            SURFACE_101_X = 0; SURFACE_101_Y = P.bar_height;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 snapped home");
                                        }
                                    }

                                    SurfaceAction::SnapEnd => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let (ex, ey) = snap_end_pos(WINDOWS[1].desc.width, WINDOWS[1].desc.height);
                                            WINDOWS[1].desc.x = ex; WINDOWS[1].desc.y = ey;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 100 snapped end");
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let (ex, ey) = snap_end_pos(SURFACE_101_W, SURFACE_101_H);
                                            SURFACE_101_X = ex; SURFACE_101_Y = ey;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 101 snapped end");
                                        }
                                    }

                                    SurfaceAction::ShrinkWidth => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let new_w = SURFACE_100_W.saturating_sub(P.resize_step);
                                            let (new_w, _) = clamp_surface_size(WINDOWS[1].desc.x, WINDOWS[1].desc.y, new_w, SURFACE_100_H);
                                            if new_w != SURFACE_100_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (WINDOWS[1].desc.y as u64) << 32 | WINDOWS[1].desc.x as u64,
                                                    (SURFACE_100_H as u64) << 32 | new_w as u64);
                                                SURFACE_100_W = new_w;
                                                WINDOWS[1].desc.width = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 100 width shrunk to {}", new_w);
                                            }
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let new_w = SURFACE_101_W.saturating_sub(P.resize_step);
                                            let (new_w, _) = clamp_surface_size(SURFACE_101_X, SURFACE_101_Y, new_w, SURFACE_101_H);
                                            if new_w != SURFACE_101_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC,
                                                    (SURFACE_101_Y as u64) << 32 | SURFACE_101_X as u64,
                                                    (SURFACE_101_H as u64) << 32 | new_w as u64);
                                                SURFACE_101_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 101 width shrunk to {}", new_w);
                                            }
                                        }
                                    }

                                    SurfaceAction::GrowWidth => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let new_w = SURFACE_100_W + P.resize_step;
                                            let (new_w, _) = clamp_surface_size(WINDOWS[1].desc.x, WINDOWS[1].desc.y, new_w, SURFACE_100_H);
                                            if new_w != SURFACE_100_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (WINDOWS[1].desc.y as u64) << 32 | WINDOWS[1].desc.x as u64,
                                                    (SURFACE_100_H as u64) << 32 | new_w as u64);
                                                SURFACE_100_W = new_w;
                                                WINDOWS[1].desc.width = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 100 width grown to {}", new_w);
                                            }
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let new_w = SURFACE_101_W + P.resize_step;
                                            let (new_w, _) = clamp_surface_size(SURFACE_101_X, SURFACE_101_Y, new_w, SURFACE_101_H);
                                            if new_w != SURFACE_101_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC,
                                                    (SURFACE_101_Y as u64) << 32 | SURFACE_101_X as u64,
                                                    (SURFACE_101_H as u64) << 32 | new_w as u64);
                                                SURFACE_101_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 101 width grown to {}", new_w);
                                            }
                                        }
                                    }

                                    SurfaceAction::ShrinkHeight => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let new_h = SURFACE_100_H.saturating_sub(P.resize_step);
                                            let (_, new_h) = clamp_surface_size(WINDOWS[1].desc.x, WINDOWS[1].desc.y, SURFACE_100_W, new_h);
                                            if new_h != SURFACE_100_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (WINDOWS[1].desc.y as u64) << 32 | WINDOWS[1].desc.x as u64,
                                                    (new_h as u64) << 32 | SURFACE_100_W as u64);
                                                SURFACE_100_H = new_h;
                                                WINDOWS[1].desc.height = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 100 height shrunk to {}", new_h);
                                            }
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let new_h = SURFACE_101_H.saturating_sub(P.resize_step);
                                            let (_, new_h) = clamp_surface_size(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, new_h);
                                            if new_h != SURFACE_101_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC,
                                                    (SURFACE_101_Y as u64) << 32 | SURFACE_101_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_101_W as u64);
                                                SURFACE_101_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 101 height shrunk to {}", new_h);
                                            }
                                        }
                                    }

                                    SurfaceAction::GrowHeight => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let new_h = SURFACE_100_H + P.resize_step;
                                            let (_, new_h) = clamp_surface_size(WINDOWS[1].desc.x, WINDOWS[1].desc.y, SURFACE_100_W, new_h);
                                            if new_h != SURFACE_100_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (WINDOWS[1].desc.y as u64) << 32 | WINDOWS[1].desc.x as u64,
                                                    (new_h as u64) << 32 | SURFACE_100_W as u64);
                                                SURFACE_100_H = new_h;
                                                WINDOWS[1].desc.height = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 100 height grown to {}", new_h);
                                            }
                                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            let new_h = SURFACE_101_H + P.resize_step;
                                            let (_, new_h) = clamp_surface_size(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, new_h);
                                            if new_h != SURFACE_101_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC,
                                                    (SURFACE_101_Y as u64) << 32 | SURFACE_101_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_101_W as u64);
                                                SURFACE_101_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 101 height grown to {}", new_h);
                                            }
                                        }
                                    }

                                    SurfaceAction::LegacyFocusToggle => {
                                        FOCUS_ID = if FOCUS_ID == 2 { 1 } else { 2 };
                                        mutated = true;
                                        serial_println!("[silk-shell] Legacy focus switched to window {}", FOCUS_ID);
                                    }

                                    // Arrow keys: dispatched via scancode block below
                                    // to preserve existing break-code movement behavior
                                    SurfaceAction::MoveLeft |
                                    SurfaceAction::MoveRight |
                                    SurfaceAction::MoveUp |
                                    SurfaceAction::MoveDown => {}
                                }
                            }
                        }

                        // ── Arrow keys (both make and break, matching existing behavior) ──
                        let step = P.move_step;
                        let focused = FOCUSED_SURFACE_ID;
                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                            let focus_id = FOCUS_ID;
                            for w in WINDOWS.iter_mut() {
                                if w.desc.window_id == focus_id && focus_id != 1 {
                                    match scancode {
                                        0x4B => { w.desc.x -= step; mutated = true; }
                                        0x4D => { w.desc.x += step; mutated = true; }
                                        0x48 => { w.desc.y -= step; mutated = true; }
                                        0x50 => { w.desc.y += step; mutated = true; }
                                        _ => {}
                                    }
                                    // Clamp to content area after movement
                                    let (cx, cy) = clamp_position(w.desc.x, w.desc.y, w.desc.width, w.desc.height);
                                    w.desc.x = cx; w.desc.y = cy;
                                }
                            }
                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                            match scancode {
                                0x4B => { SURFACE_101_X -= step; mutated = true; }
                                0x4D => { SURFACE_101_X += step; mutated = true; }
                                0x48 => { SURFACE_101_Y -= step; mutated = true; }
                                0x50 => { SURFACE_101_Y += step; mutated = true; }
                                _ => {}
                            }
                            // Clamp to content area after movement
                            let (cx, cy) = clamp_position(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H);
                            SURFACE_101_X = cx; SURFACE_101_Y = cy;
                        }
                    }
                }
                _ => {
                    // pdx_reply(ERR_CAP_INVALID); // Only reply if it was a call
                }
            }
        }

        if mutated {
            emit_snapshot();
        }

        sys_yield();
    }
}
