#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_listen_raw, pdx_reply, sys_yield, sys_set_state, serial_println, WindowDescriptor,
    SLOT_DISPLAY, SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, OP_SILKBAR_FOCUS_STATE,
    SVC_STATE_LISTENING, ERR_CAP_INVALID, EV_KEY, EV_REL, EV_ABS, EV_BTN,
};
use silkbar_model::{DEFAULT_SILK_BAR, hit_test_action, Action, PANEL_X, PANEL_Y, PANEL_W, PANEL_H};

// Local Opcodes
pub const OP_DISPLAY_SET_SNAPSHOT: u64 = 0x15;
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;
pub const OP_HID_EVENT: u64 = 0x202;
pub const OP_USB_MOUSE_REPORT: u64 = 0x260;
const SHELL_USB_MOUSE_RECEIVE_UNPARK_PROOF_V1: bool = true;
pub const OP_SURFACE_UPDATE: u64 = 0xEB;
pub const SURFACE_ID_APP: u64 = 100;
pub const SURFACE_ID_STATIC: u64 = 101;
pub const SURFACE_ID_TEST3: u64 = 102;
pub const SURFACE_ID_TEST4: u64 = 103;
pub const SURFACE_ID_LINEN: u64 = 200;
pub const SURFACE_ID_CURSOR: u64 = 0x90; // 144 — OS-owned cursor, no collision with app IDs
pub const SURFACE_ID_LAUNCHER: u64 = 0x92; // 146 — launcher panel surface, toggled by launcher button
pub const SURFACE_ID_STATUS: u64 = 0x93; // 147 — status panel surface, toggled by status chip click
pub const SURFACE_ID_CLOCK: u64 = 0x94; // 148 — clock panel surface, toggled by clock click
pub const SURFACE_ID_BELL: u64 = 0x95; // 149 — bell panel surface, toggled by bell click

// OS-owned surface ID registry:
//   0x90  cursor
//   0x92  launcher panel
//   0x93  status/quick-settings panel
//   0x94  clock panel
//   0x95  reserved (Bell panel)
//   100+  app surfaces (SURFACE_ID_APP, SURFACE_ID_STATIC, etc.)
pub const OP_SURFACE_DESTROY: u64 = 0xEE;

// ── Policy Model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    FocusToggle,
    Focus100, Focus101, Focus102, Focus103, Focus200,
    DestroyFocused,
    RecreateFocused,
    ResetAll,
    SnapLeft, SnapRight, Maximize, Center,
    SnapHome, SnapEnd,
    ShrinkWidth, GrowWidth, ShrinkHeight, GrowHeight,
    LegacyFocusToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    Launcher,
    Status,
    Clock,
    Bell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractionState {
    Idle,
    ClickPending,
    Dragging { surface_id: u64, current_x: i32, current_y: i32 },
    PanelActive { panel: PanelKind },
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
    boot_rect_102: (i32, i32, u32, u32),
    boot_rect_103: (i32, i32, u32, u32),
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
    boot_rect_102: (50, 60, 350, 150),
    boot_rect_103: (900, 560, 300, 120),
};

fn scancode_to_action(scancode: u8) -> Option<SurfaceAction> {
    match scancode {
        0x0F => Some(SurfaceAction::FocusToggle),
        0x3C => Some(SurfaceAction::DestroyFocused),
        0x02 => Some(SurfaceAction::Focus100),
        0x03 => Some(SurfaceAction::Focus101),
        0x04 => Some(SurfaceAction::Focus102),
        0x05 => Some(SurfaceAction::Focus103),
        0x06 => Some(SurfaceAction::Focus200),
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
static mut SURFACE_102_ALIVE: bool = true;
static mut SURFACE_102_X: i32 = 50;
static mut SURFACE_102_Y: i32 = 60;
static mut SURFACE_102_W: u32 = 350;
static mut SURFACE_102_H: u32 = 150;
static mut SURFACE_103_ALIVE: bool = true;
static mut SURFACE_103_X: i32 = 900;
static mut SURFACE_103_Y: i32 = 560;
static mut SURFACE_103_W: u32 = 300;
static mut SURFACE_103_H: u32 = 120;
static mut SNAPSHOT: [WindowDescriptor; 16] = [
    WindowDescriptor { window_id: 0, buffer_handle: 0, x: 0, y: 0, width: 0, height: 0, z_index: 0, focus_state: 0 }; 16
];
// ── Pointer input state (updated by EV_ABS/EV_REL/EV_BTN, no compositor side effects) ──
static mut POINTER_X: i32 = 0;
static mut POINTER_Y: i32 = 0;
static mut POINTER_BUTTONS: u8 = 0; // bitmask: bit0=left, bit1=right, bit2=middle
static mut POINTER_WHEEL_ACCUM: i32 = 0;
static mut POINTER_USB_STATE_INIT: bool = false;
static mut INTERACTION: InteractionState = InteractionState::Idle;
/// Number of allowed [shell.interaction.transition] log lines remaining.
/// Uses AtomicU32 (not static mut) to guarantee the compiler cannot elide
/// the decrement — shared references to static mut are UB and get optimized.
static INTERACTION_LOG_BUDGET: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(16);
// Panel surface-alive tracking (separate from interaction state)
static mut LAUNCHER_ACTIVE: bool = false;
// Status panel toggle state
static mut STATUS_ACTIVE: bool = false;
// Clock panel toggle state
static mut CLOCK_ACTIVE: bool = false;
// Bell panel toggle state
static mut BELL_ACTIVE: bool = false;
// Linen surface 200 position tracking (stable — linen never moves)
static mut SURFACE_200_X: i32 = 900;
static mut SURFACE_200_Y: i32 = 500;
static mut SURFACE_200_W: u32 = 300;
static mut SURFACE_200_H: u32 = 150;

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
        // Surface 102 position update
        if SURFACE_102_ALIVE {
            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_TEST3, SURFACE_102_X as u64, SURFACE_102_Y as u64);
        }
        // Surface 103 position update
        if SURFACE_103_ALIVE {
            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_TEST4, SURFACE_103_X as u64, SURFACE_103_Y as u64);
        }
    }
}

/// Returns true if (px, py) is within the given surface's bounds.
/// Accesses surface position from static mut (caller must ensure unsafe context).
fn point_in_surface(px: i32, py: i32, sid: u64) -> bool {
    unsafe {
        let (x, y, w, h) = match sid {
            SURFACE_ID_APP    => (WINDOWS[1].desc.x, WINDOWS[1].desc.y, SURFACE_100_W, SURFACE_100_H),
            SURFACE_ID_STATIC => (SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H),
            SURFACE_ID_TEST3  => (SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H),
            SURFACE_ID_TEST4  => (SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H),
            SURFACE_ID_LINEN  => (SURFACE_200_X, SURFACE_200_Y, SURFACE_200_W, SURFACE_200_H),
            // OS-owned surfaces: cursor and panels are known but non-focusable —
            // log nonfocusable.reject, not unknown.reject.
            SURFACE_ID_CURSOR
            | SURFACE_ID_LAUNCHER
            | SURFACE_ID_STATUS
            | SURFACE_ID_CLOCK
            | SURFACE_ID_BELL => {
                serial_println!("[shell.surface.nonfocusable.reject] point_in_surface id={:#x}", sid);
                return false;
            }
            _ => {
                serial_println!("[shell.surface.unknown.reject] point_in_surface id={}", sid);
                return false;
            }
        };
        px >= x && px < (x + w as i32) && py >= y && py < (y + h as i32)
    }
}

/// Returns true if the surface is alive (not destroyed).
fn surface_is_alive(sid: u64) -> bool {
    match sid {
        SURFACE_ID_APP      => unsafe { SURFACE_100_ALIVE },
        SURFACE_ID_STATIC   => unsafe { SURFACE_101_ALIVE },
        SURFACE_ID_TEST3    => unsafe { SURFACE_102_ALIVE },
        SURFACE_ID_TEST4    => unsafe { SURFACE_103_ALIVE },
        SURFACE_ID_LINEN    => true,  // linen never destroys its surface
        SURFACE_ID_CURSOR   => true,  // cursor never destroyed
        SURFACE_ID_LAUNCHER => unsafe { LAUNCHER_ACTIVE },
        SURFACE_ID_STATUS   => unsafe { STATUS_ACTIVE },
        SURFACE_ID_CLOCK    => unsafe { CLOCK_ACTIVE },
        SURFACE_ID_BELL     => unsafe { BELL_ACTIVE },
        _ => {
            serial_println!("[shell.surface.unknown.reject] surface_is_alive id={}", sid);
            false
        }
    }
}

/// Budget for [shell.surface.focus.accept] and [shell.surface.focus.fallback]
/// markers. Hot-path accept markers are budgeted; reject/dead markers stay unbudgeted.
static SURFACE_FOCUS_ACCEPT_BUDGET: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(8);

/// If the current focused surface is dead, clear focus to first alive surface
/// in z-order, or to 0 (none). Logs reject/fallback/none markers.
/// Call before any focus-dependent operation.
unsafe fn clear_focus_if_dead() {
    let focused = FOCUSED_SURFACE_ID;
    if focused != 0 && !surface_is_alive(focused) {
        serial_println!("[shell.surface.focus.clear.dead] id={}", focused);
        let z_order = [SURFACE_ID_LINEN, SURFACE_ID_TEST4,
                       SURFACE_ID_TEST3, SURFACE_ID_STATIC, SURFACE_ID_APP];
        let mut found = false;
        for &sid in &z_order {
            if sid == focused { continue; }
            if surface_is_alive(sid) {
                if try_set_focus(sid) {
                    found = true;
                }
                break;
            }
        }
        if !found {
            try_set_focus(0);
            serial_println!("[shell.surface.focus.clear.none]");
        }
    }
}

/// If currently dragging a surface that is no longer alive, cancel the drag.
unsafe fn clear_drag_if_dead() {
    if let InteractionState::Dragging { surface_id, .. } = INTERACTION {
        if !surface_is_alive(surface_id) {
            serial_println!("[shell.surface.drag.cancel.dead] id={}", surface_id);
            try_transition(InteractionState::Idle);
        }
    }
}

/// Returns true if the surface is shell-managed (draggable in V1).
fn is_shell_surface(sid: u64) -> bool {
    sid == SURFACE_ID_APP || sid == SURFACE_ID_STATIC
    || sid == SURFACE_ID_TEST3 || sid == SURFACE_ID_TEST4
}

/// Returns true if the surface ID is eligible for click/keyboard focus.
/// OS-owned surfaces (cursor, panels) are intentionally excluded —
/// they are rendered and toggled by the shell but never take focus.
fn is_focusable_surface(sid: u64) -> bool {
    sid == SURFACE_ID_APP || sid == SURFACE_ID_STATIC
    || sid == SURFACE_ID_TEST3 || sid == SURFACE_ID_TEST4
    || sid == SURFACE_ID_LINEN
}

/// Try to set focus to a surface. Returns true if focus was applied.
/// Guards: surface must be focusable and alive.
/// Clearing focus (sid=0) is always allowed (resets to no surface).
/// Emits unbudgeted reject markers for nonfocusable or dead surfaces.
unsafe fn try_set_focus(sid: u64) -> bool {
    if sid == 0 {
        FOCUSED_SURFACE_ID = 0;
        pdx_call(SLOT_DISPLAY, 0xED, 0, 0, 0);
        return true;
    }
    if !is_focusable_surface(sid) {
        serial_println!("[shell.focus.reject.nonfocusable] id={}", sid);
        return false;
    }
    if !surface_is_alive(sid) {
        serial_println!("[shell.focus.reject.dead] id={}", sid);
        return false;
    }
    FOCUSED_SURFACE_ID = sid;
    pdx_call(SLOT_DISPLAY, 0xED, sid, 0, 0);
    true
}

/// Guarded transition between interaction states.
/// Logs allowed transitions as `[shell.interaction.transition]` and
/// forbidden transitions as `[shell.interaction.forbidden]`.
/// Does nothing (no state change) on forbidden transitions.
unsafe fn try_transition(next: InteractionState) {
    let current = INTERACTION;
    let allowed = match (current, next) {
        // Idle → any state allowed (click, panel open)
        (InteractionState::Idle, _) => true,
        // ClickPending → Dragging (drag start), Idle (release), or PanelActive (silkbar panel open)
        (InteractionState::ClickPending, InteractionState::Dragging { .. }) => true,
        (InteractionState::ClickPending, InteractionState::Idle) => true,
        (InteractionState::ClickPending, InteractionState::PanelActive { .. }) => true,
        // Dragging → Idle (release)
        (InteractionState::Dragging { .. }, InteractionState::Idle) => true,
        // PanelActive → Idle (panel close) or ClickPending (click while panel open)
        (InteractionState::PanelActive { .. }, InteractionState::Idle) => true,
        (InteractionState::PanelActive { .. }, InteractionState::ClickPending) => true,
        // All other transitions forbidden
        _ => false,
    };
    if allowed {
        // Log first N transitions, then suppress.
        // Uses AtomicU32 (not static mut + volatile) to guarantee the compiler
        // cannot elide the decrement (shared ref to static mut is UB).
        let remaining = INTERACTION_LOG_BUDGET.load(core::sync::atomic::Ordering::Relaxed);
        if remaining > 0 {
            serial_println!("[shell.interaction.transition] from={:?} to={:?}", current, next);
            INTERACTION_LOG_BUDGET.store(remaining - 1u32, core::sync::atomic::Ordering::Relaxed);
        }
        INTERACTION = next;
    } else {
        serial_println!("[shell.interaction.forbidden] from={:?} to={:?}", current, next);
    }
}

/// Toggle an OS-owned panel surface open/closed.
/// - If `*active` is false: creates surface `surface_id` at (x, y) with size (w, h).
/// - If `*active` is true: destroys surface `surface_id`.
/// Emits `[shell.{label}.open/close.start/ok] id={surface_id:#x}` markers.
/// Preserves existing marker naming convention for all three panels.
unsafe fn toggle_os_panel(active: &mut bool, kind: PanelKind, surface_id: u64, label: &str, x: u32, y: u32, w: u32, h: u32) -> bool {
    if !*active {
        serial_println!("[shell.{}.open.start] id={:#x}", label, surface_id);
        pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
            (y as u64) << 32 | x as u64,
            (h as u64) << 32 | w as u64);
        serial_println!("[shell.{}.open.ok] id={:#x}", label, surface_id);
        *active = true;
        try_transition(InteractionState::PanelActive { panel: kind });
    } else {
        serial_println!("[shell.{}.close.start] id={:#x}", label, surface_id);
        pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
        serial_println!("[shell.{}.close.ok] id={:#x}", label, surface_id);
        *active = false;
        try_transition(InteractionState::Idle);
    }
    true
}

/// Handle a left-click within the SilkBar top strip (y < 50).
/// Uses hit_test_action() from silkbar-model to determine what was clicked,
/// then dispatches the action (workspace switch, launcher, etc.).
fn handle_silkbar_click(px: i32, py: i32) -> bool {
    if py < 0 || py >= 50 || px < 0 {
        return false;
    }
    let ux = px as usize;
    let uy = py as usize;
    if uy < PANEL_Y || uy >= PANEL_Y + PANEL_H || ux < PANEL_X || ux >= PANEL_X + PANEL_W {
        return false;
    }
    let action = hit_test_action(&DEFAULT_SILK_BAR, ux, uy);
    match action {
        Action::None => false,
        Action::SwitchWorkspace(n) => {
            let ws_idx = n.saturating_sub(1).min(4);
            serial_println!("[shell.silkbar.click] target=workspace index={} x={} y={}", n, ux, uy);
            pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, ws_idx as u64, 0, 0);
            true
        }
        Action::OpenLauncher => {
            serial_println!("[shell.silkbar.click] target=launcher x={} y={}", ux, uy);
            unsafe { toggle_os_panel(&mut LAUNCHER_ACTIVE, PanelKind::Launcher, SURFACE_ID_LAUNCHER, "launcher", 80, 55, 240, 360); }
            true
        }
        Action::OpenClock => {
            serial_println!("[shell.silkbar.click] target=clock x={} y={}", ux, uy);
            unsafe { toggle_os_panel(&mut CLOCK_ACTIVE, PanelKind::Clock, SURFACE_ID_CLOCK, "clock", 1000, 55, 240, 300); }
            true
        }
        Action::ToggleModule(_module) => {
            serial_println!("[shell.silkbar.click] target=status x={} y={}", ux, uy);
            unsafe { toggle_os_panel(&mut STATUS_ACTIVE, PanelKind::Status, SURFACE_ID_STATUS, "status", 860, 55, 200, 300); }
            true
        }
        Action::OpenBell => {
            serial_println!("[shell.silkbar.click] target=bell x={} y={}", ux, uy);
            unsafe { toggle_os_panel(&mut BELL_ACTIVE, PanelKind::Bell, SURFACE_ID_BELL, "bell", 600, 55, 240, 300); }
            true
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
        // Keep window-id 2 present from boot so all existing WINDOWS[1]-based
        // surface-100 policy paths remain valid under pointer/keyboard input.
        WINDOWS.push(WindowState {
            desc: WindowDescriptor {
                window_id: 2,
                buffer_handle: 0,
                x: 100, y: 100, width: 800, height: 500,
                z_index: 1, focus_state: 1,
            }
        });
        FOCUS_ID = 2;

        sys_set_state(SVC_STATE_LISTENING);
    }
    serial_println!("[silk-shell] AUTHORITATIVE WM LISTENING (PDX SLOT 6)");

    // Stage 2B: advertise workspace 0 active to SilkBar
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, 0, 0, 0);
    // Stage 2C: one-time focus advertisement (shell)
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, 0, 0);
    serial_println!("[silk-shell] Boot workspace advertisement sent to SilkBar");

    // Stage: cursor surface — created first so it occupies SURFACES slot 0,
    // winning composite Pass 1 over all other non-focused surfaces.
    serial_println!("[shell.cursor_surface.create.start] id={:#x}", SURFACE_ID_CURSOR);
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_CURSOR,
        ((P.height / 2) as u64) << 32 | (P.width / 2) as u64,
        (18u64 << 32) | 12u64);
    serial_println!("[shell.cursor_surface.create.ok]");

    // Stage: boot-time safe inline surface create (0xEC — client-supplied id)
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (100u64 << 32) | 100u64, (500u64 << 32) | 800u64);
    serial_println!("[silk-shell] Boot 0xEC surface 100 create sent to sexdisplay");
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (160u64 << 32) | 180u64, (300u64 << 32) | 500u64);
    serial_println!("[silk-shell] Boot 0xEC surface 101 create sent to sexdisplay");
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (60u64 << 32) | 50u64, (150u64 << 32) | 350u64);
    serial_println!("[silk-shell] Boot 0xEC surface 102 create sent to sexdisplay");
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (560u64 << 32) | 900u64, (300u64 << 32) | 120u64);
    serial_println!("[silk-shell] Boot 0xEC surface 103 create sent to sexdisplay");

    // Initialize focus on surface 100 (syncs sexdisplay z-order + color)
    pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_APP, 0, 0);
    serial_println!("[silk-shell] Boot focus set to surface 100");

    loop {
        // Runtime containment: park without syscall while null-jump root cause is isolated.
        if !SHELL_USB_MOUSE_RECEIVE_UNPARK_PROOF_V1 {
            core::hint::spin_loop();
            continue;
        }

        let mut mutated = false;

        let msg = pdx_listen_raw(0);
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
                OP_USB_MOUSE_REPORT => {
                    let buttons = msg.arg1 as u8;
                    let packed = msg.arg2;
                    let dx = (packed as u8) as i8;
                    let dy = ((packed >> 8) as u8) as i8;
                    let wheel = ((packed >> 16) as u8) as i8;
                    serial_println!("[shell.recv.usb_mouse]");
                    serial_println!(
                        "[shell.recv.usb_mouse.decode.ok] buttons={:#x} dx={} dy={} wheel={}",
                        buttons,
                        dx,
                        dy,
                        wheel
                    );
                    serial_println!("[shell.pointer.usb_state.start]");
                    unsafe {
                        // Surface-lifetime safety guards before any focus/drag operation
                        clear_focus_if_dead();
                        clear_drag_if_dead();
                        if !POINTER_USB_STATE_INIT {
                            POINTER_X = P.width / 2;
                            POINTER_Y = P.height / 2;
                            POINTER_USB_STATE_INIT = true;
                        }
                        let max_x = P.width.saturating_sub(1);
                        let max_y = P.height.saturating_sub(1);
                        POINTER_X = POINTER_X.saturating_add(dx as i32).clamp(0, max_x);
                        POINTER_Y = POINTER_Y.saturating_add(dy as i32).clamp(0, max_y);
                        POINTER_BUTTONS = buttons & 0x07;
                        POINTER_WHEEL_ACCUM = POINTER_WHEEL_ACCUM.saturating_add(wheel as i32);
                        serial_println!(
                            "[shell.pointer.usb_state.ok] x={} y={} buttons={:#x} wheel={}",
                            POINTER_X,
                            POINTER_Y,
                            POINTER_BUTTONS,
                            POINTER_WHEEL_ACCUM
                        );
                        if dx != 0 || dy != 0 || buttons != 0 || wheel != 0 {
                            serial_println!(
                                "[shell.pointer.usb_state.nonzero.ok] x={} y={} buttons={:#x} wheel={} dx={} dy={}",
                                POINTER_X,
                                POINTER_Y,
                                POINTER_BUTTONS,
                                POINTER_WHEEL_ACCUM,
                                dx,
                                dy
                            );
                        }
                        // Left-button down edge → click-to-focus hit-test.
                        let left_held = (buttons & 0x01) != 0;
                        if left_held && (INTERACTION == InteractionState::Idle || matches!(INTERACTION, InteractionState::PanelActive { .. })) {
                            try_transition(InteractionState::ClickPending);
                            serial_println!("[shell.click_focus.down] x={} y={} buttons={:#x}", POINTER_X, POINTER_Y, buttons);
                            let focused = FOCUSED_SURFACE_ID;
                            if !point_in_surface(POINTER_X, POINTER_Y, focused) {
                                let z_order = [SURFACE_ID_LINEN, SURFACE_ID_TEST4,
                                               SURFACE_ID_TEST3, SURFACE_ID_STATIC, SURFACE_ID_APP];
                                let mut hit_id = 0u64;
                                for &sid in z_order.iter() {
                                    if sid == focused { continue; }
                                    if !surface_is_alive(sid) { continue; }
                                    if point_in_surface(POINTER_X, POINTER_Y, sid) {
                                        hit_id = sid;
                                        break;
                                    }
                                }
                                if hit_id != 0 {
                                    serial_println!("[shell.click_focus.hit] id={}", hit_id);
                                    serial_println!("[shell.click_focus.send.start] id={}", hit_id);
                                    try_set_focus(hit_id);
                                    serial_println!("[shell.click_focus.send.ok] id={}", hit_id);
                                } else {
                                    serial_println!("[shell.click_focus.miss]");
                                }
                            } else {
                                serial_println!("[shell.click_focus.hit] id={}", focused);
                            }
                            // SilkBar intercept: if pointer is in top strip, handle and skip drag
                            if !handle_silkbar_click(POINTER_X, POINTER_Y) && is_shell_surface(FOCUSED_SURFACE_ID)
                                && point_in_surface(POINTER_X, POINTER_Y, FOCUSED_SURFACE_ID)
                            {
                                try_transition(InteractionState::Dragging { surface_id: FOCUSED_SURFACE_ID, current_x: POINTER_X, current_y: POINTER_Y });
                                serial_println!("[shell.drag.start] id={} x={} y={}", FOCUSED_SURFACE_ID, POINTER_X, POINTER_Y);
                            }
                        } else if !left_held {
                            match INTERACTION {
                                InteractionState::ClickPending => {
                                    try_transition(InteractionState::Idle);
                                }
                                InteractionState::Dragging { surface_id, .. } => {
                                    serial_println!("[shell.drag.end] id={} x={} y={}", surface_id, POINTER_X, POINTER_Y);
                                    try_transition(InteractionState::Idle);
                                }
                                _ => {}
                            }
                        }
                        // Move cursor surface to updated pointer position.
                        serial_println!("[shell.cursor_surface.move.start] id={:#x} x={} y={}", SURFACE_ID_CURSOR, POINTER_X, POINTER_Y);
                        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, POINTER_X as u64, POINTER_Y as u64);
                        serial_println!("[shell.cursor_surface.move.ok]");
                        // Budgeted diagnostic for cursor position after USB mouse report.
                        unsafe {
                            static mut CURSOR_MOVE_BUDGET: u32 = 16;
                            let remaining = &mut CURSOR_MOVE_BUDGET;
                            if *remaining > 0 {
                                *remaining -= 1;
                                serial_println!("[shell.cursor.move] x={} y={}", POINTER_X, POINTER_Y);
                            }
                        }
                        // ── USB drag movement: move focused surface by delta while button held ──
                        // clear_drag_if_dead() transitions to Idle if the drag target died,
                        // so the next `if matches!` will naturally skip movement.
                        clear_drag_if_dead();
                        if matches!(INTERACTION, InteractionState::Dragging { .. }) {
                            let focused = FOCUSED_SURFACE_ID;
                            let mut moved = false;
                            if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                if let Some(w) = WINDOWS.get_mut(1) {
                                    w.desc.x = w.desc.x.wrapping_add(dx as i32);
                                    w.desc.y = w.desc.y.wrapping_add(dy as i32);
                                    let (cx, cy) = clamp_position(w.desc.x, w.desc.y, SURFACE_100_W, SURFACE_100_H);
                                    w.desc.x = cx; w.desc.y = cy;
                                    moved = true;
                                }
                            } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                SURFACE_101_X = SURFACE_101_X.wrapping_add(dx as i32);
                                SURFACE_101_Y = SURFACE_101_Y.wrapping_add(dy as i32);
                                let (cx, cy) = clamp_position(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H);
                                SURFACE_101_X = cx; SURFACE_101_Y = cy;
                                moved = true;
                            } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                SURFACE_102_X = SURFACE_102_X.wrapping_add(dx as i32);
                                SURFACE_102_Y = SURFACE_102_Y.wrapping_add(dy as i32);
                                let (cx, cy) = clamp_position(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H);
                                SURFACE_102_X = cx; SURFACE_102_Y = cy;
                                moved = true;
                            } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                SURFACE_103_X = SURFACE_103_X.wrapping_add(dx as i32);
                                SURFACE_103_Y = SURFACE_103_Y.wrapping_add(dy as i32);
                                let (cx, cy) = clamp_position(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H);
                                SURFACE_103_X = cx; SURFACE_103_Y = cy;
                                moved = true;
                            }
                            if moved {
                                mutated = true;
                                serial_println!("[shell.drag.move] id={} x={} y={} dx={} dy={}", focused, POINTER_X, POINTER_Y, dx, dy);
                                serial_println!("[shell.drag.send.ok] id={}", focused);
                            }
                        }
                    }
                    pdx_reply(0);
                }
                OP_HID_EVENT => {
                    let scancode = msg.arg0 as u8;
                    let value = msg.arg1; // 1=pressed, 0=released
                    let event_class = msg.arg2; // EV_KEY, EV_REL, EV_ABS, EV_BTN

                    unsafe {
                        // ── Event-class dispatch ──
                        if event_class == EV_KEY && value == 1 {
                            // ── Make-code dispatch via policy lookup ──────────────
                            if let Some(action) = scancode_to_action(scancode) {
                                match action {
                                    SurfaceAction::FocusToggle => {
                                        let current = FOCUSED_SURFACE_ID;
                                        if current == SURFACE_ID_APP && try_set_focus(SURFACE_ID_STATIC) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        } else if current == SURFACE_ID_STATIC && try_set_focus(SURFACE_ID_TEST3) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        } else if current == SURFACE_ID_TEST3 && try_set_focus(SURFACE_ID_TEST4) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        } else if current == SURFACE_ID_TEST4 && try_set_focus(SURFACE_ID_LINEN) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface {}", FOCUSED_SURFACE_ID);
                                        } else if current == SURFACE_ID_LINEN && try_set_focus(SURFACE_ID_APP) {
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
                                        } else if target == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            SURFACE_102_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 102");
                                        } else if target == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            SURFACE_103_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 103");
                                        }
                                        if destroyed {
                                            if target == SURFACE_ID_APP {
                                                if SURFACE_101_ALIVE && try_set_focus(SURFACE_ID_STATIC) { serial_println!("[silk-shell] Auto-switched focus to surface 101"); }
                                                else if SURFACE_102_ALIVE && try_set_focus(SURFACE_ID_TEST3) { serial_println!("[silk-shell] Auto-switched focus to surface 102"); }
                                                else if SURFACE_103_ALIVE && try_set_focus(SURFACE_ID_TEST4) { serial_println!("[silk-shell] Auto-switched focus to surface 103"); }
                                            } else if target == SURFACE_ID_STATIC {
                                                if SURFACE_100_ALIVE && try_set_focus(SURFACE_ID_APP) { serial_println!("[silk-shell] Auto-switched focus to surface 100"); }
                                                else if SURFACE_102_ALIVE && try_set_focus(SURFACE_ID_TEST3) { serial_println!("[silk-shell] Auto-switched focus to surface 102"); }
                                                else if SURFACE_103_ALIVE && try_set_focus(SURFACE_ID_TEST4) { serial_println!("[silk-shell] Auto-switched focus to surface 103"); }
                                            } else if target == SURFACE_ID_TEST3 {
                                                if SURFACE_100_ALIVE && try_set_focus(SURFACE_ID_APP) { serial_println!("[silk-shell] Auto-switched focus to surface 100"); }
                                                else if SURFACE_101_ALIVE && try_set_focus(SURFACE_ID_STATIC) { serial_println!("[silk-shell] Auto-switched focus to surface 101"); }
                                                else if SURFACE_103_ALIVE && try_set_focus(SURFACE_ID_TEST4) { serial_println!("[silk-shell] Auto-switched focus to surface 103"); }
                                            } else if target == SURFACE_ID_TEST4 {
                                                if SURFACE_100_ALIVE && try_set_focus(SURFACE_ID_APP) { serial_println!("[silk-shell] Auto-switched focus to surface 100"); }
                                                else if SURFACE_101_ALIVE && try_set_focus(SURFACE_ID_STATIC) { serial_println!("[silk-shell] Auto-switched focus to surface 101"); }
                                                else if SURFACE_102_ALIVE && try_set_focus(SURFACE_ID_TEST3) { serial_println!("[silk-shell] Auto-switched focus to surface 102"); }
                                            }
                                            mutated = true;
                                        }
                                    }

                                    SurfaceAction::Focus100 => {
                                        if try_set_focus(SURFACE_ID_APP) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 100");
                                        }
                                    }

                                    SurfaceAction::Focus101 => {
                                        if try_set_focus(SURFACE_ID_STATIC) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 101");
                                        }
                                    }

                                    SurfaceAction::Focus102 => {
                                        if try_set_focus(SURFACE_ID_TEST3) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 102");
                                        }
                                    }

                                    SurfaceAction::Focus103 => {
                                        if try_set_focus(SURFACE_ID_TEST4) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 103");
                                        }
                                    }

                                    SurfaceAction::Focus200 => {
                                        if try_set_focus(SURFACE_ID_LINEN) {
                                            mutated = true;
                                            serial_println!("[silk-shell] Focus switched to surface 200");
                                        }
                                    }

                                    SurfaceAction::RecreateFocused => {
                                        if FOCUSED_SURFACE_ID == SURFACE_ID_APP && !SURFACE_100_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_100;
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
                                        } else if FOCUSED_SURFACE_ID == SURFACE_ID_TEST3 && !SURFACE_102_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_102;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_102_ALIVE = true;
                                            SURFACE_102_X = rx; SURFACE_102_Y = ry;
                                            SURFACE_102_W = rw; SURFACE_102_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 102");
                                        } else if FOCUSED_SURFACE_ID == SURFACE_ID_TEST4 && !SURFACE_103_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_103;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_103_ALIVE = true;
                                            SURFACE_103_X = rx; SURFACE_103_Y = ry;
                                            SURFACE_103_W = rw; SURFACE_103_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 103");
                                        }
                                        else if FOCUSED_SURFACE_ID == 0 && !SURFACE_100_ALIVE && !SURFACE_101_ALIVE && !SURFACE_102_ALIVE && !SURFACE_103_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_100;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_100_ALIVE = true;
                                            WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                                            SURFACE_100_W = rw; SURFACE_100_H = rh;
                                            WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
                                            try_set_focus(SURFACE_ID_APP);
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

                                        let (rx3, ry3, rw3, rh3) = P.boot_rect_102;
                                        SURFACE_102_ALIVE = true;
                                        SURFACE_102_X = rx3; SURFACE_102_Y = ry3;
                                        SURFACE_102_W = rw3; SURFACE_102_H = rh3;

                                        let (rx4, ry4, rw4, rh4) = P.boot_rect_103;
                                        SURFACE_103_ALIVE = true;
                                        SURFACE_103_X = rx4; SURFACE_103_Y = ry4;
                                        SURFACE_103_W = rw4; SURFACE_103_H = rh4;

                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_STATIC, (ry2 as u64) << 32 | rx2 as u64, (rh2 as u64) << 32 | rw2 as u64);
                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry3 as u64) << 32 | rx3 as u64, (rh3 as u64) << 32 | rw3 as u64);
                                        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry4 as u64) << 32 | rx4 as u64, (rh4 as u64) << 32 | rw4 as u64);
                                        try_set_focus(SURFACE_ID_APP);

                                        mutated = true;
                                        serial_println!("[silk-shell] Reset all surfaces to boot state");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_102_X = rx; SURFACE_102_Y = ry;
                                            SURFACE_102_W = rw; SURFACE_102_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 snapped to left half");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_103_X = rx; SURFACE_103_Y = ry;
                                            SURFACE_103_W = rw; SURFACE_103_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 snapped to left half");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_102_X = rx; SURFACE_102_Y = ry;
                                            SURFACE_102_W = rw; SURFACE_102_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 snapped to right half");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_103_X = rx; SURFACE_103_Y = ry;
                                            SURFACE_103_W = rw; SURFACE_103_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 snapped to right half");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_102_X = rx; SURFACE_102_Y = ry;
                                            SURFACE_102_W = rw; SURFACE_102_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 maximized");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_103_X = rx; SURFACE_103_Y = ry;
                                            SURFACE_103_W = rw; SURFACE_103_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 maximized");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_102;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_102_X = rx; SURFACE_102_Y = ry;
                                            SURFACE_102_W = rw; SURFACE_102_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 centered");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let (rx, ry, rw, rh) = P.boot_rect_103;
                                            pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4, (ry as u64) << 32 | rx as u64, (rh as u64) << 32 | rw as u64);
                                            SURFACE_103_X = rx; SURFACE_103_Y = ry;
                                            SURFACE_103_W = rw; SURFACE_103_H = rh;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 centered");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            SURFACE_102_X = 0; SURFACE_102_Y = P.bar_height;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 snapped home");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            SURFACE_103_X = 0; SURFACE_103_Y = P.bar_height;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 snapped home");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let (ex, ey) = snap_end_pos(SURFACE_102_W, SURFACE_102_H);
                                            SURFACE_102_X = ex; SURFACE_102_Y = ey;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 102 snapped end");
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let (ex, ey) = snap_end_pos(SURFACE_103_W, SURFACE_103_H);
                                            SURFACE_103_X = ex; SURFACE_103_Y = ey;
                                            mutated = true;
                                            serial_println!("[silk-shell] Surface 103 snapped end");
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let new_w = SURFACE_102_W.saturating_sub(P.resize_step);
                                            let (new_w, _) = clamp_surface_size(SURFACE_102_X, SURFACE_102_Y, new_w, SURFACE_102_H);
                                            if new_w != SURFACE_102_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3,
                                                    (SURFACE_102_Y as u64) << 32 | SURFACE_102_X as u64,
                                                    (SURFACE_102_H as u64) << 32 | new_w as u64);
                                                SURFACE_102_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 102 width shrunk to {}", new_w);
                                            }
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let new_w = SURFACE_103_W.saturating_sub(P.resize_step);
                                            let (new_w, _) = clamp_surface_size(SURFACE_103_X, SURFACE_103_Y, new_w, SURFACE_103_H);
                                            if new_w != SURFACE_103_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4,
                                                    (SURFACE_103_Y as u64) << 32 | SURFACE_103_X as u64,
                                                    (SURFACE_103_H as u64) << 32 | new_w as u64);
                                                SURFACE_103_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 103 width shrunk to {}", new_w);
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let new_w = SURFACE_102_W + P.resize_step;
                                            let (new_w, _) = clamp_surface_size(SURFACE_102_X, SURFACE_102_Y, new_w, SURFACE_102_H);
                                            if new_w != SURFACE_102_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3,
                                                    (SURFACE_102_Y as u64) << 32 | SURFACE_102_X as u64,
                                                    (SURFACE_102_H as u64) << 32 | new_w as u64);
                                                SURFACE_102_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 102 width grown to {}", new_w);
                                            }
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let new_w = SURFACE_103_W + P.resize_step;
                                            let (new_w, _) = clamp_surface_size(SURFACE_103_X, SURFACE_103_Y, new_w, SURFACE_103_H);
                                            if new_w != SURFACE_103_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4,
                                                    (SURFACE_103_Y as u64) << 32 | SURFACE_103_X as u64,
                                                    (SURFACE_103_H as u64) << 32 | new_w as u64);
                                                SURFACE_103_W = new_w;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 103 width grown to {}", new_w);
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let new_h = SURFACE_102_H.saturating_sub(P.resize_step);
                                            let (_, new_h) = clamp_surface_size(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, new_h);
                                            if new_h != SURFACE_102_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3,
                                                    (SURFACE_102_Y as u64) << 32 | SURFACE_102_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_102_W as u64);
                                                SURFACE_102_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 102 height shrunk to {}", new_h);
                                            }
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let new_h = SURFACE_103_H.saturating_sub(P.resize_step);
                                            let (_, new_h) = clamp_surface_size(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, new_h);
                                            if new_h != SURFACE_103_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4,
                                                    (SURFACE_103_Y as u64) << 32 | SURFACE_103_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_103_W as u64);
                                                SURFACE_103_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 103 height shrunk to {}", new_h);
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
                                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            let new_h = SURFACE_102_H + P.resize_step;
                                            let (_, new_h) = clamp_surface_size(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, new_h);
                                            if new_h != SURFACE_102_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST3,
                                                    (SURFACE_102_Y as u64) << 32 | SURFACE_102_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_102_W as u64);
                                                SURFACE_102_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 102 height grown to {}", new_h);
                                            }
                                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            let new_h = SURFACE_103_H + P.resize_step;
                                            let (_, new_h) = clamp_surface_size(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, new_h);
                                            if new_h != SURFACE_103_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_TEST4,
                                                    (SURFACE_103_Y as u64) << 32 | SURFACE_103_X as u64,
                                                    (new_h as u64) << 32 | SURFACE_103_W as u64);
                                                SURFACE_103_H = new_h;
                                                mutated = true;
                                                serial_println!("[silk-shell] Surface 103 height grown to {}", new_h);
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

                        // ── Arrow keys (make-code only, value == 1) ──
                        let step = P.move_step;
                        let focused = FOCUSED_SURFACE_ID;
                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE && value == 1 {
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
                        } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE && value == 1 {
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
                        } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE && value == 1 {
                            match scancode {
                                0x4B => { SURFACE_102_X -= step; mutated = true; }
                                0x4D => { SURFACE_102_X += step; mutated = true; }
                                0x48 => { SURFACE_102_Y -= step; mutated = true; }
                                0x50 => { SURFACE_102_Y += step; mutated = true; }
                                _ => {}
                            }
                            let (cx, cy) = clamp_position(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H);
                            SURFACE_102_X = cx; SURFACE_102_Y = cy;
                        } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE && value == 1 {
                            match scancode {
                                0x4B => { SURFACE_103_X -= step; mutated = true; }
                                0x4D => { SURFACE_103_X += step; mutated = true; }
                                0x48 => { SURFACE_103_Y -= step; mutated = true; }
                                0x50 => { SURFACE_103_Y += step; mutated = true; }
                                _ => {}
                            }
                            let (cx, cy) = clamp_position(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H);
                            SURFACE_103_X = cx; SURFACE_103_Y = cy;
                        }

                        // ── Pointer event state updates (no compositor side effects) ──
                        if event_class == EV_ABS {
                            POINTER_X = msg.arg0 as i32;
                            POINTER_Y = msg.arg1 as i32;
                            serial_println!("[silk-shell] Pointer ABS ({}, {})", POINTER_X, POINTER_Y);
                        } else if event_class == EV_REL {
                            let dx = msg.arg0 as i32;
                            let dy = msg.arg1 as i32;
                            POINTER_X = POINTER_X.wrapping_add(dx);
                            POINTER_Y = POINTER_Y.wrapping_add(dy);

                            // ── Drag movement: move focused surface by delta while button held ──
                            // clear_drag_if_dead() transitions to Idle if the drag target died,
                            // so the next `if matches!` will naturally skip movement.
                            clear_drag_if_dead();
                            if matches!(INTERACTION, InteractionState::Dragging { .. }) {
                                let focused = FOCUSED_SURFACE_ID;
                                let mut moved = false;
                                if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                    if let Some(w) = WINDOWS.get_mut(1) {
                                        w.desc.x = w.desc.x.wrapping_add(dx);
                                        w.desc.y = w.desc.y.wrapping_add(dy);
                                        let (cx, cy) = clamp_position(w.desc.x, w.desc.y, SURFACE_100_W, SURFACE_100_H);
                                        w.desc.x = cx; w.desc.y = cy;
                                        moved = true;
                                    }
                                } else if focused == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                    SURFACE_101_X = SURFACE_101_X.wrapping_add(dx);
                                    SURFACE_101_Y = SURFACE_101_Y.wrapping_add(dy);
                                    let (cx, cy) = clamp_position(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H);
                                    SURFACE_101_X = cx; SURFACE_101_Y = cy;
                                    moved = true;
                                } else if focused == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                    SURFACE_102_X = SURFACE_102_X.wrapping_add(dx);
                                    SURFACE_102_Y = SURFACE_102_Y.wrapping_add(dy);
                                    let (cx, cy) = clamp_position(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H);
                                    SURFACE_102_X = cx; SURFACE_102_Y = cy;
                                    moved = true;
                                } else if focused == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                    SURFACE_103_X = SURFACE_103_X.wrapping_add(dx);
                                    SURFACE_103_Y = SURFACE_103_Y.wrapping_add(dy);
                                    let (cx, cy) = clamp_position(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H);
                                    SURFACE_103_X = cx; SURFACE_103_Y = cy;
                                    moved = true;
                                }
                                if moved {
                                    mutated = true;
                                    serial_println!("[shell.drag.move] id={} x={} y={} dx={} dy={}", focused, POINTER_X, POINTER_Y, dx, dy);
                                    serial_println!("[shell.drag.send.ok] id={}", focused);
                                }
                            }

                            serial_println!("[silk-shell] Pointer REL d=({},{}) pos=({},{})",
                                dx, dy, POINTER_X, POINTER_Y);
                        } else if event_class == EV_BTN {
                            let button = msg.arg0 as u8;
                            let pressed = msg.arg1 != 0;
                            if pressed {
                                POINTER_BUTTONS |= 1u8.checked_shl(button.saturating_sub(1) as u32).unwrap_or(0);
                            } else {
                                POINTER_BUTTONS &= !(1u8.checked_shl(button.saturating_sub(1) as u32).unwrap_or(0));
                            }
                            serial_println!("[silk-shell] Pointer BTN {} {} buttons={:#x}",
                                button, if pressed { "dn" } else { "up" }, POINTER_BUTTONS);

                            // Surface-lifetime safety guards before any focus/drag operation
                            clear_focus_if_dead();
                            clear_drag_if_dead();

                            // ── Click-to-focus: left-button press edge (0→1 transition only) ──
                            if button == 1 {
                                if pressed && (INTERACTION == InteractionState::Idle || matches!(INTERACTION, InteractionState::PanelActive { .. })) {
                                    try_transition(InteractionState::ClickPending);
                                    // Hit-test in visual z-order: focused first, then reverse slot order
                                    let focused = FOCUSED_SURFACE_ID;
                                    if !point_in_surface(POINTER_X, POINTER_Y, focused) {
                                        let z_order = [SURFACE_ID_LINEN, SURFACE_ID_TEST4,
                                                       SURFACE_ID_TEST3, SURFACE_ID_STATIC, SURFACE_ID_APP];
                                        let mut hit_id = 0u64;
                                        for &sid in &z_order {
                                            if sid == focused { continue; }
                                            if !surface_is_alive(sid) { continue; }
                                            if point_in_surface(POINTER_X, POINTER_Y, sid) {
                                                hit_id = sid;
                                                break;
                                            }
                                        }
                                        if hit_id != 0 {
                                            try_set_focus(hit_id);
                                            serial_println!("[silk-shell] Click focus surface {}", FOCUSED_SURFACE_ID);
                                        }
                                    }
                                    // SilkBar intercept: if pointer is in top strip, handle and skip drag
                                    if !handle_silkbar_click(POINTER_X, POINTER_Y) && is_shell_surface(FOCUSED_SURFACE_ID)
                                        && point_in_surface(POINTER_X, POINTER_Y, FOCUSED_SURFACE_ID)
                                    {
                                        try_transition(InteractionState::Dragging { surface_id: FOCUSED_SURFACE_ID, current_x: POINTER_X, current_y: POINTER_Y });
                                        serial_println!("[shell.drag.start] id={} x={} y={}", FOCUSED_SURFACE_ID, POINTER_X, POINTER_Y);
                                    }
                                } else if !pressed {
                                    match INTERACTION {
                                        InteractionState::ClickPending => {
                                            try_transition(InteractionState::Idle);
                                        }
                                        InteractionState::Dragging { surface_id, .. } => {
                                            serial_println!("[shell.drag.end] id={} x={} y={}", surface_id, POINTER_X, POINTER_Y);
                                            try_transition(InteractionState::Idle);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            _ => {
                serial_println!("[pdx.opcode.unknown] shell type_id={:#x} caller={}", msg.type_id, msg.caller_pd);
            }
        }

        if mutated {
            emit_snapshot();
        }

        sys_yield();
    }
}
