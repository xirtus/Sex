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
    RestoreMinimized,
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

/// Typed result of a hit-test. Distinguishes app surfaces from chrome elements
/// and background for future Frame Chrome input routing.
/// V1: Surface and None are produced. SilkBar is returned by handle_silkbar_click
/// separately. FrameChrome is produced from rim/tab-strip geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitTarget {
    /// No surface or chrome element at this position.
    None,
    /// A clickable app/desktop surface. The u64 is the surface_id.
    Surface(u64),
    /// Future: frame chrome (tab strip, resize handle, close button, neon rim).
    FrameChrome { frame_id: u32, kind: u32 },
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
        0x49 => Some(SurfaceAction::RestoreMinimized),
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

// ── Frame Chrome Model ─────────────────────────────────────────────────────────
// Frame = tiled container owning one or more Tabs.
// Tab = shell membership wrapper around an existing hardcoded surface_id.
// In V1, exactly one frame exists with one tab wrapping surface 100 (APP).
// Future phases extend to multi-tab per frame, multi-frame layout.

/// Maximum tabs per frame (overkill for 4 app surfaces, allows future Chrome tabs).
const MAX_TABS_PER_FRAME: u8 = 8;
/// Maximum concurrent frames (overkill for current app count, allows future splits).
const MAX_FRAMES: usize = 4;

/// A tab wraps an existing surface_id with shell-level metadata.
/// The surface remains the app/display object; the tab is shell policy only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ShellTab {
    surface_id: u64,
    /// Reserved for future tab title string handle or content ID.
    title_id: u64,
    /// Reserved for future flags (pinned, muted, loading, etc.).
    flags: u32,
}

/// A frame is a tiled container that owns one or more tabs.
/// The active tab determines which surface is visible/interactable.
/// Frame chrome (neon rim, tab strip) is rendered by sexdisplay based on
/// policy-driven descriptors emitted by silk-shell. This phase defines
/// the model only — no renderer changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ShellFrame {
    frame_id: u32,
    /// Index into tabs[] for the currently active tab.
    active_tab: u8,
    /// Number of valid entries in tabs[] (must be >= 1 for a valid frame).
    tab_count: u8,
    /// Fixed-size tab array. Unused entries are None.
    tabs: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize],
    /// Reserved for future flags (split orientation, pinned state, etc.).
    flags: u32,
    /// Saved normal (pre-zoom) geometry. Valid when FRAME_FLAG_ZOOMED is set.
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}

static mut WINDOWS: Vec<WindowState> = Vec::new();
/// Frame chrome model: fixed-size array of frames, each with fixed-size tab array.
/// No heap allocation for frame/tab state — all static.
static mut FRAMES: [Option<ShellFrame>; MAX_FRAMES] = [None; MAX_FRAMES];
// ── Frame Chrome Hover State ────────────────────────────────────────────────────
// Tracked per-pointer-move. Hover does not affect focus or drag behavior.
// Updated by update_frame_hover_at() once per event loop iteration.
// Hover kind constants (reserved kinds for future chrome elements).
const HOVER_NONE: u32 = 0;
const HOVER_FRAME_BODY: u32 = 1;    // app content area
const HOVER_FRAME_RIM: u32 = 2;     // future: neon rim
const HOVER_TAB_STRIP: u32 = 3;     // future: tab strip

// ── Frame Chrome Hit-Production Constants ──────────────────────────────────
/// Chrome hit-target kind for the 4px neon rim edge band.
const FRAME_CHROME_RIM: u32 = 1;
/// Chrome hit-target kind for a tab strip band (reserved, not produced in V1).
const FRAME_CHROME_TAB_STRIP: u32 = 2;
/// Thickness of the neon rim edge band in pixels.
const FRAME_RIM_PX: i32 = 4;
/// Height of the tab strip band in pixels (0 = disabled in V1).
const FRAME_TAB_STRIP_PX: i32 = 0;

// ── Frame Light Kind Constants (model only, no actions in V1) ──────────
/// No light hovered / default state.
const FRAME_LIGHT_NONE: u32 = 0;
/// Red close light — close active tab/frame (future action).
const FRAME_LIGHT_CLOSE: u32 = 1;
/// Yellow minimize light — minimize/collapse frame (future action).
const FRAME_LIGHT_MINIMIZE: u32 = 2;
/// Green zoom light — zoom/maximize frame (future action).
const FRAME_LIGHT_ZOOM: u32 = 3;

/// Width and height of each frame light square in pixels (fits within 4px rim).
const FRAME_LIGHT_SIZE_PX: i32 = 4;
/// Gap between adjacent frame lights in pixels.
const FRAME_LIGHT_GAP_PX: i32 = 2;

/// ShellFrame.flags: frame is minimized (hidden via 0xEE, not destroyed).
const FRAME_FLAG_MINIMIZED: u32 = 1 << 0;
/// ShellFrame.flags: frame is zoomed/maximized (fills content area below SilkBar).
const FRAME_FLAG_ZOOMED: u32 = 1 << 1;

// ── Selected Window Option Bits (model only, no action behavior in V1) ──
/// Bit: selected frame can be closed/destroyed.
const OPTION_CLOSE: u32 = 1;
/// Bit: selected frame can be zoomed/maximized.
const OPTION_ZOOM: u32 = 2;
/// Bit: selected frame can be minimized/hidden.
const OPTION_MINIMIZE: u32 = 4;
/// Bit: selected frame can be moved via rim drag.
const OPTION_MOVE: u32 = 8;

static mut HOVERED_FRAME_ID: u32 = 0;
static mut HOVER_KIND: u32 = HOVER_NONE;
static mut HOVERED_FRAME_LIGHT: u32 = FRAME_LIGHT_NONE;
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

/// Get the bounding box of a surface, if it has geometry.
/// Returns None for OS-owned surfaces (cursor, panels) and invalid IDs.
/// Used by chrome hit-testing to compute rim/tab-strip regions.
/// Duplicates the bounds match from point_in_surface to avoid refactoring it.
unsafe fn get_surface_bounds(sid: u64) -> Option<(i32, i32, u32, u32)> {
    match sid {
        SURFACE_ID_APP    => Some((WINDOWS[1].desc.x, WINDOWS[1].desc.y, WINDOWS[1].desc.width, WINDOWS[1].desc.height)),
        SURFACE_ID_STATIC => Some((SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H)),
        SURFACE_ID_TEST3  => Some((SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H)),
        SURFACE_ID_TEST4  => Some((SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H)),
        SURFACE_ID_LINEN  => Some((SURFACE_200_X, SURFACE_200_Y, SURFACE_200_W, SURFACE_200_H)),
        _ => None,
    }
}

/// Returns true if (px, py) is within the given surface's bounds.
/// Guards: returns false if surface is dead/invalid.
/// Accesses surface position from static mut (caller must ensure unsafe context).
fn point_in_surface(px: i32, py: i32, sid: u64) -> bool {
    unsafe {
        // Self-defending: skip dead surfaces regardless of caller precondition.
        // This ensures no hit-test or drag-start can select a destroyed surface.
        if !surface_is_alive(sid) {
            serial_println!("[shell.surface.dead.skip] id={} reason=inactive", sid);
            return false;
        }
        let (x, y, w, h) = match sid {
            SURFACE_ID_APP    => (WINDOWS[1].desc.x, WINDOWS[1].desc.y, WINDOWS[1].desc.width, WINDOWS[1].desc.height),
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

// ── Frame Chrome Query Helpers ─────────────────────────────────────────────────
// These are shell-policy queries that map between surface_id (display/input
// object) and the Frame/Tab model (window management abstraction).
// In V1 the mapping is 1:1: one frame, one tab, one surface. Future phases
// extend to N tabs per frame and N frames.

/// Find the frame_id that owns a tab with the given surface_id, if any.
unsafe fn frame_for_surface(surface_id: u64) -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            for t in frame.tabs.iter() {
                if let Some(tab) = t {
                    if tab.surface_id == surface_id {
                        return Some(frame.frame_id);
                    }
                }
            }
        }
    }
    None
}

/// Get the active surface_id for a given frame_id, if any.
/// Returns the surface_id of the active tab in the specified frame.
unsafe fn active_surface_for_frame(frame_id: u32) -> Option<u64> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    return Some(tab.surface_id);
                }
            }
        }
    }
    None
}

// ── Selected Window Options Model ──────────────────────────────────────────
/// Returns the frame_id of the currently selected/focused frame, if any.
unsafe fn selected_frame_id() -> Option<u32> {
    frame_for_surface(FOCUSED_SURFACE_ID)
}

/// Returns the surface_id of the currently selected surface, if valid.
unsafe fn selected_surface_id() -> Option<u64> {
    let sid = FOCUSED_SURFACE_ID;
    if sid != 0 && surface_is_alive(sid) { Some(sid) } else { None }
}

/// Compute the options mask for the currently selected window.
/// V1: MOVE is set for frame-owned surfaces. Other bits reserved.
/// No action behavior implemented — model only.
unsafe fn selected_window_options_mask() -> u32 {
    let mut mask = 0u32;
    if let Some(_fid) = selected_frame_id() {
        // Frame-owned surface: can be moved via rim drag.
        mask |= OPTION_MOVE;
        // Future: CLOSE if destroyable (not linen), ZOOM if resizable, MINIMIZE if minimizable.
    }
    // Non-frame surfaces get no options in V1 (standalone surfaces are legacy/app content).
    mask
}

/// Returns true if the given surface can be safely closed/destroyed.
/// OS-owned surfaces (linen, cursor, panels) and unknown surfaces cannot be closed.
unsafe fn is_closeable_surface(surface_id: u64) -> bool {
    match surface_id {
        SURFACE_ID_LINEN | SURFACE_ID_CURSOR
        | SURFACE_ID_LAUNCHER | SURFACE_ID_STATUS
        | SURFACE_ID_CLOCK | SURFACE_ID_BELL => false,
        _ => surface_is_alive(surface_id),
    }
}

/// Close the given surface: mark inactive via its alive flag, notify sexdisplay
/// via 0xEE opcode, and fall back focus if the closed surface was focused.
/// Reuses the same destroy mechanism as keyboard SurfaceAction::DestroyFocused.
/// Returns true if the surface was actually destroyed.
unsafe fn close_surface_from_frame_light(surface_id: u64) -> bool {
    if !surface_is_alive(surface_id) {
        return false;
    }
    match surface_id {
        SURFACE_ID_APP    => SURFACE_100_ALIVE = false,
        SURFACE_ID_STATIC => SURFACE_101_ALIVE = false,
        SURFACE_ID_TEST3  => SURFACE_102_ALIVE = false,
        SURFACE_ID_TEST4  => SURFACE_103_ALIVE = false,
        _ => return false, // unknown or non-closeable surface
    }
    pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
    // Focus fallback: if the closed surface was focused, clear_focus_if_dead
    // will auto-switch to the next alive surface in z-order.
    clear_focus_if_dead();
    true
}

/// Returns true if the given frame is currently minimized.
unsafe fn frame_is_minimized(frame_id: u32) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id && (frame.flags & FRAME_FLAG_MINIMIZED) != 0 {
                return true;
            }
        }
    }
    false
}

/// Set or clear the minimized flag on the given frame.
unsafe fn set_frame_minimized(frame_id: u32, minimized: bool) {
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if minimized {
                    frame.flags |= FRAME_FLAG_MINIMIZED;
                } else {
                    frame.flags &= !FRAME_FLAG_MINIMIZED;
                }
                break;
            }
        }
    }
}

/// Find the first minimized frame's ID, if any.
/// Used by keyboard restore to locate a frame to un-minimize.
unsafe fn first_minimized_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 {
                return Some(frame.frame_id);
            }
        }
    }
    None
}

/// Minimize the active surface of the given frame: hide via 0xEE, set flag,
/// clear focus and drag if the surface was focused or being dragged.
/// Returns true if the surface was actually hidden.
unsafe fn minimize_frame(frame_id: u32) -> bool {
    if frame_is_minimized(frame_id) {
        return false; // already minimized
    }
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return false,
    };
    if !surface_is_alive(surface_id) {
        return false;
    }
    // Mark frame as minimized.
    set_frame_minimized(frame_id, true);
    // Hide surface on display.
    pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
    // Clear drag if dragging this surface.
    clear_drag_if_dead();
    // Fall back focus if this surface was focused.
    clear_focus_if_dead();
    unsafe {
        static mut FRAME_MINIMIZE_BUDGET: u32 = 8;
        let b = &mut FRAME_MINIMIZE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.minimize] frame={} surface={}", frame_id, surface_id);
        }
    }
    true
}

/// Restore a minimized frame: re-activate its surface via 0xEC, clear the
/// minimized flag, and set focus to the restored surface.
/// Returns true if the frame was actually restored.
unsafe fn restore_minimized_frame(frame_id: u32) -> bool {
    if !frame_is_minimized(frame_id) {
        return false; // not minimized
    }
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return false,
    };
    if !surface_is_alive(surface_id) {
        return false;
    }
    // Clear minimized flag.
    set_frame_minimized(frame_id, false);
    // Re-activate surface on display via 0xEC upsert.
    // If frame was zoomed before minimize, restore to maximized geometry.
    if frame_is_zoomed(frame_id) {
        let (zx, zy, zw, zh) = layout_maximize();
        pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
            (zy as u64) << 32 | zx as u64,
            (zh as u64) << 32 | zw as u64);
        update_local_geometry(surface_id, zx, zy, zw, zh);
    } else {
        let bounds = get_surface_bounds(surface_id);
        if let Some((rx, ry, rw, rh)) = bounds {
            pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
                (ry as u64) << 32 | rx as u64,
                (rh as u64) << 32 | rw as u64);
        } else {
            return false; // geometry unavailable
        }
    }
    // Focus the restored surface.
    try_set_focus(surface_id);
    unsafe {
        static mut FRAME_RESTORE_BUDGET: u32 = 8;
        let b = &mut FRAME_RESTORE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.restore] frame={} surface={}", frame_id, surface_id);
        }
    }
    true
}

// ── Frame Zoom/Maximize Helpers ─────────────────────────────────────────────────

/// Returns true if the given frame is currently zoomed (maximized).
unsafe fn frame_is_zoomed(frame_id: u32) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id && (frame.flags & FRAME_FLAG_ZOOMED) != 0 {
                return true;
            }
        }
    }
    false
}

/// Set or clear the zoomed flag on the given frame.
unsafe fn set_frame_zoomed(frame_id: u32, zoomed: bool) {
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if zoomed {
                    frame.flags |= FRAME_FLAG_ZOOMED;
                } else {
                    frame.flags &= !FRAME_FLAG_ZOOMED;
                }
                break;
            }
        }
    }
}

/// Update local shell geometry statics to match a 0xEC update sent to sexdisplay.
/// This keeps shell hit-testing in sync with the display renderer.
/// Also fixes the stale-dimension bug on surface 100 (SURFACE_100_W/H were never
/// updated after resize operations).
unsafe fn update_local_geometry(surface_id: u64, x: i32, y: i32, w: u32, h: u32) {
    match surface_id {
        SURFACE_ID_APP => {
            WINDOWS[1].desc.x = x;
            WINDOWS[1].desc.y = y;
            WINDOWS[1].desc.width = w;
            WINDOWS[1].desc.height = h;
            SURFACE_100_W = w;
            SURFACE_100_H = h;
        }
        SURFACE_ID_STATIC => {
            SURFACE_101_X = x; SURFACE_101_Y = y;
            SURFACE_101_W = w; SURFACE_101_H = h;
        }
        SURFACE_ID_TEST3 => {
            SURFACE_102_X = x; SURFACE_102_Y = y;
            SURFACE_102_W = w; SURFACE_102_H = h;
        }
        SURFACE_ID_TEST4 => {
            SURFACE_103_X = x; SURFACE_103_Y = y;
            SURFACE_103_W = w; SURFACE_103_H = h;
        }
        _ => {}
    }
}

/// Zoom (maximize) the active surface of the given frame to fill the area below
/// the SilkBar. Saves the current normal geometry in ShellFrame.normal_* for
/// later unzoom. Sends 0xEC with layout_maximize() geometry to sexdisplay.
/// Returns true if the surface was actually zoomed.
unsafe fn zoom_frame(frame_id: u32) -> bool {
    if frame_is_zoomed(frame_id) {
        return false; // already zoomed
    }
    if frame_is_minimized(frame_id) {
        return false; // cannot zoom a minimized frame
    }
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return false,
    };
    if !surface_is_alive(surface_id) {
        return false;
    }
    // Save current normal geometry for unzoom.
    let bounds = get_surface_bounds(surface_id);
    let (nx, ny, nw, nh) = match bounds {
        Some(b) => b,
        None => return false,
    };
    // Store normal geometry in ShellFrame.
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                frame.normal_x = nx;
                frame.normal_y = ny;
                frame.normal_w = nw;
                frame.normal_h = nh;
                break;
            }
        }
    }
    // Set zoomed flag.
    set_frame_zoomed(frame_id, true);
    // Send maximized geometry to sexdisplay.
    let (zx, zy, zw, zh) = layout_maximize();
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (zy as u64) << 32 | zx as u64,
        (zh as u64) << 32 | zw as u64);
    // Update local geometry to match display.
    update_local_geometry(surface_id, zx, zy, zw, zh);
    // Preserve focus (zoom does not change focus).
    unsafe {
        static mut FRAME_ZOOM_BUDGET: u32 = 8;
        let b = &mut FRAME_ZOOM_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.zoom] frame={} surface={}", frame_id, surface_id);
        }
    }
    true
}

/// Unzoom (restore) the active surface of the given frame to its saved normal
/// geometry. Sends 0xEC with the stored ShellFrame.normal_* rect to sexdisplay.
/// Returns true if the surface was actually unzoomed.
unsafe fn unzoom_frame(frame_id: u32) -> bool {
    if !frame_is_zoomed(frame_id) {
        return false; // not zoomed
    }
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return false,
    };
    if !surface_is_alive(surface_id) {
        return false;
    }
    // Retrieve normal geometry from ShellFrame.
    let (nx, ny, nw, nh) = match FRAMES.iter().find_map(|f| {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                Some((frame.normal_x, frame.normal_y, frame.normal_w, frame.normal_h))
            } else { None }
        } else { None }
    }) {
        Some(b) => b,
        None => return false,
    };
    // Clear zoomed flag.
    set_frame_zoomed(frame_id, false);
    // Send normal geometry to sexdisplay.
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (ny as u64) << 32 | nx as u64,
        (nh as u64) << 32 | nw as u64);
    // Update local geometry to match display.
    update_local_geometry(surface_id, nx, ny, nw, nh);
    // Preserve focus.
    unsafe {
        static mut FRAME_UNZOOM_BUDGET: u32 = 8;
        let b = &mut FRAME_UNZOOM_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.unzoom] frame={} surface={}", frame_id, surface_id);
        }
    }
    true
}

/// Toggle zoom state for the given frame. If zoomed, unzoom. If not zoomed, zoom.
/// Returns true if the state changed.
unsafe fn toggle_zoom_frame(frame_id: u32) -> bool {
    if frame_is_zoomed(frame_id) {
        unzoom_frame(frame_id)
    } else {
        zoom_frame(frame_id)
    }
}

/// Map a Frame Light kind to its corresponding selected-window option bit.
/// Returns 0 for FRAME_LIGHT_NONE or unrecognized light kinds.
fn frame_light_to_option_mask(light: u32) -> u32 {
    match light {
        FRAME_LIGHT_CLOSE => OPTION_CLOSE,
        FRAME_LIGHT_MINIMIZE => OPTION_MINIMIZE,
        FRAME_LIGHT_ZOOM => OPTION_ZOOM,
        _ => 0,
    }
}

/// Detect which Frame Light the pointer is over, based on position within
/// the top rim band of a frame-owned surface. Lights are 4×4 squares at the
/// top-left of the surface, within the rim. Order: CLOSE, MINIMIZE, ZOOM
/// (left to right). Returns FRAME_LIGHT_NONE if not over any light.
unsafe fn frame_light_at(frame_id: u32, x: i32, y: i32) -> u32 {
    // Resolve active surface for this frame.
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return FRAME_LIGHT_NONE,
    };
    // Get surface bounds for light geometry.
    let bounds = match get_surface_bounds(surface_id) {
        Some(b) => b,
        None => return FRAME_LIGHT_NONE,
    };
    let (sx, sy, _sw, _sh) = bounds;

    // Lights live in the top rim band only (height = FRAME_RIM_PX).
    let top_rim_bottom = sy + FRAME_RIM_PX;
    if y < sy || y >= top_rim_bottom {
        return FRAME_LIGHT_NONE;
    }

    // X position relative to surface left edge.
    let lx = x - sx;

    // CLOSE: gap from left edge.
    if lx >= FRAME_LIGHT_GAP_PX && lx < FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX {
        return FRAME_LIGHT_CLOSE;
    }
    // MINIMIZE: gap + size + gap.
    let l2_start = FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX;
    if lx >= l2_start && lx < l2_start + FRAME_LIGHT_SIZE_PX {
        return FRAME_LIGHT_MINIMIZE;
    }
    // ZOOM: gap + size + gap + size + gap.
    let l3_start = l2_start + FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX;
    if lx >= l3_start && lx < l3_start + FRAME_LIGHT_SIZE_PX {
        return FRAME_LIGHT_ZOOM;
    }

    FRAME_LIGHT_NONE
}

// ── Frame Chrome Hover Update ──────────────────────────────────────────────────
/// Update frame chrome hover state from current pointer position.
/// Called once per event loop iteration. Skips during active drag.
/// Returns true if hover state changed (for diagnostic gating).
/// Does not modify focus, drag, or any interaction state.
/// Does not produce FrameChrome hits yet — V1 only maps Surface hits to frames.
unsafe fn update_frame_hover_at(x: i32, y: i32) -> bool {
    // Skip during active drag — pointer is captured by drag action.
    if matches!(INTERACTION, InteractionState::Dragging { .. }) {
        return false;
    }

    let new_light: u32;
    let (new_frame_id, new_kind) = if y < P.bar_height {
        // SilkBar area: no frame hover, no light hover.
        new_light = FRAME_LIGHT_NONE;
        (0u32, HOVER_NONE)
    } else {
        let target = hit_test_at(x, y);
        match target {
            HitTarget::Surface(sid) => {
                match frame_for_surface(sid) {
                    Some(fid) => {
                        new_light = frame_light_at(fid, x, y);
                        (fid, HOVER_FRAME_BODY)
                    }
                    None => {
                        new_light = FRAME_LIGHT_NONE;
                        (0u32, HOVER_NONE)
                    }
                }
            }
            HitTarget::FrameChrome { frame_id, kind } => {
                // Detect which frame light (close/minimize/zoom) the pointer
                // is over within the top rim band of the frame's active surface.
                new_light = frame_light_at(frame_id, x, y);
                (frame_id, kind)
            }
            HitTarget::None => {
                new_light = FRAME_LIGHT_NONE;
                (0u32, HOVER_NONE)
            }
        }
    };

    let changed = HOVERED_FRAME_ID != new_frame_id || HOVER_KIND != new_kind;
    let light_changed = HOVERED_FRAME_LIGHT != new_light;
    if changed || light_changed {
        unsafe {
            static mut HOVER_STATE_CHANGE_BUDGET: u32 = 6;
            let b = &mut HOVER_STATE_CHANGE_BUDGET;
            if *b > 0 {
                *b -= 1;
                if new_frame_id != 0 {
                    serial_println!("[shell.frame.hover.set] frame={} kind={}", new_frame_id, new_kind);
                } else {
                    serial_println!("[shell.frame.hover.clear]");
                }
            }
        }
        HOVERED_FRAME_ID = new_frame_id;
        HOVER_KIND = new_kind;
        HOVERED_FRAME_LIGHT = new_light;
        if light_changed {
            unsafe {
                static mut FRAME_LIGHT_HOVER_BUDGET: u32 = 8;
                let b = &mut FRAME_LIGHT_HOVER_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[shell.frame.light.hover] frame={} light={}",
                        new_frame_id, new_light);
                }
            }
        }
    }
    changed || light_changed
}

/// Try to set focus to a surface. Returns true if focus was applied.
/// Guards: surface must be focusable and alive.
/// Clearing focus (sid=0) is always allowed (resets to no surface).
/// Emits unbudgeted reject markers for nonfocusable or dead surfaces.
/// On success, emits [shell.focus.set] or [shell.focus.clear] markers
/// for deterministic focus-tracking in synthetic proofs.
/// On success, also emits budgeted [shell.selected.options] marker
/// with the selected frame/surface and computed options mask.
unsafe fn try_set_focus(sid: u64) -> bool {
    if sid == 0 {
        FOCUSED_SURFACE_ID = 0;
        pdx_call(SLOT_DISPLAY, 0xED, 0, 0, 0);
        serial_println!("[shell.focus.clear] id=0");
        // Selected window: focus cleared → no selection, mask=0.
        unsafe {
            static mut SELECTED_OPTIONS_CLEAR_BUDGET: u32 = 4;
            let b = &mut SELECTED_OPTIONS_CLEAR_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[shell.selected.options] frame=0 surface=0 mask=0");
            }
        }
        // Send cleared options mask to silkbar.
        pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 0, 0, 0);
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
    serial_println!("[shell.focus.set] id={}", sid);
    pdx_call(SLOT_DISPLAY, 0xED, sid, 0, 0);
    // Selected window options: frame, surface, and computed mask.
    unsafe {
        static mut SELECTED_OPTIONS_SET_BUDGET: u32 = 8;
        let b = &mut SELECTED_OPTIONS_SET_BUDGET;
        if *b > 0 {
            *b -= 1;
            let frame = selected_frame_id();
            let mask = selected_window_options_mask();
            serial_println!("[shell.selected.options] frame={} surface={} mask={:#x}",
                match frame { Some(f) => f as u64, None => 0 },
                sid, mask);
        }
    }
    // Send live options mask to silkbar for display.
    let live_mask = selected_window_options_mask();
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, live_mask as u64, 0);
    unsafe {
        static mut SELECTED_OPTIONS_SEND_BUDGET: u32 = 8;
        let b = &mut SELECTED_OPTIONS_SEND_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.selected.options.send] surface={} mask={:#x}", sid, live_mask);
        }
    }
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

/// Move the surface tracked by the current Drag state by (dx, dy).
/// Returns true if the surface was actually moved.
/// Uses the Drag state's recorded surface_id, not FOCUSED_SURFACE_ID,
/// so keyboard focus changes during drag do not corrupt the target.
unsafe fn drag_move_focused(dx: i32, dy: i32) -> bool {
    if let InteractionState::Dragging { surface_id, .. } = INTERACTION {
        let mut moved = false;
        if surface_id == SURFACE_ID_APP && SURFACE_100_ALIVE {
            if let Some(w) = WINDOWS.get_mut(1) {
                w.desc.x = w.desc.x.wrapping_add(dx);
                w.desc.y = w.desc.y.wrapping_add(dy);
                let (cx, cy) = clamp_position(w.desc.x, w.desc.y, SURFACE_100_W, SURFACE_100_H);
                w.desc.x = cx; w.desc.y = cy;
                moved = true;
            }
        } else if surface_id == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
            SURFACE_101_X = SURFACE_101_X.wrapping_add(dx);
            SURFACE_101_Y = SURFACE_101_Y.wrapping_add(dy);
            let (cx, cy) = clamp_position(SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H);
            SURFACE_101_X = cx; SURFACE_101_Y = cy;
            moved = true;
        } else if surface_id == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
            SURFACE_102_X = SURFACE_102_X.wrapping_add(dx);
            SURFACE_102_Y = SURFACE_102_Y.wrapping_add(dy);
            let (cx, cy) = clamp_position(SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H);
            SURFACE_102_X = cx; SURFACE_102_Y = cy;
            moved = true;
        } else if surface_id == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
            SURFACE_103_X = SURFACE_103_X.wrapping_add(dx);
            SURFACE_103_Y = SURFACE_103_Y.wrapping_add(dy);
            let (cx, cy) = clamp_position(SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H);
            SURFACE_103_X = cx; SURFACE_103_Y = cy;
            moved = true;
        }
        if moved {
            serial_println!("[shell.drag.move] id={} x={} y={} dx={} dy={}", surface_id, POINTER_X, POINTER_Y, dx, dy);
            serial_println!("[shell.drag.send.ok] id={}", surface_id);
            // Integrated contract diagnostic: logs drag target surface_id and
            // current FOCUSED_SURFACE_ID. When id == focus, drag target matches
            // the focused surface (normal case). If a FocusToggle occurs during
            // drag, they would differ, proving drag_move_focused reads from
            // InteractionState::Dragging, not FOCUSED_SURFACE_ID.
            unsafe {
                static mut INTEGRATED_DRAG_TARGET_BUDGET: u32 = 4;
                let b = &mut INTEGRATED_DRAG_TARGET_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[shell.integrated.drag_target] id={} focus={}", surface_id, FOCUSED_SURFACE_ID);
                }
            }
        }
        moved
    } else {
        false
    }
}

/// Check if (x, y) hits frame chrome (rim or tab strip) for a given surface.
/// Priority: tab strip > rim > None (content area).
/// Returns None if the surface has no frame, or point is in content area.
unsafe fn hit_test_surface_chrome(x: i32, y: i32, sid: u64) -> Option<HitTarget> {
    let bounds = get_surface_bounds(sid)?;
    let (sx, sy, sw, sh) = bounds;
    // Find the frame that owns this surface — no chrome for unowned surfaces (linen, standalone).
    let frame_id = frame_for_surface(sid)?;

    // Tab strip (top band): highest priority. Gated on FRAME_TAB_STRIP_PX > 0.
    if FRAME_TAB_STRIP_PX > 0 {
        let strip_top = sy;
        let strip_bot = sy + FRAME_TAB_STRIP_PX;
        if y >= strip_top && y < strip_bot && x >= sx && x < (sx + sw as i32) {
            return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
        }
    }

    // Rim (edge band): check all four edges of the surface.
    let right = sx + sw as i32 - 1;
    let bottom = sy + sh as i32 - 1;
    let in_rim =
        (x >= sx && x < sx + FRAME_RIM_PX)                            // left edge
        || (x > right - FRAME_RIM_PX && x <= right)                   // right edge
        || (y >= sy && y < sy + FRAME_RIM_PX)                         // top edge
        || (y > bottom - FRAME_RIM_PX && y <= bottom);                // bottom edge
    if in_rim {
        return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_RIM });
    }

    None // content area — not a chrome hit
}

/// Perform a pure hit-test at (x, y), returning the typed target.
/// Priority order: focused surface (with chrome check) → z-order (with chrome check) → None.
/// Produces FrameChrome targets for rim/tab-strip hits on frame-owned surfaces.
/// Does NOT check SilkBar or trigger any side effects.
/// SilkBar intercept is handled separately by handle_silkbar_click().
unsafe fn hit_test_at(x: i32, y: i32) -> HitTarget {
    let focused = FOCUSED_SURFACE_ID;
    if point_in_surface(x, y, focused) {
        // Chrome check: rim/tab-strip takes priority over content area.
        if let Some(chrome_target) = hit_test_surface_chrome(x, y, focused) {
            if let HitTarget::FrameChrome { frame_id, kind } = chrome_target {
                unsafe {
                    static mut CHROME_HIT_PRODUCED_BUDGET: u32 = 6;
                    let b = &mut CHROME_HIT_PRODUCED_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[shell.hit_target.chrome] frame={} kind={} x={} y={}",
                            frame_id, kind, x, y);
                    }
                }
            }
            return chrome_target;
        }
        return HitTarget::Surface(focused);
    }
    let z_order = [SURFACE_ID_LINEN, SURFACE_ID_TEST4,
                   SURFACE_ID_TEST3, SURFACE_ID_STATIC, SURFACE_ID_APP];
    for &sid in &z_order {
        if sid == focused { continue; }
        if !surface_is_alive(sid) {
            serial_println!("[shell.hit_test.skip] id={} reason=dead", sid);
            continue;
        }
        if point_in_surface(x, y, sid) {
            // Chrome check for z-order surfaces too (if frame-owned).
            if let Some(chrome_target) = hit_test_surface_chrome(x, y, sid) {
                if let HitTarget::FrameChrome { frame_id, kind } = chrome_target {
                    unsafe {
                        static mut CHROME_HIT_Z_BUDGET: u32 = 4;
                        let b = &mut CHROME_HIT_Z_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[shell.hit_target.chrome] frame={} kind={} x={} y={} z=1",
                                frame_id, kind, x, y);
                        }
                    }
                }
                return chrome_target;
            }
            return HitTarget::Surface(sid);
        }
    }
    HitTarget::None
}

/// Extract the surface_id from a HitTarget, if it represents a surface hit.
unsafe fn hit_target_surface(target: HitTarget) -> Option<u64> {
    match target {
        HitTarget::Surface(sid) => Some(sid),
        _ => None,
    }
}

/// Classify a hit-target + silkbar flag into the diagnostic label used by
/// budgeted [shell.click.real.target] markers.
fn hit_target_label(target: HitTarget, silkbar_handled: bool) -> (&'static str, u64) {
    if silkbar_handled {
        ("chrome", 0u64)
    } else {
        match target {
            HitTarget::Surface(sid) => ("app", sid),
            HitTarget::FrameChrome { frame_id, .. } => ("chrome_frame", frame_id as u64),
            HitTarget::None => ("none", 0u64),
        }
    }
}

/// Perform hit-test at (px, py) and update focus if a different surface is hit.
/// Priority order: focused surface → z-order fallback → None.
/// FrameChrome rim hits start a frame-resolved surface drag without focus change.
/// FrameChrome non-rim hits (tab strip, reserved) are captured as no-op.
/// Returns the typed HitTarget and whether SilkBar handled the click.
/// SilkBar intercept runs after hit-test but before drag starts.
/// Emits [shell.click_focus.down/hit/miss] markers.
/// Emits [shell.frame.rim.drag.start] for rim drag, [shell.frame.chrome.capture] for other chrome.
unsafe fn click_hit_test_and_focus(px: i32, py: i32, buttons_val: u8) -> (HitTarget, bool) {
    serial_println!("[shell.click_focus.down] x={} y={} buttons={:#x}", px, py, buttons_val);
    let target = hit_test_at(px, py);
    match target {
        HitTarget::Surface(sid) => {
            if sid != FOCUSED_SURFACE_ID {
                serial_println!("[shell.click_focus.hit] id={}", sid);
                serial_println!("[shell.click_focus.send.start] id={}", sid);
                try_set_focus(sid);
                serial_println!("[shell.click_focus.send.ok] id={}", sid);
            } else {
                serial_println!("[shell.click_focus.hit] id={}", sid);
            }
        }
        HitTarget::None => {
            serial_println!("[shell.click_focus.miss]");
        }
        HitTarget::FrameChrome { frame_id, kind } => {
            if kind == FRAME_CHROME_RIM {
                // Check if pointer is over a Frame Light before proceeding.
                let light = frame_light_at(frame_id, px, py);
                if light == FRAME_LIGHT_CLOSE {
                    // ── CLOSE action: destroy active surface ──
                    if let Some(surface_id) = active_surface_for_frame(frame_id) {
                        if is_closeable_surface(surface_id) {
                            if close_surface_from_frame_light(surface_id) {
                                unsafe {
                                    static mut FRAME_LIGHT_CLOSE_BUDGET: u32 = 8;
                                    let b = &mut FRAME_LIGHT_CLOSE_BUDGET;
                                    if *b > 0 {
                                        *b -= 1;
                                        serial_println!("[shell.frame.light.close] frame={} surface={}",
                                            frame_id, surface_id);
                                    }
                                }
                            } else {
                                serial_println!("[shell.frame.light.close.reject] frame={} surface={} reason=failed",
                                    frame_id, surface_id);
                            }
                        } else {
                            serial_println!("[shell.frame.light.close.reject] frame={} surface={} reason=not_closeable",
                                frame_id, surface_id);
                        }
                    } else {
                        serial_println!("[shell.frame.light.close.reject] frame={} reason=no_active_surface",
                            frame_id);
                    }
                } else if light == FRAME_LIGHT_MINIMIZE {
                    // ── MINIMIZE action: hide active surface ──
                    if !minimize_frame(frame_id) {
                        unsafe {
                            static mut FRAME_MINIMIZE_REJECT_BUDGET: u32 = 4;
                            let b = &mut FRAME_MINIMIZE_REJECT_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[shell.frame.minimize.reject] frame={} reason=not_minimizable",
                                    frame_id);
                            }
                        }
                    }
                } else if light == FRAME_LIGHT_ZOOM {
                    // ── ZOOM action: toggle zoom/unzoom ──
                    if !toggle_zoom_frame(frame_id) {
                        unsafe {
                            static mut FRAME_ZOOM_REJECT_BUDGET: u32 = 4;
                            let b = &mut FRAME_ZOOM_REJECT_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[shell.frame.zoom.reject] frame={} reason=not_zoomable",
                                    frame_id);
                            }
                        }
                    }
                } else {
                    // ── Rim drag (no light hovered) ──
                    if let Some(surface_id) = active_surface_for_frame(frame_id) {
                        if surface_is_alive(surface_id) {
                            try_transition(InteractionState::Dragging { surface_id, current_x: px, current_y: py });
                            unsafe {
                                static mut RIM_DRAG_START_BUDGET: u32 = 8;
                                let b = &mut RIM_DRAG_START_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!("[shell.frame.rim.drag.start] frame={} surface={} x={} y={}",
                                        frame_id, surface_id, px, py);
                                }
                            }
                        } else {
                            serial_println!("[shell.frame.rim.drag.reject] frame={} reason=dead", frame_id);
                        }
                    } else {
                        serial_println!("[shell.frame.rim.drag.reject] frame={} reason=no_active_surface", frame_id);
                    }
                }
            } else {
                // Non-rim chrome (tab strip, reserved): capture/no-op.
                unsafe {
                    static mut CHROME_CAPTURE_BUDGET: u32 = 4;
                    let b = &mut CHROME_CAPTURE_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[shell.frame.chrome.capture] frame={} kind={} x={} y={}",
                            frame_id, kind, px, py);
                    }
                }
            }
        }
    }
    // SilkBar intercept: if pointer is in top strip, handle and skip drag
    let silkbar_handled = handle_silkbar_click(px, py);
    // Drag-start only on content area (not chrome rim/tab strip).
    // Rim drag is already started in the match arm above — skip content drag and skip the
    // "drag skipped" diagnostic for rim. Non-rim chrome remains a no-op with diagnostic.
    let is_content_hit = matches!(target, HitTarget::Surface(..) | HitTarget::None);
    if !silkbar_handled && is_content_hit && is_shell_surface(FOCUSED_SURFACE_ID)
        && point_in_surface(px, py, FOCUSED_SURFACE_ID)
    {
        try_transition(InteractionState::Dragging { surface_id: FOCUSED_SURFACE_ID, current_x: px, current_y: py });
        serial_println!("[shell.drag.start] id={} x={} y={}", FOCUSED_SURFACE_ID, px, py);
    } else if !silkbar_handled && matches!(target, HitTarget::FrameChrome { kind: FRAME_CHROME_TAB_STRIP, .. }) {
        serial_println!("[shell.drag.skip.chrome] kind=tab_strip x={} y={}", px, py);
    }
    (target, silkbar_handled)
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

        // Initialize frame chrome model: one frame, one tab wrapping surface 100.
        // normal_* initialized to boot geometry of surface 100 for zoom restore.
        let boot_x: i32 = 100;
        let boot_y: i32 = 100;
        let boot_w: u32 = 800;
        let boot_h: u32 = 500;
        FRAMES[0] = Some(ShellFrame {
            frame_id: 1,
            active_tab: 0,
            tab_count: 1,
            tabs: [
                Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
                None, None, None, None, None, None, None,
            ],
            flags: 0,
            normal_x: boot_x,
            normal_y: boot_y,
            normal_w: boot_w,
            normal_h: boot_h,
        });
        serial_println!("[shell.frame.model.init] frames=1 tabs=1");

        sys_set_state(SVC_STATE_LISTENING);
    }
    serial_println!("[silk-shell] AUTHORITATIVE WM LISTENING (PDX SLOT 6)");

    // Stage 2B: advertise workspace 0 active to SilkBar
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, 0, 0, 0);
    // Stage 2C: focus advertisement (shell) with initial selected-window options mask.
    let boot_options_mask = unsafe { selected_window_options_mask() };
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, boot_options_mask as u64, 0);
    serial_println!("[silk-shell] Boot workspace advertisement sent to SilkBar");

    // Frame Lights model: prove constants and mapping exist.
    unsafe {
        static mut FRAME_LIGHT_MODEL_BUDGET: u32 = 1;
        if FRAME_LIGHT_MODEL_BUDGET > 0 {
            FRAME_LIGHT_MODEL_BUDGET -= 1;
            serial_println!("[shell.frame.light.model] close={} minimize={} zoom={} mask={:#x}",
                FRAME_LIGHT_CLOSE, FRAME_LIGHT_MINIMIZE, FRAME_LIGHT_ZOOM,
                frame_light_to_option_mask(FRAME_LIGHT_ZOOM));
        }
    }

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
                        // POINTER_X/Y is NOT updated from dx/dy here.
                        // Real USB cursor movement comes from HID EV_REL (forwarded by sexinput's
                        // normalizer). This eliminates the dx/dy double-apply bug where both the USB
                        // handler and the HID EV_REL handler applied the same delta.
                        // The synthetic click-focus proof uses EV_ABS before button down to set
                        // the click position explicitly, so it is unaffected by this change.
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
                            let (target, silkbar_handled) = click_hit_test_and_focus(POINTER_X, POINTER_Y, buttons);
                            // Budgeted real-click target marker.
                            // Fires for OP_USB_MOUSE_REPORT path (real USB + synthetic click-focus proof).
                            // Budget 16: synthetic proof consumes ~1 slot, real clicks use the rest.
                            unsafe {
                                static mut CLICK_REAL_TARGET_BUDGET: u32 = 16;
                                let rem = &mut CLICK_REAL_TARGET_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    let (kind, target_id) = hit_target_label(target, silkbar_handled);
                                    serial_println!("[shell.click.real.target] x={} y={} target={} kind={}",
                                        POINTER_X, POINTER_Y, target_id, kind);
                                }
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
                        // ── USB drag movement: move drag target surface by delta while button held ──
                        // clear_drag_if_dead() transitions to Idle if the drag target died,
                        // so the next call will naturally skip movement.
                        // Uses the Drag state's recorded surface_id (not FOCUSED_SURFACE_ID)
                        // so focus changes during drag do not corrupt the target.
                        clear_drag_if_dead();
                        if drag_move_focused(dx as i32, dy as i32) {
                            mutated = true;
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

                                    SurfaceAction::RestoreMinimized => {
                                        if let Some(frame_id) = first_minimized_frame_id() {
                                            if restore_minimized_frame(frame_id) {
                                                mutated = true;
                                                serial_println!("[silk-shell] Restored minimized frame {}", frame_id);
                                            }
                                        } else {
                                            serial_println!("[silk-shell] No minimized frame to restore");
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
                            // Budgeted liveness: shell received EV_REL from sexinput.
                            unsafe {
                                static mut HID_REL_LIVE_BUDGET: u32 = 16;
                                let rem = &mut HID_REL_LIVE_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[shell.hid.rel.live] n=0 x={} y={} dx={} dy={}",
                                        POINTER_X, POINTER_Y, dx, dy);
                                }
                            }
                            // Initialize POINTER_X/Y on first EV_REL (real USB path).
                            // The USB handler also does this for OP_USB_MOUSE_REPORT path
                            // (synthetic proof). Whichever fires first sets the center position.
                            if !POINTER_USB_STATE_INIT {
                                POINTER_X = P.width / 2;
                                POINTER_Y = P.height / 2;
                                POINTER_USB_STATE_INIT = true;
                            }
                            POINTER_X = POINTER_X.wrapping_add(dx);
                            POINTER_Y = POINTER_Y.wrapping_add(dy);

                            // ── Drag movement: move drag target surface by delta while button held ──
                            // clear_drag_if_dead() transitions to Idle if the drag target died,
                            // so the next call will naturally skip movement.
                            // Uses the Drag state's recorded surface_id (not FOCUSED_SURFACE_ID)
                            // so focus changes during drag do not corrupt the target.
                            clear_drag_if_dead();
                            if drag_move_focused(dx, dy) {
                                mutated = true;
                            }

                            serial_println!("[silk-shell] Pointer REL d=({},{}) pos=({},{})",
                                dx, dy, POINTER_X, POINTER_Y);
                            // Budgeted marker: shell sends cursor surface update to display.
                            unsafe {
                                static mut SHELL_CURSOR_SURFACE_UPDATE_BUDGET: u32 = 16;
                                let rem = &mut SHELL_CURSOR_SURFACE_UPDATE_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[shell.cursor.surface.update] n=0 x={} y={}", POINTER_X, POINTER_Y);
                                }
                            }
                            // Move cursor surface to updated pointer position.
                            serial_println!("[shell.cursor_surface.move.start] id={:#x} x={} y={}", SURFACE_ID_CURSOR, POINTER_X, POINTER_Y);
                            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, POINTER_X as u64, POINTER_Y as u64);
                            serial_println!("[shell.cursor_surface.move.ok]");
                            // Budgeted diagnostic for cursor position after HID REL event (real USB movement).
                            unsafe {
                                static mut CURSOR_MOVE_BUDGET_REL: u32 = 16;
                                let remaining = &mut CURSOR_MOVE_BUDGET_REL;
                                if *remaining > 0 {
                                    *remaining -= 1;
                                    serial_println!("[shell.cursor.move] x={} y={}", POINTER_X, POINTER_Y);
                                }
                            }
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
                                    let (target, silkbar_handled) = click_hit_test_and_focus(POINTER_X, POINTER_Y, POINTER_BUTTONS);
                                    // Budgeted real-click target marker.
                                    // Fires for HID EV_BTN path (real USB buttons + synthetic drag proof).
                                    unsafe {
                                        static mut CLICK_REAL_TARGET_BUDGET_BTN: u32 = 16;
                                        let rem = &mut CLICK_REAL_TARGET_BUDGET_BTN;
                                        if *rem > 0 {
                                            *rem -= 1;
                                            let (kind, target_id) = hit_target_label(target, silkbar_handled);
                                            serial_println!("[shell.click.real.target] x={} y={} target={} kind={}",
                                                POINTER_X, POINTER_Y, target_id, kind);
                                        }
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

        // ── Frame chrome hover update (once per event, after all state updates) ──
        unsafe {
            update_frame_hover_at(POINTER_X, POINTER_Y);
        }

        if mutated {
            emit_snapshot();
        }

        sys_yield();
    }
}
