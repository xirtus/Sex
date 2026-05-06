#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_listen_raw, pdx_reply, sys_yield, sys_set_state, serial_println, WindowDescriptor,
    SLOT_DISPLAY, SLOT_SILKBAR, SLOT_SEXSTORE, SLOT_QUIL, OP_QUIL_PING,
    OP_SILKBAR_WORKSPACE_ACTIVE, OP_SILKBAR_FOCUS_STATE,
    OP_SURFACE_TAB_INFO, OP_APPEARANCE_TOKENS,
    SVC_STATE_LISTENING, ERR_CAP_INVALID, EV_KEY, EV_REL, EV_ABS, EV_BTN,
};
use silkbar_model::{DEFAULT_SILK_BAR, hit_test_action, Action, PANEL_X, PANEL_Y, PANEL_W, PANEL_H};

// Local Opcodes
pub const OP_DISPLAY_SET_SNAPSHOT: u64 = 0x15;

// Sexstore K/V opcodes (local copies to avoid sex-pdx ABI hash update).
// Matches servers/sexstore/src/main.rs.
const OP_KV_GET: u64 = 0xB0;
const OP_KV_PUT: u64 = 0xB1;

// Scene Settings protocol opcode (local; promote to sex-pdx when Settings app PD exists).
const OP_SCENE_SETTINGS_CMD: u64 = 0xFB;

// Scene Settings command IDs (arg0 values for OP_SCENE_SETTINGS_CMD).
const CMD_SET_PRESET: u64 = 1;
const CMD_CYCLE_PRESET: u64 = 2;
const CMD_SET_TINT: u64 = 3;
const CMD_CYCLE_TINT: u64 = 4;
const CMD_SET_CHROME_FLAGS: u64 = 5;
const CMD_TOGGLE_TOP_BAR: u64 = 6;
const CMD_SET_ACCESSIBILITY: u64 = 7;
const CMD_RESET_DEFAULTS: u64 = 8;

// Scene Settings protocol synthetic proof gate.
// Build with SEXOS_SCENE_SETTINGS_PROTOCOL_PROOF=1 to enable.
// Default (unset): zero behavior change.
const SCENE_SETTINGS_PROTOCOL_PROOF_ENABLED: bool =
    option_env!("SEXOS_SCENE_SETTINGS_PROTOCOL_PROOF").is_some();

/// Synthetic proof stage counter. Advances 0..4 then stops forever.
static mut SCENE_SETTINGS_PROTOCOL_PROOF_STAGE: u8 = 0;

/// App surface request protocol synthetic proof gate.
/// Build with SEXOS_APP_SURFACE_REQ_PROOF=1 to enable.
/// Default (unset): zero behavior change.
const APP_SURFACE_REQ_PROOF_ENABLED: bool =
    option_env!("SEXOS_APP_SURFACE_REQ_PROOF").is_some();

/// Synthetic proof stage counter for app surface request proof. Advances 0..3 then stops.
static mut APP_SURFACE_REQ_PROOF_STAGE: u8 = 0;

// Well-known key ID for scene appearance settings blob.
const SCENE_SETTINGS_KEY_APPEARANCE: u64 = 0x01;

// Packed blob magic/version constants (byte 0, byte 1 in the u64).
const SCENE_BLOB_MAGIC:   u8 = 0xAC;
const SCENE_BLOB_VERSION: u8 = 0x01;
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;
pub const OP_HID_EVENT: u64 = 0x202;
pub const OP_USB_MOUSE_REPORT: u64 = 0x260;
const SHELL_USB_MOUSE_RECEIVE_UNPARK_PROOF_V1: bool = true;
pub const OP_SURFACE_UPDATE: u64 = 0xEB;

/// App surface request: app-like PD → silk-shell via SLOT_SHELL (6).
/// arg0 = surface_id (must be >= 200 for user surfaces, non-zero)
/// arg1 = title_id (opaque u64 for tab title, must be non-zero)
/// arg2 = reserved (future: packed geometry)
/// Shell validates and if accepted: creates Frame+Tab, registers lifecycle,
/// and upserts on sexdisplay via 0xEC. App never writes framebuffer.
pub const OP_APP_SURFACE_REQ: u64 = 0xFA;

pub const SURFACE_ID_APP: u64 = 100;
pub const SURFACE_ID_STATIC: u64 = 101;
pub const SURFACE_ID_TEST3: u64 = 102;
pub const SURFACE_ID_TEST4: u64 = 103;
pub const SURFACE_ID_LINEN: u64 = 200;
pub const SURFACE_ID_QUIL: u64 = 201;
pub const SURFACE_ID_MESH: u64 = 202;
pub const SURFACE_ID_COLLAR: u64 = 203;
pub const SURFACE_ID_BELL_PLACEHOLDER: u64 = 204;
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
//   0x95  bell panel
//   0x96  scene settings panel
//   100+  app surfaces (SURFACE_ID_APP, SURFACE_ID_STATIC, etc.)
//   200   linen (file viewer)
//   201   quil (editor stub)
//   202   mesh (diagnostic graph placeholder)
//   203   collar (authority placeholder)
//   204   bell placeholder (attention firewall)
//   0x98  command palette (action router)
pub const SURFACE_ID_SCENE_SETTINGS: u64 = 0x96;
pub const SURFACE_ID_ATLAS_OVERLAY: u64 = 0x97; // 151 — Atlas overview surface, toggled by F10
pub const SURFACE_ID_COMMAND_PALETTE: u64 = 0x98; // 152 — Command palette overlay, toggled by backtick
pub const SURFACE_ID_SPINDLE: u64 = 0x99;        // 153 — Spindle terminal console, toggled by Scroll Lock

/// Placeholder grant_ref value: no real Collar grant exists yet.
/// All current object/buffer grant_ref fields use this value.
/// Non-zero grant_ref would indicate a real Collar capability grant (deferred).
pub const GRANT_REF_STUB: u64 = 0;

/// 0xEE — deactivate surface on sexdisplay (active=false).
/// Sexdisplay does NOT free resources — callers must manage lifecycle.
/// Used for both permanent destroy AND temporary hide; the shell's
/// lifecycle FSM (A3/A6) tracks the semantic difference.
pub const OP_SURFACE_DEACTIVATE: u64 = 0xEE;

// ── App Surface Registry ──────────────────────────────────────────────────────
// Compile-time registry for OS-managed app surfaces (frame-owned, shell-tracked).
// Provides documentation, startup duplicate validation, and optional lookup.
// Lookup helpers use the registry for app surface focusable/closeable conformance.
// surface_is_alive remains hardcoded (no alive field in spec).

/// Specification for a shell-managed app surface.
struct AppSurfaceSpec {
    surface_id: u64,
    frame_id: u32,
    name: &'static str,
    boot_x: i32,
    boot_y: i32,
    boot_w: u32,
    boot_h: u32,
    closeable: bool,
    focusable: bool,
}

/// Known OS-managed app surfaces. Validated at boot for duplicates.
const APP_SURFACES: [AppSurfaceSpec; 7] = [
    AppSurfaceSpec {
        surface_id: SURFACE_ID_LINEN,
        frame_id: LINEN_FRAME_ID,
        name: "linen",
        boot_x: LINEN_BOOT_X,
        boot_y: LINEN_BOOT_Y,
        boot_w: LINEN_BOOT_W,
        boot_h: LINEN_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_QUIL,
        frame_id: QUIL_FRAME_ID,
        name: "quil",
        boot_x: QUIL_BOOT_X,
        boot_y: QUIL_BOOT_Y,
        boot_w: QUIL_BOOT_W,
        boot_h: QUIL_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_MESH,
        frame_id: MESH_FRAME_ID,
        name: "mesh",
        boot_x: MESH_BOOT_X,
        boot_y: MESH_BOOT_Y,
        boot_w: MESH_BOOT_W,
        boot_h: MESH_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_COLLAR,
        frame_id: COLLAR_FRAME_ID,
        name: "collar",
        boot_x: COLLAR_BOOT_X,
        boot_y: COLLAR_BOOT_Y,
        boot_w: COLLAR_BOOT_W,
        boot_h: COLLAR_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_BELL_PLACEHOLDER,
        frame_id: BELL_FRAME_ID,
        name: "bell",
        boot_x: BELL_BOOT_X,
        boot_y: BELL_BOOT_Y,
        boot_w: BELL_BOOT_W,
        boot_h: BELL_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_COMMAND_PALETTE,
        frame_id: COMMAND_PALETTE_FRAME_ID,
        name: "command_palette",
        boot_x: COMMAND_PALETTE_BOOT_X,
        boot_y: COMMAND_PALETTE_BOOT_Y,
        boot_w: COMMAND_PALETTE_BOOT_W,
        boot_h: COMMAND_PALETTE_BOOT_H,
        closeable: false,
        focusable: true,
    },
    AppSurfaceSpec {
        surface_id: SURFACE_ID_SPINDLE,
        frame_id: SPINDLE_FRAME_ID,
        name: "spindle",
        boot_x: SPINDLE_BOOT_X,
        boot_y: SPINDLE_BOOT_Y,
        boot_w: SPINDLE_BOOT_W,
        boot_h: SPINDLE_BOOT_H,
        closeable: false,
        focusable: true,
    },
];

// ── J1: Linen Object Table ────────────────────────────────────────────────────
// In-memory, static-only Linen object model. No filesystem, no storage, no PDX.
// See docs/handoff/J1_LINEN_OBJECT_TABLE_V1.md

/// Maximum number of tracked Linen objects.
const LINEN_MAX_OBJECTS: usize = 16;

/// Kind of Linen object. Maps to H1 §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LinenObjectKind {
    Project = 0,
    Document = 1,
    CodeFile = 2,
    MediaAsset = 3,
    BuildArtifact = 4,
    Folder = 5,
    Reference = 6,
    ImportPlaceholder = 7,
    BellEventReference = 8,
    QuilWorkspaceReference = 9,
    MeshDiagnosticReference = 10,
}

/// Lifecycle state of a Linen object. Maps to H1 §3 lifecycle_state field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LinenObjectState {
    Allocated = 0,
    Loaded = 1,
    Modified = 2,
    Saved = 3,
    Archived = 4,
}

/// Fixed-size Linen object record. All fields are scalar or fixed-cap.
#[derive(Debug, Clone, Copy)]
struct LinenObject {
    object_id: u64,
    kind: LinenObjectKind,
    state: LinenObjectState,
    parent_id: u64,
    project_id: u64,
    grant_ref: u64,
    linked_surface_id: u64,
    flags: u32,
    display_name: &'static str,
}

/// In-memory Linen object table. No heap, no filesystem, no storage.
/// Indexed linearly; searched by object_id on access.
static mut LINEN_OBJECTS: [Option<LinenObject>; LINEN_MAX_OBJECTS] = [None; LINEN_MAX_OBJECTS];

/// Shell-local selection state for Linen objects.
/// 0 = unset (repaired to first valid on first access via linen_selected_object_id()).
/// Only meaningful when Linen surface is focused (FOCUSED_SURFACE_ID == SURFACE_ID_LINEN).
static mut SELECTED_LINEN_OBJECT_ID: u64 = 0;

/// Seed objects for initial Linen workspace. 6 objects covering key kinds.
const LINEN_SEED_OBJECTS: [LinenObject; 6] = [
    LinenObject {
        object_id: 1,
        kind: LinenObjectKind::Project,
        state: LinenObjectState::Loaded,
        parent_id: 0,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "SexOS Kernel",
    },
    LinenObject {
        object_id: 2,
        kind: LinenObjectKind::Document,
        state: LinenObjectState::Saved,
        parent_id: 1,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Compositor Lifecycle Spec",
    },
    LinenObject {
        object_id: 3,
        kind: LinenObjectKind::CodeFile,
        state: LinenObjectState::Loaded,
        parent_id: 1,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: SURFACE_ID_LINEN,
        flags: 0,
        display_name: "Silk Shell main.rs",
    },
    LinenObject {
        object_id: 4,
        kind: LinenObjectKind::MediaAsset,
        state: LinenObjectState::Saved,
        parent_id: 0,
        project_id: 0,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Desktop Screenshot",
    },
    LinenObject {
        object_id: 5,
        kind: LinenObjectKind::BuildArtifact,
        state: LinenObjectState::Saved,
        parent_id: 1,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Current ISO Build",
    },
    LinenObject {
        object_id: 6,
        kind: LinenObjectKind::Folder,
        state: LinenObjectState::Loaded,
        parent_id: 0,
        project_id: 0,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Drafts",
    },
];

/// Initialize the Linen object table with seed objects.
/// Called once during boot. Emits proof markers for each seed object.
unsafe fn linen_object_table_init() {
    for (i, obj) in LINEN_SEED_OBJECTS.iter().enumerate() {
        if i < LINEN_MAX_OBJECTS {
            LINEN_OBJECTS[i] = Some(*obj);
            serial_println!("[linen.object.seed] id={} kind={} name={}", obj.object_id, obj.kind as u8, obj.display_name);
        }
    }
    serial_println!("[linen.object_table.init] count={}", LINEN_SEED_OBJECTS.len());
}

/// Return the number of Linen objects currently in the table.
unsafe fn linen_object_count() -> usize {
    let mut count = 0;
    for slot in LINEN_OBJECTS.iter() {
        if slot.is_some() {
            count += 1;
        }
    }
    count
}

/// Find a Linen object by its object_id. Returns None if not found.
unsafe fn linen_object_by_id(id: u64) -> Option<LinenObject> {
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            if obj.object_id == id {
                return Some(*obj);
            }
        }
    }
    None
}

/// Return a human-readable name for a LinenObjectKind.
fn linen_object_kind_name(kind: LinenObjectKind) -> &'static str {
    match kind {
        LinenObjectKind::Project => "Project",
        LinenObjectKind::Document => "Document",
        LinenObjectKind::CodeFile => "CodeFile",
        LinenObjectKind::MediaAsset => "MediaAsset",
        LinenObjectKind::BuildArtifact => "BuildArtifact",
        LinenObjectKind::Folder => "Folder",
        LinenObjectKind::Reference => "Reference",
        LinenObjectKind::ImportPlaceholder => "ImportPlaceholder",
        LinenObjectKind::BellEventReference => "BellEventRef",
        LinenObjectKind::QuilWorkspaceReference => "QuilWorkspaceRef",
        LinenObjectKind::MeshDiagnosticReference => "MeshDiagRef",
    }
}

/// Return a human-readable name for a LinenObjectState.
fn linen_object_state_name(state: LinenObjectState) -> &'static str {
    match state {
        LinenObjectState::Allocated => "Allocated",
        LinenObjectState::Loaded => "Loaded",
        LinenObjectState::Modified => "Modified",
        LinenObjectState::Saved => "Saved",
        LinenObjectState::Archived => "Archived",
    }
}

/// Return the currently selected Linen object ID.
/// If SELECTED_LINEN_OBJECT_ID is 0 (unset), repairs to first valid object.
/// Always returns a valid object_id (≥1) or 0 if no objects exist.
unsafe fn linen_selected_object_id() -> u64 {
    if SELECTED_LINEN_OBJECT_ID == 0 {
        linen_select_first_valid_object();
        if SELECTED_LINEN_OBJECT_ID == 0 {
            serial_println!("[linen.object_select.reject] reason=no_objects");
            return 0;
        }
        serial_println!("[linen.object_select.repair] id={}", SELECTED_LINEN_OBJECT_ID);
    }
    serial_println!("[linen.object_select.current] id={}", SELECTED_LINEN_OBJECT_ID);
    SELECTED_LINEN_OBJECT_ID
}

/// Set selection to the first valid Linen object (lowest object_id).
unsafe fn linen_select_first_valid_object() {
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            SELECTED_LINEN_OBJECT_ID = obj.object_id;
            serial_println!("[linen.object_select.current] id={}", obj.object_id);
            return;
        }
    }
    SELECTED_LINEN_OBJECT_ID = 0;
}

/// Advance selection to the next valid Linen object. Wraps around.
/// No-op if fewer than 2 objects exist. Guarded to only fire when
/// Linen is focused (see K4 doc — temporary global debug keys otherwise).
unsafe fn linen_select_next_object() {
    let current = SELECTED_LINEN_OBJECT_ID;
    let mut found_current = false;
    let mut first_valid: u64 = 0;
    let mut next_valid: u64 = 0;
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            if first_valid == 0 { first_valid = obj.object_id; }
            if found_current && next_valid == 0 {
                next_valid = obj.object_id;
                break;
            }
            if obj.object_id == current {
                found_current = true;
            }
        }
    }
    if next_valid != 0 {
        SELECTED_LINEN_OBJECT_ID = next_valid;
        serial_println!("[linen.object_select.next] prev={} next={}", current, next_valid);
    } else if first_valid != 0 && current != first_valid {
        // Wrap around to first valid.
        SELECTED_LINEN_OBJECT_ID = first_valid;
        serial_println!("[linen.object_select.next] prev={} next={} wrap", current, first_valid);
    } else {
        serial_println!("[linen.object_select.reject] reason=single_object id={}", current);
    }
}

/// Move selection to the previous valid Linen object. Wraps around.
/// No-op if fewer than 2 objects exist. Guarded to only fire when
/// Linen is focused (see K4 doc — temporary global debug keys otherwise).
unsafe fn linen_select_prev_object() {
    let current = SELECTED_LINEN_OBJECT_ID;
    let mut prev_valid: u64 = 0;
    let mut last_valid: u64 = 0;
    let mut first_valid: u64 = 0;
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            if first_valid == 0 { first_valid = obj.object_id; }
            if obj.object_id == current {
                break;
            }
            prev_valid = obj.object_id;
            last_valid = obj.object_id;
        }
    }
    if prev_valid != 0 && prev_valid != current {
        SELECTED_LINEN_OBJECT_ID = prev_valid;
        serial_println!("[linen.object_select.prev] prev={} current={}", prev_valid, current);
    } else if last_valid != 0 && last_valid != current {
        // Wrap around to last valid.
        SELECTED_LINEN_OBJECT_ID = last_valid;
        serial_println!("[linen.object_select.prev] prev={} current={} wrap", last_valid, current);
    } else {
        serial_println!("[linen.object_select.reject] reason=single_object id={}", current);
    }
}

/// Maximum visible rows in the Linen object list placeholder UI.
const LINEN_LIST_MAX_ROWS: u8 = 8;
/// Height of each object row in the list, in pixels.
const LINEN_LIST_ROW_H: u32 = 24;
/// Gap between rows.
const LINEN_LIST_ROW_GAP: u32 = 2;
/// Header bar color: teal-green, top of Linen surface.
const LINEN_LIST_HEADER_COLOR: u32 = 0x0038563A;
/// Header bar height.
const LINEN_LIST_HEADER_H: u32 = 28;
/// Number of rows with visual accent bars (rect_indices 3-7 within MAX_RECTS=8).
const LINEN_LIST_ACCENT_BARS: u8 = 5;
/// Background color for the Linen list area behind all rows.
const LINEN_LIST_BG_COLOR: u32 = 0x000C1420; // dark slate
/// Width of the left accent bar per row, in pixels.
const LINEN_ACCENT_BAR_W: u32 = 5;
/// Linen surface height sized to fit all visual row rects without clipping.
/// = HEADER_H + ACCENT_BARS * (ROW_H + ROW_GAP) + 10px margin = 28 + 5*26 + 10 = 168.
const LINEN_SURFACE_VISUAL_H: u32 =
    LINEN_LIST_HEADER_H + LINEN_LIST_ACCENT_BARS as u32 * (LINEN_LIST_ROW_H + LINEN_LIST_ROW_GAP) + 10;

/// Kind-to-visual-color mapping for object list rows.
/// Each LinenObjectKind gets a distinctive accent color for its row indicator.
fn linen_kind_color(kind: LinenObjectKind) -> u32 {
    match kind {
        LinenObjectKind::Project => 0x004080C0,           // blue
        LinenObjectKind::Document => 0x0040C080,          // green
        LinenObjectKind::CodeFile => 0x00C0A040,          // amber
        LinenObjectKind::MediaAsset => 0x00C04080,        // magenta
        LinenObjectKind::BuildArtifact => 0x00806040,     // brown
        LinenObjectKind::Folder => 0x00808080,            // grey
        LinenObjectKind::Reference => 0x006060C0,         // indigo
        LinenObjectKind::ImportPlaceholder => 0x00C06040, // orange
        LinenObjectKind::BellEventReference => 0x00C04040,// red
        LinenObjectKind::QuilWorkspaceReference => 0x0040C0C0, // cyan
        LinenObjectKind::MeshDiagnosticReference => 0x00A060C0, // violet
    }
}

/// Return the accent color for the currently selected Linen object.
/// Derives from the selected object's kind via linen_kind_color().
/// Falls back to the default header color if no object is selected.
unsafe fn linen_selected_object_accent() -> u32 {
    let id = SELECTED_LINEN_OBJECT_ID;
    if id == 0 {
        return LINEN_LIST_HEADER_COLOR;
    }
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            if obj.object_id == id {
                return linen_kind_color(obj.kind);
            }
        }
    }
    LINEN_LIST_HEADER_COLOR
}

/// Render the Linen object list using the Silk list row visual canon.
/// rect_index allocation (fits within sexdisplay MAX_RECTS=8):
///   0: header bar (selected-object accent color)
///   1: shared list background (neutral dark slate)
///   2: selected row highlight (full-width bright accent)
///   3-7: per-row left accent bars (5px wide, kind-colored)
unsafe fn linen_render_object_list() {
    // Get current Linen surface geometry from tracked vars.
    let w = SURFACE_200_W;
    let h = SURFACE_200_H;
    if w == 0 || h == 0 { return; }

    serial_println!("[linen.object_list.render] w={} h={}", w, h);

    // Determine header color based on currently selected object's kind.
    let header_color = linen_selected_object_accent();
    serial_println!("[linen.selection_visual.header] object_id={} color={:#010x}",
        SELECTED_LINEN_OBJECT_ID, header_color);

    // Draw header bar at top of surface using selection accent color (rect_index=0).
    // arg2 format: (rect_index<<56)|(color_rgb<<32)|(sh<<16)|sw
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        0u64,  // position (0,0) — top-left corner
        ((header_color as u64) << 32)
            | ((LINEN_LIST_HEADER_H as u64) << 16)
            | w as u64);

    let count = linen_object_count();

    // ── List background (rect_index=1) ───────────────────────────────────────
    // Single neutral rect behind all rows. Dark slate provides contrast for
    // accent bars and selected row highlight.
    let list_bg_h = LINEN_LIST_ACCENT_BARS as u32 * (LINEN_LIST_ROW_H + LINEN_LIST_ROW_GAP) - LINEN_LIST_ROW_GAP;
    let list_bg_y = LINEN_LIST_HEADER_H;
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        (list_bg_y as u64) << 32 | 0u64,
        (1u64 << 56)
            | ((LINEN_LIST_BG_COLOR as u64) << 32)
            | ((list_bg_h as u64) << 16)
            | w as u64);
    serial_println!("[linen.bg_rect] y={} h={}", list_bg_y, list_bg_h);

    // ── Emit row markers and accent bars (rect_indices 3-7) ──────────────────
    // Track selected row position for the highlight rect (rect_index=2).
    let mut rows_emitted: u8 = 0;
    let mut selected_row_pos: Option<u32> = None;
    for i in 0..LINEN_MAX_OBJECTS {
        if let Some(obj) = LINEN_OBJECTS[i] {
            if rows_emitted >= LINEN_LIST_MAX_ROWS {
                serial_println!("[linen.object_list.skip] id={} reason=max_rows", obj.object_id);
                continue;
            }
            let kind_name = linen_object_kind_name(obj.kind);
            let state_name = linen_object_state_name(obj.state);
            let is_selected = obj.object_id == SELECTED_LINEN_OBJECT_ID;
            let selected_flag = if is_selected { "true" } else { "false" };
            serial_println!("[linen.object_list.row] id={} kind={} state={} name={} selected={}",
                obj.object_id, kind_name, state_name, obj.display_name, selected_flag);

            // Track selected row position for later highlight rect.
            let row_y = LINEN_LIST_HEADER_H
                + rows_emitted as u32 * (LINEN_LIST_ROW_H + LINEN_LIST_ROW_GAP);
            if is_selected {
                selected_row_pos = Some(row_y);
            }

            // Left accent bar (rect_index = 3 + rows_emitted, max 5 rows).
            if rows_emitted < LINEN_LIST_ACCENT_BARS {
                let accent_index = (rows_emitted as u64 + 3) & 0x7; // 3,4,5,6,7
                let accent_color = if is_selected { header_color } else { linen_kind_color(obj.kind) };
                pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
                    (row_y as u64) << 32 | 0u64,
                    (accent_index << 56)
                        | ((accent_color as u64) << 32)
                        | ((LINEN_LIST_ROW_H as u64) << 16)
                        | LINEN_ACCENT_BAR_W as u64);
                serial_println!("[linen.row_visual.accent] index={} id={} kind={} color={:#010x} selected={}",
                    accent_index, obj.object_id, kind_name, accent_color, selected_flag);
            } else {
                serial_println!("[linen.row_visual.skip] id={} reason=accent_budget", obj.object_id);
            }

            rows_emitted += 1;
        }
    }

    // ── Selected row highlight (rect_index=2) ───────────────────────────────
    // Full-width bright accent bar at the selected row's y position.
    match selected_row_pos {
        Some(sel_y) => {
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
                (sel_y as u64) << 32 | 0u64,
                (2u64 << 56)
                    | ((header_color as u64) << 32)
                    | ((LINEN_LIST_ROW_H as u64) << 16)
                    | w as u64);
            serial_println!("[linen.row_visual.selected] y={} color={:#010x}", sel_y, header_color);
        }
        None => {
            serial_println!("[linen.row.reject] id={} reason=not_found_in_visible_rows",
                SELECTED_LINEN_OBJECT_ID);
        }
    }

    serial_println!("[linen.object_select.current] id={}", SELECTED_LINEN_OBJECT_ID);
    serial_println!("[linen.object_list.done] count={} rows={}", count, rows_emitted);
}

// ── J3: Quil Buffer Table ────────────────────────────────────────────────────
// In-memory, static-only Quil buffer model. No editor, parser, compiler, build.
// See docs/handoff/J3_QUIL_BUFFER_TABLE_V1.md

/// Maximum number of tracked Quil buffers.
const QUIL_MAX_BUFFERS: usize = 16;

/// Dynamic J4-created buffer IDs start here to avoid colliding with seed buffer IDs (1-6).
/// Seed buffers: 1-6 (manually curated, low namespace).
/// Dynamic buffers: QUIL_DYNAMIC_BUFFER_ID_BASE + object_id (high namespace, no overlap).
const QUIL_DYNAMIC_BUFFER_ID_BASE: u64 = 1000;

/// Maximum visible rows in the Quil buffer list placeholder UI.
const QUIL_LIST_MAX_ROWS: u8 = 8;
/// Header bar color: deep blue-purple, distinct from Linen teal-green header.
const QUIL_LIST_HEADER_COLOR: u32 = 0x00302E56;
/// Header bar height in pixels.
const QUIL_LIST_HEADER_H: u32 = 28;
/// Number of rows with visual accent bars (rect_indices 3-7 within MAX_RECTS=8).
const QUIL_LIST_ACCENT_BARS: u8 = 5;
/// Height of each buffer row in the list, in pixels.
const QUIL_LIST_ROW_H: u32 = 24;
/// Vertical gap between row fill rects, in pixels.
const QUIL_LIST_ROW_GAP: u32 = 2;
/// Background color for the Quil list area behind all rows.
const QUIL_LIST_BG_COLOR: u32 = 0x000C1420; // dark slate (matching Linen)
/// Width of the left accent bar per row, in pixels.
const QUIL_ACCENT_BAR_W: u32 = 5;
/// Kind of Quil buffer. Maps to H2 §2 workstation object types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum QuilBufferKind {
    Text = 0,
    Code = 1,
    DesignNote = 2,
    ReviewNote = 3,
    Diagnostic = 4,
    BuildOutput = 5,
    AgentTask = 6,
    LinenObjectView = 7,
}

/// Lifecycle state of a Quil buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum QuilBufferState {
    Allocated = 0,
    Open = 1,
    Dirty = 2,
    ReadOnly = 3,
    Closed = 4,
    Missing = 5,
}

/// Fixed-size Quil buffer record. All fields are scalar or fixed-cap.
#[derive(Debug, Clone, Copy)]
struct QuilBuffer {
    buffer_id: u64,
    kind: QuilBufferKind,
    state: QuilBufferState,
    linen_object_ref: u64,
    project_id: u64,
    grant_ref: u64,
    linked_surface_id: u64,
    flags: u32,
    display_name: &'static str,
}

/// In-memory Quil buffer table. No heap, no filesystem, no storage.
/// Indexed linearly; searched by buffer_id on access.
static mut QUIL_BUFFERS: [Option<QuilBuffer>; QUIL_MAX_BUFFERS] = [None; QUIL_MAX_BUFFERS];

/// Seed buffers for initial Quil workspace. 6 buffers covering key kinds.
const QUIL_SEED_BUFFERS: [QuilBuffer; 6] = [
    QuilBuffer {
        buffer_id: 1,
        kind: QuilBufferKind::Code,
        state: QuilBufferState::Open,
        linen_object_ref: 0,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: SURFACE_ID_QUIL,
        flags: 0,
        display_name: "main.rs",
    },
    QuilBuffer {
        buffer_id: 2,
        kind: QuilBufferKind::Text,
        state: QuilBufferState::Open,
        linen_object_ref: 2,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: SURFACE_ID_QUIL,
        flags: 0,
        display_name: "Compositor Lifecycle Spec",
    },
    QuilBuffer {
        buffer_id: 3,
        kind: QuilBufferKind::DesignNote,
        state: QuilBufferState::Open,
        linen_object_ref: 0,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Frame Tiling Design",
    },
    QuilBuffer {
        buffer_id: 4,
        kind: QuilBufferKind::BuildOutput,
        state: QuilBufferState::ReadOnly,
        linen_object_ref: 5,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Current ISO Build",
    },
    QuilBuffer {
        buffer_id: 5,
        kind: QuilBufferKind::ReviewNote,
        state: QuilBufferState::Open,
        linen_object_ref: 0,
        project_id: 1,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Review: A7 Opcode Audit",
    },
    QuilBuffer {
        buffer_id: 6,
        kind: QuilBufferKind::AgentTask,
        state: QuilBufferState::Allocated,
        linen_object_ref: 0,
        project_id: 0,
        grant_ref: 0,
        linked_surface_id: 0,
        flags: 0,
        display_name: "Refactor tiling loop",
    },
];

/// Initialize the Quil buffer table with seed buffers.
/// Called once during boot. Emits proof markers for each seed buffer.
unsafe fn quil_buffer_table_init() {
    for (i, buf) in QUIL_SEED_BUFFERS.iter().enumerate() {
        if i < QUIL_MAX_BUFFERS {
            QUIL_BUFFERS[i] = Some(*buf);
            serial_println!("[quil.buffer.seed] id={} kind={} name={}", buf.buffer_id, buf.kind as u8, buf.display_name);
        }
    }
    serial_println!("[quil.buffer_table.init] count={}", QUIL_SEED_BUFFERS.len());
}

/// Return the number of Quil buffers currently in the table.
unsafe fn quil_buffer_count() -> usize {
    let mut count = 0;
    for slot in QUIL_BUFFERS.iter() {
        if slot.is_some() {
            count += 1;
        }
    }
    count
}

/// Find a Quil buffer by its buffer_id. Returns None if not found.
unsafe fn quil_buffer_by_id(id: u64) -> Option<QuilBuffer> {
    for slot in QUIL_BUFFERS.iter() {
        if let Some(buf) = slot {
            if buf.buffer_id == id {
                return Some(*buf);
            }
        }
    }
    None
}

/// Return a human-readable name for a QuilBufferKind.
fn quil_buffer_kind_name(kind: QuilBufferKind) -> &'static str {
    match kind {
        QuilBufferKind::Text => "Text",
        QuilBufferKind::Code => "Code",
        QuilBufferKind::DesignNote => "DesignNote",
        QuilBufferKind::ReviewNote => "ReviewNote",
        QuilBufferKind::Diagnostic => "Diagnostic",
        QuilBufferKind::BuildOutput => "BuildOutput",
        QuilBufferKind::AgentTask => "AgentTask",
        QuilBufferKind::LinenObjectView => "LinenObjectView",
    }
}

/// Return a human-readable name for a QuilBufferState.
fn quil_buffer_state_name(state: QuilBufferState) -> &'static str {
    match state {
        QuilBufferState::Allocated => "Allocated",
        QuilBufferState::Open => "Open",
        QuilBufferState::Dirty => "Dirty",
        QuilBufferState::ReadOnly => "ReadOnly",
        QuilBufferState::Closed => "Closed",
        QuilBufferState::Missing => "Missing",
    }
}

/// Return a deterministic accent color for a QuilBufferKind.
/// Mirrors linen_kind_color() palette but with distinct hues.
fn quil_buffer_kind_color(kind: QuilBufferKind) -> u32 {
    match kind {
        QuilBufferKind::Text => 0x00808080,            // grey
        QuilBufferKind::Code => 0x0040A060,            // green-teal
        QuilBufferKind::DesignNote => 0x004060C0,      // blue
        QuilBufferKind::ReviewNote => 0x00C06040,      // orange
        QuilBufferKind::Diagnostic => 0x00C04080,      // magenta
        QuilBufferKind::BuildOutput => 0x00806040,     // brown
        QuilBufferKind::AgentTask => 0x006080C0,       // steel blue
        QuilBufferKind::LinenObjectView => 0x00A060C0, // violet
    }
}

// ── K3: Quil Buffer List Placeholder UI ─────────────────────────────────────
// Minimal buffer list rendered via proof markers inside the Quil surface.
// Mirrors J2 linen_render_object_list pattern. No editor, no text rendering.
// See docs/handoff/K3_QUIL_BUFFER_LIST_PLACEHOLDER_UI_V1.md

/// Render the Quil buffer list using the Silk list row visual canon.
/// rect_index allocation (fits within sexdisplay MAX_RECTS=8):
///   0: header bar (static blue-purple)
///   1: shared list background (neutral dark slate)
///   2: selected row highlight — suppressed (no Quil selection model yet)
///   3-7: per-row left accent bars (5px wide, buffer-kind-colored)
unsafe fn quil_render_buffer_list() {
    let w = SURFACE_201_W;
    let h = SURFACE_201_H;
    if w == 0 || h == 0 { return; }

    serial_println!("[quil.buffer_list.render] w={} h={}", w, h);

    // Draw header bar at top of surface (rect_index=0).
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL,
        (0u64 << 32) | 0u64,
        ((QUIL_LIST_HEADER_COLOR as u64) << 32)
            | ((QUIL_LIST_HEADER_H as u64) << 16)
            | w as u64);

    let count = quil_buffer_count();

    // ── List background (rect_index=1) ───────────────────────────────────
    let list_bg_h = QUIL_LIST_ACCENT_BARS as u32 * (QUIL_LIST_ROW_H + QUIL_LIST_ROW_GAP) - QUIL_LIST_ROW_GAP;
    let list_bg_y = QUIL_LIST_HEADER_H;
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL,
        (list_bg_y as u64) << 32 | 0u64,
        (1u64 << 56)
            | ((QUIL_LIST_BG_COLOR as u64) << 32)
            | ((list_bg_h as u64) << 16)
            | w as u64);
    serial_println!("[quil.bg_rect] y={} h={}", list_bg_y, list_bg_h);

    // ── Selected row highlight (rect_index=2) ───────────────────────────
    // Suppressed — Quil has no buffer selection model. No OOB to guard.
    serial_println!("[quil.row.reject] reason=no_selection_model");

    // ── Emit row markers and accent bars (rect_indices 3-7) ─────────────
    let mut rows_emitted: u8 = 0;
    for i in 0..QUIL_MAX_BUFFERS {
        if let Some(buf) = QUIL_BUFFERS[i] {
            if rows_emitted >= QUIL_LIST_MAX_ROWS {
                serial_println!("[quil.buffer_list.skip] id={} reason=max_rows", buf.buffer_id);
                continue;
            }
            let kind_name = quil_buffer_kind_name(buf.kind);
            let state_name = quil_buffer_state_name(buf.state);
            serial_println!("[quil.buffer_list.row] buffer_id={} kind={} state={} linen_ref={} surface_id={} name={}",
                buf.buffer_id, kind_name, state_name, buf.linen_object_ref,
                buf.linked_surface_id, buf.display_name);

            // Left accent bar (rect_index = 3 + rows_emitted, max 5 rows).
            if rows_emitted < QUIL_LIST_ACCENT_BARS {
                let accent_index = (rows_emitted as u64 + 3) & 0x7;
                let row_y = QUIL_LIST_HEADER_H
                    + rows_emitted as u32 * (QUIL_LIST_ROW_H + QUIL_LIST_ROW_GAP);
                let accent_color = quil_buffer_kind_color(buf.kind);
                pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL,
                    (row_y as u64) << 32 | 0u64,
                    (accent_index << 56)
                        | ((accent_color as u64) << 32)
                        | ((QUIL_LIST_ROW_H as u64) << 16)
                        | QUIL_ACCENT_BAR_W as u64);
                serial_println!("[quil.row_visual.accent] index={} id={} kind={} color={:#010x}",
                    accent_index, buf.buffer_id, kind_name, accent_color);
            } else {
                serial_println!("[quil.row_visual.skip] id={} reason=accent_budget", buf.buffer_id);
            }

            rows_emitted += 1;
        }
    }
    serial_println!("[quil.buffer_list.done] count={} rows={}", count, rows_emitted);
}
// ── K2C: Seed Coherence Init ──────────────────────────────────────────────────
// Called once at boot after both tables init.
// For seed buffers that pre-declare a linen_object_ref AND a non-zero linked_surface_id,
// synchronize the matching LinenObject.linked_surface_id to match.
// This makes both tables agree on "this object is displayed on surface X"
// without requiring a full J4/J5/J7 proof trail for seed pre-links.
// See docs/handoff/K2C_SEED_COHERENCE_V1.md

unsafe fn linen_quil_seed_coherence_init() {
    let mut linked: usize = 0;
    for slot in QUIL_BUFFERS.iter() {
        if let Some(buf) = slot {
            if buf.linen_object_ref != 0 && buf.linked_surface_id != 0 {
                for obj_slot in LINEN_OBJECTS.iter_mut() {
                    if let Some(o) = obj_slot {
                        if o.object_id == buf.linen_object_ref && o.linked_surface_id != buf.linked_surface_id {
                            o.linked_surface_id = buf.linked_surface_id;
                            serial_println!("[linen.quil.seed_link] object_id={} buffer_id={} surface_id={}",
                                o.object_id, buf.buffer_id, buf.linked_surface_id);
                            linked += 1;
                        }
                        if o.object_id == buf.linen_object_ref { break; }
                    }
                }
            }
        }
    }
    serial_println!("[linen.quil.seed_coherence.done] linked={}", linked);
}

// ── J4: Linen → Quil Buffer Link ─────────────────────────────────────────────
// Shell-local ID linking: open a Linen object into a Quil buffer slot.
// No editor, no storage, no parser/compiler/build, no PDX calls.
// See docs/handoff/J4_LINEN_OBJECT_TO_QUIL_BUFFER_V1.md

/// Open a Linen object into a Quil buffer using shell-local IDs only.
/// Links LINEN_OBJECTS[object_id] <-> QUIL_BUFFERS[buffer_id] via ref fields.
/// Returns true if the link was established and Quil surface is focused.
unsafe fn open_linen_object_in_quil(object_id: u64) -> bool {
    serial_println!("[linen.quil.open.request] id={}", object_id);

    // 1. Find the LinenObject by ID.
    let mut found_obj: Option<LinenObject> = None;
    for slot in LINEN_OBJECTS.iter() {
        if let Some(obj) = slot {
            if obj.object_id == object_id {
                found_obj = Some(*obj);
                break;
            }
        }
    }
    let obj = match found_obj {
        Some(o) => o,
        None => {
            serial_println!("[linen.quil.open.reject.missing] id={}", object_id);
            return false;
        }
    };

    // 2. Check grant_ref — still allow link but emit no_grant marker.
    if obj.grant_ref == 0 {
        serial_println!("[linen.quil.open.no_grant] id={} kind={}", object_id, obj.kind as u8);
    }

    // 2.5 C2: Check Collar gate before linking.
    // Grant table lookup replaces AllowStub with Allow/Deny.
    // Caller identity derived from FOCUSED_SURFACE_ID inside gate.
    let decision = collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0);
    if decision != CollarDecision::Allow {
        serial_println!("[linen.quil.open.reject.collar] decision={}", decision as u8);
        return false;
    }

    // 3. Map LinenObjectKind to QuilBufferKind for the linked buffer.
    let buf_kind = match obj.kind {
        LinenObjectKind::CodeFile => QuilBufferKind::Code,
        LinenObjectKind::MediaAsset => QuilBufferKind::LinenObjectView,
        LinenObjectKind::BuildArtifact => QuilBufferKind::BuildOutput,
        _ => QuilBufferKind::Text,
    };

    // 4. Find existing buffer for this object, or create one in an empty slot.
    // Dynamic buffer IDs use QUIL_DYNAMIC_BUFFER_ID_BASE + object_id to avoid
    // colliding with seed buffer IDs (1-6) which occupy the low namespace.
    let dynamic_buffer_id = QUIL_DYNAMIC_BUFFER_ID_BASE + object_id;
    let mut buffer_created = false;
    let mut found_buf = false;
    for slot in QUIL_BUFFERS.iter_mut() {
        if let Some(buf) = slot {
            if buf.linen_object_ref == object_id {
                // Reuse existing dynamic buffer for this object.
                buf.state = QuilBufferState::Open;
                buf.linked_surface_id = SURFACE_ID_QUIL;
                found_buf = true;
                serial_println!("[linen.quil.open.reuse_existing] object_id={} buffer_id={}", object_id, buf.buffer_id);
                break;
            }
        }
    }
    if !found_buf {
        // Pre-flight: verify the dynamic_buffer_id is not already taken by a different ref.
        for slot in QUIL_BUFFERS.iter() {
            if let Some(buf) = slot {
                if buf.buffer_id == dynamic_buffer_id && buf.linen_object_ref != object_id {
                    serial_println!("[linen.quil.open.reject.buffer_id_collision] dynamic_id={} existing_ref={}", dynamic_buffer_id, buf.linen_object_ref);
                    return false;
                }
            }
        }
        // Allocate a free slot.
        for slot in QUIL_BUFFERS.iter_mut() {
            if slot.is_none() {
                serial_println!("[linen.quil.open.dynamic_id] object_id={} dynamic_buffer_id={}", object_id, dynamic_buffer_id);
                *slot = Some(QuilBuffer {
                    buffer_id: dynamic_buffer_id,
                    kind: buf_kind,
                    state: QuilBufferState::Open,
                    linen_object_ref: object_id,
                    project_id: obj.project_id,
                    grant_ref: obj.grant_ref,
                    linked_surface_id: SURFACE_ID_QUIL,
                    flags: 0,
                    display_name: obj.display_name,
                });
                buffer_created = true;
                found_buf = true;
                break;
            }
        }
    }
    if !found_buf {
        serial_println!("[linen.quil.open.reject.full] object_id={}", object_id);
        return false;
    }

    // 5. Update LinenObject's linked_surface_id in-place.
    for slot in LINEN_OBJECTS.iter_mut() {
        if let Some(o) = slot {
            if o.object_id == object_id {
                o.linked_surface_id = SURFACE_ID_QUIL;
                break;
            }
        }
    }

    // 6. Emit buffer link proof marker.
    serial_println!("[linen.quil.buffer.linked] object_id={} buffer_id={} kind={}",
        object_id, dynamic_buffer_id, buf_kind as u8);

    // 7. Open Quil surface if not already visible.
    let quil_opened = open_quil_in_active_scene();
    if quil_opened {
        serial_println!("[linen.quil.quil_opened] object_id={}", object_id);
    }

    serial_println!("[linen.quil.done] object_id={} buffer_created={}", object_id, buffer_created);

    // 8. J6: Emit Mesh diagnostic link facts for all Linen↔Quil links.
    mesh_emit_linen_quil_links();

    // 9. J7: Emit Bell placeholder event for the new link.
    bell_emit_object_link_event(object_id, dynamic_buffer_id);

    // 10. K3: Refresh Quil buffer list to show the new dynamic buffer.
    quil_render_buffer_list();

    true
}

// ── C2: Collar-Gated Operation Policy ─────────────────────────────────────────
// V2 policy table replaces J5 AllowStub with grant table lookup for
// LinkObjectToBuffer. No real Collar PD, no ABI changes, no persistence.
// See docs/handoff/C2_COLLAR_POLICY_TABLE_V2.md

/// Operation kinds that may require Collar authority.
/// J5 stub — no real authority checks, just proof markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarOperation {
    OpenObject = 0,
    RenameObject = 1,
    ArchiveObject = 2,
    SaveBuffer = 3,
    BuildTarget = 4,
    RunTarget = 5,
    LinkObjectToBuffer = 6,
}

/// Decision from the Collar operation gate.
/// V2: Allow/Deny replaces AllowStub for wired operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarDecision {
    /// V2: Operation permitted — grant table match found.
    Allow = 0,
    /// V2: Operation denied — no matching active grant.
    Deny = 1,
    /// Referenced Linen object not found.
    DenyMissingObject = 2,
    /// Referenced Quil buffer not found.
    DenyMissingBuffer = 3,
    /// Operation would require a real Collar grant (future).
    NeedsGrantLater = 4,
    /// Operation blocked by STOP FIRST policy.
    BlockedStopFirst = 5,
}

/// C2: Collar policy gate. Checks grant table for wired operations.
///
/// Policy:
/// - OpenObject, LinkObjectToBuffer → grant table lookup (Allow/Deny)
/// - SaveBuffer, BuildTarget, RunTarget → BlockedStopFirst (STOP FIRST policy)
/// - RenameObject, ArchiveObject → NeedsGrantLater (requires real Collar)
/// - If object_id != 0 and not found in LINEN_OBJECTS → DenyMissingObject
/// - If buffer_id != 0 and not found in QUIL_BUFFERS → DenyMissingBuffer
/// - Caller identity derived from FOCUSED_SURFACE_ID (single-threaded dispatch)
unsafe fn collar_check_operation_stub(
    op: CollarOperation,
    object_id: u64,
    buffer_id: u64,
) -> CollarDecision {
    let caller_sid = FOCUSED_SURFACE_ID;
    serial_println!("[collar.policy.check] op={} object_id={} buffer_id={} caller_sid={}",
        op as u8, object_id, buffer_id, caller_sid);

    // Validate object_id if non-zero.
    if object_id != 0 {
        let mut found = false;
        for slot in LINEN_OBJECTS.iter() {
            if let Some(obj) = slot {
                if obj.object_id == object_id {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            serial_println!("[collar.gate.reject] reason=missing_object op={} object_id={}", op as u8, object_id);
            let d = CollarDecision::DenyMissingObject;
            record_collar_audit(op, object_id, caller_sid, d, 0, 1);
            return d;
        }
    }

    // Validate buffer_id if non-zero.
    if buffer_id != 0 {
        let buf = quil_buffer_by_id(buffer_id);
        if buf.is_none() {
            serial_println!("[collar.gate.reject] reason=missing_buffer op={} buffer_id={}", op as u8, buffer_id);
            let d = CollarDecision::DenyMissingBuffer;
            record_collar_audit(op, buffer_id, caller_sid, d, 0, 2);
            return d;
        }
    }

    match op {
        CollarOperation::OpenObject | CollarOperation::LinkObjectToBuffer => {
            // V2: Grant table lookup replaces AllowStub.
            let target_id = object_id;
            let mut found_grant = false;
            for slot in COLLAR_GRANTS.iter() {
                if let Some(grant) = slot {
                    if grant.state != CollarGrantState::Active { continue; }
                    if grant.subject_id != caller_sid { continue; }
                    if grant.object_id != target_id { continue; }
                    if (grant.operation_mask & (1 << (op as u64))) == 0 { continue; }
                    found_grant = true;
                    serial_println!("[collar.grant.match] grant_id={} subject={} object={} op={}",
                        grant.grant_id, grant.subject_id, grant.object_id, op as u8);
                    serial_println!("[collar.policy.allow] op={} object={} caller={} grant={}",
                        op as u8, target_id, caller_sid, grant.grant_id);
                    record_collar_audit(op, target_id, caller_sid, CollarDecision::Allow, grant.grant_id, 0);
                    return CollarDecision::Allow;
                }
            }
            // No matching active grant found — deny.
            serial_println!("[collar.grant.reject] reason=no_grant op={} object={} caller={}",
                op as u8, target_id, caller_sid);
            serial_println!("[collar.policy.deny] op={} object={} caller={} reason=no_grant",
                op as u8, target_id, caller_sid);
            record_collar_audit(op, target_id, caller_sid, CollarDecision::Deny, 0, 3);
            CollarDecision::Deny
        }
        CollarOperation::SaveBuffer | CollarOperation::BuildTarget | CollarOperation::RunTarget => {
            serial_println!("[collar.gate.reject] reason=stop_first op={}", op as u8);
            let d = CollarDecision::BlockedStopFirst;
            record_collar_audit(op, object_id, caller_sid, d, 0, 4);
            d
        }
        CollarOperation::RenameObject | CollarOperation::ArchiveObject => {
            serial_println!("[collar.gate.needs_grant] op={}", op as u8);
            let d = CollarDecision::NeedsGrantLater;
            record_collar_audit(op, object_id, caller_sid, d, 0, 5);
            d
        }
    }
}

// ── C2: Collar Grant Table + Audit Ring ───────────────────────────────────────
// Shell-local V2 policy table. Replaces AllowStub for LinkObjectToBuffer.
// No real Collar PD, no ABI changes, no persistence.
// See docs/handoff/C2_COLLAR_POLICY_TABLE_V2.md

/// State of a Collar grant record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CollarGrantState {
    Active = 0,
    Revoked = 1,
    Expired = 2,
    Tombstoned = 3,
}

/// A single Collar grant record. Fixed-size, no heap, no strings.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarGrant {
    grant_id: u64,
    subject_id: u64,
    object_id: u64,
    operation_mask: u64,
    generation: u64,
    state: CollarGrantState,
}

/// A single Collar audit event record.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CollarAuditEvent {
    event_id: u64,
    operation: CollarOperation,
    object_id: u64,
    subject_id: u64,
    decision: CollarDecision,
    grant_ref: u64,
    reason: u64,
}

const COLLAR_GRANT_CAP: usize = 32;
const COLLAR_AUDIT_CAP: usize = 64;

static mut COLLAR_GRANTS: [Option<CollarGrant>; COLLAR_GRANT_CAP] = [None; COLLAR_GRANT_CAP];
static mut COLLAR_GRANT_GENERATION: u64 = 1; // 0 reserved

static mut COLLAR_AUDIT_EVENTS: [Option<CollarAuditEvent>; COLLAR_AUDIT_CAP] = [None; COLLAR_AUDIT_CAP];
static mut COLLAR_AUDIT_WRITE_INDEX: u64 = 0;

/// Record an audit event in the Collar audit ring.
unsafe fn record_collar_audit(
    op: CollarOperation,
    object_id: u64,
    subject_id: u64,
    decision: CollarDecision,
    grant_ref: u64,
    reason: u64,
) {
    let idx = (COLLAR_AUDIT_WRITE_INDEX as usize) % COLLAR_AUDIT_CAP;
    let prev_event_id = COLLAR_AUDIT_WRITE_INDEX;
    COLLAR_AUDIT_EVENTS[idx] = Some(CollarAuditEvent {
        event_id: prev_event_id,
        operation: op,
        object_id,
        subject_id,
        decision,
        grant_ref,
        reason,
    });
    COLLAR_AUDIT_WRITE_INDEX += 1;
    if prev_event_id as usize >= COLLAR_AUDIT_CAP {
        serial_println!("[collar.audit.overwrite] idx={}", idx);
    }
    serial_println!("[collar.audit.write] event_id={} op={} object={} subject={} decision={} grant={} reason={}",
        prev_event_id, op as u8, object_id, subject_id, decision as u8, grant_ref, reason);
}

/// Initialize Collar auto-grants at boot.
/// Creates Active grants for seed objects to known surfaces.
unsafe fn collar_init_grants() {
    let mut count = 0u64;
    for slot in LINEN_OBJECTS.iter() {
        let obj = match slot {
            Some(o) => o,
            None => continue,
        };
        // Auto-grant for Linen surface (SURFACE_ID_LINEN = 200).
        let gen = COLLAR_GRANT_GENERATION;
        COLLAR_GRANT_GENERATION = COLLAR_GRANT_GENERATION.wrapping_add(1);
        let idx = (gen as usize).wrapping_sub(1) % COLLAR_GRANT_CAP;
        COLLAR_GRANTS[idx] = Some(CollarGrant {
            grant_id: gen,
            subject_id: SURFACE_ID_LINEN,
            object_id: obj.object_id,
            operation_mask: 1 << (CollarOperation::LinkObjectToBuffer as u64),
            generation: gen,
            state: CollarGrantState::Active,
        });
        serial_println!("[collar.grant.auto] grant_id={} subject={} object={} op=LinkObjectToBuffer",
            gen, SURFACE_ID_LINEN, obj.object_id);
        count += 1;

        // Auto-grant for Mesh surface (SURFACE_ID_MESH = 202).
        // Mesh can open any linked object's Quil view.
        let gen2 = COLLAR_GRANT_GENERATION;
        COLLAR_GRANT_GENERATION = COLLAR_GRANT_GENERATION.wrapping_add(1);
        let idx2 = (gen2 as usize).wrapping_sub(1) % COLLAR_GRANT_CAP;
        COLLAR_GRANTS[idx2] = Some(CollarGrant {
            grant_id: gen2,
            subject_id: SURFACE_ID_MESH,
            object_id: obj.object_id,
            operation_mask: 1 << (CollarOperation::LinkObjectToBuffer as u64),
            generation: gen2,
            state: CollarGrantState::Active,
        });
        serial_println!("[collar.grant.auto] grant_id={} subject={} object={} op=LinkObjectToBuffer",
            gen2, SURFACE_ID_MESH, obj.object_id);
        count += 1;
    }
    serial_println!("[collar.grant.init] count={} generation={}", count, COLLAR_GRANT_GENERATION);
}
// Shell-local fact ring for topology/relationship data. Replaces proof-marker-only
// diagnostics with real bounded Mesh fact memory. No Mesh PD, no IPC/ABI changes,
// no rendering.
// See docs/handoff/J6_MESH_OBJECT_LINKS_V1.md
// See docs/handoff/N2_MESH_SHELL_LOCAL_FACT_RING_V1.md

/// Kinds of Mesh facts stored in the shell-local fact ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MeshFactKind {
    /// A Linen object was linked to a Quil buffer.
    ObjectLinkedToBuffer = 0,
}

/// A single Mesh fact record stored in the shell-local ring buffer.
/// Fixed-size scalars only. No pointers, no strings, no heap.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct MeshFact {
    /// Monotonic fact ID (incremented per stored fact).
    fact_id: u64,
    /// Fact kind (V1: only ObjectLinkedToBuffer).
    kind: MeshFactKind,
    /// Primary subject ID (Linen object_id for ObjectLinkedToBuffer).
    subject_id: u64,
    /// Primary object ID (Quil buffer_id for ObjectLinkedToBuffer).
    object_id: u64,
    /// Secondary reference (linked_surface_id for ObjectLinkedToBuffer).
    ref_id: u64,
    /// Monotonic counter for ordering (MESH_FACT_WRITE_INDEX at write time).
    sequence: u64,
}

/// Capacity of the Mesh fact ring buffer (power of 2 for efficient modulo).
const MESH_FACT_RING_CAP: usize = 32;
/// Ring buffer of Mesh facts. Index = write_index % MESH_FACT_RING_CAP.
static mut MESH_FACTS: [Option<MeshFact>; MESH_FACT_RING_CAP] = [None; MESH_FACT_RING_CAP];
/// Next write index into the ring (monotonic, wraps via modulo).
static mut MESH_FACT_WRITE_INDEX: u64 = 0;
/// Global fact sequence counter (incremented per written fact).
static mut MESH_FACT_SEQUENCE: u64 = 0;

/// Write a Mesh fact into the shell-local fact ring.
/// Overwrites oldest entry when ring is full. Emits proof markers.
unsafe fn mesh_record_fact(kind: MeshFactKind, subject_id: u64, object_id: u64, ref_id: u64) {
    let idx = (MESH_FACT_WRITE_INDEX as usize) % MESH_FACT_RING_CAP;
    let seq = MESH_FACT_SEQUENCE;
    MESH_FACT_SEQUENCE += 1;
    let prev = MESH_FACTS[idx].replace(MeshFact {
        fact_id: seq,
        kind,
        subject_id,
        object_id,
        ref_id,
        sequence: MESH_FACT_WRITE_INDEX,
    });
    MESH_FACT_WRITE_INDEX += 1;
    if prev.is_some() {
        serial_println!("[mesh.fact.overwrite] idx={} prev_fact_id={}",
            idx, prev.unwrap().fact_id);
    }
    serial_println!("[mesh.fact.write] idx={} fact_id={} kind={:?} subject_id={} object_id={} ref_id={}",
        idx, seq, kind, subject_id, object_id, ref_id);
    serial_println!("[mesh.fact.done] count={} fact_id={}",
        core::cmp::min(MESH_FACT_WRITE_INDEX as usize, MESH_FACT_RING_CAP), seq);
    // N4: Live refresh Mesh fact list if Mesh surface is visible.
    if mesh_is_visible_in_active_scene() {
        serial_println!("[mesh.render.refresh] reason=visible_after_fact kind={:?}", kind);
        mesh_render_fact_list();
    }
}

/// Return the number of Mesh facts currently in the ring.
unsafe fn mesh_fact_count() -> usize {
    let total = MESH_FACT_WRITE_INDEX;
    if total == 0 { return 0; }
    core::cmp::min(total as usize, MESH_FACT_RING_CAP)
}

/// Iterate Mesh facts from newest to oldest, calling `f` for each.
/// Read-only. Does not mutate the ring. Bounded by MESH_FACT_RING_CAP.
unsafe fn mesh_for_each_fact<F>(mut f: F) where F: FnMut(&MeshFact) {
    let total = MESH_FACT_WRITE_INDEX;
    let count = mesh_fact_count();
    if count == 0 { return; }
    // Newest-first: iterate backwards from (total-1) % cap.
    let start = (total as usize).wrapping_sub(1) % MESH_FACT_RING_CAP;
    for i in 0..count {
        let idx = (start + MESH_FACT_RING_CAP - i) % MESH_FACT_RING_CAP;
        if let Some(ref fact) = MESH_FACTS[idx] {
            f(fact);
        }
    }
}

/// Scan the Quil buffer table and emit diagnostic link facts for Mesh.
///
/// For each buffer with a non-zero linen_object_ref:
/// - If the referenced LinenObject exists, emit a [mesh.object_link.row] proof
///   marker AND record an ObjectLinkedToBuffer fact in the shell-local ring.
/// - If the referenced LinenObject is missing (stale ref), emit a
///   [mesh.object_link.reject.missing_object] marker. No fact recorded.
///
/// Link facts are IDs and kind names only — no object contents, no file paths,
/// no raw pointers, no authority mutation.
unsafe fn mesh_emit_linen_quil_links() {
    serial_println!("[mesh.object_link.start]");
    let mut link_count: usize = 0;
    let mut stale_count: usize = 0;
    for slot in QUIL_BUFFERS.iter() {
        if let Some(buf) = slot {
            if buf.linen_object_ref != 0 {
                let obj = linen_object_by_id(buf.linen_object_ref);
                match obj {
                    Some(o) => {
                        serial_println!(
                            "[mesh.object_link.row] object_id={} object_kind={} buffer_id={} buffer_kind={} surface_id={}",
                            o.object_id,
                            linen_object_kind_name(o.kind),
                            buf.buffer_id,
                            quil_buffer_kind_name(buf.kind),
                            buf.linked_surface_id,
                        );
                        // Record fact in shell-local ring for valid links only.
                        mesh_record_fact(
                            MeshFactKind::ObjectLinkedToBuffer,
                            o.object_id,
                            buf.buffer_id,
                            buf.linked_surface_id,
                        );
                        link_count += 1;
                    }
                    None => {
                        serial_println!(
                            "[mesh.object_link.reject.missing_object] buffer_id={} linen_object_ref={}",
                            buf.buffer_id,
                            buf.linen_object_ref,
                        );
                        stale_count += 1;
                    }
                }
            }
        }
    }
    serial_println!("[mesh.object_link.done] links={} stale={}", link_count, stale_count);
}

/// Derive a deterministic row color from a Mesh fact.
/// For ObjectLinkedToBuffer, derives color from the linked Linen object's kind.
/// Falls back to Mesh amber diagnostic color if object is not found.
unsafe fn mesh_fact_row_color(fact: &MeshFact) -> u32 {
    match fact.kind {
        MeshFactKind::ObjectLinkedToBuffer => {
            if let Some(obj) = linen_object_by_id(fact.subject_id) {
                linen_kind_color(obj.kind)
            } else {
                0x00383010 // Mesh amber fallback
            }
        }
    }
}

/// Returns true if the Mesh placeholder surface is currently visible
/// in the active scene (frame exists, not minimized, surface alive).
unsafe fn mesh_is_visible_in_active_scene() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(MESH_FRAME_ID) {
                    if surface_is_alive(sid) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Render the Mesh fact ring as row fill rects inside the Mesh placeholder surface.
/// Uses existing multi-rect pattern (header + fact rows). Read-only over ring.
unsafe fn mesh_render_fact_list() {
    let w = SURFACE_202_W;
    let h = SURFACE_202_H;
    if w == 0 || h == 0 { return; }

    // Clamp selected row to valid range after ring changes.
    let visible = mesh_visible_fact_count();
    if visible == 0 {
        MESH_SELECTED_ROW = 0;
    } else if MESH_SELECTED_ROW >= visible {
        let old = MESH_SELECTED_ROW;
        MESH_SELECTED_ROW = visible.wrapping_sub(1);
        serial_println!("[mesh.selection.repair] old={} new={} count={}", old, MESH_SELECTED_ROW, visible);
    }
    serial_println!("[mesh.selection.current] row={} visible={}", MESH_SELECTED_ROW, visible);

    serial_println!("[mesh.fact_list.render] w={} h={} count={}", w, h, mesh_fact_count());

    // Draw header bar at top of surface (rect_index=0).
    // arg2 format: (rect_index<<56)|(color_rgb<<32)|(sh<<16)|sw
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_MESH,
        (0u64 << 32) | 0u64,  // position (0,0)
        ((MESH_PLACEHOLDER_COLOR as u64) << 32)
            | ((MESH_LIST_HEADER_H as u64) << 16)
            | w as u64);

    // Emit row markers and visual fill rects, newest-first.
    let mut rows_emitted: u8 = 0;
    let mut rects_sent: u8 = 0;
    mesh_for_each_fact(|fact| {
        if rows_emitted >= MESH_LIST_ROW_RECTS {
            serial_println!("[mesh.fact_list.skip] fact_id={} reason=max_rows", fact.fact_id);
            return;
        }
        serial_println!("[mesh.fact_list.row] fact_id={} kind={:?} subject_id={} object_id={} ref_id={}",
            fact.fact_id, fact.kind, fact.subject_id, fact.object_id, fact.ref_id);

        // Send visual row rect if within fill-rect slot budget (slots 1-7; slot 0 = header).
        if rows_emitted < MESH_LIST_ROW_RECTS {
            let rect_index = (rows_emitted as u64 + 1) & 0xF;
            let row_y = MESH_LIST_HEADER_H
                + rows_emitted as u32 * (MESH_LIST_ROW_H + MESH_LIST_ROW_GAP);
            let base_color = mesh_fact_row_color(fact);
            let row_color = if rows_emitted == MESH_SELECTED_ROW {
                let highlighted = mesh_selected_row_highlight(base_color);
                serial_println!("[mesh.selection_visual.row] fact_id={} index={} base={:#010x} highlight={:#010x}",
                    fact.fact_id, rows_emitted, base_color, highlighted);
                highlighted
            } else {
                base_color
            };
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_MESH,
                (row_y as u64) << 32 | 0u64,
                (rect_index << 56)
                    | ((row_color as u64) << 32)
                    | ((MESH_LIST_ROW_H as u64) << 16)
                    | w as u64);
            serial_println!("[mesh.row_visual.rect] index={} fact_id={} kind={:?} color={:#010x}",
                rect_index, fact.fact_id, fact.kind, row_color);
            rects_sent += 1;
        } else {
            serial_println!("[mesh.row_visual.skip] fact_id={} reason=rect_budget", fact.fact_id);
        }
        rows_emitted += 1;
    });
    serial_println!("[mesh.fact_list.done] count={} rows={} rects={}", mesh_fact_count(), rows_emitted, rects_sent);
}

/// Count facts visible in the Mesh list (capped at MESH_LIST_ROW_RECTS).
unsafe fn mesh_visible_fact_count() -> u8 {
    let count = mesh_fact_count();
    if count == 0 { return 0; }
    core::cmp::min(count as u8, MESH_LIST_ROW_RECTS)
}

/// Brighten a 0x00RRGGBB color for selected row highlighting.
/// Adds 0x40 (~25%) to each RGB component with per-channel clamping.
fn mesh_selected_row_highlight(color: u32) -> u32 {
    let r = core::cmp::min(((color >> 16) & 0xFF).wrapping_add(0x40), 0xFF);
    let g = core::cmp::min(((color >> 8) & 0xFF).wrapping_add(0x40), 0xFF);
    let b = core::cmp::min((color & 0xFF).wrapping_add(0x40), 0xFF);
    (r << 16) | (g << 8) | b
}

/// Advance Mesh selection to the next visible fact row. Wraps around.
unsafe fn mesh_select_next_row() {
    let count = mesh_visible_fact_count();
    if count <= 1 {
        serial_println!("[mesh.selection.reject] reason=single_or_empty count={}", count);
        return;
    }
    let current = MESH_SELECTED_ROW;
    let next = if current + 1 >= count { 0 } else { current + 1 };
    MESH_SELECTED_ROW = next;
    serial_println!("[mesh.selection.next] prev={} next={}", current, next);
    mesh_render_fact_list();
}

/// Move Mesh selection to the previous visible fact row. Wraps around.
unsafe fn mesh_select_prev_row() {
    let count = mesh_visible_fact_count();
    if count <= 1 {
        serial_println!("[mesh.selection.reject] reason=single_or_empty count={}", count);
        return;
    }
    let current = MESH_SELECTED_ROW;
    let prev = if current == 0 { count - 1 } else { current - 1 };
    MESH_SELECTED_ROW = prev;
    serial_println!("[mesh.selection.prev] prev={} next={}", current, prev);
    mesh_render_fact_list();
}

/// Return a copy of the Mesh fact at the currently selected visible row.
/// Iterates the ring newest-first (same order as mesh_for_each_fact)
/// to map MESH_SELECTED_ROW to the corresponding fact. Returns None if
/// the ring is empty or the selected index has no fact.
unsafe fn mesh_selected_fact_snapshot() -> Option<MeshFact> {
    let total = MESH_FACT_WRITE_INDEX;
    let count = mesh_fact_count();
    if count == 0 { return None; }
    let start = (total as usize).wrapping_sub(1) % MESH_FACT_RING_CAP;
    for i in 0..count {
        let idx = (start + MESH_FACT_RING_CAP - i) % MESH_FACT_RING_CAP;
        if let Some(fact) = MESH_FACTS[idx] {
            if (i as u8) == MESH_SELECTED_ROW {
                return Some(fact);
            }
        }
    }
    None
}

/// Emit proof markers for the currently selected Mesh fact.
/// Returns true if a valid fact was found and proof markers emitted.
/// Returns false if rejected (not focused, no fact).
/// No action, no ack, no delete, no Mesh PD. Proof-marker-only stub.
unsafe fn mesh_emit_selected_fact_detail_proof() -> bool {
    if FOCUSED_SURFACE_ID != SURFACE_ID_MESH {
        serial_println!("[mesh.detail.reject] reason=not_focused");
        return false;
    }
    let fact = match mesh_selected_fact_snapshot() {
        Some(f) => f,
        None => {
            serial_println!("[mesh.detail.reject] reason=no_fact");
            return false;
        }
    };
    serial_println!("[mesh.detail.open] fact_id={} kind={:?}", fact.fact_id, fact.kind);
    match fact.kind {
        MeshFactKind::ObjectLinkedToBuffer => {
            serial_println!("[mesh.detail.fact] fact_id={} kind=ObjectLinkedToBuffer subject_id={} object_id={} ref_id={}",
                fact.fact_id, fact.subject_id, fact.object_id, fact.ref_id);
            serial_println!("[mesh.detail.object_link] subject_id={} object_id={} ref_id={}",
                fact.subject_id, fact.object_id, fact.ref_id);
        }
    }
    serial_println!("[mesh.detail.done] fact_id={}", fact.fact_id);
    true
}

/// N11: Focus Linen surface and select the object referenced by a Mesh fact.
/// Only the subject_id (Linen object_id) from ObjectLinkedToBuffer facts is used.
/// No buffer creation, no linking, no Collar gate.
unsafe fn mesh_focus_linen_at_selected_fact(fact: &MeshFact) {
    serial_println!("[mesh.action.focus_linen] subject_id={}", fact.subject_id);
    open_linen_in_active_scene();
    SELECTED_LINEN_OBJECT_ID = fact.subject_id;
    linen_render_object_list();
}

// ── J7: Bell Object Link Event Stub ──────────────────────────────────────────
// Shell-local Bell placeholder event for Linen→Quil buffer links.
// No real queue, no notification UI, no new PDX ops, no renderer changes.
// See docs/handoff/J7_BELL_OBJECT_LINK_EVENT_V1.md

/// Kinds of Bell events that can be emitted as shell-local stubs.
/// Real Bell will have a richer category/priority system (see G1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellEventKind {
    ObjectLinkedToBuffer = 0,
    ObjectOpenRequested = 1,
    OperationNeedsGrant = 2,
    DiagnosticOnly = 3,
}

/// A single Bell event record stored in the shell-local ring buffer.
/// V1 supports only ObjectLinkedToBuffer. Fixed-size scalars only.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellEvent {
    /// Monotonic event ID (incremented per stored event).
    event_id: u64,
    /// Event kind (V1: only ObjectLinkedToBuffer is actually emitted).
    kind: BellEventKind,
    /// The Linen object_id involved (0 if not applicable).
    object_id: u64,
    /// The Quil buffer_id involved (0 if not applicable).
    buffer_id: u64,
    /// Monotonic counter at time of event (for ordering).
    sequence: u64,
}

/// Shell-local Bell event ring buffer (size = power of 2 for efficient modulo).
/// Overwrites oldest entry when full. Mirrors TOMBSTONE_RING pattern.
const BELL_RING_CAP: usize = 16;
/// Ring buffer of Bell events. Index = write_index % BELL_RING_CAP.
static mut BELL_EVENTS: [Option<BellEvent>; BELL_RING_CAP] = [None; BELL_RING_CAP];
/// Next write index into the ring (monotonic, wraps via modulo).
static mut BELL_RING_WRITE_INDEX: u64 = 0;
/// Global event sequence counter (incremented per written event).
static mut BELL_EVENT_SEQUENCE: u64 = 0;

/// Write an ObjectLinkedToBuffer event into the shell-local Bell ring.
/// Overwrites oldest entry when ring is full. Emits proof markers.
unsafe fn bell_record_event(object_id: u64, buffer_id: u64) {
    let idx = (BELL_RING_WRITE_INDEX as usize) % BELL_RING_CAP;
    let seq = BELL_EVENT_SEQUENCE;
    BELL_EVENT_SEQUENCE += 1;
    let prev = BELL_EVENTS[idx].replace(BellEvent {
        event_id: seq,
        kind: BellEventKind::ObjectLinkedToBuffer,
        object_id,
        buffer_id,
        sequence: BELL_RING_WRITE_INDEX,
    });
    BELL_RING_WRITE_INDEX += 1;
    if prev.is_some() {
        serial_println!("[bell.ring.overwrite] idx={} prev_event_id={}",
            idx, prev.unwrap().event_id);
    }
    serial_println!("[bell.ring.write] idx={} event_id={} object_id={} buffer_id={}",
        idx, seq, object_id, buffer_id);
}

/// Return the number of Bell events currently in the ring.
unsafe fn bell_ring_count() -> usize {
    let total = BELL_RING_WRITE_INDEX;
    if total == 0 { return 0; }
    core::cmp::min(total as usize, BELL_RING_CAP)
}

/// Iterate Bell events from newest to oldest, calling `f` for each.
/// Read-only. Does not mutate the ring. Bounded by BELL_RING_CAP.
unsafe fn bell_for_each_event<F>(mut f: F) where F: FnMut(&BellEvent) {
    let total = BELL_RING_WRITE_INDEX;
    let count = bell_ring_count();
    if count == 0 { return; }
    // Newest-first: iterate backwards from (total-1) % cap.
    let start = (total as usize).wrapping_sub(1) % BELL_RING_CAP;
    for i in 0..count {
        let idx = (start + BELL_RING_CAP - i) % BELL_RING_CAP;
        if let Some(ref ev) = BELL_EVENTS[idx] {
            f(ev);
        }
    }
}

/// Emit a shell-local Bell event for a Linen→Quil object link.
///
/// Validates object_id and buffer_id via existing local helpers, then writes
/// to the shell-local Bell ring and emits proof markers.
unsafe fn bell_emit_object_link_event(object_id: u64, buffer_id: u64) {
    serial_println!("[bell.event.stub] kind=ObjectLinkedToBuffer object_id={} buffer_id={}", object_id, buffer_id);

    // Validate object exists.
    let obj = linen_object_by_id(object_id);
    let obj_valid = obj.is_some();

    // Validate buffer exists and references the expected object.
    let buf = quil_buffer_by_id(buffer_id);
    let buf_valid = buf.is_some();

    if !obj_valid || !buf_valid {
        serial_println!("[bell.event.reject.missing] object_valid={} buffer_valid={}", obj_valid, buf_valid);
        serial_println!("[bell.event.done] reason=rejected");
        return;
    }

    let obj = obj.unwrap();
    let buf = buf.unwrap();

    // Verify the buffer actually references this object (cross-check).
    if buf.linen_object_ref != object_id {
        serial_println!("[bell.event.reject.missing] reason=buffer_ref_mismatch expected={} actual={}", object_id, buf.linen_object_ref);
        serial_println!("[bell.event.done] reason=rejected");
        return;
    }

    serial_println!(
        "[bell.event.object_link] object_id={} object_kind={} buffer_id={} buffer_kind={}",
        object_id,
        linen_object_kind_name(obj.kind),
        buffer_id,
        quil_buffer_kind_name(buf.kind),
    );

    // Write to shell-local ring buffer.
    bell_record_event(object_id, buffer_id);
    serial_println!("[bell.ring.done] count={} event_id={}",
        bell_ring_count(), BELL_EVENT_SEQUENCE - 1);

    // Refresh Bell surface if it is currently visible in the active scene.
    if bell_is_visible_in_active_scene() {
        serial_println!("[bell.render.refresh] reason=visible_after_link object_id={} buffer_id={}", object_id, buffer_id);
        bell_render_event_list();
    }

    serial_println!("[bell.event.done] reason=emitted");
}

// ── A3: Lifecycle State Model ─────────────────────────────────────────────────
// Additive metadata only. No behavior change.
// See docs/handoff/A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1.md

/// Canonical lifecycle states for a shell-managed surface.
/// Focus is NOT a lifecycle state — tracked separately via FocusRef.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LifecycleState {
    /// SurfaceId reserved but no frame mapped. No display state.
    Allocated = 0,
    /// Surface attached to a Frame. Has geometry but may not be visible.
    Mapped = 1,
    /// Surface in active scene, frame not minimized, z-order includes it.
    Visible = 2,
    /// Surface's frame is in a non-active scene. No input routing.
    Hidden = 3,
    /// Frame collapsed. Surface hidden. No pointer focus.
    Minimized = 4,
    /// Close requested — irreversible. Must transition to Tombstoned.
    Closing = 5,
    /// Surface dead but record exists. Cannot receive focus.
    Tombstoned = 6,
    /// Terminal. SurfaceId eligible for reuse only with generation safety.
    Destroyed = 7,
}

/// A validated reference to a surface that may be focused.
/// The generation field detects stale references after lifecycle transitions.
#[derive(Debug, Clone, Copy)]
struct FocusRef {
    surface_id: u64,
    generation: u64,
}

/// Per-surface lifecycle metadata.
#[derive(Debug, Clone, Copy)]
struct SurfaceLifecycle {
    state: LifecycleState,
    generation: u64,
}

/// A6: Reason for a tombstone event recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum TombstoneReason {
    /// Normal frame-light close button.
    CloseRequested = 0,
    /// Focus cleared because surface is dead or lifecycle-invalid.
    FocusCleared = 1,
    /// Drag cancelled because surface is dead.
    DragCancelled = 2,
    /// Keyboard-triggered DestroyFocused action.
    DestroyCommand = 3,
    /// Final Tombstoned -> Destroyed transition (future, not yet wired).
    FinalDestroy = 4,
}

/// A6: Tombstone event — records a surface death transition with context.
/// Stored in a fixed-size ring buffer for observability and proof.
#[derive(Debug, Clone, Copy)]
struct TombstoneEvent {
    /// The surface whose lifecycle changed.
    surface_id: u64,
    /// Lifecycle generation at time of recording.
    generation: u64,
    /// LifecycleState before the transition.
    old_state: LifecycleState,
    /// LifecycleState after the transition (or same for focus/drag clears).
    new_state: LifecycleState,
    /// Why this event was recorded.
    reason: TombstoneReason,
    /// Frame the surface belongs to (0 if unknown/no frame).
    frame_id: u32,
    /// Tab index within the frame (0 if unknown/single-tab).
    tab_index: u8,
}

/// Maximum number of tracked lifecycle surfaces.
const LIFECYCLE_MAX_SURFACES: usize = 32;

/// Lifecycle metadata for all tracked surfaces.
/// Indexed linearly; searched by surface_id on access.
/// Preserves existing hardcoded SurfaceIds — no dynamic allocation.
static mut LIFECYCLE_TABLE: [Option<(u64, SurfaceLifecycle)>; LIFECYCLE_MAX_SURFACES] = [None; LIFECYCLE_MAX_SURFACES];

/// Global monotonic lifecycle generation counter.
/// Incremented on transitions that invalidate stale references:
/// entering Closing, Closing->Tombstoned, Tombstoned->Destroyed.
/// Starts at 1. 0 reserved for no-surface / uninitialized.
static mut LIFECYCLE_GENERATION: u64 = 1;

/// FocusRef shadow of FOCUSED_SURFACE_ID. Updated in parallel via sync_focus_ref().
/// Does not change focus behavior — purely additive for A4 readiness.
static mut FOCUSED_SURFACE: Option<FocusRef> = None;


/// Validate the app surface registry at boot.
/// Checks for duplicate surface_ids and frame_ids.
/// Logs a diagnostic marker — does NOT halt on duplicate (shell continues safely).
unsafe fn app_surface_registry_validate() {
    let count = APP_SURFACES.len();
    let mut valid = true;
    for i in 0..count {
        for j in (i + 1)..count {
            if APP_SURFACES[i].surface_id == APP_SURFACES[j].surface_id {
                serial_println!("[shell.app_registry.duplicate] surface_id={} entries={},{}",
                    APP_SURFACES[i].surface_id, i, j);
                valid = false;
            }
            if APP_SURFACES[i].frame_id == APP_SURFACES[j].frame_id {
                serial_println!("[shell.app_registry.duplicate] frame_id={} entries={},{}",
                    APP_SURFACES[i].frame_id, i, j);
                valid = false;
            }
        }
    }
    if valid {
        serial_println!("[shell.app_registry.valid] count={}", count);
    } else {
        serial_println!("[shell.app_registry.error] duplicate detected — check surface_id/frame_id allocation");
    }
}

/// Lookup an app surface spec by surface_id. Returns None for non-registered surfaces.
fn app_surface_spec(surface_id: u64) -> Option<&'static AppSurfaceSpec> {
    APP_SURFACES.iter().find(|s| s.surface_id == surface_id)
}

/// Lookup an app surface spec by frame_id. Returns None for non-registered frames.
#[allow(dead_code)]
fn app_surface_spec_by_frame(frame_id: u32) -> Option<&'static AppSurfaceSpec> {
    APP_SURFACES.iter().find(|s| s.frame_id == frame_id)
}

// ── Scene Render Token presets ────────────────────────────────────────────────
// Fields: [focus_surface, frame_rim, frame_top_bar, active_tab,
//          inactive_tab, close_light, minimize_light, zoom_light]
const PRESET_COUNT: usize = 4;
type TokenPreset = [u32; 8];

static TOKEN_PRESETS: [TokenPreset; PRESET_COUNT] = [
    // 0: BottleGlass (default teal — matches DEFAULT_RENDER_TOKENS in sexdisplay)
    [0x007AAFA4, 0x00B8F2E8, 0x0088C2B7, 0x007AAFA4, 0x006080B0, 0x00FF4444, 0x00FFCC44, 0x0044FF44],
    // 1: VioletGlass (Silk canon purple)
    [0x00503080, 0x00A060FF, 0x00604090, 0x00503080, 0x00302050, 0x00FF4080, 0x00FFAA00, 0x0040FF80],
    // 2: GraphiteGlass (dark neutral)
    [0x00282828, 0x00808080, 0x00404040, 0x00505050, 0x00303030, 0x00CC4444, 0x00CCAA44, 0x0044CC44],
    // 3: HighContrast (accessibility proof)
    [0x00000000, 0x00FFFFFF, 0x00111111, 0x00FFFF00, 0x00555555, 0x00FF4444, 0x00FFDD00, 0x0000FF44],
];

/// In-memory appearance settings state. No persistence in V1.
/// Initialized to BottleGlass defaults at compile time.
#[derive(Clone, Copy)]
struct SceneAppearanceState {
    /// Active preset index (0..PRESET_COUNT-1).
    preset_idx: u8,
    /// 0 = use preset colors; nonzero = substitute nonzero custom_colors over preset.
    use_custom_colors: u8,
    /// Chrome layout flags (all bits reserved in V1; top bar via 0xFD, not here).
    chrome_flags: u8,
    /// Accessibility flags (bit 0 = high_contrast, bit 1 = colorblind_safe,
    /// bit 2 = stronger_focus_ring, bit 3 = larger_targets).
    accessibility_flags: u8,
    /// Custom color overrides (same layout as TokenPreset).
    /// Only slots with nonzero value override the preset when use_custom_colors != 0.
    custom_colors: [u32; 8],
}

const DEFAULT_SCENE_APPEARANCE: SceneAppearanceState = SceneAppearanceState {
    preset_idx: 0,
    use_custom_colors: 0,
    chrome_flags: 0,
    accessibility_flags: 0,
    custom_colors: [0u32; 8],
};

static mut SCENE_APPEARANCE_STATE: SceneAppearanceState = DEFAULT_SCENE_APPEARANCE;

const TINT_COUNT: usize = 5;
type TintBundle = [u32; 8];

// Slot order: [focus_surface, frame_rim, frame_top_bar, active_tab, inactive_tab,
//              close_light, minimize_light, zoom_light]
// Zero in any slot = keep preset value (handled by resolve_scene_render_tokens).
// Semantic lights (slots 5/6/7) are zero in all tints.
static CUSTOM_TINT_BUNDLES: [TintBundle; TINT_COUNT] = [
    // 0: Clear — all zeros → use_custom_colors = 0 (clean preset)
    [0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 1: WarmTint — amber/copper rim + topbar
    [0x00000000, 0x00D4822A, 0x00B86420, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 2: CoolTint — icy blue rim + topbar
    [0x00000000, 0x0080C8FF, 0x004488CC, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 3: CoralTint — coral focus_surface + pink rim
    [0x00CC5566, 0x00FF8090, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 4: GoldTint — gold rim + active tab
    [0x00000000, 0x00DDBB00, 0x00000000, 0x00DDBB00, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
];

static mut ACTIVE_TINT_IDX: u8 = 0;

/// Set when a GET is in flight at boot. Guards against misinterpreting
/// a PUT ack (0x00 or 0x02) as a GET result. Cleared when reply arrives.
static mut SEXSTORE_LOAD_PENDING: bool = false;

#[inline]
fn pack_u32_pair(lo: u32, hi: u32) -> u64 {
    (lo as u64) | ((hi as u64) << 32)
}

/// Push a token preset to sexdisplay via OP_APPEARANCE_TOKENS (0xFC).
/// Two sequential pdx_call messages; sexdisplay state machine disambiguates calls.
unsafe fn push_token_preset(p: &TokenPreset) {
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(p[0], p[1]),
        pack_u32_pair(p[2], p[3]),
        pack_u32_pair(p[4], p[5]),
    );
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(p[6], p[7]),
        0u64, // appearance_flags=0, effect_levels=0
        0u64, // reserved
    );
}

/// Resolve current SCENE_APPEARANCE_STATE to a sendable TokenPreset.
/// Starts from the active preset; substitutes nonzero custom_colors slots if enabled.
unsafe fn resolve_scene_render_tokens() -> TokenPreset {
    let idx = (SCENE_APPEARANCE_STATE.preset_idx as usize) % PRESET_COUNT;
    let base = TOKEN_PRESETS[idx];
    if SCENE_APPEARANCE_STATE.use_custom_colors == 0 {
        return base;
    }
    let mut result = base;
    let custom = &SCENE_APPEARANCE_STATE.custom_colors;
    for i in 0..8 {
        if custom[i] != 0 {
            result[i] = custom[i];
        }
    }
    result
}

/// Push resolved appearance tokens at boot. Preserves [shell.appearance.tokens.send] marker.
unsafe fn send_scene_render_tokens() {
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    unsafe {
        static mut SHELL_TOKEN_SEND_BUDGET: u32 = 4;
        if SHELL_TOKEN_SEND_BUDGET > 0 {
            SHELL_TOKEN_SEND_BUDGET -= 1;
            serial_println!("[shell.appearance.tokens.send] seq=2 sent");
        }
        static mut STATE_BUDGET: u32 = 1;
        if STATE_BUDGET > 0 {
            STATE_BUDGET -= 1;
            serial_println!("[shell.appearance.state] preset={} custom={} chrome={} access={}",
                SCENE_APPEARANCE_STATE.preset_idx,
                SCENE_APPEARANCE_STATE.use_custom_colors,
                SCENE_APPEARANCE_STATE.chrome_flags,
                SCENE_APPEARANCE_STATE.accessibility_flags);
        }
    }
}

/// Pack current preset_idx + chrome_flags + accessibility_flags into a
/// single u64 for sexstore PUT. Layout (little-endian):
///
///   Byte 0: magic     = 0xAC
///   Byte 1: version   = 0x01
///   Byte 2: preset_idx
///   Byte 3: chrome_flags
///   Byte 4: accessibility_flags
///   Byte 5-6: reserved (0)
///   Byte 7: checksum  = XOR(byte0 .. byte6) & 0x7F  (bit 7 cleared, preserves bit 63 = 0)
fn pack_scene_settings_blob(preset_idx: u8, chrome: u8, access: u8) -> u64 {
    let b: [u8; 8] = [
        SCENE_BLOB_MAGIC,
        SCENE_BLOB_VERSION,
        preset_idx,
        chrome,
        access,
        0u8,
        0u8,
        0u8, // placeholder for checksum
    ];
    // Mask to 7 bits: bit 7 of byte 7 = bit 63 of the u64, reserved for REPLY_STATUS_BIT.
    let chk: u8 = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
    let mut out = b;
    out[7] = chk;
    u64::from_le_bytes(out)
}

/// Unpack and validate a u64 from sexstore GET reply.
/// Returns Some((preset_idx, chrome_flags, accessibility_flags)) on success,
/// None if magic/version/checksum mismatch.
/// Caller must clamp preset_idx to PRESET_COUNT-1 before use.
fn unpack_scene_settings_blob(blob: u64) -> Option<(u8, u8, u8)> {
    let b: [u8; 8] = blob.to_le_bytes();
    if b[0] != SCENE_BLOB_MAGIC || b[1] != SCENE_BLOB_VERSION {
        return None;
    }
    let expected: u8 = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
    if b[7] != expected {
        return None;
    }
    Some((b[2], b[3], b[4]))
}

// ── E* storage reply protocol — local constants (match servers/sexstore/src/main.rs) ──
const STORE_REPLY_STATUS_BIT: u64 = 0x8000_0000_0000_0000;
const STORE_KV_OK:            u64 = 0x00;
const STORE_KV_NOT_FOUND:     u64 = 0x01;
const STORE_KV_FULL:          u64 = 0x02;
const STORE_KV_INVALID_KEY:   u64 = 0x03;
const STORE_KV_INVALID_VALUE: u64 = 0x04;
const STORE_KV_DENIED:        u64 = 0x05;

#[inline(always)]
fn store_reply_is_status(reply: u64) -> bool { reply & STORE_REPLY_STATUS_BIT != 0 }
#[inline(always)]
fn store_reply_status(reply: u64) -> u64 { reply & !STORE_REPLY_STATUS_BIT }
#[inline(always)]
fn store_reply_is_value(reply: u64) -> bool { reply & STORE_REPLY_STATUS_BIT == 0 }

/// Handle a validated GET reply: apply persisted fields, reset ephemeral
/// state to defaults, re-send tokens to sexdisplay.
unsafe fn handle_sexstore_get_reply(value: u64) {
    // E* protocol: bit 63 = 1 means status reply, not stored value.
    if store_reply_is_status(value) {
        let code = store_reply_status(value);
        unsafe {
            static mut STATUS_BUDGET: u32 = 4;
            if STATUS_BUDGET > 0 {
                STATUS_BUDGET -= 1;
                match code {
                    STORE_KV_NOT_FOUND => serial_println!("[shell.store.reply.status] code=not_found key=0x01"),
                    STORE_KV_DENIED    => serial_println!("[shell.store.reply.status] code=denied key=0x01"),
                    STORE_KV_FULL      => serial_println!("[shell.store.reply.status] code=full key=0x01"),
                    STORE_KV_INVALID_KEY   => serial_println!("[shell.store.reply.status] code=invalid_key key=0x01"),
                    STORE_KV_INVALID_VALUE => serial_println!("[shell.store.reply.status] code=invalid_value key=0x01"),
                    STORE_KV_OK        => serial_println!("[shell.store.reply.status] code=ok key=0x01"),
                    _                  => serial_println!("[shell.store.reply.reject] code={:#x} key=0x01", code),
                }
            }
        }
        // Any status on GET → keep defaults already applied at boot.
        unsafe {
            static mut DEFAULT_BUDGET: u32 = 2;
            if DEFAULT_BUDGET > 0 { DEFAULT_BUDGET -= 1; serial_println!("[shell.store.default] reason=status_reply"); }
        }
        return;
    }
    serial_println!("[shell.store.reply.value] key=0x01");
    if let Some((preset, chrome, access)) = unpack_scene_settings_blob(value) {
        let clamped_preset = if (preset as usize) < PRESET_COUNT { preset } else { 0 };
        SCENE_APPEARANCE_STATE.preset_idx = clamped_preset;
        SCENE_APPEARANCE_STATE.chrome_flags = chrome;
        SCENE_APPEARANCE_STATE.accessibility_flags = access;
        // Ephemeral state (not persisted): reset to defaults.
        SCENE_APPEARANCE_STATE.use_custom_colors = 0;
        SCENE_APPEARANCE_STATE.custom_colors = [0u32; 8];
        ACTIVE_TINT_IDX = 0;
        // Re-send tokens with restored settings.
        let tokens = resolve_scene_render_tokens();
        push_token_preset(&tokens);
        unsafe {
            static mut LOAD_OK_BUDGET: u32 = 1;
            if LOAD_OK_BUDGET > 0 {
                LOAD_OK_BUDGET -= 1;
                serial_println!("[shell.scene.settings.load] ok=1 preset={} chrome={} access={}",
                    clamped_preset, chrome, access);
            }
        }
    } else {
        // Blob invalid: either sexstore returned 0 (not found) or
        // magic/version/checksum didn't match (corrupt).
        // Keep defaults already sent at boot.
        unsafe {
            static mut LOAD_FAIL_BUDGET: u32 = 1;
            if LOAD_FAIL_BUDGET > 0 {
                LOAD_FAIL_BUDGET -= 1;
                if value == 0 {
                    serial_println!("[shell.scene.settings.load] ok=0 not-found");
                } else {
                    serial_println!("[shell.scene.settings.load] ok=0 corrupt");
                }
            }
        }
    }
}

/// Fire GET at boot to request persisted scene appearance settings.
/// Default tokens are already sent before this call; the reply is handled
/// asynchronously in the main loop via type_id == 0x1.
unsafe fn boot_load_scene_settings() {
    let (status, _) = pdx_call(SLOT_SEXSTORE, OP_KV_GET, SCENE_SETTINGS_KEY_APPEARANCE, 0, 0);
    unsafe {
        static mut BOOT_LOAD_BUDGET: u32 = 1;
        if BOOT_LOAD_BUDGET > 0 {
            BOOT_LOAD_BUDGET -= 1;
            if status == 0 {
                SEXSTORE_LOAD_PENDING = true;
                serial_println!("[shell.scene.settings.load.request] ok=1 pending");
            } else {
                serial_println!("[shell.scene.settings.load.request] ok=0 status={}", status);
            }
        }
    }
}

/// Advance to next preset (wrapping), clear custom override, and push resolved tokens.
unsafe fn cycle_scene_render_token_preset() {
    SCENE_APPEARANCE_STATE.preset_idx =
        (SCENE_APPEARANCE_STATE.preset_idx + 1) % PRESET_COUNT as u8;
    SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    ACTIVE_TINT_IDX = 0;
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    unsafe {
        static mut CYCLE_BUDGET: u32 = 16;
        if CYCLE_BUDGET > 0 {
            CYCLE_BUDGET -= 1;
            serial_println!("[shell.appearance.preset] idx={}", SCENE_APPEARANCE_STATE.preset_idx);
        }
    }

    // ── PERSIST: save new preset_idx to sexstore (fire-and-forget) ──
    let blob = pack_scene_settings_blob(
        SCENE_APPEARANCE_STATE.preset_idx,
        SCENE_APPEARANCE_STATE.chrome_flags,
        SCENE_APPEARANCE_STATE.accessibility_flags,
    );
    pdx_call(SLOT_SEXSTORE, OP_KV_PUT, SCENE_SETTINGS_KEY_APPEARANCE, blob, 0);
    unsafe {
        static mut SAVE_BUDGET: u32 = 16;
        if SAVE_BUDGET > 0 {
            SAVE_BUDGET -= 1;
            serial_println!("[shell.scene.settings.save] preset={}", SCENE_APPEARANCE_STATE.preset_idx);
        }
    }
}

unsafe fn apply_custom_tint_bundle(idx: usize) {
    if idx == 0 {
        SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    } else {
        let bundle = &CUSTOM_TINT_BUNDLES[idx];
        for i in 0..8 {
            SCENE_APPEARANCE_STATE.custom_colors[i] = bundle[i];
        }
        SCENE_APPEARANCE_STATE.use_custom_colors = 1;
    }
}

unsafe fn cycle_custom_tint() {
    ACTIVE_TINT_IDX = (ACTIVE_TINT_IDX + 1) % TINT_COUNT as u8;
    apply_custom_tint_bundle(ACTIVE_TINT_IDX as usize);
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    unsafe {
        static mut TINT_BUDGET: u32 = 32;
        if TINT_BUDGET > 0 {
            TINT_BUDGET -= 1;
            serial_println!("[shell.appearance.custom] mode=tint tint={}", ACTIVE_TINT_IDX);
        }
    }
}

// ── Scene Settings command handler ───────────────────────────────────────
/// Handle a Scene Settings IPC command from any caller.
/// cmd: one of CMD_SET_PRESET..CMD_RESET_DEFAULTS
/// value: command-specific argument (preset index, tint index, flags, etc.)
/// _flags: reserved for future use; ignored in V1.
///
/// Mutation and persistence rules per SCENE_SETTINGS_PROTOCOL_PLAN_V1.
/// Never blocks. All commands are safe — invalid inputs are silently clamped or ignored.
unsafe fn handle_scene_settings_cmd(cmd: u64, value: u64, _flags: u64) {
    static mut CMD_BUDGET: u32 = 32;
    let b = &mut CMD_BUDGET;
    match cmd {
        CMD_SET_PRESET => {
            let idx = if (value as usize) < PRESET_COUNT { value as u8 } else { 0 };
            SCENE_APPEARANCE_STATE.preset_idx = idx;
            SCENE_APPEARANCE_STATE.use_custom_colors = 0;
            SCENE_APPEARANCE_STATE.custom_colors = [0u32; 8];
            ACTIVE_TINT_IDX = 0;
            let tokens = resolve_scene_render_tokens();
            push_token_preset(&tokens);
            let blob = pack_scene_settings_blob(
                SCENE_APPEARANCE_STATE.preset_idx,
                SCENE_APPEARANCE_STATE.chrome_flags,
                SCENE_APPEARANCE_STATE.accessibility_flags,
            );
            pdx_call(SLOT_SEXSTORE, OP_KV_PUT, SCENE_SETTINGS_KEY_APPEARANCE, blob, 0);
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=1 preset={} ok=1", idx); }
        }
        CMD_CYCLE_PRESET => {
            cycle_scene_render_token_preset();
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=2 ok=1"); }
        }
        CMD_SET_TINT => {
            let idx = (value as u8) % TINT_COUNT as u8;
            ACTIVE_TINT_IDX = idx;
            apply_custom_tint_bundle(idx as usize);
            let tokens = resolve_scene_render_tokens();
            push_token_preset(&tokens);
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=3 tint={} ok=1", idx); }
        }
        CMD_CYCLE_TINT => {
            cycle_custom_tint();
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=4 ok=1"); }
        }
        CMD_SET_CHROME_FLAGS => {
            SCENE_APPEARANCE_STATE.chrome_flags = value as u8;
            let tokens = resolve_scene_render_tokens();
            push_token_preset(&tokens);
            let blob = pack_scene_settings_blob(
                SCENE_APPEARANCE_STATE.preset_idx,
                SCENE_APPEARANCE_STATE.chrome_flags,
                SCENE_APPEARANCE_STATE.accessibility_flags,
            );
            pdx_call(SLOT_SEXSTORE, OP_KV_PUT, SCENE_SETTINGS_KEY_APPEARANCE, blob, 0);
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=5 flags={} ok=1", value as u8); }
        }
        CMD_TOGGLE_TOP_BAR => {
            toggle_top_bar_for_active_frame();
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=6 ok=1"); }
        }
        CMD_SET_ACCESSIBILITY => {
            SCENE_APPEARANCE_STATE.accessibility_flags = value as u8;
            let tokens = resolve_scene_render_tokens();
            push_token_preset(&tokens);
            let blob = pack_scene_settings_blob(
                SCENE_APPEARANCE_STATE.preset_idx,
                SCENE_APPEARANCE_STATE.chrome_flags,
                SCENE_APPEARANCE_STATE.accessibility_flags,
            );
            pdx_call(SLOT_SEXSTORE, OP_KV_PUT, SCENE_SETTINGS_KEY_APPEARANCE, blob, 0);
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=7 flags={} ok=1", value as u8); }
        }
        CMD_RESET_DEFAULTS => {
            SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE;
            ACTIVE_TINT_IDX = 0;
            let tokens = resolve_scene_render_tokens();
            push_token_preset(&tokens);
            let blob = pack_scene_settings_blob(
                SCENE_APPEARANCE_STATE.preset_idx,
                SCENE_APPEARANCE_STATE.chrome_flags,
                SCENE_APPEARANCE_STATE.accessibility_flags,
            );
            pdx_call(SLOT_SEXSTORE, OP_KV_PUT, SCENE_SETTINGS_KEY_APPEARANCE, blob, 0);
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd=8 ok=1"); }
        }
        _ => {
            // Unknown command — log and ignore.
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.cmd] cmd={} ok=0 unknown", cmd); }
        }
    }
}

// ── Policy Model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    FocusToggle,
    Focus100, Focus101, Focus102, Focus103, Focus200,
    DestroyFocused,
    RecreateFocused,
    RestoreMinimized,
    ToggleTopBar,
    ToggleLinen,       // F8 — open/focus/toggle Linen surface
    ToggleQuil,        // F9 — open/focus/toggle Quil surface
    ToggleMesh,        // F12 — open/focus/toggle Mesh placeholder
    ToggleCollar,      // Insert — open/focus/toggle Collar placeholder
    ToggleBell,        // PageDown — open/focus/toggle Bell placeholder
    ToggleSpindle,     // Scroll Lock — open/focus/toggle Spindle terminal
    ToggleAtlas,       // F10 — toggle Atlas overview mode
    ToggleCommandPalette, // backtick — toggle command palette
    ToggleSceneSettingsPanel,
    CycleRenderTokenPreset,
    CycleCustomTint,
    ResetAll,
    SnapLeft, SnapRight, Maximize, Center,
    SnapHome, SnapEnd,
    ShrinkWidth, GrowWidth, ShrinkHeight, GrowHeight,
    LegacyFocusToggle,
    // K4: Cycle Linen object selection forward (J) / backward (K).
    SelectNextLinenObject,
    SelectPrevLinenObject,
    // J4: Open selected Linen object into a Quil buffer.
    OpenObjectInQuil,
    // D3: Accessibility keyboard actions using semantic node tree.
    AccessFocusNext,
    AccessFocusPrev,
    AccessActivate,
    // D3B: Complete shell-owned accessibility keyboard actions.
    AccessClose,         // close focused frame (F11)
    AccessZoomToggle,    // toggle zoom on focused frame (Esc)
    AccessSceneNext,     // switch to next scene (deferred binding)
    AccessScenePrev,     // switch to previous scene (deferred binding)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    Launcher,
    Status,
    Clock,
    Bell,
    Settings,
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
        0x01 => Some(SurfaceAction::AccessZoomToggle), // Esc
        0x0F => Some(SurfaceAction::AccessFocusNext), // Tab
        0x0E => Some(SurfaceAction::AccessFocusPrev),  // Backspace
        0x1C => Some(SurfaceAction::AccessActivate),   // Enter
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
        0x3E => Some(SurfaceAction::ToggleTopBar),           // F4
        0x3F => Some(SurfaceAction::CycleRenderTokenPreset), // F5
        0x40 => Some(SurfaceAction::CycleCustomTint),        // F6
        0x41 => Some(SurfaceAction::ToggleSceneSettingsPanel), // F7
        0x3B => Some(SurfaceAction::LegacyFocusToggle),
        0x42 => Some(SurfaceAction::ToggleLinen),    // F8
        0x43 => Some(SurfaceAction::ToggleQuil),     // F9
        0x44 => Some(SurfaceAction::ToggleAtlas),    // F10
        0x46 => Some(SurfaceAction::ToggleSpindle),  // Scroll Lock
        0x57 => Some(SurfaceAction::AccessClose),    // F11
        0x58 => Some(SurfaceAction::ToggleMesh),    // F12
        0x59 => Some(SurfaceAction::OpenObjectInQuil), // test trigger (not standard PS/2 key)
        0x52 => Some(SurfaceAction::ToggleCollar), // Insert
        0x51 => Some(SurfaceAction::ToggleBell),   // PageDown
	        0x47 => Some(SurfaceAction::SnapHome),
	        0x4F => Some(SurfaceAction::SnapEnd),
	        0x4B => Some(SurfaceAction::MoveLeft),
	        0x4D => Some(SurfaceAction::MoveRight),
	        0x48 => Some(SurfaceAction::MoveUp),
	        0x50 => Some(SurfaceAction::MoveDown),
	        // K4: Linen selection cycling - gated to Linen-focused state in handler.
	        0x24 => Some(SurfaceAction::SelectNextLinenObject), // J key
	        0x25 => Some(SurfaceAction::SelectPrevLinenObject), // K key
	        // K11: Command palette toggle - backtick/tilde.
	        0x29 => Some(SurfaceAction::ToggleCommandPalette), // backtick
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

/// Deterministic tiling for visible frames in the active scene.
/// Collects non-minimized frames in the active scene, then assigns
/// each frame's active-tab surface a position in the content area
/// below the SilkBar. Called after snap, maximize, close, restore,
/// or scene switch.
/// Layout scheme (V1):
///   1 frame  → full content area
///   2 frames → left/right split
///   3 frames → top-left, top-right, bottom-full
///   4 frames → 2x2 grid
///   5+       → stacked rows (full width, equal height)
unsafe fn tile_visible_frames() {
    serial_println!("[shell.tile.delegate] from=tile_visible_frames to=tile_active_scene_frames");
    tile_active_scene_frames();
}

/// B3: Deterministic tiling for the active scene.
/// Layout rules: 1=full, 2=vertical split, 3=master+stack, 4=2x2 grid, 5+=rows.
/// Filters: skips Minimized, Zoomed, Closing, Tombstoned, Destroyed, Hidden,
/// dead surfaces, stale-generation tabs, and lifecyle-non-focusable surfaces.
/// Sends 0xEC (upsert geometry) to sexdisplay for each tiled surface.
/// After tiling, validates current focus — clears if invalid.
unsafe fn tile_active_scene_frames() {
    serial_println!("[shell.tile.begin]");

    // Collect tiling candidates from the active scene.
    let mut tiles: [u64; MAX_FRAMES] = [0; MAX_FRAMES];
    let mut count: usize = 0;

    for frame_slot in FRAMES.iter() {
        if let Some(frame) = frame_slot {
            // Only tile frames in the active scene.
            if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
            // Minimized frames are hidden via 0xEE — skip tiling.
            if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
            // Zoomed frames occupy full content area via layout_maximize().
            // Tiling would overwrite the zoomed position.
            if (frame.flags & FRAME_FLAG_ZOOMED) != 0 { continue; }

            if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                let sid = tab.surface_id;

                // B3: Skip dead surfaces.
                if !surface_is_alive(sid) {
                    serial_println!("[tiling.frame.skip] sid={} reason=dead", sid);
                    serial_println!("[shell.tile.skip_dead] sid={} reason=dead", sid);
                    continue;
                }
                // B3: Skip tombstoned surfaces.
                if is_tombstoned(sid) {
                    serial_println!("[tiling.frame.skip] sid={} reason=tombstoned", sid);
                    serial_println!("[shell.tile.skip_dead] sid={} reason=tombstoned", sid);
                    continue;
                }
                // B3: Skip surfaces in non-focusable lifecycle states
                // (Closing, Destroyed, Hidden, Allocated).
                if !surface_is_lifecycle_focusable(sid) {
                    serial_println!("[tiling.frame.skip] sid={} reason=lifecycle", sid);
                    serial_println!("[shell.tile.skip_dead] sid={} reason=lifecycle", sid);
                    continue;
                }
                // B3: Skip surfaces with stale generation.
                if let Some(fr) = make_focus_ref(sid) {
                    if !focus_ref_is_current(&fr) {
                        serial_println!("[tiling.frame.skip] sid={} reason=generation", sid);
                        serial_println!("[shell.tile.skip_dead] sid={} reason=generation", sid);
                        continue;
                    }
                }

                if count < MAX_FRAMES {
                    tiles[count] = sid;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        // No tileable frames in active scene — clear stale focus, drag, hover.
        clear_focus_if_dead();
        clear_drag_if_dead();
        clear_hover_if_dead();
        clear_hover_if_wrong_scene();
        HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
        serial_println!("[shell.tile.reject] reason=no_tileable_frames");
        serial_println!("[tiling.done] frames=0");
        return;
    }

    let cw: u32 = P.width as u32;
    let ch: u32 = (P.height - P.bar_height) as u32;

    for i in 0..count {
        let sid = tiles[i];

        let (rx, ry, rw, rh) = if count == 1 {
            (0i32, P.bar_height, cw, ch)
        } else if count == 2 {
            let half_w = cw / 2;
            if i == 0 {
                (0i32, P.bar_height, half_w, ch)
            } else {
                (half_w as i32, P.bar_height, cw - half_w, ch)
            }
        } else if count == 3 {
            let half_w = cw / 2;
            let half_h = ch / 2;
            match i {
                0 => (0i32, P.bar_height, half_w, ch),
                1 => (half_w as i32, P.bar_height, cw - half_w, half_h),
                2 => (half_w as i32, P.bar_height + half_h as i32, cw - half_w, ch - half_h),
                _ => (0i32, P.bar_height, cw, ch),
            }
        } else if count == 4 {
            let half_w = cw / 2;
            let half_h = ch / 2;
            match i {
                0 => (0i32, P.bar_height, half_w, half_h),
                1 => (half_w as i32, P.bar_height, cw - half_w, half_h),
                2 => (0i32, P.bar_height + half_h as i32, half_w, ch - half_h),
                3 => (half_w as i32, P.bar_height + half_h as i32, cw - half_w, ch - half_h),
                _ => (0i32, P.bar_height, cw, ch),
            }
        } else {
            let row_h = ch / count as u32;
            let y_off = P.bar_height + (row_h * i as u32) as i32;
            (0i32, y_off, cw, if i + 1 == count { ch - (row_h * i as u32) } else { row_h })
        };

        // [shell.tile.apply]: budgeted tiled geometry application proof.
        static mut SHELL_TILE_APPLY_BUDGET: u32 = 24;
        let b = &mut SHELL_TILE_APPLY_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.tile.apply] sid={} x={} y={} w={} h={}", sid, rx, ry, rw, rh);
        }

        // Update local shadow state.
        match sid {
            SURFACE_ID_APP => {
                if let Some(w) = WINDOWS.get_mut(1) {
                    w.desc.x = rx; w.desc.y = ry;
                    w.desc.width = rw; w.desc.height = rh;
                }
            }
            SURFACE_ID_STATIC => {
                SURFACE_101_X = rx; SURFACE_101_Y = ry;
                SURFACE_101_W = rw; SURFACE_101_H = rh;
            }
            SURFACE_ID_TEST3 => {
                SURFACE_102_X = rx; SURFACE_102_Y = ry;
                SURFACE_102_W = rw; SURFACE_102_H = rh;
            }
            SURFACE_ID_TEST4 => {
                SURFACE_103_X = rx; SURFACE_103_Y = ry;
                SURFACE_103_W = rw; SURFACE_103_H = rh;
            }
            SURFACE_ID_LINEN => {
                SURFACE_200_X = rx; SURFACE_200_Y = ry;
                SURFACE_200_W = rw; SURFACE_200_H = rh;
            }
            SURFACE_ID_QUIL => {
                SURFACE_201_X = rx; SURFACE_201_Y = ry;
                SURFACE_201_W = rw; SURFACE_201_H = rh;
            }
            SURFACE_ID_MESH => {
                SURFACE_202_X = rx; SURFACE_202_Y = ry;
                SURFACE_202_W = rw; SURFACE_202_H = rh;
            }
            SURFACE_ID_COLLAR => {
                SURFACE_203_X = rx; SURFACE_203_Y = ry;
                SURFACE_203_W = rw; SURFACE_203_H = rh;
            }
            SURFACE_ID_BELL_PLACEHOLDER => {
                SURFACE_204_X = rx; SURFACE_204_Y = ry;
                SURFACE_204_W = rw; SURFACE_204_H = rh;
            }
            SURFACE_ID_SPINDLE => {
                SURFACE_0x99_X = rx; SURFACE_0x99_Y = ry;
                SURFACE_0x99_W = rw; SURFACE_0x99_H = rh;
            }
            _ => {}
        }

        // Send 0xEC upsert to sexdisplay (A7-proven move/resize primitive).
        pdx_call(SLOT_DISPLAY, 0xEC, sid,
            (ry as u64) << 32 | rx as u64,
            (rh as u64) << 32 | rw as u64);

        // Quil visual placeholder: set distinctive fill rect after geometry update.
        if sid == SURFACE_ID_QUIL {
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0,
                (QUIL_PLACEHOLDER_COLOR as u64) << 32 | ((rh as u64) << 16) | rw as u64);
        }
    }

    // B3: After tiling, validate current focus.
    // Current focus survives only if B2 guards still pass.
    let focused = FOCUSED_SURFACE_ID;
    if focused != 0 {
        let still_valid = surface_is_alive(focused)
            && !is_tombstoned(focused)
            && surface_is_lifecycle_focusable(focused)
            && surface_in_active_scene(focused);

        if !still_valid {
            serial_println!("[tiling.focus.clear] sid={} reason=invalid_after_tiling", focused);
            // Delegate to try_set_focus for full guard validation.
            // If no candidate is valid, focus clears to 0.
            if count > 0 {
                try_set_focus(tiles[0]);
            } else {
                try_set_focus(0);
            }
        }
    }

    serial_println!("[shell.tile.after_lifecycle] frames={}", count);
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
const MAX_FRAMES: usize = 9;

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
    /// The workspace/scene this frame belongs to.
    scene_id: u8,
    /// Reserved for future flags (split orientation, pinned state, etc.).
    flags: u32,
    /// Saved normal (pre-zoom) geometry. Valid when FRAME_FLAG_ZOOMED is set.
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}

// ── B1: Scene/Frame/Tab Core Model (type-safe wrappers) ──────────────────────
// Compact type-safe identifiers for new scene/frame/tab code.
// Existing code continues to use raw u8/u32 for backward compatibility.

/// B1: Type-safe scene identifier (0..WORKSPACE_COUNT-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct SceneId(u8);

/// B1: Type-safe frame identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct FrameId(u32);

/// B1: Type-safe tab index within a frame's tab stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct TabIndex(u8);

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

// ── Chrome Template Model ─────────────────────────────────────────────────────
/// Simple rectangle for chrome geometry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Rect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl Rect {
    const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Rect { x, y, w, h }
    }

    fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w as i32
            && py >= self.y && py < self.y + self.h as i32
    }
}

/// Data-driven chrome template for Silk panels/frames.
/// Centralizes all geometry constants that the shell uses for hit-testing
/// and dispatch. No visual behavior change -- values match current defaults.
/// Future Glass Chrome can read/change template values safely.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ChromeTemplate {
    /// Neon rim thickness in pixels (all edges).
    rim_px: i32,
    /// Top bar chrome band height in pixels (0 = disabled/minimal mode).
    top_bar_height_px: i32,
    /// Frame light square size in minimal/4px mode.
    light_size_px: i32,
    /// Gap between adjacent frame lights in minimal mode.
    light_gap_px: i32,
    /// Frame light square size in default/top-bar mode.
    top_bar_light_size_px: i32,
    /// Gap between adjacent frame lights in default/top-bar mode.
    top_bar_light_gap_px: i32,
    /// Frame Lights exclusion zone width in top-bar mode.
    top_bar_light_exclusion_px: i32,
    /// Frame Lights exclusion zone width in minimal mode.
    tab_light_exclusion_px: i32,
    /// Minimum tab strip slot width.
    tab_min_width_px: i32,
    /// Tab strip band height in minimal mode.
    tab_strip_px: i32,
    // Scene Settings panel geometry
    settings_panel_x: u32,
    settings_panel_y: u32,
    settings_panel_w: u32,
    settings_panel_h: u32,
    // Scene Settings panel control rects (reserved; all zero in V1)
    control_preset_up: Rect,
    control_preset_down: Rect,
    control_reset: Rect,
    control_close: Rect,
    control_topbar_toggle: Rect,
}

/// Default chrome template matching current hardcoded values.
/// No visual behavior change. Future Glass Chrome can derive from this.
const SILK_CHROME_TEMPLATE_DEFAULT: ChromeTemplate = ChromeTemplate {
    rim_px: 4,
    top_bar_height_px: 16,
    light_size_px: 4,
    light_gap_px: 2,
    top_bar_light_size_px: 8,
    top_bar_light_gap_px: 4,
    top_bar_light_exclusion_px: 40,
    tab_light_exclusion_px: 20,
    tab_min_width_px: 12,
    tab_strip_px: 4,
    settings_panel_x: 870,
    settings_panel_y: 60,
    settings_panel_w: 340,
    settings_panel_h: 280,
    control_preset_up: Rect::new(0, 0, 0, 0),
    control_preset_down: Rect::new(0, 0, 0, 0),
    control_reset: Rect::new(0, 0, 0, 0),
    control_close: Rect::new(0, 0, 0, 0),
    control_topbar_toggle: Rect::new(0, 0, 0, 0),
};

// ── Frame Chrome Hit-Production Constants ──────────────────────────────────
/// Chrome hit-target kind for the 4px neon rim edge band.
const FRAME_CHROME_RIM: u32 = 1;
/// Chrome hit-target kind for a tab strip band (reserved, not produced in V1).
const FRAME_CHROME_TAB_STRIP: u32 = 2;
/// Thickness of the neon rim edge band in pixels.
const FRAME_RIM_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.rim_px;
/// Height of the tab strip band in pixels (0 = disabled in V1).
const FRAME_TAB_STRIP_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.tab_strip_px;
/// X-width of the Frame Lights exclusion zone in the top rim band.
/// Covers: gap(2) + close(4) + gap(2) + minimize(4) + gap(2) + zoom(4) + gap(2) = 20px.
const FRAME_TAB_LIGHT_EXCLUSION_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.tab_light_exclusion_px;
/// Minimum width of a single tab block in the tab strip.
const FRAME_TAB_MIN_WIDTH_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.tab_min_width_px;

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
const FRAME_LIGHT_SIZE_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.light_size_px;
/// Gap between adjacent frame lights in pixels.
const FRAME_LIGHT_GAP_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.light_gap_px;

// ── Top Bar Geometry Constants (default mode) ─────────────────────────────────
/// Height of the top bar chrome band in default mode (replaces top rim).
/// The 4px neon rim on the top edge is replaced by this taller band.
const FRAME_TOP_BAR_HEIGHT_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.top_bar_height_px;
/// Width and height of each frame light in default mode (larger than minimal mode).
const FRAME_TOP_BAR_LIGHT_SIZE_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_size_px;
/// Gap between adjacent frame lights in default mode.
const FRAME_TOP_BAR_LIGHT_GAP_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_gap_px;
/// X-width of the Frame Lights exclusion zone in default mode.
/// Covers: gap(4) + close(8) + gap(4) + minimize(8) + gap(4) + zoom(8) + gap(4) = 40px.
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: i32 = SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_exclusion_px;

/// ShellFrame.flags: frame is minimized (hidden via 0xEE, not destroyed).
const FRAME_FLAG_MINIMIZED: u32 = 1 << 0;
/// ShellFrame.flags: frame is zoomed/maximized (fills content area below SilkBar).
const FRAME_FLAG_ZOOMED: u32 = 1 << 1;
/// ShellFrame.flags: frame has top bar chrome band (default mode).
/// When clear (minimal mode), only 4px neon rim is rendered.
const FRAME_FLAG_TOP_BAR: u32 = 1 << 2;

// ── D2: Accessibility Node Model ──────────────────────────────────────────
/// Maximum semantic nodes in V1 flat tree. 64 covers ~32 frames+tabs +
/// SilkBar + scenes + Atlas + placeholders.
const MAX_ACCESS_NODES: usize = 64;

/// Semantic role for an access node. Shell chrome only in V1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum AccessRole {
    SilkBar          = 1,
    SceneChip        = 2,
    LauncherButton   = 3,
    StatusChip       = 4,
    ClockDisplay     = 5,
    BellIndicator    = 6,
    Frame            = 7,
    Tab              = 8,
    FrameLightClose  = 9,
    FrameLightMinimize = 10,
    FrameLightZoom   = 11,
    AtlasCard        = 12,
    SettingsPanel    = 13,
    Panel            = 14,
    AppPlaceholder   = 15,
    Desktop          = 16,
}

/// State flags for an access node. u16 bitmask.
type AccessStateFlags = u16;
const ACCESS_FOCUSED:   AccessStateFlags = 1 << 0;
const ACCESS_SELECTED:  AccessStateFlags = 1 << 1;
const ACCESS_VISIBLE:   AccessStateFlags = 1 << 2;
const ACCESS_HIDDEN:    AccessStateFlags = 1 << 3;
const ACCESS_MINIMIZED: AccessStateFlags = 1 << 4;
const ACCESS_ZOOMED:    AccessStateFlags = 1 << 5;
const ACCESS_DISABLED:  AccessStateFlags = 1 << 6;

/// Action flags for an access node. u16 bitmask.
type AccessActionFlags = u16;
const ACT_FOCUS:        AccessActionFlags = 1 << 0;
const ACT_ACTIVATE:     AccessActionFlags = 1 << 1;
const ACT_CLOSE:        AccessActionFlags = 1 << 2;
const ACT_MINIMIZE:     AccessActionFlags = 1 << 3;
const ACT_RESTORE:      AccessActionFlags = 1 << 4;
const ACT_ZOOM:         AccessActionFlags = 1 << 5;
const ACT_UNZOOM:       AccessActionFlags = 1 << 6;
const ACT_SWITCH_SCENE: AccessActionFlags = 1 << 7;
const ACT_CYCLE_ACCENT: AccessActionFlags = 1 << 8;
const ACT_TOGGLE_PIN:   AccessActionFlags = 1 << 9;

/// Target reference: which surface/frame/scene this node maps to.
/// All fields are 0/NONE when not applicable.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct AccessTargetRef {
    surface_id: u64,
    frame_id: u32,
    scene_id: u8,
}

/// A semantic access node. Fixed-size, no heap, no String.
/// Label is [u8; 32] — enough for V1 shell chrome labels.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct AccessNode {
    node_id: u32,
    role: AccessRole,
    state: AccessStateFlags,
    actions: AccessActionFlags,
    target: AccessTargetRef,
    label: [u8; 32],
}

/// Insert a byte-slice label into a fixed [u8; 32] array, null-terminated.
/// Copies at most 31 bytes + null terminator.
fn access_copy_label(dst: &mut [u8; 32], src: &[u8]) {
    let len = src.len().min(31);
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
}

/// Returns true if the target surface (if any) is a valid semantic target.
/// Excludes tombstoned/destroyed/dead surfaces. Frame-only surfaces without
/// a surface_id are considered valid (they carry frame-level semantics).
unsafe fn access_node_is_valid_target(target: &AccessTargetRef) -> bool {
    if target.surface_id == 0 && target.frame_id == 0 && target.scene_id == 0xFF {
        return false;
    }
    if target.surface_id != 0 {
        if !surface_is_alive(target.surface_id) || is_tombstoned(target.surface_id) {
            return false;
        }
    }
    if target.frame_id != 0 {
        let frame = FRAMES.iter().flatten().find(|f| f.frame_id == target.frame_id);
        if frame.is_none() {
            return false;
        }
    }
    true
}

/// Emit a scene chip node into the node array at the given index.
/// Returns the next index, or index if out of space.
unsafe fn access_emit_scene_node(nodes: &mut [Option<AccessNode>; MAX_ACCESS_NODES], idx: usize, scene_id: u8) -> usize {
    if idx >= MAX_ACCESS_NODES { return idx; }
    if !validate_scene_id(scene_id) { return idx; }

    let s = &SCENES[scene_id as usize];
    let flags = s.flags;
    let is_active = scene_id == ACTIVE_SCENE_IDX;

    let mut state: AccessStateFlags = 0;
    if is_active { state |= ACCESS_FOCUSED | ACCESS_SELECTED | ACCESS_VISIBLE; }
    if (flags & SCENE_FLAG_EMPTY) != 0 { state |= ACCESS_HIDDEN; }

    let mut actions: AccessActionFlags = 0;
    if !is_active { actions |= ACT_SWITCH_SCENE; }

    let mut label = [0u8; 32];
    // Trim null bytes from the scene label for a clean access label.
    let label_trimmed = core::str::from_utf8(&s.label).map(|s| s.trim_end_matches('\0')).unwrap_or("scene");
    let label_bytes = label_trimmed.as_bytes();
    access_copy_label(&mut label, label_bytes);

    nodes[idx] = Some(AccessNode {
        node_id: 0x1000 | scene_id as u32,
        role: AccessRole::SceneChip,
        state,
        actions,
        target: AccessTargetRef { surface_id: 0, frame_id: 0, scene_id },
        label,
    });

    static mut ACCESS_SCENE_BUDGET: u32 = 8;
    let b = &mut ACCESS_SCENE_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[access.node.scene] id={} role=SceneChip state={:#x} actions={:#x}", scene_id, state, actions); }

    idx + 1
}

/// Emit a frame node (and its tabs + lights) into the node array.
/// Returns the next index, or index if out of space.
unsafe fn access_emit_frame_node(nodes: &mut [Option<AccessNode>; MAX_ACCESS_NODES], idx: usize, frame: &ShellFrame) -> usize {
    if idx >= MAX_ACCESS_NODES { return idx; }

    let fid = frame.frame_id;
    let sid = active_surface_for_frame(fid).unwrap_or(0);
    let minimized = (frame.flags & FRAME_FLAG_MINIMIZED) != 0;
    let zoomed = (frame.flags & FRAME_FLAG_ZOOMED) != 0;
    let in_active_scene = frame.scene_id == ACTIVE_SCENE_IDX;
    let is_focused = sid != 0 && FOCUSED_SURFACE_ID == sid;

    // Skip if surface is dead
    if sid != 0 && (!surface_is_alive(sid) || is_tombstoned(sid)) {
        static mut ACCESS_SKIP_DEAD_BUDGET: u32 = 8;
        let b = &mut ACCESS_SKIP_DEAD_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[access.node.skip_dead] frame={} sid={}", fid, sid); }
        return idx;
    }

    let mut state: AccessStateFlags = 0;
    if is_focused { state |= ACCESS_FOCUSED; }
    if in_active_scene && !minimized { state |= ACCESS_VISIBLE; }
    if !in_active_scene { state |= ACCESS_HIDDEN; }
    if minimized { state |= ACCESS_MINIMIZED; }
    if zoomed { state |= ACCESS_ZOOMED; }

    let mut actions: AccessActionFlags = 0;
    if sid != 0 && in_active_scene && !minimized { actions |= ACT_FOCUS | ACT_ACTIVATE; }
    if sid != 0 && !minimized { actions |= ACT_MINIMIZE | ACT_CLOSE; }
    if minimized && sid != 0 { actions |= ACT_RESTORE; }
    if !zoomed && sid != 0 { actions |= ACT_ZOOM; }
    if zoomed { actions |= ACT_UNZOOM; }

    // Derive label from app spec or fallback to "Frame".
    let mut label = [0u8; 32];
    if let Some(spec) = app_surface_spec_by_frame(fid) {
        access_copy_label(&mut label, spec.name.as_bytes());
    } else {
        access_copy_label(&mut label, b"Frame");
    }

    nodes[idx] = Some(AccessNode {
        node_id: 0x2000 | fid,
        role: AccessRole::Frame,
        state,
        actions,
        target: AccessTargetRef { surface_id: sid, frame_id: fid, scene_id: frame.scene_id },
        label,
    });

    static mut ACCESS_FRAME_BUDGET: u32 = 8;
    let b = &mut ACCESS_FRAME_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[access.node.frame] id={} state={:#x} actions={:#x}", fid, state, actions); }

    idx + 1
}

/// Emit semantic nodes for all shell UI elements.
/// Populates the fixed-size buffer and returns the count of emitted nodes.
unsafe fn access_emit_shell_nodes(nodes: &mut [Option<AccessNode>; MAX_ACCESS_NODES]) -> usize {
    let mut idx = 0;

    // 1. Scene nodes
    for si in 0..ATLAS_MAX_SCENES as u8 {
        idx = access_emit_scene_node(nodes, idx, si);
    }

    // 2. Frame nodes (with implicit tabs + lights)
    for frame_slot in FRAMES.iter() {
        if let Some(ref frame) = frame_slot {
            idx = access_emit_frame_node(nodes, idx, frame);
        }
    }

    // 3. Quil placeholder (alive surfaces only)
    if surface_is_alive(SURFACE_ID_QUIL) && !unsafe { is_tombstoned(SURFACE_ID_QUIL) } {
        if idx < MAX_ACCESS_NODES {
            let mut label = [0u8; 32];
            access_copy_label(&mut label, b"Quil");
            nodes[idx] = Some(AccessNode {
                node_id: 0x3000 | SURFACE_ID_QUIL as u32,
                role: AccessRole::AppPlaceholder,
                state: ACCESS_VISIBLE,
                actions: ACT_FOCUS | ACT_ACTIVATE,
                target: AccessTargetRef { surface_id: SURFACE_ID_QUIL, frame_id: 0, scene_id: 0 },
                label,
            });
            idx += 1;
        }
    }

    // 4. Linen placeholder
    if surface_is_alive(SURFACE_ID_LINEN) && !is_tombstoned(SURFACE_ID_LINEN) {
        if idx < MAX_ACCESS_NODES {
            let mut label = [0u8; 32];
            access_copy_label(&mut label, b"Linen");
            nodes[idx] = Some(AccessNode {
                node_id: 0x3001,
                role: AccessRole::AppPlaceholder,
                state: ACCESS_VISIBLE,
                actions: ACT_FOCUS | ACT_ACTIVATE,
                target: AccessTargetRef { surface_id: SURFACE_ID_LINEN, frame_id: 0, scene_id: 0 },
                label,
            });
            idx += 1;
        }
    }

    static mut ACCESS_EMIT_BUDGET: u32 = 8;
    let b = &mut ACCESS_EMIT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[access.node.emit] count={}", idx); }

    idx
}

/// D3: Handle an accessibility keyboard action by building the semantic node
/// tree and dispatching to the appropriate lifecycle-safe path.
///
/// For AccessFocusNext/AccessFocusPrev: finds the next/prev valid node in the
/// semantic tree and focuses it via try_set_focus().
///
/// For AccessActivate: validates the focused node's action flags and dispatches
/// the appropriate existing action (close, minimize, restore, zoom, etc.)
/// based on the node's role and state.
///
/// All actions validate targets through lifecycle-safe paths. No new lifecycle
/// or focus semantics are created.
unsafe fn access_handle_keyboard_action(action: SurfaceAction) -> bool {
    let mut nodes: [Option<AccessNode>; MAX_ACCESS_NODES] = [None; MAX_ACCESS_NODES];
    let count = access_emit_shell_nodes(&mut nodes);
    if count == 0 { return false; }

    match action {
        SurfaceAction::AccessFocusNext | SurfaceAction::AccessFocusPrev => {
            let forward = action == SurfaceAction::AccessFocusNext;
            let current_sid = FOCUSED_SURFACE_ID;

            // Find current position in tree, then scan forward/backward.
            let start = if current_sid == 0 { 0 } else {
                // Find index of current focused surface
                let mut pos = 0;
                let mut found = false;
                for i in 0..count {
                    if let Some(ref node) = nodes[i] {
                        if node.target.surface_id != 0 && node.target.surface_id == current_sid {
                            pos = i;
                            found = true;
                            break;
                        }
                        pos += 1;
                    }
                }
                if !found { 0 } else { pos }
            };

            // Scan for the next valid node with a focusable surface.
            let len = count;
            for offset in 1..=len {
                let idx = if forward {
                    (start + offset) % len
                } else {
                    (start + len - offset) % len
                };
                if let Some(ref node) = nodes[idx] {
                    let sid = node.target.surface_id;
                    if sid != 0 && surface_is_alive(sid) && !is_tombstoned(sid)
                        && surface_is_lifecycle_focusable(sid)
                    {
                        let label = core::str::from_utf8(&node.label)
                            .unwrap_or("?").trim_end_matches('\0');
                        static mut ACCESS_FOCUS_NEXT_BUDGET: u32 = 8;
                        let b = &mut ACCESS_FOCUS_NEXT_BUDGET;
                        if *b > 0 { *b -= 1;
                            serial_println!("[access.action.focus_next] from={} to={} role={:?} label={}",
                                current_sid, sid, node.role, label);
                        }
                        return try_set_focus(sid);
                    }
                }
            }
            // No valid focus target found.
            static mut ACCESS_FOCUS_EMPTY_BUDGET: u32 = 4;
            let b = &mut ACCESS_FOCUS_EMPTY_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=focus_next reason=no_targets"); }
            false
        }

        SurfaceAction::AccessActivate => {
            let sid = FOCUSED_SURFACE_ID;
            if sid == 0 {
                static mut ACCESS_ACTIVATE_EMPTY_BUDGET: u32 = 4;
                let b = &mut ACCESS_ACTIVATE_EMPTY_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=activate reason=no_focus"); }
                return false;
            }
            if !surface_is_alive(sid) || is_tombstoned(sid) {
                static mut ACCESS_ACTIVATE_DEAD_BUDGET: u32 = 4;
                let b = &mut ACCESS_ACTIVATE_DEAD_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=activate reason=dead target={}", sid); }
                return false;
            }

            // Find the frame for this surface to determine available actions.
            let frame_id = frame_for_surface(sid).unwrap_or(0);

            // Activate dispatches based on the focused node's role and state.
            // For frames: try_set_focus (already focused), then minimize/restore/close.
            // For scene chips: switch_scene.
            // For placeholders: focus (already set).
            if let Some(frame_id_val) = frame_for_surface(sid) {
                // Surface has a frame — toggle minimize/restore as default activate action.
                if (FRAMES.iter().flatten()
                    .find(|f| f.frame_id == frame_id_val)
                    .map(|f| (f.flags & FRAME_FLAG_MINIMIZED) != 0))
                    .unwrap_or(false)
                {
                    // Minimized → restore
                    restore_minimized_frame(frame_id_val);
                    static mut ACCESS_ACTIVATE_RESTORE_BUDGET: u32 = 4;
                    let b = &mut ACCESS_ACTIVATE_RESTORE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[access.action.allow] action=activate target={} dispatch=restore", sid); }
                } else {
                    // Visible → minimize
                    minimize_frame(frame_id_val);
                    static mut ACCESS_ACTIVATE_MINIMIZE_BUDGET: u32 = 4;
                    let b = &mut ACCESS_ACTIVATE_MINIMIZE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[access.action.allow] action=activate target={} dispatch=minimize", sid); }
                }
            }
            // For non-frame surfaces (placeholders, panels), activate is a no-op
            // since they're already focused.
            static mut ACCESS_ACTIVATE_OK_BUDGET: u32 = 8;
            let b = &mut ACCESS_ACTIVATE_OK_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.keyboard.alt] action=activate target={}", sid); }
            true
        }

        // ── D3B: Close focused frame ──
        SurfaceAction::AccessClose => {
            let sid = FOCUSED_SURFACE_ID;
            if sid == 0 {
                static mut ACCESS_CLOSE_NOFOCUS_BUDGET: u32 = 4;
                let b = &mut ACCESS_CLOSE_NOFOCUS_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=close reason=no_focus"); }
                return false;
            }
            if !surface_is_alive(sid) || is_tombstoned(sid) {
                static mut ACCESS_CLOSE_DEAD_BUDGET: u32 = 4;
                let b = &mut ACCESS_CLOSE_DEAD_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=close reason=dead target={}", sid); }
                return false;
            }
            if close_surface_from_frame_light(sid) {
                static mut ACCESS_CLOSE_OK_BUDGET: u32 = 8;
                let b = &mut ACCESS_CLOSE_OK_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.close] target={}", sid); }
                return true;
            }
            static mut ACCESS_CLOSE_FAIL_BUDGET: u32 = 4;
            let b = &mut ACCESS_CLOSE_FAIL_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=close reason=failed target={}", sid); }
            false
        }

        // ── D3B: Toggle zoom on focused frame ──
        SurfaceAction::AccessZoomToggle => {
            let sid = FOCUSED_SURFACE_ID;
            if sid == 0 {
                static mut ACCESS_ZOOM_NOFOCUS_BUDGET: u32 = 4;
                let b = &mut ACCESS_ZOOM_NOFOCUS_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=zoom reason=no_focus"); }
                return false;
            }
            if !surface_is_alive(sid) || is_tombstoned(sid) {
                static mut ACCESS_ZOOM_DEAD_BUDGET: u32 = 4;
                let b = &mut ACCESS_ZOOM_DEAD_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=zoom reason=dead target={}", sid); }
                return false;
            }
            if let Some(fid) = frame_for_surface(sid) {
                if toggle_zoom_frame(fid) {
                    static mut ACCESS_ZOOM_OK_BUDGET: u32 = 8;
                    let b = &mut ACCESS_ZOOM_OK_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[access.action.zoom] frame={} target={}", fid, sid); }
                    return true;
                }
            }
            static mut ACCESS_ZOOM_FAIL_BUDGET: u32 = 4;
            let b = &mut ACCESS_ZOOM_FAIL_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.action.reject] action=zoom reason=failed target={}", sid); }
            false
        }

        // ── D3B: Scene switch helpers (bindings deferred) ──
        SurfaceAction::AccessSceneNext => {
            next_scene();
            static mut ACCESS_SCENE_NEXT_BUDGET: u32 = 8;
            let b = &mut ACCESS_SCENE_NEXT_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.action.scene_switch] dir=next"); }
            true
        }
        SurfaceAction::AccessScenePrev => {
            prev_scene();
            static mut ACCESS_SCENE_PREV_BUDGET: u32 = 8;
            let b = &mut ACCESS_SCENE_PREV_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[access.action.scene_switch] dir=prev"); }
            true
        }

        _ => false,
    }
}

// ── D4: Focus Description Proof ──────────────────────────────────────────
/// Compute a deterministic numeric token for a label byte array.
/// Simple DJB2-like hash over null-terminated [u8; 32]. No heap, no String.
/// Only shell-owned static/bounded labels are hashed — never app-provided names.
fn access_label_token(label: &[u8; 32]) -> u32 {
    let mut hash: u32 = 5381;
    for &b in label.iter() {
        if b == 0 { break; }
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    hash
}

/// Describe a semantic access node using numeric tokens only.
/// No app text, no document names, no user content — only shell-owned
/// role/id/state/action tokens.
/// Marker: [access.focus.describe] with structured fields.
unsafe fn access_describe_node(node: &AccessNode) {
    let label_token = access_label_token(&node.label);
    static mut ACCESS_DESCRIBE_NODE_BUDGET: u32 = 32;
    let b = &mut ACCESS_DESCRIBE_NODE_BUDGET;
    if *b > 0 {
        *b -= 1;
        serial_println!(
            "[access.focus.describe] node_id={} role={} state={:#x} actions={:#x} target_sid={} target_fid={} target_scene={} label_token={:#x}",
            node.node_id, node.role as u8, node.state, node.actions,
            node.target.surface_id, node.target.frame_id, node.target.scene_id,
            label_token
        );
        serial_println!(
            "[access.focus.label_token] node_id={} token={:#x}",
            node.node_id, label_token
        );
    }
}

/// Build the D2 semantic tree and find the focused node, then describe it.
/// Called from try_set_focus() after every successful focus change.
/// Pure logging — never mutates focus, lifecycle, or frame state.
unsafe fn access_describe_focus() {
    let mut nodes: [Option<AccessNode>; MAX_ACCESS_NODES] = [None; MAX_ACCESS_NODES];
    let count = access_emit_shell_nodes(&mut nodes);
    if count == 0 {
        static mut ACCESS_DESCRIBE_EMPTY_BUDGET: u32 = 8;
        let b = &mut ACCESS_DESCRIBE_EMPTY_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[access.focus.describe.reject] reason=empty_tree"); }
        return;
    }

    let sid = FOCUSED_SURFACE_ID;
    if sid == 0 {
        static mut ACCESS_DESCRIBE_NOFOCUS_BUDGET: u32 = 8;
        let b = &mut ACCESS_DESCRIBE_NOFOCUS_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[access.focus.describe.reject] reason=no_focus"); }
        return;
    }

    for i in 0..count {
        if let Some(ref node) = nodes[i] {
            if node.target.surface_id == sid {
                if !surface_is_alive(sid) || is_tombstoned(sid) {
                    static mut ACCESS_DESCRIBE_SKIP_BUDGET: u32 = 8;
                    let b = &mut ACCESS_DESCRIBE_SKIP_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[access.focus.describe.skip_dead] target={}", sid); }
                    return;
                }
                access_describe_node(node);
                return;
            }
        }
    }

    // Focused surface not found in semantic tree.
    static mut ACCESS_DESCRIBE_NOTFOUND_BUDGET: u32 = 8;
    let b = &mut ACCESS_DESCRIBE_NOTFOUND_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[access.focus.describe.reject] reason=not_in_tree target={}", sid); }
}

// ── Atlas Overview Model ─────────────────────────────────────────────────────
/// Atlas is Silk's shell-owned map of all Scenes.
/// It sits above Scene in the abstraction stack:
///   Silk → Atlas → Scene → Frame → Tab → Surface
/// V1 is data/model only: no rendering, no sexdisplay changes.
/// Future phases add Atlas toggle action, card rendering, scene select, previews.

/// Maximum scenes tracked by Atlas (equals WORKSPACE_COUNT).
const ATLAS_MAX_SCENES: usize = 5;
/// Maximum frames tracked per scene descriptor (equals MAX_FRAMES).
const ATLAS_MAX_FRAMES_PER_SCENE: usize = 9;
/// Length of fixed-size scene label byte array (no heap strings).
const ATLAS_LABEL_LEN: usize = 16;

/// SceneDescriptor flags
const SCENE_FLAG_ACTIVE: u8         = 1 << 0;  // this scene is active
const SCENE_FLAG_EMPTY: u8          = 1 << 1;  // scene has no frames
const SCENE_FLAG_HAS_FOCUS: u8      = 1 << 2;  // scene contains focused surface
const SCENE_FLAG_HAS_MINIMIZED: u8  = 1 << 3;  // scene has at least one minimized frame
const SCENE_FLAG_HAS_ZOOMED: u8     = 1 << 4;  // scene has at least one zoomed frame

// ── Scene Accent Token Constants ────────────────────────────────────────────
/// Index of the Clear (default, no accent) tint bundle.
const ACCENT_DEFAULT: u8 = 0;
/// Index of the WarmTint bundle (amber/copper).
const ACCENT_WARM: u8    = 1;
/// Index of the CoolTint bundle (icy blue).
const ACCENT_COOL: u8    = 2;
/// Index of the CoralTint bundle (pink/coral).
const ACCENT_CORAL: u8   = 3;
/// Index of the GoldTint bundle (gold).
const ACCENT_GOLD: u8    = 4;
/// Number of valid accent tokens (matches CUSTOM_TINT_BUNDLES count).
const ACCENT_COUNT: u8   = 5;

// ── Atlas Render Constants (card layout, colors) ─────────────────────────────
/// Atlas card width in pixels.
const ATLAS_CARD_W: u32 = 220;
/// Atlas card height in pixels.
const ATLAS_CARD_H: u32 = 150;
/// Gap between adjacent cards.
const ATLAS_CARD_GAP: i32 = 24;
/// Number of cards in first row (3 for 5 total).
const ATLAS_CARDS_ROW0: usize = 3;
/// Number of cards in second row (2 for 5 total).
const ATLAS_CARDS_ROW1: usize = 2;
/// Small frame indicator block width.
const ATLAS_FRAME_BLOCK_W: u32 = 36;
/// Small frame indicator block height.
const ATLAS_FRAME_BLOCK_H: u32 = 28;
/// Gap between frame blocks within a card.
const ATLAS_FRAME_BLOCK_GAP: i32 = 8;
/// Padding from card edge to frame blocks.
const ATLAS_FRAME_PAD: i32 = 12;
/// Upper card area height (above frame blocks) for scene color block.
const ATLAS_CARD_TOP_H: u32 = 100;

/// Atlas card colors (ARGB). Dim, non-saturated palette.
const ATLAS_COLOR_BG: u32 = 0x00182850;          // dark navy overlay background
const ATLAS_COLOR_CARD_ACTIVE: u32 = 0x004468c0; // brighter blue — active scene
const ATLAS_COLOR_CARD_SCENE: u32 = 0x00284878;  // medium blue — non-active scene
const ATLAS_COLOR_CARD_EMPTY: u32 = 0x00182850;  // dim — empty scene
const ATLAS_COLOR_FRAME_NORMAL: u32 = 0x003860a0; // frame block (normal)
const ATLAS_COLOR_FRAME_ZOOMED: u32 = 0x0048c080; // frame block (zoomed)
const ATLAS_COLOR_FRAME_MINIMIZED: u32 = 0x00304060; // frame block (minimized)
/// Bright cyan border drawn around the currently selected Atlas card.
const ATLAS_COLOR_SELECT: u32 = 0x0080e0ff;

// ── C3 Atlas Visual Polish Tokens (SilkGlass palette) ────────────────
/// Alias: background dark violet-black depth.
const ATLAS_BG_COLOR: u32 = ATLAS_COLOR_BG;
/// Alias: default card fill for non-active, non-empty scenes.
const ATLAS_CARD_COLOR: u32 = ATLAS_COLOR_CARD_SCENE;
/// Alias: active scene card fill.
const ATLAS_CARD_ACTIVE_COLOR: u32 = ATLAS_COLOR_CARD_ACTIVE;
/// Alias: empty scene card fill.
const ATLAS_CARD_EMPTY_COLOR: u32 = ATLAS_COLOR_CARD_EMPTY;
/// Alias: minimized frame hint block.
const ATLAS_CARD_MINIMIZED_HINT_COLOR: u32 = ATLAS_COLOR_FRAME_MINIMIZED;
/// Alias: zoomed frame hint block.
const ATLAS_CARD_ZOOMED_HINT_COLOR: u32 = ATLAS_COLOR_FRAME_ZOOMED;
/// Card fill for the nav-selected scene (violet-blue accent).
const ATLAS_CARD_SELECTED_COLOR: u32 = 0x005050ff;
/// Neon rim for the active scene card when not selected (muted cyan).
const ATLAS_CARD_ACTIVE_RIM_COLOR: u32 = 0x004090c0;
/// Atlas accent card colors (ARGB). Maps ACCENT_DEFAULT..ACCENT_GOLD.
/// These are the rim colors from CUSTOM_TINT_BUNDLES, adapted for card fill.
const ATLAS_ACCENT_COLORS: [u32; ACCENT_COUNT as usize] = [
    0x00000000, // 0: Clear (use default card color)
    0x00805020, // 1: Warm — amber/dark copper (from rim 0xD4822A, dimmed for card)
    0x00205080, // 2: Cool — muted icy blue (from rim 0x80C8FF, dimmed for card)
    0x00804050, // 3: Coral — muted pink (from rim 0xFF8090, dimmed for card)
    0x00807000, // 4: Gold — muted gold (from rim 0xDDBB00, dimmed for card)
];

/// Color of the pinned indicator dot (bright gold).
const ATLAS_PIN_COLOR: u32 = 0x00FFDD44;
/// Muted rim for inactive scene cards (very dim).
const ATLAS_CARD_INACTIVE_RIM_COLOR: u32 = 0x00204060;

/// ── Atlas Scene Tile Preview Polish (ATLAS_SCENE_TILE_PREVIEW_POLISH_V1) ──
/// Small visual distinctions for active/tiled/focused scenes.
/// Bright green dot on cards whose scene contains the focused surface.
const ATLAS_FOCUS_MARKER_COLOR: u32 = 0x0080FF80;
const ATLAS_FOCUS_MARKER_SIZE: u32 = 6;
/// Light violet accent bar below card top for scenes with >1 visible frame.
const ATLAS_TILE_COUNT_BAR_COLOR: u32 = 0x00C0C0FF;
const ATLAS_TILE_COUNT_BAR_H: u32 = 3;

/// Describes one Scene for the Atlas overview.
/// Derived from current shell state, not independently mutable.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SceneDescriptor {
    /// Scene index (0..ATLAS_MAX_SCENES-1).
    scene_id: u32,
    /// Human-readable label (fixed bytes, zero-padded). V1: index-based default.
    label: [u8; ATLAS_LABEL_LEN],
    /// Flags: SCENE_FLAG_*
    flags: u8,
    /// Accent token: index into ATLAS_ACCENT_COLORS (0..ACCENT_COUNT). 0 = clear.
    accent: u8,
    /// Pinned flag: when true, scene survives frame-close operations.
    pinned: bool,
    /// Focused frame_id in this scene, or 0 if none.
    focused_frame_id: u32,
    /// Number of valid entries in frame_ids[].
    frame_count: u8,
    /// Fixed-size array of frame IDs present in this scene.
    frame_ids: [u32; ATLAS_MAX_FRAMES_PER_SCENE],
}

/// Runtime per-scene tracking state managed by silk-shell.
/// Renderer receives only derived surface/display operations.
#[derive(Debug, Clone, Copy)]
struct Scene {
    /// Scene flags: SCENE_FLAG_*.
    flags: u8,
    /// Human-readable fixed label, zero-padded.
    label: [u8; ATLAS_LABEL_LEN],
    /// Accent token: index into CUSTOM_TINT_BUNDLES (0..ACCENT_COUNT).
    /// 0 = Clear/default (no accent). Used to differentiate scene chrome.
    accent: u8,
    /// Pinned flag: when true, the scene survives frame-close operations
    /// and is not auto-destroyed when empty. Default false in V1.
    pinned: bool,
}

/// Atlas snapshot: the shell's map of all Scenes, derived from existing state.
/// Produced by atlas_capture_snapshot() after scene switches and layout changes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct AtlasSnapshot {
    /// The currently active scene_id.
    active_scene_id: u32,
    /// Number of valid entries in scenes[] (always ATLAS_MAX_SCENES in V1).
    scene_count: u8,
    /// Descriptors for all scenes, indexed by scene_id.
    scenes: [SceneDescriptor; ATLAS_MAX_SCENES],
}

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
static mut FOCUSED_SURFACE_ID: u64 = SURFACE_ID_QUIL;
/// Active workspace/scene index (0..WORKSPACE_COUNT-1).
static mut ACTIVE_SCENE_IDX: u8 = 0;
/// Atlas snapshot: derived overview of all Scenes.
/// Updated by atlas_capture_snapshot() after scene switch or layout change.
static mut ATLAS_SNAPSHOT: AtlasSnapshot = AtlasSnapshot {
    active_scene_id: 0,
    scene_count: ATLAS_MAX_SCENES as u8,
    scenes: [SceneDescriptor {
        scene_id: 0,
        label: [0u8; ATLAS_LABEL_LEN],
        flags: 0,
        accent: 0,
        pinned: false,
        focused_frame_id: 0,
        frame_count: 0,
        frame_ids: [0u32; ATLAS_MAX_FRAMES_PER_SCENE],
    }; ATLAS_MAX_SCENES],
};

/// B1 runtime per-scene tracking state, indexed by scene_id.
static mut SCENES: [Scene; ATLAS_MAX_SCENES] = [Scene {
    flags: SCENE_FLAG_EMPTY,
    label: [0u8; ATLAS_LABEL_LEN],
    accent: 0,
    pinned: false,
}; ATLAS_MAX_SCENES];

/// Atlas mode enabled: when true, the shell is in overview mode (no rendering yet in V1).
/// Toggled by F10 (ToggleAtlas). State-only — no visual behavior changes in V1.
static mut ATLAS_MODE_ENABLED: bool = false;
/// Index of the currently selected scene in Atlas mode (0..4).
/// Reset to active_scene_id when entering Atlas. Updated by arrow key navigation.
static mut ATLAS_SELECTED_SCENE: u8 = 0;
/// A6: Tombstone event ring buffer (size = power of 2 for efficient modulo).
/// Overwrites oldest entry when full. Replaces old TOMBSTONES u64 ring.
const TOMBSTONE_RING_SIZE: usize = 16;
static mut TOMBSTONE_RING: [Option<TombstoneEvent>; TOMBSTONE_RING_SIZE] = [None; TOMBSTONE_RING_SIZE];
static mut TOMBSTONE_RING_NEXT: usize = 0;
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
// Scene Settings panel toggle state
static mut SCENE_SETTINGS_ACTIVE: bool = false;
// Scene Settings panel geometry (static position, no text labels in V1)
const SCENE_SETTINGS_PANEL_X: u32 = SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_x;
const SCENE_SETTINGS_PANEL_Y: u32 = SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_y;
const SCENE_SETTINGS_PANEL_W: u32 = SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_w;
const SCENE_SETTINGS_PANEL_H: u32 = SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_h;
// Linen surface 200 position tracking (stable — linen never moves)
static mut SURFACE_200_X: i32 = 900;
static mut SURFACE_200_Y: i32 = 500;
static mut SURFACE_200_W: u32 = 300;
static mut SURFACE_200_H: u32 = LINEN_SURFACE_VISUAL_H;
// Quil surface 201 position tracking
static mut SURFACE_201_X: i32 = 100;
static mut SURFACE_201_Y: i32 = 100;
static mut SURFACE_201_W: u32 = 640;
static mut SURFACE_201_H: u32 = 480;
// Mesh surface 202 position tracking
static mut SURFACE_202_X: i32 = 200;
static mut SURFACE_202_Y: i32 = 100;
static mut SURFACE_202_W: u32 = 640;
static mut SURFACE_202_H: u32 = 480;
// Collar surface 203 position tracking
static mut SURFACE_203_X: i32 = 300;
static mut SURFACE_203_Y: i32 = 100;
static mut SURFACE_203_W: u32 = 640;
static mut SURFACE_203_H: u32 = 480;
// Bell placeholder surface 204 position tracking
static mut SURFACE_204_X: i32 = 400;
static mut SURFACE_204_Y: i32 = 100;
static mut SURFACE_204_W: u32 = 640;
static mut SURFACE_204_H: u32 = 480;
// Spindle terminal surface 0x99 position tracking
static mut SURFACE_0x99_X: i32 = SPINDLE_BOOT_X;
static mut SURFACE_0x99_Y: i32 = SPINDLE_BOOT_Y;
static mut SURFACE_0x99_W: u32 = SPINDLE_BOOT_W;
static mut SURFACE_0x99_H: u32 = SPINDLE_BOOT_H;

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
        if let Some(w) = WINDOWS.get(1) {
            if SURFACE_100_ALIVE {
                pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_APP, w.desc.x as u64, w.desc.y as u64);
            }
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
        // Linen surface 200 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_LINEN, SURFACE_200_X as u64, SURFACE_200_Y as u64);
        // Quil surface 201 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_QUIL, SURFACE_201_X as u64, SURFACE_201_Y as u64);
        // Mesh surface 202 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_MESH, SURFACE_202_X as u64, SURFACE_202_Y as u64);
        // Collar surface 203 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_COLLAR, SURFACE_203_X as u64, SURFACE_203_Y as u64);
        // Bell placeholder surface 204 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_BELL_PLACEHOLDER, SURFACE_204_X as u64, SURFACE_204_Y as u64);
    }
}

/// Get the bounding box of a surface, if it has geometry.
/// Returns None for OS-owned surfaces (cursor, panels) and invalid IDs.
/// Used by chrome hit-testing to compute rim/tab-strip regions.
/// Duplicates the bounds match from point_in_surface to avoid refactoring it.
unsafe fn get_surface_bounds(sid: u64) -> Option<(i32, i32, u32, u32)> {
    match sid {
        SURFACE_ID_APP    => WINDOWS.get(1).map(|w| (w.desc.x, w.desc.y, w.desc.width, w.desc.height)),
        SURFACE_ID_STATIC => Some((SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H)),
        SURFACE_ID_TEST3  => Some((SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H)),
        SURFACE_ID_TEST4  => Some((SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H)),
        SURFACE_ID_LINEN  => Some((SURFACE_200_X, SURFACE_200_Y, SURFACE_200_W, SURFACE_200_H)),
        SURFACE_ID_QUIL   => Some((SURFACE_201_X, SURFACE_201_Y, SURFACE_201_W, SURFACE_201_H)),
        SURFACE_ID_MESH   => Some((SURFACE_202_X, SURFACE_202_Y, SURFACE_202_W, SURFACE_202_H)),
        SURFACE_ID_COLLAR => Some((SURFACE_203_X, SURFACE_203_Y, SURFACE_203_W, SURFACE_203_H)),
        SURFACE_ID_BELL_PLACEHOLDER => Some((SURFACE_204_X, SURFACE_204_Y, SURFACE_204_W, SURFACE_204_H)),
        SURFACE_ID_SPINDLE => Some((SURFACE_0x99_X, SURFACE_0x99_Y, SURFACE_0x99_W, SURFACE_0x99_H)),
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
            SURFACE_ID_APP    => WINDOWS.get(1).map_or((0,0,0,0), |w| (w.desc.x, w.desc.y, w.desc.width, w.desc.height)),
            SURFACE_ID_STATIC => (SURFACE_101_X, SURFACE_101_Y, SURFACE_101_W, SURFACE_101_H),
            SURFACE_ID_TEST3  => (SURFACE_102_X, SURFACE_102_Y, SURFACE_102_W, SURFACE_102_H),
            SURFACE_ID_TEST4  => (SURFACE_103_X, SURFACE_103_Y, SURFACE_103_W, SURFACE_103_H),
            SURFACE_ID_LINEN  => (SURFACE_200_X, SURFACE_200_Y, SURFACE_200_W, SURFACE_200_H),
            SURFACE_ID_QUIL   => (SURFACE_201_X, SURFACE_201_Y, SURFACE_201_W, SURFACE_201_H),
            SURFACE_ID_MESH   => (SURFACE_202_X, SURFACE_202_Y, SURFACE_202_W, SURFACE_202_H),
            SURFACE_ID_COLLAR => (SURFACE_203_X, SURFACE_203_Y, SURFACE_203_W, SURFACE_203_H),
            SURFACE_ID_BELL_PLACEHOLDER => (SURFACE_204_X, SURFACE_204_Y, SURFACE_204_W, SURFACE_204_H),
            SURFACE_ID_SPINDLE => (SURFACE_0x99_X, SURFACE_0x99_Y, SURFACE_0x99_W, SURFACE_0x99_H),
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
        SURFACE_ID_QUIL     => true,  // quil never destroys its surface
        SURFACE_ID_MESH     => true,  // mesh never destroys its surface
        SURFACE_ID_COLLAR   => true,  // collar never destroys its surface
        SURFACE_ID_BELL_PLACEHOLDER => true,  // bell placeholder never destroys its surface
        SURFACE_ID_SPINDLE  => true,  // spindle never destroys its surface
        SURFACE_ID_CURSOR   => true,  // cursor never destroyed
        SURFACE_ID_LAUNCHER => unsafe { LAUNCHER_ACTIVE },
        SURFACE_ID_STATUS   => unsafe { STATUS_ACTIVE },
        SURFACE_ID_CLOCK    => unsafe { CLOCK_ACTIVE },
        SURFACE_ID_BELL     => unsafe { BELL_ACTIVE },
        SURFACE_ID_SCENE_SETTINGS => unsafe { SCENE_SETTINGS_ACTIVE },
        _ => {
            serial_println!("[shell.surface.unknown.reject] surface_is_alive id={}", sid);
            false
        }
    }
}

/// A6: Record a tombstone event in the ring buffer.
/// Overwrites oldest entry when full. Replaces old tombstone_surface().
unsafe fn record_tombstone_event(
    sid: u64,
    old_state: LifecycleState,
    new_state: LifecycleState,
    reason: TombstoneReason,
) {
    let generation = surface_generation(sid).unwrap_or(0);
    let frame_id = frame_for_surface(sid).unwrap_or(0);
    let tab_index: u8 = 0; // single-tab in V1 — always 0
    let idx = TOMBSTONE_RING_NEXT;
    TOMBSTONE_RING[idx] = Some(TombstoneEvent {
        surface_id: sid,
        generation,
        old_state,
        new_state,
        reason,
        frame_id,
        tab_index,
    });
    TOMBSTONE_RING_NEXT = (idx + 1) % TOMBSTONE_RING_SIZE;
    serial_println!("[tombstone.event.record] sid={} old={:?} new={:?} reason={:?} gen={}",
        sid, old_state, new_state, reason, generation);
    serial_println!("[lifecycle.tombstone.record] sid={} old={:?} new={:?} reason={:?} gen={}",
        sid, old_state, new_state, reason, generation);
}

/// Returns true if `sid` is in the tombstone ring (recently closed, must
/// not be focused, dragged, hovered, or restored as live).
unsafe fn is_tombstoned(sid: u64) -> bool {
    for i in 0..TOMBSTONE_RING_SIZE {
        if let Some(ref event) = TOMBSTONE_RING[i] {
            if event.surface_id == sid {
                return true;
            }
        }
    }
    false
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
    if focused != 0 && (!surface_is_alive(focused) || !surface_is_lifecycle_focusable(focused)) {
        serial_println!("[shell.focus.clear_dead] sid={} reason=invalid", focused);
        if !surface_is_alive(focused) {
            serial_println!("[focus.ref.clear] id={} reason=dead", focused);
            // A6: Record tombstone when focus is cleared because the surface is dead.
            let st = lifecycle_state(focused).unwrap_or(LifecycleState::Allocated);
            record_tombstone_event(focused, st, st, TombstoneReason::FocusCleared);
        } else {
            serial_println!("[focus.ref.clear] id={} reason=not_focusable lifecycle={:?}",
                focused, lifecycle_state(focused));
        }
        let z_order = [SURFACE_ID_QUIL, SURFACE_ID_MESH, SURFACE_ID_COLLAR, SURFACE_ID_BELL_PLACEHOLDER, SURFACE_ID_LINEN, SURFACE_ID_TEST4,
                       SURFACE_ID_TEST3, SURFACE_ID_STATIC, SURFACE_ID_APP];
        let mut found = false;
        for &sid in &z_order {
            if sid == focused { continue; }
            if surface_is_alive(sid) && surface_is_lifecycle_focusable(sid) {
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
        if !surface_is_alive(surface_id) || !surface_is_lifecycle_focusable(surface_id) {
            serial_println!("[shell.drag.clear_dead] sid={} reason=invalid", surface_id);
            serial_println!("[shell.surface.drag.cancel.dead] id={}", surface_id);
            // A6: Record tombstone for drag cancelled due to dead surface.
            let st = lifecycle_state(surface_id).unwrap_or(LifecycleState::Allocated);
            record_tombstone_event(surface_id, st, st, TombstoneReason::DragCancelled);
            try_transition(InteractionState::Idle);
        }
    }
}

/// If the hovered frame's active surface is dead or tombstoned, clear hover.
/// Emits [shell.hover.clear.dead] with frame and surface id.
unsafe fn clear_hover_if_dead() {
    if HOVERED_FRAME_ID != 0 {
        if let Some(sid) = active_surface_for_frame(HOVERED_FRAME_ID) {
            if !surface_is_alive(sid) || is_tombstoned(sid) {
                serial_println!("[shell.hover.clear.dead] frame={} surface={} reason=dead", HOVERED_FRAME_ID, sid);
                HOVERED_FRAME_ID = 0;
                HOVER_KIND = HOVER_NONE;
                HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
            }
        }
    }
}

/// If currently dragging a surface that belongs to a non-active scene,
/// cancel the drag. Call after scene switch.
unsafe fn clear_drag_if_wrong_scene() {
    if let InteractionState::Dragging { surface_id, .. } = INTERACTION {
        if !surface_in_active_scene(surface_id) {
            static mut DRAG_SCENE_BUDGET: u32 = 4;
            let b = &mut DRAG_SCENE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.scene.drag.clear.wrong-scene] id={}", surface_id); }
            try_transition(InteractionState::Idle);
        }
    }
}



// ── A3: Lifecycle State Helpers ──────────────────────────────────────────────────
// Additive metadata only. No behavior change.
// All helpers are unsafe because they access static mut LIFECYCLE_TABLE.

/// Register a surface with its initial lifecycle state. Called once at boot.
/// Returns false if the table is full or the surface is already registered.
unsafe fn lifecycle_register(sid: u64, initial_state: LifecycleState) -> bool {
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if let Some((registered_sid, _)) = &LIFECYCLE_TABLE[i] {
            if *registered_sid == sid {
                // Already registered — update state on re-init.
                LIFECYCLE_TABLE[i] = Some((sid, SurfaceLifecycle {
                    state: initial_state,
                    generation: 1,
                }));
                return true;
            }
        }
    }
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if LIFECYCLE_TABLE[i].is_none() {
            LIFECYCLE_TABLE[i] = Some((sid, SurfaceLifecycle {
                state: initial_state,
                generation: 1,
            }));
            return true;
        }
    }
    false
}

/// Lookup the lifecycle state for a surface. Returns None for unknown surfaces.
unsafe fn lifecycle_state(sid: u64) -> Option<LifecycleState> {
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if let Some((registered_sid, ref record)) = LIFECYCLE_TABLE[i] {
            if registered_sid == sid {
                return Some(record.state);
            }
        }
    }
    None
}

/// Set the lifecycle state for a surface. Returns false if surface unknown.
/// Bumps the surface generation on transitions that invalidate stale references.
/// Additive only — does not change any behavioral boolean or flag.
unsafe fn set_lifecycle_state(sid: u64, next: LifecycleState) -> bool {
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if let Some((registered_sid, ref mut record)) = &mut LIFECYCLE_TABLE[i] {
            if *registered_sid == sid {
                let prev = record.state;
                if prev == next {
                    return true; // no-op
                }
                // Bump generation on transitions that invalidate stale references.
                let bump = matches!((prev, next),
                    (LifecycleState::Visible, LifecycleState::Closing)
                    | (LifecycleState::Hidden, LifecycleState::Closing)
                    | (LifecycleState::Minimized, LifecycleState::Closing)
                    | (LifecycleState::Closing, LifecycleState::Tombstoned)
                    | (LifecycleState::Tombstoned, LifecycleState::Destroyed)
                    | (_, LifecycleState::Destroyed)
                );
                if bump {
                    let new_gen = LIFECYCLE_GENERATION.wrapping_add(1);
                    if new_gen == 0 {
                        serial_println!("[lifecycle.generation.bump.wrap] sid={} prev={:?} next={:?}", sid, prev, next);
                        // STOP FIRST: wraparound requires audit of all FocusRef references.
                        // Saturate — do not wrap to 0.
                    } else {
                        LIFECYCLE_GENERATION = new_gen;
                        record.generation = new_gen;
                        serial_println!("[lifecycle.generation.bump] sid={} gen={} prev={:?} next={:?}", sid, new_gen, prev, next);
                    }
                }
                record.state = next;
                serial_println!("[lifecycle.transition.allow] sid={} from={:?} to={:?}", sid, prev, next);
                return true;
            }
        }
    }
    serial_println!("[lifecycle.transition.reject] sid={} reason=unknown_surface", sid);
    false
}

/// Lookup the current generation for a surface. Returns None for unknown surfaces.
unsafe fn surface_generation(sid: u64) -> Option<u64> {
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if let Some((registered_sid, ref record)) = LIFECYCLE_TABLE[i] {
            if registered_sid == sid {
                return Some(record.generation);
            }
        }
    }
    None
}

/// Bump the generation for a surface. Returns false if surface unknown.
unsafe fn bump_surface_generation(sid: u64) -> bool {
    for i in 0..LIFECYCLE_MAX_SURFACES {
        if let Some((registered_sid, ref mut record)) = &mut LIFECYCLE_TABLE[i] {
            if *registered_sid == sid {
                let new_gen = LIFECYCLE_GENERATION.wrapping_add(1);
                if new_gen == 0 {
                    serial_println!("[lifecycle.generation.bump.wrap] sid={}", sid);
                    return false;
                }
                LIFECYCLE_GENERATION = new_gen;
                record.generation = new_gen;
                serial_println!("[lifecycle.generation.bump] sid={} gen={}", sid, new_gen);
                return true;
            }
        }
    }
    false
}

/// Create a FocusRef for a surface. Returns None for unknown or zero surface_id.
unsafe fn make_focus_ref(sid: u64) -> Option<FocusRef> {
    if sid == 0 {
        return None;
    }
    let gen = surface_generation(sid)?;
    let fr = FocusRef { surface_id: sid, generation: gen };
    serial_println!("[lifecycle.focusref.make] sid={} gen={}", sid, gen);
    Some(fr)
}

/// Returns true if a FocusRef is still current (generation matches).
/// If the surface is unknown, returns false.
unsafe fn focus_ref_is_current(r: &FocusRef) -> bool {
    if let Some(current_gen) = surface_generation(r.surface_id) {
        if r.generation == current_gen {
            return true;
        }
        serial_println!("[lifecycle.focusref.reject] sid={} ref_gen={} current_gen={}",
            r.surface_id, r.generation, current_gen);
        false
    } else {
        serial_println!("[lifecycle.focusref.reject] sid={} reason=unknown_surface",
            r.surface_id);
        false
    }
}

/// Sync FOCUSED_SURFACE (FocusRef) from FOCUSED_SURFACE_ID.
/// Additive — does not change focus behavior.
unsafe fn sync_focus_ref() {
    let sid = FOCUSED_SURFACE_ID;
    if sid == 0 {
        FOCUSED_SURFACE = None;
    } else {
        FOCUSED_SURFACE = make_focus_ref(sid);
    }
}

/// Returns true if the surface is in a live lifecycle state
/// (Visible, Mapped, Hidden, or Minimized).
unsafe fn surface_is_lifecycle_live(sid: u64) -> bool {
    match lifecycle_state(sid) {
        Some(LifecycleState::Visible) | Some(LifecycleState::Mapped)
        | Some(LifecycleState::Hidden) | Some(LifecycleState::Minimized) => true,
        _ => false,
    }
}

/// Returns true if the surface can receive focus based on lifecycle state alone
/// (Visible or Mapped). Does not check scene membership, frame flags, or caller.
unsafe fn surface_is_lifecycle_focusable(sid: u64) -> bool {
    match lifecycle_state(sid) {
        Some(LifecycleState::Visible) | Some(LifecycleState::Mapped) => true,
        _ => false,
    }
}

/// Register all known surfaces with appropriate initial lifecycle states.
/// Called once at boot after frame initialization.
unsafe fn lifecycle_init_all() {
    // Boot app surfaces — all start Visible in active scene.
    lifecycle_register(SURFACE_ID_APP, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_STATIC, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_TEST3, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_TEST4, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_LINEN, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_QUIL, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_MESH, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_COLLAR, LifecycleState::Visible);
    lifecycle_register(SURFACE_ID_BELL_PLACEHOLDER, LifecycleState::Visible);
    // Cursor — always present, no frame.
    lifecycle_register(SURFACE_ID_CURSOR, LifecycleState::Mapped);
    // Panel surfaces — start Allocated (inactive, toggled on demand).
    lifecycle_register(SURFACE_ID_LAUNCHER, LifecycleState::Allocated);
    lifecycle_register(SURFACE_ID_STATUS, LifecycleState::Allocated);
    lifecycle_register(SURFACE_ID_CLOCK, LifecycleState::Allocated);
    lifecycle_register(SURFACE_ID_BELL, LifecycleState::Allocated);
    lifecycle_register(SURFACE_ID_SCENE_SETTINGS, LifecycleState::Allocated);
    // Atlas overlay — starts Allocated (toggled by F10).
    lifecycle_register(SURFACE_ID_ATLAS_OVERLAY, LifecycleState::Allocated);
    serial_println!("[lifecycle.state.init] lifecycle model initialized");
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
    // Registry lookup: app surfaces use their focusable field
    || app_surface_spec(sid).map_or(false, |s| s.focusable)
}

// ── Scene / Workspace Helpers ──────────────────────────────────────────────────
/// Return true if the given surface belongs to the active scene.
/// Surfaces with no frame association (panels, cursor) always pass.
unsafe fn surface_in_active_scene(sid: u64) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            for tab in frame.tabs.iter() {
                if let Some(t) = tab {
                    if t.surface_id == sid {
                        return frame.scene_id == ACTIVE_SCENE_IDX;
                    }
                }
            }
        }
    }
    // Surface not found in any frame.
    // Panels (0x90-0x96) and cursor are always visible regardless of scene.
    // Frame-owned surfaces (Linen, Quil) are NOT visible without a frame.
    if sid == SURFACE_ID_LINEN || sid == SURFACE_ID_QUIL || sid == SURFACE_ID_MESH
        || sid == SURFACE_ID_COLLAR || sid == SURFACE_ID_BELL_PLACEHOLDER {
        return false;
    }
    true // panels/cursor always visible
}

/// Return the scene_id of the frame containing this surface, if any.
/// Surfaces with no frame association (panels, cursor) return None.
/// B2: Used by try_set_focus to reject focus for surfaces in inactive scenes.
unsafe fn surface_scene_id(sid: u64) -> Option<u8> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            for tab in frame.tabs.iter() {
                if let Some(t) = tab {
                    if t.surface_id == sid {
                        return Some(frame.scene_id);
                    }
                }
            }
        }
    }
    None
}

/// If the focused surface belongs to a frame in a non-active scene,
/// clear focus to a surface in the active scene.
unsafe fn clear_focus_if_wrong_scene() {
    let focused = FOCUSED_SURFACE_ID;
    if focused != 0 && !surface_in_active_scene(focused) {
        serial_println!("[scene.focus.reject.inactive] sid={} scene=wrong_focused", focused);
        serial_println!("[shell.scene.focus.clear.wrong-scene] id={}", focused);
        // Try to focus the first alive surface in the active scene.
        let mut found = false;
        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    if surface_is_alive(tab.surface_id) && !is_tombstoned(tab.surface_id)
                            && surface_is_lifecycle_focusable(tab.surface_id) {
                        if try_set_focus(tab.surface_id) {
                            found = true;
                            break;
                        }
                    }
                }
            }
        }
        if !found {
            try_set_focus(0);
            serial_println!("[shell.scene.focus.clear.none]");
        }
    }
}

/// Sync lifecycle state to reflect scene visibility after a scene switch.
/// Active scene surfaces → Visible, inactive → Hidden.
/// Preserves Minimized, Closing, Tombstoned, Destroyed states unchanged.
/// Additive lifecycle metadata only — no display or focus changes.
unsafe fn sync_lifecycle_scene_visibility() {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            let in_active = frame.scene_id == ACTIVE_SCENE_IDX;
            let minimized = (frame.flags & FRAME_FLAG_MINIMIZED) != 0;
            for tab in frame.tabs.iter() {
                if let Some(t) = tab {
                    let sid = t.surface_id;
                    if !surface_is_alive(sid) { continue; }
                    if minimized { continue; }
                    if let Some(state) = lifecycle_state(sid) {
                        match state {
                            LifecycleState::Closing
                            | LifecycleState::Tombstoned
                            | LifecycleState::Destroyed => continue,
                            _ => {}
                        }
                    }
                    if in_active {
                        set_lifecycle_state(sid, LifecycleState::Visible);
                    } else {
                        set_lifecycle_state(sid, LifecycleState::Hidden);
                    }
                }
            }
        }
    }
    static mut SCENE_LIFECYCLE_VIS_BUDGET: u32 = 8;
    let b = &mut SCENE_LIFECYCLE_VIS_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[lifecycle.scene.sync] active={}", ACTIVE_SCENE_IDX); }
}

/// Hide surfaces belonging to non-active scenes, show surfaces belonging
/// to the active scene. Called after ACTIVE_SCENE_IDX changes.
unsafe fn sync_scene_visibility() {
    // Metadata first: sync lifecycle state before updating display.
    sync_lifecycle_scene_visibility();
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            let in_active = frame.scene_id == ACTIVE_SCENE_IDX;
            for tab in frame.tabs.iter() {
                if let Some(t) = tab {
                    let sid = t.surface_id;
                    if surface_is_alive(sid) && in_active {
                        // Re-activate surface on display.
                        let bounds = get_surface_bounds(sid);
                        if let Some((rx, ry, rw, rh)) = bounds {
                            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                                (ry as u64) << 32 | rx as u64,
                                (rh as u64) << 32 | rw as u64);
                        }
                    } else if surface_is_alive(sid) && !in_active {
                        // Hide surface on display.
                        pdx_call(SLOT_DISPLAY, 0xEE, sid, 0, 0);
                    }
                }
            }
        }
    }
    static mut SCENE_VISIBILITY_BUDGET: u32 = 8;
    let b = &mut SCENE_VISIBILITY_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.scene.visibility] sync"); }
}

// ── Scene Shortcut Command Helpers (SCENE_SHORTCUTS_V1) ─────────────────────
// These are deterministic command-level helpers for scene/frame/tab actions,
// callable from key bindings, synthetic input, or existing code paths.
// They use only the existing Scene/Frame/Tab model — no new IPC, no kernel changes.

/// Maximum workspace/scene count (0..WORKSPACE_COUNT-1).
const WORKSPACE_COUNT: u8 = 5;

// ── Atlas Capture ─────────────────────────────────────────────────────────────

/// Build a default fixed-size label for a scene index.
/// V1: "Scene N" padded with zeros. Future: user-settable labels.
fn atlas_default_label(scene_id: u32) -> [u8; ATLAS_LABEL_LEN] {
    let mut label = [0u8; ATLAS_LABEL_LEN];
    // Write "Scene X" where X is 0-9 as ASCII.
    let prefix = b"Scene ";
    let n = core::cmp::min(prefix.len(), ATLAS_LABEL_LEN.saturating_sub(2));
    label[..n].copy_from_slice(&prefix[..n]);
    if n < ATLAS_LABEL_LEN {
        label[n] = b'0' + (scene_id as u8).min(9);
    }
    label
}

/// B1: initialize shell-local Scene state.
/// Safe under current single-shell mutation model; no IPC, no allocation.
unsafe fn scene_init_all() {
    // Accent defaults cycle through available tints for visual distinction.
    let default_accents: [u8; ATLAS_MAX_SCENES] = [
        ACCENT_DEFAULT, // Scene 0: Clear (no accent)
        ACCENT_WARM,    // Scene 1: Warm amber/copper
        ACCENT_COOL,    // Scene 2: Cool icy blue
        ACCENT_CORAL,   // Scene 3: Coral pink
        ACCENT_GOLD,    // Scene 4: Gold
    ];

    for si in 0..ATLAS_MAX_SCENES {
        SCENES[si] = Scene {
            flags: SCENE_FLAG_EMPTY,
            label: atlas_default_label(si as u32),
            accent: default_accents[si],
            pinned: false,
        };
    }

    for si in 0..ATLAS_MAX_SCENES {
        scene_update_flags(si as u8);
    }

    static mut SETTINGS_INIT_BUDGET: u32 = 4;
    let b = &mut SETTINGS_INIT_BUDGET;
    if *b > 0 {
        *b -= 1;
        serial_println!("[atlas.scene.settings.init] scenes={} accents=[{},{},{},{},{}]",
            ATLAS_MAX_SCENES,
            SCENES[0].accent, SCENES[1].accent, SCENES[2].accent,
            SCENES[3].accent, SCENES[4].accent);
    }

    serial_println!("[scene.core.init] scenes={}", ATLAS_MAX_SCENES);
}

/// B1: recompute scene flags from shell-local frame state.
unsafe fn scene_update_flags(scene_idx: u8) {
    let idx = scene_idx as usize;
    if idx >= ATLAS_MAX_SCENES {
        return;
    }

    let mut flags: u8 = 0;
    let mut has_frames = false;

    for frame_slot in FRAMES.iter() {
        if let Some(frame) = frame_slot {
            if frame.scene_id as usize != idx {
                continue;
            }

            // A8+: Skip frames whose active tab surface is dead or tombstoned.
            if let Some(sid) = active_surface_for_frame(frame.frame_id) {
                if !surface_is_alive(sid) || is_tombstoned(sid) {
                    continue;
                }
            }

            has_frames = true;

            if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 {
                flags |= SCENE_FLAG_HAS_MINIMIZED;
            }

            if (frame.flags & FRAME_FLAG_ZOOMED) != 0 {
                flags |= SCENE_FLAG_HAS_ZOOMED;
            }
        }
    }

    if !has_frames {
        flags |= SCENE_FLAG_EMPTY;
    }

    SCENES[idx].flags = flags;
}

// ── Scene Settings Helpers (ATLAS_SCENE_SETTINGS_MODEL_V1) ──────────────────
/// Validate that a scene_id is within the valid range (0..ATLAS_MAX_SCENES).
#[inline]
unsafe fn validate_scene_id(scene_id: u8) -> bool {
    (scene_id as usize) < ATLAS_MAX_SCENES
}

/// Return the accent token for a scene, or ACCENT_DEFAULT if invalid.
#[inline]
unsafe fn scene_accent_token(scene_id: u8) -> u8 {
    let idx = scene_id as usize;
    if idx >= ATLAS_MAX_SCENES {
        static mut SETTINGS_REJECT_BUDGET: u32 = 8;
        let b = &mut SETTINGS_REJECT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.reject] fn=accent id={}", scene_id); }
        return ACCENT_DEFAULT;
    }
    SCENES[idx].accent
}

/// Return whether a scene is pinned, or false if invalid.
#[inline]
unsafe fn scene_is_pinned(scene_id: u8) -> bool {
    let idx = scene_id as usize;
    if idx >= ATLAS_MAX_SCENES {
        static mut SETTINGS_REJECT_BUDGET: u32 = 8;
        let b = &mut SETTINGS_REJECT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.reject] fn=pinned id={}", scene_id); }
        return false;
    }
    static mut SETTINGS_READ_BUDGET: u32 = 32;
    let b = &mut SETTINGS_READ_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.read] fn=pinned id={} val={}", scene_id, SCENES[idx].pinned); }
    SCENES[idx].pinned
}

/// Return a copy of the scene label, or zeroed array if invalid.
#[inline]
unsafe fn scene_label_token(scene_id: u8) -> [u8; ATLAS_LABEL_LEN] {
    let idx = scene_id as usize;
    if idx >= ATLAS_MAX_SCENES {
        static mut SETTINGS_REJECT_BUDGET: u32 = 8;
        let b = &mut SETTINGS_REJECT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.reject] fn=label id={}", scene_id); }
        return [0u8; ATLAS_LABEL_LEN];
    }
    SCENES[idx].label
}

/// Capture current shell state into the ATLAS_SNAPSHOT.
/// Derives SceneDescriptors from existing FRAMES, ACTIVE_SCENE_IDX, FOCUSED_SURFACE_ID.
/// Safe: no allocation, no IPC, no sexdisplay changes.
/// Called after scene switch and layout mutations.
unsafe fn atlas_capture_snapshot() {
    let mut snapshot = AtlasSnapshot {
        active_scene_id: ACTIVE_SCENE_IDX as u32,
        scene_count: ATLAS_MAX_SCENES as u8,
        scenes: [SceneDescriptor {
            scene_id: 0,
            label: [0u8; ATLAS_LABEL_LEN],
            flags: 0,
            focused_frame_id: 0,
            frame_count: 0,
            frame_ids: [0u32; ATLAS_MAX_FRAMES_PER_SCENE],
            accent: 0,
            pinned: false,
        }; ATLAS_MAX_SCENES],
    };

    // C1: [atlas.snapshot.start] — begin capture with lifecycle filtering.
    serial_println!("[atlas.snapshot.start]");

    // Derive focused frame for active scene.
    let active_focused_frame = selected_frame_id().unwrap_or(0);

    // B1: Refresh scene flags from FRAMES state.
    for si in 0..ATLAS_MAX_SCENES {
        scene_update_flags(si as u8);
    }

    for scene_idx in 0..ATLAS_MAX_SCENES {
        let sd = &mut snapshot.scenes[scene_idx];
        sd.scene_id = scene_idx as u32;
        sd.label = SCENES[scene_idx].label;
        sd.accent = SCENES[scene_idx].accent;
        sd.pinned = SCENES[scene_idx].pinned;

        let mut frame_count: u8 = 0;

        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id as usize != scene_idx { continue; }
                // C1: Skip minimized frames — hidden via 0xEE, not visible in tiling.
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 {
                    serial_println!("[atlas.snapshot.skip] scene={} frame={} reason=minimized", scene_idx, frame.frame_id);
                    continue;
                }
                // C1: Skip frames with dead, tombstoned, or lifecycle-invalid active tab.
                if let Some(sid) = active_surface_for_frame(frame.frame_id) {
                    if !surface_is_alive(sid) {
                        serial_println!("[atlas.snapshot.skip] scene={} frame={} sid={} reason=dead", scene_idx, frame.frame_id, sid);
                        continue;
                    }
                    if is_tombstoned(sid) {
                        serial_println!("[atlas.snapshot.skip] scene={} frame={} sid={} reason=tombstoned", scene_idx, frame.frame_id, sid);
                        continue;
                    }
                    // C1: Skip surfaces in non-tileable lifecycle states.
                    if let Some(state) = lifecycle_state(sid) {
                        match state {
                            LifecycleState::Closing | LifecycleState::Destroyed
                            | LifecycleState::Hidden => {
                                serial_println!("[atlas.snapshot.skip] scene={} frame={} sid={} reason=lifecycle:{:?}",
                                    scene_idx, frame.frame_id, sid, state);
                                continue;
                            }
                            _ => {}
                        }
                    }
                    // C1: Skip surfaces with stale generation.
                    if let Some(fr) = make_focus_ref(sid) {
                        if !focus_ref_is_current(&fr) {
                            serial_println!("[atlas.snapshot.skip] scene={} frame={} sid={} reason=generation",
                                scene_idx, frame.frame_id, sid);
                            continue;
                        }
                    }
                } else {
                    continue;
                }
                if frame_count >= ATLAS_MAX_FRAMES_PER_SCENE as u8 { break; }
                sd.frame_ids[frame_count as usize] = frame.frame_id;
                frame_count += 1;
                serial_println!("[atlas.snapshot.frame] scene={} frame={}", scene_idx, frame.frame_id);
            }
        }

        sd.frame_count = frame_count;
        // B1: Use cached scene flags instead of re-deriving.
        sd.flags = SCENES[scene_idx].flags;

        // Focus: only the active scene has a tracked focused frame.
        if scene_idx == ACTIVE_SCENE_IDX as usize {
            sd.flags |= SCENE_FLAG_ACTIVE;
            if active_focused_frame != 0 {
                sd.focused_frame_id = active_focused_frame;
                sd.flags |= SCENE_FLAG_HAS_FOCUS;
            }
        }

        serial_println!("[atlas.snapshot.scene] scene={} frames={} flags={:#x}",
            scene_idx, sd.frame_count, sd.flags);
    }

    ATLAS_SNAPSHOT = snapshot;

    static mut ATLAS_CAPTURE_BUDGET: u32 = 8;
    let b = &mut ATLAS_CAPTURE_BUDGET;
    if *b > 0 {
        *b -= 1;
        let active = ACTIVE_SCENE_IDX;
        serial_println!("[shell.atlas.capture] scenes={} active={}",
            ATLAS_MAX_SCENES, active);
    }
}

/// Returns true if Atlas overview mode is currently enabled.
/// State-only in V1 — no visual behavior change.
#[allow(dead_code)]
unsafe fn atlas_is_enabled() -> bool {
    ATLAS_MODE_ENABLED
}

/// Toggle Atlas overview mode on/off.
/// On enter: captures Atlas snapshot, clears hover/drag state.
/// On exit: nothing extra (normal shell mode resumes).
/// No sexdisplay changes, no rendering in V1.
unsafe fn atlas_toggle() {
    if ATLAS_MODE_ENABLED {
        // Exiting Atlas: clear overlay, restore normal rendering.
        atlas_clear_stub();
        ATLAS_MODE_ENABLED = false;
        serial_println!("[atlas.view.exit]");
        static mut ATLAS_EXIT_BUDGET: u32 = 4;
        let b = &mut ATLAS_EXIT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.atlas.exit]"); }
    } else {
        // Entering Atlas: render overlay, clear stale hover/drag.
        ATLAS_MODE_ENABLED = true;
        ATLAS_SELECTED_SCENE = ACTIVE_SCENE_IDX;
        serial_println!("[atlas.nav.enter.select] scene={}", ATLAS_SELECTED_SCENE);
        atlas_render_stub();
        clear_hover_if_wrong_scene();
        clear_drag_if_dead();
        serial_println!("[atlas.view.enter]");
        static mut ATLAS_ENTER_BUDGET: u32 = 4;
        let b = &mut ATLAS_ENTER_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.atlas.enter]"); }
    }
}

/// Exit Atlas overview mode if currently enabled. No-op if already in normal mode.
#[allow(dead_code)]
unsafe fn atlas_exit() {
    if ATLAS_MODE_ENABLED {
        ATLAS_MODE_ENABLED = false;
        atlas_capture_snapshot();
        static mut ATLAS_EXIT_BUDGET: u32 = 4;
        let b = &mut ATLAS_EXIT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.atlas.exit]"); }
    }
}

// ── Atlas Render Stub ─────────────────────────────────────────────────────────

/// Compute card position for a given scene index (0..4).
/// Layout: row 0 has 3 cards (scenes 0,1,2), row 1 has 2 cards (scenes 3,4),
/// each row centered horizontally in the content area.
fn atlas_card_pos(scene_idx: usize, cw: u32) -> (i32, i32, u32, u32) {
    let card_w = ATLAS_CARD_W;
    let card_h = ATLAS_CARD_H;
    let gap = ATLAS_CARD_GAP;
    // Y offset from overlay top: row 0 at 30px, row 1 below with gap.
    let (row, col) = if scene_idx < ATLAS_CARDS_ROW0 {
        (0i32, scene_idx as i32)
    } else {
        (1i32, (scene_idx - ATLAS_CARDS_ROW0) as i32)
    };
    let cards_in_row = if row == 0 { ATLAS_CARDS_ROW0 as i32 } else { ATLAS_CARDS_ROW1 as i32 };
    let total_w = (card_w * cards_in_row as u32) + (gap as u32 * (cards_in_row as u32 - 1));
    let start_x = ((cw as i32 - total_w as i32) / 2).max(0);
    let x = start_x + col * (card_w as i32 + gap);
    let y = 30 + row * (card_h as i32 + gap);
    (x, y, card_w, card_h)
}

/// Hit-test Atlas scene cards at screen position (px, py).
/// Returns the scene index (0..4) if the point is within a card,
/// or None if the click misses all cards.
/// Coordinate conversion: overlay surface starts at y=P.bar_height,
/// and atlas_card_pos() returns positions relative to the overlay.
fn atlas_scene_at_point(px: i32, py: i32) -> Option<u8> {
    let cw = P.width as u32;
    let local_y = py - P.bar_height;
    if cw == 0 || local_y < 0 { return None; }
    for scene_idx in 0..ATLAS_MAX_SCENES {
        let (cx, cy, card_w, card_h) = atlas_card_pos(scene_idx, cw);
        if px >= cx && px < cx + card_w as i32
            && local_y >= cy && local_y < cy + card_h as i32
        {
            return Some(scene_idx as u8);
        }
    }
    None
}

/// Handle keyboard input while Atlas mode is enabled.
/// All scancodes except F10 (0x44) are routed here when ATLAS_MODE_ENABLED is true.
/// Navigation: arrows move the selected card in the 3+2 layout.
/// Confirm (Enter): switch to selected scene and exit Atlas.
/// Cancel (Esc): exit Atlas without switching.
/// Number keys 1-5: switch directly to scene (N-1).
unsafe fn handle_atlas_keyboard(scancode: u8) -> bool {
    // ── Number keys 1-5: direct scene select ──
    if scancode >= 0x02 && scancode <= 0x06 {
        let scene_idx = (scancode - 0x02) as u8;
        if scene_idx < ATLAS_MAX_SCENES as u8 {
            serial_println!("[atlas.nav.activate] scene={} keys=number", scene_idx);
            pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_ATLAS_OVERLAY, 0, 0);
            if scene_idx != ACTIVE_SCENE_IDX {
                switch_scene(scene_idx);
            } else {
                sync_scene_visibility();
                clear_focus_if_dead();
                clear_drag_if_dead();
                clear_hover_if_dead();
                clear_hover_if_wrong_scene();
                tile_active_scene_frames();
                snap_capture_layout();
            }
            // C2: Emit focus result after scene activation.
            if FOCUSED_SURFACE_ID != 0 {
                serial_println!("[atlas.nav.focus.commit] scene={} sid={}", scene_idx, FOCUSED_SURFACE_ID);
            } else {
                serial_println!("[atlas.nav.focus.empty] scene={}", scene_idx);
            }
            ATLAS_MODE_ENABLED = false;
            static mut ATLAS_CONFIRM_BUDGET: u32 = 4;
            let b = &mut ATLAS_CONFIRM_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.confirm] id={}", scene_idx); }
            return true;
        }
    }

    match scancode {
        0x4B => { // Left arrow
            let sel = ATLAS_SELECTED_SCENE;
            ATLAS_SELECTED_SCENE = match sel {
                0 => 2, 1 => 0, 2 => 1,
                3 => 4, 4 => 3,
                _ => sel,
            };
            if ATLAS_SELECTED_SCENE != sel {
                serial_println!("[atlas.nav.move] dir=left from={} to={}", sel, ATLAS_SELECTED_SCENE);
            }
            atlas_render_stub();
            static mut ATLAS_KEY_BUDGET: u32 = 4;
            let b = &mut ATLAS_KEY_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.key] dir=right sel={}", ATLAS_SELECTED_SCENE); }
        }
        0x4D => { // Right arrow
            let sel = ATLAS_SELECTED_SCENE;
            ATLAS_SELECTED_SCENE = match sel {
                0 => 1, 1 => 2, 2 => 0,
                3 => 4, 4 => 3,
                _ => sel,
            };
            if ATLAS_SELECTED_SCENE != sel {
                serial_println!("[atlas.nav.move] dir=right from={} to={}", sel, ATLAS_SELECTED_SCENE);
            }
            atlas_render_stub();
            static mut ATLAS_KEY_BUDGET: u32 = 4;
            let b = &mut ATLAS_KEY_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.key] dir=right sel={}", ATLAS_SELECTED_SCENE); }
        }
        0x48 => { // Up arrow
            let sel = ATLAS_SELECTED_SCENE;
            ATLAS_SELECTED_SCENE = match sel {
                3 => 0, 4 => 1,
                _ => sel,
            };
            if ATLAS_SELECTED_SCENE != sel {
                serial_println!("[atlas.nav.move] dir=up from={} to={}", sel, ATLAS_SELECTED_SCENE);
                atlas_render_stub();
                static mut ATLAS_KEY_BUDGET: u32 = 4;
                let b = &mut ATLAS_KEY_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[shell.atlas.key] dir=up sel={}", ATLAS_SELECTED_SCENE); }
            }
        }
        0x50 => { // Down arrow
            let sel = ATLAS_SELECTED_SCENE;
            ATLAS_SELECTED_SCENE = match sel {
                0 => 3, 1 => 4, 2 => 4,
                _ => sel,
            };
            if ATLAS_SELECTED_SCENE != sel {
                serial_println!("[atlas.nav.move] dir=down from={} to={}", sel, ATLAS_SELECTED_SCENE);
                atlas_render_stub();
                static mut ATLAS_KEY_BUDGET: u32 = 4;
                let b = &mut ATLAS_KEY_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[shell.atlas.key] dir=down sel={}", ATLAS_SELECTED_SCENE); }
            }
        }
        0x1C => { // Enter - confirm selection
            let scene_idx = ATLAS_SELECTED_SCENE;
            serial_println!("[atlas.nav.activate] scene={} keys=number", scene_idx);
            pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_ATLAS_OVERLAY, 0, 0);
            if scene_idx != ACTIVE_SCENE_IDX {
                switch_scene(scene_idx);
            } else {
                sync_scene_visibility();
                clear_focus_if_dead();
                clear_drag_if_dead();
                clear_hover_if_dead();
                clear_hover_if_wrong_scene();
                tile_active_scene_frames();
                snap_capture_layout();
            }
            // C2: Emit focus result after scene activation.
            if FOCUSED_SURFACE_ID != 0 {
                serial_println!("[atlas.nav.focus.commit] scene={} sid={}", scene_idx, FOCUSED_SURFACE_ID);
            } else {
                serial_println!("[atlas.nav.focus.empty] scene={}", scene_idx);
            }
            ATLAS_MODE_ENABLED = false;
            static mut ATLAS_CONFIRM_BUDGET: u32 = 4;
            let b = &mut ATLAS_CONFIRM_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.confirm] id={}", scene_idx); }
        }
        0x01 => { // Escape - cancel, exit Atlas without switching
            serial_println!("[atlas.nav.cancel] scene={}", ACTIVE_SCENE_IDX);
            atlas_clear_stub();
            // C2: Emit focus result after cancel.
            if FOCUSED_SURFACE_ID != 0 {
                serial_println!("[atlas.nav.focus.commit] scene={} sid={}", ACTIVE_SCENE_IDX, FOCUSED_SURFACE_ID);
            } else {
                serial_println!("[atlas.nav.focus.empty] scene={}", ACTIVE_SCENE_IDX);
            }
            ATLAS_MODE_ENABLED = false;
            static mut ATLAS_CANCEL_BUDGET: u32 = 4;
            let b = &mut ATLAS_CANCEL_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.cancel]"); }
        }
        0x1E => { // 'A' — cycle accent token for selected scene
            let sel = ATLAS_SELECTED_SCENE;
            if validate_scene_id(sel) {
                let idx = sel as usize;
                let new_accent = (SCENES[idx].accent + 1) % ACCENT_COUNT;
                SCENES[idx].accent = new_accent;
                static mut ATLAS_ACCENT_BUDGET: u32 = 16;
                let b = &mut ATLAS_ACCENT_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.accent] scene={} accent={}", sel, new_accent); }
            } else {
                static mut ATLAS_UI_REJECT_BUDGET: u32 = 8;
                let b = &mut ATLAS_UI_REJECT_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.ui.reject] fn=accent scene={}", sel); }
            }
        }
        0x19 => { // 'P' — toggle pinned flag for selected scene
            let sel = ATLAS_SELECTED_SCENE;
            if validate_scene_id(sel) {
                let idx = sel as usize;
                let new_pinned = !SCENES[idx].pinned;
                SCENES[idx].pinned = new_pinned;
                static mut ATLAS_PIN_BUDGET: u32 = 16;
                let b = &mut ATLAS_PIN_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.pin] scene={} pinned={}", sel, new_pinned); }
            } else {
                static mut ATLAS_UI_REJECT_BUDGET: u32 = 8;
                let b = &mut ATLAS_UI_REJECT_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[atlas.scene.settings.ui.reject] fn=pin scene={}", sel); }
            }
        }
        _ => {
            static mut ATLAS_KEY_BUDGET: u32 = 4;
            let b = &mut ATLAS_KEY_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.key] scancode={:#x} noop", scancode); }
        }
    }
    true
}

/// Render Atlas overview using existing 0xEC/0xEF/0xEE protocol.
/// Creates a shell-owned overlay surface and draws scene cards as fill rects.
/// No sexdisplay changes, no new ABI, no thumbnails.
unsafe fn atlas_render_stub() {
    // Capture fresh snapshot before rendering.
    atlas_capture_snapshot();

    let cw = P.width as u32;
    let ch = (P.height - P.bar_height) as u32;
    if cw == 0 || ch == 0 { return; }

    // Create Atlas overlay surface (full content area, below SilkBar).
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_ATLAS_OVERLAY,
        (P.bar_height as u64) << 32 | 0u64,
        (ch as u64) << 32 | cw as u64);
    // A3: Track Atlas overlay lifecycle: Allocated -> Mapped.
    set_lifecycle_state(SURFACE_ID_ATLAS_OVERLAY, LifecycleState::Mapped);
    // Fill background with dark overlay color.
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
        0,
        (ATLAS_COLOR_BG as u64) << 32 | (ch as u64) << 16 | cw as u64);

    // C3 proof: visual tokens active.
    serial_println!("[atlas.visual.tokens]");
    // Draw cards for all scenes.
    for scene_idx in 0..ATLAS_MAX_SCENES {
        let sd = &ATLAS_SNAPSHOT.scenes[scene_idx];
        let (cx, cy, card_w, card_h) = atlas_card_pos(scene_idx, cw);
        let is_selected = ATLAS_SELECTED_SCENE < ATLAS_MAX_SCENES as u8
            && scene_idx == ATLAS_SELECTED_SCENE as usize;

        // Determine card color: selected card gets accent fill.
        let card_color = if is_selected {
            serial_println!("[atlas.visual.selected] scene={}", scene_idx);
            ATLAS_CARD_SELECTED_COLOR
        } else if (sd.flags & SCENE_FLAG_EMPTY) != 0 {
            ATLAS_CARD_EMPTY_COLOR
        } else if sd.accent != ACCENT_DEFAULT && (sd.accent as usize) < ACCENT_COUNT as usize {
            // Use accent color for non-empty, non-selected scenes with non-zero accent.
            let accent_color = ATLAS_ACCENT_COLORS[sd.accent as usize];
            static mut ATLAS_ACCENT_VISUAL_BUDGET: u32 = 8;
            if ATLAS_ACCENT_VISUAL_BUDGET > 0 {
                ATLAS_ACCENT_VISUAL_BUDGET -= 1;
                serial_println!("[atlas.scene.visual.accent] scene={} accent={}", scene_idx, sd.accent);
            }
            accent_color
        } else if sd.accent != ACCENT_DEFAULT {
            // Accent set but out of bounds — reject marker.
            static mut ATLAS_VISUAL_REJECT_BUDGET: u32 = 8;
            if ATLAS_VISUAL_REJECT_BUDGET > 0 {
                ATLAS_VISUAL_REJECT_BUDGET -= 1;
                serial_println!("[atlas.scene.visual.reject] scene={} reason=accent_oob accent={}", scene_idx, sd.accent);
            }
            if (sd.flags & SCENE_FLAG_ACTIVE) != 0 {
                ATLAS_CARD_ACTIVE_COLOR
            } else {
                ATLAS_CARD_COLOR
            }
        } else if (sd.flags & SCENE_FLAG_ACTIVE) != 0 {
            ATLAS_CARD_ACTIVE_COLOR
        } else {
            ATLAS_CARD_COLOR
        };
        serial_println!("[atlas.visual.card] scene={} color={:#x}", scene_idx, card_color);

        // Draw card background (top portion, above frame blocks).
        let top_h = ATLAS_CARD_TOP_H.min(card_h);
        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
            (cy as u64) << 32 | cx as u64,
            (card_color as u64) << 32 | (top_h as u64) << 16 | card_w as u64);

        // Draw frame indicator blocks at card bottom.
        let fc = sd.frame_count as usize;
        let fb_w = ATLAS_FRAME_BLOCK_W;
        let fb_h = ATLAS_FRAME_BLOCK_H;
        let fb_gap = ATLAS_FRAME_BLOCK_GAP;
        let fb_pad = ATLAS_FRAME_PAD;
        let fb_total_w = (fc as u32).saturating_sub(1) * (fb_w + fb_gap as u32) + fb_w;
        let fb_start_x = cx + (card_w as i32 - fb_total_w as i32) / 2;
        let fb_y = cy + ATLAS_CARD_TOP_H as i32 + fb_pad;

        for fi in 0..fc.min(ATLAS_MAX_FRAMES_PER_SCENE) {
            let fb_x = fb_start_x + fi as i32 * (fb_w as i32 + fb_gap);
            // Determine frame block color based on frame flags.
            // V1: check scene-level flags as approximation.
            let fb_color = if (sd.flags & SCENE_FLAG_HAS_ZOOMED) != 0 {
                ATLAS_CARD_ZOOMED_HINT_COLOR
            } else if (sd.flags & SCENE_FLAG_HAS_MINIMIZED) != 0 {
                ATLAS_CARD_MINIMIZED_HINT_COLOR
            } else {
                ATLAS_COLOR_FRAME_NORMAL
            };
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (fb_y as u64) << 32 | fb_x as u64,
                (fb_color as u64) << 32 | (fb_h as u64) << 16 | fb_w as u64);
        }

        // Draw pinned indicator dot at top-right corner of pinned scenes.
        if sd.pinned {
            let dot_size: i32 = 8;
            let dot_x = cx + card_w as i32 - dot_size - 4;
            let dot_y = cy + 4;
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (dot_y as u64) << 32 | dot_x as u64,
                (ATLAS_PIN_COLOR as u64) << 32 | (dot_size as u64) << 16 | dot_size as u64);
            static mut ATLAS_PIN_VISUAL_BUDGET: u32 = 8;
            if ATLAS_PIN_VISUAL_BUDGET > 0 {
                ATLAS_PIN_VISUAL_BUDGET -= 1;
                serial_println!("[atlas.scene.visual.pinned] scene={}", scene_idx);
            }
        }

        // C3: scene flags proof marker.
        serial_println!("[atlas.visual.flags] scene={} flags={:#x} frames={}",
            scene_idx, sd.flags, fc);

        // Draw selection border around the currently selected card.
        if is_selected {
            let border = 2i32;
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_COLOR_SELECT as u64) << 32 | (border as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                ((cy + card_h as i32 - border) as u64) << 32 | cx as u64,
                (ATLAS_COLOR_SELECT as u64) << 32 | (border as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_COLOR_SELECT as u64) << 32 | (card_h as u64) << 16 | border as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | (cx + card_w as i32 - border) as u64,
                (ATLAS_COLOR_SELECT as u64) << 32 | (card_h as u64) << 16 | border as u64);
        } else if (sd.flags & SCENE_FLAG_ACTIVE) != 0 {
            // Active scene card gets a stronger neon rim (2px, matching selected border).
            serial_println!("[atlas.visual.active] scene={}", scene_idx);
            let rim = 2i32;
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_CARD_ACTIVE_RIM_COLOR as u64) << 32 | (rim as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                ((cy + card_h as i32 - rim) as u64) << 32 | cx as u64,
                (ATLAS_CARD_ACTIVE_RIM_COLOR as u64) << 32 | (rim as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_CARD_ACTIVE_RIM_COLOR as u64) << 32 | (card_h as u64) << 16 | rim as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | (cx + card_w as i32 - rim) as u64,
                (ATLAS_CARD_ACTIVE_RIM_COLOR as u64) << 32 | (card_h as u64) << 16 | rim as u64);
        } else if (sd.flags & SCENE_FLAG_EMPTY) == 0 {
            // Non-empty, non-active, non-selected card gets a very dim rim.
            let rim = 1i32;
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_CARD_INACTIVE_RIM_COLOR as u64) << 32 | (rim as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                ((cy + card_h as i32 - rim) as u64) << 32 | cx as u64,
                (ATLAS_CARD_INACTIVE_RIM_COLOR as u64) << 32 | (rim as u64) << 16 | card_w as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | cx as u64,
                (ATLAS_CARD_INACTIVE_RIM_COLOR as u64) << 32 | (card_h as u64) << 16 | rim as u64);
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64) << 32 | (cx + card_w as i32 - rim) as u64,
                (ATLAS_CARD_INACTIVE_RIM_COLOR as u64) << 32 | (card_h as u64) << 16 | rim as u64);
        }

        // ── Atlas Scene/Frame Polish (ATLAS_SCENE_TILE_PREVIEW_POLISH_V1) ──
        // Focus marker: bright green dot if scene contains focused surface.
        if (sd.flags & SCENE_FLAG_HAS_FOCUS) != 0 {
            let ms = ATLAS_FOCUS_MARKER_SIZE as i32;
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                (cy as u64 + 4) << 32 | (cx as u64 + 4),
                (ATLAS_FOCUS_MARKER_COLOR as u64) << 32 | (ms as u64) << 16 | ms as u64);
            static mut ATLAS_FOCUS_MARKER_BUDGET: u32 = 8;
            if ATLAS_FOCUS_MARKER_BUDGET > 0 {
                ATLAS_FOCUS_MARKER_BUDGET -= 1;
                serial_println!("[atlas.preview.focus_marker] scene={}", scene_idx);
            }
        }
        // Tile-count accent bar: thin bright bar when scene has >1 visible frame.
        if fc > 1 {
            let bar_y = cy + ATLAS_CARD_TOP_H as i32 + 2;
            let bar_margin = 16i32;
            let bar_w = card_w as i32 - 2 * bar_margin;
            if bar_w > 0 {
                pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_ATLAS_OVERLAY,
                    (bar_y as u64) << 32 | (cx as u64 + bar_margin as u64),
                    (ATLAS_TILE_COUNT_BAR_COLOR as u64) << 32
                        | (ATLAS_TILE_COUNT_BAR_H as u64) << 16
                        | bar_w as u64);
                static mut ATLAS_TILE_COUNT_BUDGET: u32 = 8;
                if ATLAS_TILE_COUNT_BUDGET > 0 {
                    ATLAS_TILE_COUNT_BUDGET -= 1;
                    serial_println!("[atlas.preview.tile_count] scene={} frames={}", scene_idx, fc);
                }
            }
        }
        // Polish live marker (budgeted — per-card-per-render).
        static mut ATLAS_POLISH_BUDGET: u32 = 8;
        if ATLAS_POLISH_BUDGET > 0 {
            ATLAS_POLISH_BUDGET -= 1;
            serial_println!("[atlas.preview.polish] scene={} active={} focus={} frames={}",
                scene_idx,
                if (sd.flags & SCENE_FLAG_ACTIVE) != 0 { 1 } else { 0 },
                if (sd.flags & SCENE_FLAG_HAS_FOCUS) != 0 { 1 } else { 0 },
                fc);
        }
    }

    static mut ATLAS_RENDER_BUDGET: u32 = 4;
    let b = &mut ATLAS_RENDER_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.atlas.render]"); }
    static mut ATLAS_PREVIEW_REFRESH_BUDGET: u32 = 4;
    let rb = &mut ATLAS_PREVIEW_REFRESH_BUDGET;
    if *rb > 0 { *rb -= 1; serial_println!("[atlas.preview.refresh] scenes={}", ATLAS_MAX_SCENES); }
}

/// Clear Atlas overlay and restore normal scene rendering.
/// Destroys the overlay surface via 0xEE, then re-syncs scene visibility
/// and re-tiles visible frames.
unsafe fn atlas_clear_stub() {
    // Hide Atlas overlay surface.
    pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_ATLAS_OVERLAY, 0, 0);
    // A3: Track Atlas overlay lifecycle: Mapped -> Allocated.
    set_lifecycle_state(SURFACE_ID_ATLAS_OVERLAY, LifecycleState::Allocated);
    // Restore current scene visibility and tiling.
    sync_scene_visibility();
    clear_focus_if_dead();
    clear_drag_if_dead();
    clear_hover_if_dead();
    clear_hover_if_wrong_scene();
    tile_active_scene_frames();
    snap_capture_layout();

    static mut ATLAS_CLEAR_BUDGET: u32 = 4;
    let b = &mut ATLAS_CLEAR_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.atlas.clear] restore"); }
}

/// Switch active scene to a specific index. Safe: clamps to WORKSPACE_COUNT-1.
/// Calls sync_scene_visibility(), clears focus/drag/hover, re-tiles visible frames.
unsafe fn switch_scene(scene_idx: u8) {
    let idx = scene_idx.min(WORKSPACE_COUNT - 1);
    if idx == ACTIVE_SCENE_IDX { return; }
    let prev = ACTIVE_SCENE_IDX;
    ACTIVE_SCENE_IDX = idx;
    sync_scene_visibility();
    clear_focus_if_wrong_scene();
    clear_drag_if_dead();
    clear_drag_if_wrong_scene();
    clear_hover_if_wrong_scene();
    tile_active_scene_frames();
    serial_println!("[shell.interact.tile.return] source=scene.switch");
    snap_capture_layout();
    // B2: Update scene flags for the previous scene before switching.
    scene_update_flags(prev);
    // B1: Update scene flags for the new active scene.
    scene_update_flags(idx);
    // Capture Atlas snapshot after scene switch.
    atlas_capture_snapshot();
    static mut SCENE_SWITCH_SHORTCUT_BUDGET: u32 = 4;
    let b = &mut SCENE_SWITCH_SHORTCUT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.scene.shortcut.switch] from={} to={}", prev, ACTIVE_SCENE_IDX); }
    serial_println!("[scene.switch] from={} to={}", prev, idx);
    serial_println!("[shell.interact.scene.switch] from={} to={}", prev, idx);
}

/// Advance to the next workspace (wraps around).
unsafe fn next_scene() {
    let next = (ACTIVE_SCENE_IDX + 1) % WORKSPACE_COUNT;
    switch_scene(next);
}

/// Go to the previous workspace (wraps around).
unsafe fn prev_scene() {
    let prev = if ACTIVE_SCENE_IDX == 0 { WORKSPACE_COUNT - 1 } else { ACTIVE_SCENE_IDX - 1 };
    switch_scene(prev);
}

/// Find the first non-minimized frame in the active scene, optionally starting
/// from `start_frame_id` (exclusive, wrapping). Used by next/prev frame helpers.
/// Returns None if no valid frame exists in the active scene.
unsafe fn next_frame_in_scene(start_frame_id: u32, forward: bool) -> Option<u32> {
    let mut best: Option<u32> = None;
    if forward {
        // Find the next frame after start_frame_id (wrapping to 0).
        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    if !surface_is_alive(tab.surface_id) { continue; }
                    if is_tombstoned(tab.surface_id) { continue; }
                } else { continue; }
                if frame.frame_id <= start_frame_id { continue; }
                if best.map_or(true, |b| frame.frame_id < b) {
                    best = Some(frame.frame_id);
                }
            }
        }
        // Wrap: if no frame after start, try from the beginning.
        if best.is_none() {
            for f in FRAMES.iter() {
                if let Some(frame) = f {
                    if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                    if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
                    if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                        if !surface_is_alive(tab.surface_id) { continue; }
                        if is_tombstoned(tab.surface_id) { continue; }
                    } else { continue; }
                    if frame.frame_id >= start_frame_id { continue; }
                    if best.map_or(true, |b| frame.frame_id < b) {
                        best = Some(frame.frame_id);
                    }
                }
            }
        }
    } else {
        // Find the previous frame before start_frame_id (wrapping to end).
        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    if !surface_is_alive(tab.surface_id) { continue; }
                    if is_tombstoned(tab.surface_id) { continue; }
                } else { continue; }
                if frame.frame_id >= start_frame_id { continue; }
                if best.map_or(true, |b| frame.frame_id > b) {
                    best = Some(frame.frame_id);
                }
            }
        }
        // Wrap: if no frame before start, try from the end.
        if best.is_none() {
            for f in FRAMES.iter() {
                if let Some(frame) = f {
                    if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                    if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
                    if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                        if !surface_is_alive(tab.surface_id) { continue; }
                        if is_tombstoned(tab.surface_id) { continue; }
                    } else { continue; }
                    if frame.frame_id <= start_frame_id { continue; }
                    if best.map_or(true, |b| frame.frame_id > b) {
                        best = Some(frame.frame_id);
                    }
                }
            }
        }
    }
    best
}

/// Move focus to the next non-minimized frame in the active scene (wrapping).
unsafe fn focus_next_frame() {
    let current_fid = selected_frame_id().unwrap_or(0);
    if let Some(next_fid) = next_frame_in_scene(current_fid, true) {
        if let Some(sid) = active_surface_for_frame(next_fid) {
            if try_set_focus(sid) {
                static mut FOCUS_NEXT_FRAME_BUDGET: u32 = 4;
                let b = &mut FOCUS_NEXT_FRAME_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[shell.shortcut.focus_next_frame] frame={}", next_fid); }
            }
        }
    }
}

/// Move focus to the previous non-minimized frame in the active scene (wrapping).
unsafe fn focus_prev_frame() {
    let current_fid = selected_frame_id().unwrap_or(MAX_FRAMES as u32);
    if let Some(prev_fid) = next_frame_in_scene(current_fid, false) {
        if let Some(sid) = active_surface_for_frame(prev_fid) {
            if try_set_focus(sid) {
                static mut FOCUS_PREV_FRAME_BUDGET: u32 = 4;
                let b = &mut FOCUS_PREV_FRAME_BUDGET;
                if *b > 0 { *b -= 1; serial_println!("[shell.shortcut.focus_prev_frame] frame={}", prev_fid); }
            }
        }
    }
}

/// Switch to the next tab in the focused frame (wraps around).
/// Does nothing if only one tab is present.
unsafe fn focus_next_tab() {
    let fid = match selected_frame_id() {
        Some(f) => f,
        None => return,
    };
    let frame = match FRAMES.iter().find_map(|f| {
        if let Some(fr) = f { if fr.frame_id == fid { Some(fr) } else { None } } else { None }
    }) {
        Some(fr) => fr,
        None => return,
    };
    if frame.tab_count <= 1 { return; }
    let next_tab = (frame.active_tab as u32 + 1) % frame.tab_count as u32;
    // Drop the borrow before calling switch_to_tab (which mutates FRAMES).
    drop(frame);
    if switch_to_tab(fid, next_tab) {
        static mut FOCUS_NEXT_TAB_BUDGET: u32 = 4;
        let b = &mut FOCUS_NEXT_TAB_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.shortcut.focus_next_tab] frame={} tab={}", fid, next_tab); }
    }
}

/// Switch to the previous tab in the focused frame (wraps around).
/// Does nothing if only one tab is present.
unsafe fn focus_prev_tab() {
    let fid = match selected_frame_id() {
        Some(f) => f,
        None => return,
    };
    let frame = match FRAMES.iter().find_map(|f| {
        if let Some(fr) = f { if fr.frame_id == fid { Some(fr) } else { None } } else { None }
    }) {
        Some(fr) => fr,
        None => return,
    };
    if frame.tab_count <= 1 { return; }
    let prev_tab = if frame.active_tab == 0 { frame.tab_count - 1 } else { frame.active_tab - 1 };
    drop(frame);
    if switch_to_tab(fid, prev_tab as u32) {
        static mut FOCUS_PREV_TAB_BUDGET: u32 = 4;
        let b = &mut FOCUS_PREV_TAB_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.shortcut.focus_prev_tab] frame={} tab={}", fid, prev_tab); }
    }
}

/// Toggle minimize/restore for the frame containing the focused surface.
/// If minimized → restore. If not minimized → minimize.
/// Returns true if state changed.
unsafe fn toggle_minimize_focused_frame() -> bool {
    let fid = match selected_frame_id() {
        Some(f) => f,
        None => return false,
    };
    if frame_is_minimized(fid) {
        restore_minimized_frame(fid)
    } else {
        minimize_frame(fid)
    }
}

/// Toggle zoom/unzoom for the frame containing the focused surface.
/// Returns true if state changed.
unsafe fn toggle_zoom_focused_frame() -> bool {
    let fid = match selected_frame_id() {
        Some(f) => f,
        None => return false,
    };
    toggle_zoom_frame(fid)
}

/// Close the focused frame's active tab (or entire frame if last tab).
/// Falls back to close_surface_from_frame_light on the active surface.
/// Safe: only destroys closeable surfaces, clears focus, tombstones.
unsafe fn close_focused_tab_or_frame_safe() -> bool {
    let fid = match selected_frame_id() {
        Some(f) => f,
        None => return false,
    };
    let sid = match active_surface_for_frame(fid) {
        Some(s) => s,
        None => return false,
    };
    if is_closeable_surface(sid) {
        close_surface_from_frame_light(sid)
    } else {
        false
    }
}

// ── App Surface Request Handler (APP_SURFACE_LAUNCH_CONTRACT_V1) ────────────
// Contract: a userland app-like PD requests one surface via IPC to silk-shell.
// Silk-shell validates, creates ShellFrame+ShellTab, registers lifecycle,
// and upserts on sexdisplay via 0xEC. App never writes framebuffer.
// Focus ownership remains shell-only.

/// Handle an app surface creation request from a userland PD (or synthetic proof).
/// Returns true if accepted and surface was created, false if rejected.
/// Rejection reasons: zero surface_id, zero title_id, already registered,
/// reserved surface ID range (< 200), or no free frame slot.
unsafe fn handle_app_surface_req(surface_id: u64, title_id: u64, caller_pd: u32) -> bool {
    // Validate: non-zero surface_id
    if surface_id == 0 {
        serial_println!("[shell.app_surface.reject] reason=zero_surface_id caller={}", caller_pd);
        return false;
    }
    // Validate: non-zero title_id
    if title_id == 0 {
        serial_println!("[shell.app_surface.reject] reason=zero_title_id sid={} caller={}", surface_id, caller_pd);
        return false;
    }
    // Validate: not already registered in lifecycle
    if lifecycle_state(surface_id).is_some() {
        serial_println!("[shell.app_surface.reject] reason=already_registered sid={} caller={}", surface_id, caller_pd);
        return false;
    }
    // Validate: surface_id in user range (>= 200 avoids OS surface collision)
    if surface_id < 200 {
        serial_println!("[shell.app_surface.reject] reason=reserved_range sid={} caller={}", surface_id, caller_pd);
        return false;
    }

    // Find free frame slot
    let mut frame_id: u32 = 0;
    let mut slot_idx: usize = 0;
    for (idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            frame_id = (idx + 10) as u32; // dynamic frame IDs start at 10 to avoid collision with boot frames
            slot_idx = idx;
            break;
        }
    }
    if frame_id == 0 {
        serial_println!("[shell.app_surface.reject] reason=no_frame_slot sid={} caller={}", surface_id, caller_pd);
        return false;
    }

    // Create the frame with one tab
    FRAMES[slot_idx] = Some(ShellFrame {
        frame_id,
        active_tab: 0,
        tab_count: 1,
        tabs: {
            let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] = [None; MAX_TABS_PER_FRAME as usize];
            t[0] = Some(ShellTab {
                surface_id,
                title_id,
                flags: 0,
            });
            t
        },
        scene_id: ACTIVE_SCENE_IDX,
        flags: FRAME_FLAG_TOP_BAR, // top bar ON for app surfaces
        normal_x: 200,
        normal_y: 100,
        normal_w: 600,
        normal_h: 400,
    });

    // Register lifecycle as Visible
    lifecycle_register(surface_id, LifecycleState::Visible);

    // Upsert on sexdisplay via 0xEC (geometry packed: arg1=(y<<32)|x, arg2=(h<<32)|w)
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (100u64) << 32 | 200u64,
        (400u64) << 32 | 600u64);

    // Re-tile and focus the new surface
    tile_active_scene_frames();
    try_set_focus(surface_id);

    serial_println!("[shell.app_surface.accept] sid={} title_id={} frame={} caller={}",
        surface_id, title_id, frame_id, caller_pd);
    true
}

// ── Linen Surface Control Helpers (LINEN_SURFACE_CONTROL_V1) ──────────────
// Make Linen a first-class shell-managed surface under Scene/Frame/Tab/Tiling.
// Linen frame is created lazily (not at boot) to preserve boot visual.

/// Frame ID reserved for Linen's ShellFrame.
const LINEN_FRAME_ID: u32 = 2;
/// Boot geometry for Linen when first opened (matches Linen's hardcoded 0xEC args).
const LINEN_BOOT_X: i32 = 900;
const LINEN_BOOT_Y: i32 = 500;
const LINEN_BOOT_W: u32 = 300;
const LINEN_BOOT_H: u32 = 150;

/// Ensure a ShellFrame exists for Linen in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
/// Does NOT change visibility or tiling — caller decides that.
unsafe fn ensure_linen_frame() -> Option<u32> {
    // Check if Linen frame already exists.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == LINEN_FRAME_ID {
                return Some(LINEN_FRAME_ID);
            }
        }
    }
    // Find an empty slot.
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: LINEN_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_LINEN,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR, // top bar ON by default
                normal_x: LINEN_BOOT_X,
                normal_y: LINEN_BOOT_Y,
                normal_w: LINEN_BOOT_W,
                normal_h: LINEN_BOOT_H,
            });
            serial_println!("[linen.placeholder.attach.frame] frame={} scene={} slot={}", LINEN_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[linen.placeholder.attach.tab] frame={} tab=0 surface={}", LINEN_FRAME_ID, SURFACE_ID_LINEN);
            static mut LINEN_CREATE_BUDGET: u32 = 4;
            let b = &mut LINEN_CREATE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.linen.frame.create] frame={} slot={}", LINEN_FRAME_ID, slot_idx); }
            return Some(LINEN_FRAME_ID);
        }
    }
    // No empty slot — log and fail.
    static mut LINEN_NOSLOT_BUDGET: u32 = 4;
    let b = &mut LINEN_NOSLOT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.linen.frame.reject] reason=no_slot"); }
    None
}

/// Open Linen in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Linen is already visible in the active scene, focuses it.
/// Returns true if Linen became visible/focused.
unsafe fn open_linen_in_active_scene() -> bool {
    // D1: duplicate guard — if Linen already visible in active scene, reject open.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == LINEN_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[linen.placeholder.reject.duplicate] frame={} scene={}", LINEN_FRAME_ID, ACTIVE_SCENE_IDX);
                // Focus existing Linen instead.
                if let Some(sid) = active_surface_for_frame(LINEN_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[linen.placeholder.focus] frame={} sid={}", LINEN_FRAME_ID, sid);
                    }
                }
                linen_render_object_list();
                return true;
            }
        }
    }

    let fid = match ensure_linen_frame() {
        Some(f) => f,
        None => return false,
    };

    // Update frame scene to current active scene.
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        // Restore (un-minimize) to make visible.
        if !restore_minimized_frame(fid) {
            return false;
        }
    } else if frame_is_zoomed(fid) {
        // Already visible and zoomed — just ensure focus.
    } else {
        // Already visible in tiling — ensure focus and re-tile.
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        // Ensure surface is shown on display (0xEC upsert).
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (LINEN_BOOT_Y as u64) << 32 | LINEN_BOOT_X as u64,
                (LINEN_BOOT_H as u64) << 32 | LINEN_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    // Focus Linen's surface.
    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[linen.placeholder.focus] frame={} sid={}", fid, sid);
    }

    serial_println!("[linen.placeholder.open] frame={}", fid);
    serial_println!("[linen.object_table.ready] count={}", linen_object_count());
    linen_render_object_list();
    snap_capture_layout();
    static mut LINEN_OPEN_BUDGET: u32 = 4;
    let b = &mut LINEN_OPEN_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.linen.open] frame={}", fid); }
    true
}

/// Focus Linen if it is already open (frame exists and not minimized in active
/// scene). If Linen is not open, call open_linen_in_active_scene().
/// Returns true if focus was set.
unsafe fn focus_or_open_linen() -> bool {
    // Check if Linen frame exists and is visible in active scene.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == LINEN_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(LINEN_FRAME_ID) {
                    if try_set_focus(sid) {
                        static mut LINEN_FOCUS_BUDGET: u32 = 4;
                        let b = &mut LINEN_FOCUS_BUDGET;
                        if *b > 0 { *b -= 1; serial_println!("[shell.linen.focus] frame={}", LINEN_FRAME_ID); }
                        return true;
                    }
                }
            }
        }
    }
    // Not visible — open it.
    open_linen_in_active_scene()
}

/// Toggle Linen visibility in the active scene. If Linen frame exists and is
/// not minimized, minimize it. Otherwise open/un-minimize it.
/// Returns true if state changed.
unsafe fn toggle_linen() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == LINEN_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                // Linen visible — minimize it.
                if minimize_frame(LINEN_FRAME_ID) {
                    static mut LINEN_TOGGLE_BUDGET: u32 = 4;
                    let b = &mut LINEN_TOGGLE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.linen.toggle.minimize] frame={}", LINEN_FRAME_ID); }
                    return true;
                }
                return false;
            }
        }
    }
    // Linen not visible — open it.
    open_linen_in_active_scene()
}

/// Return Linen's frame_id, if its frame exists.
unsafe fn linen_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == LINEN_FRAME_ID {
                return Some(LINEN_FRAME_ID);
            }
        }
    }
    None
}

// ── Quil Surface Control Helpers ──────────────────────────────────────────
// Quil is a first-class shell-managed app surface matching Linen control pattern.
// Frame is created lazily (not at boot) to preserve boot visual.

/// Frame ID reserved for Quil's ShellFrame.
const QUIL_FRAME_ID: u32 = 3;
/// Boot geometry for Quil when first opened.
const QUIL_BOOT_X: i32 = 100;
const QUIL_BOOT_Y: i32 = 100;
const QUIL_BOOT_W: u32 = 640;
const QUIL_BOOT_H: u32 = 480;

/// Fill color for the Quil visual placeholder surface (dark slate blue-gray).
/// Used via 0xEF opcode to sexdisplay when no real Quil server exists.
const QUIL_PLACEHOLDER_COLOR: u32 = 0x0018202E;

/// Ensure a ShellFrame exists for Quil in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
/// Does NOT change visibility or tiling — caller decides that.
unsafe fn ensure_quil_frame() -> Option<u32> {
    // Check if Quil frame already exists.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == QUIL_FRAME_ID {
                return Some(QUIL_FRAME_ID);
            }
        }
    }
    // Find an empty slot.
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: QUIL_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_QUIL,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR, // top bar ON by default
                normal_x: QUIL_BOOT_X,
                normal_y: QUIL_BOOT_Y,
                normal_w: QUIL_BOOT_W,
                normal_h: QUIL_BOOT_H,
            });
            serial_println!("[quil.placeholder.attach.frame] frame={} scene={} slot={}", QUIL_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[quil.placeholder.attach.tab] frame={} tab=0 surface={}", QUIL_FRAME_ID, SURFACE_ID_QUIL);
            static mut QUIL_CREATE_BUDGET: u32 = 4;
            let b = &mut QUIL_CREATE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.quil.frame.create] frame={} slot={}", QUIL_FRAME_ID, slot_idx); }
            return Some(QUIL_FRAME_ID);
        }
    }
    // No empty slot — log and fail.
    static mut QUIL_NOSLOT_BUDGET: u32 = 4;
    let b = &mut QUIL_NOSLOT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.quil.frame.reject] reason=no_slot"); }
    None
}

/// Open Quil in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Quil is already visible in the active scene, focuses it.
/// Returns true if Quil became visible/focused.
unsafe fn open_quil_in_active_scene() -> bool {
    // E1: duplicate guard — if Quil already visible in active scene, reject open.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == QUIL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[quil.placeholder.reject.duplicate] frame={} scene={}", QUIL_FRAME_ID, ACTIVE_SCENE_IDX);
                // Focus existing Quil instead.
                if let Some(sid) = active_surface_for_frame(QUIL_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[quil.placeholder.focus] frame={} sid={}", QUIL_FRAME_ID, sid);
                    }
                }
                return true;
            }
        }
    }

    let fid = match ensure_quil_frame() {
        Some(f) => f,
        None => return false,
    };

    // Update frame scene to current active scene.
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        if !restore_minimized_frame(fid) {
            return false;
        }
        static mut QUIL_RESTORE_BUDGET: u32 = 8;
        let b = &mut QUIL_RESTORE_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.quil.lifecycle.restore] frame={}", fid); }
    } else if frame_is_zoomed(fid) {
        // Already visible and zoomed — ensure focus.
    } else {
        // Already visible in tiling — ensure focus and re-tile.
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (QUIL_BOOT_Y as u64) << 32 | QUIL_BOOT_X as u64,
                (QUIL_BOOT_H as u64) << 32 | QUIL_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[quil.placeholder.focus] frame={} sid={}", fid, sid);
    }

    // Ensure Quil placeholder fill rect is set on every open (covers the
    // restore-from-minimized path where tile_visible_frames() is not called).
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0,
        (QUIL_PLACEHOLDER_COLOR as u64) << 32 | ((SURFACE_201_H as u64) << 16) | SURFACE_201_W as u64);

    // V1D: Route proof — ping Quil PD to confirm shell→Quil PDX path.
    pdx_call(SLOT_QUIL, OP_QUIL_PING, 0, 0, 0);
    static mut QUIL_ROUTE_BUDGET: u32 = 8;
    let b = &mut QUIL_ROUTE_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.quil.route.ping] fid={}", fid); }

    serial_println!("[quil.placeholder.open] frame={}", fid);
    serial_println!("[quil.buffer_table.ready] count={}", quil_buffer_count());
    // K3: Render buffer list header + proof-marker rows on every open.
    quil_render_buffer_list();
    snap_capture_layout();
    static mut QUIL_OPEN_BUDGET: u32 = 4;
    let b = &mut QUIL_OPEN_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.quil.open] frame={}", fid); }
    true
}

/// Focus Quil if it is already open (frame exists and not minimized in active
/// scene). If Quil is not open, call open_quil_in_active_scene().
/// Returns true if focus was set.
unsafe fn focus_or_open_quil() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == QUIL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(QUIL_FRAME_ID) {
                    if try_set_focus(sid) {
                        static mut QUIL_FOCUS_BUDGET: u32 = 4;
                        let b = &mut QUIL_FOCUS_BUDGET;
                        if *b > 0 { *b -= 1; serial_println!("[shell.quil.focus] frame={}", QUIL_FRAME_ID); }
                        return true;
                    }
                }
            }
        }
    }
    open_quil_in_active_scene()
}

/// Toggle Quil visibility in the active scene. If Quil frame exists and is
/// not minimized, minimize it. Otherwise open/un-minimize it.
/// Returns true if state changed.
unsafe fn toggle_quil() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == QUIL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if minimize_frame(QUIL_FRAME_ID) {
                    static mut QUIL_TOGGLE_BUDGET: u32 = 4;
                    let b = &mut QUIL_TOGGLE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.quil.lifecycle.minimize] frame={}", QUIL_FRAME_ID); }
                    return true;
                }
                return false;
            }
        }
    }
    open_quil_in_active_scene()
}

/// Return Quil's frame_id, if its frame exists.
unsafe fn quil_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == QUIL_FRAME_ID {
                return Some(QUIL_FRAME_ID);
            }
        }
    }
    None
}

// ── Mesh Surface Control Helpers ──────────────────────────────────────────
// Mesh = diagnostic capability graph placeholder.
// Mirrors Linen/Quil placeholder pattern. No live graph yet.

/// Frame ID reserved for Mesh's ShellFrame.
const MESH_FRAME_ID: u32 = 4;
/// Boot geometry for Mesh when first opened.
const MESH_BOOT_X: i32 = 200;
const MESH_BOOT_Y: i32 = 100;
const MESH_BOOT_W: u32 = 640;
const MESH_BOOT_H: u32 = 480;

/// Fill color for the Mesh visual placeholder surface (amber/diagnostic).
const MESH_PLACEHOLDER_COLOR: u32 = 0x00383010;
/// Header bar height for the Mesh fact list, in pixels.
const MESH_LIST_HEADER_H: u32 = 28;
/// Height of each fact row in the Mesh list, in pixels.
const MESH_LIST_ROW_H: u32 = 26;
/// Gap between row fill rects in the Mesh list, in pixels.
const MESH_LIST_ROW_GAP: u32 = 2;
/// Max rows with visual fill rects. Header takes rect_index=0; rows get 1-7.
const MESH_LIST_ROW_RECTS: u8 = 7;

/// Currently selected visible row index in the Mesh fact list.
/// 0 = newest fact row. Repaired during render if ring shrinks.
static mut MESH_SELECTED_ROW: u8 = 0;

/// Ensure a ShellFrame exists for Mesh in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
unsafe fn ensure_mesh_frame() -> Option<u32> {
    // Check if Mesh frame already exists.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID {
                return Some(MESH_FRAME_ID);
            }
        }
    }
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: MESH_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_MESH,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR, // top bar ON by default
                normal_x: MESH_BOOT_X,
                normal_y: MESH_BOOT_Y,
                normal_w: MESH_BOOT_W,
                normal_h: MESH_BOOT_H,
            });
            serial_println!("[mesh.placeholder.attach.frame] frame={} scene={} slot={}", MESH_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[mesh.placeholder.attach.tab] frame={} tab=0 surface={}", MESH_FRAME_ID, SURFACE_ID_MESH);
            static mut MESH_CREATE_BUDGET: u32 = 4;
            let b = &mut MESH_CREATE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.mesh.frame.create] frame={} slot={}", MESH_FRAME_ID, slot_idx); }
            return Some(MESH_FRAME_ID);
        }
    }
    // No empty slot — log and fail.
    static mut MESH_NOSLOT_BUDGET: u32 = 4;
    let b = &mut MESH_NOSLOT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.mesh.frame.reject] reason=no_slot"); }
    None
}

/// Open Mesh in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Mesh is already visible in the active scene, focuses it.
/// Returns true if Mesh became visible/focused.
unsafe fn open_mesh_in_active_scene() -> bool {
    // I1: duplicate guard — if Mesh already visible in active scene, reject open.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[mesh.placeholder.reject.duplicate] frame={} scene={}", MESH_FRAME_ID, ACTIVE_SCENE_IDX);
                // Focus existing Mesh instead.
                if let Some(sid) = active_surface_for_frame(MESH_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[mesh.placeholder.focus] frame={} sid={}", MESH_FRAME_ID, sid);
                    }
                }
                return true;
            }
        }
    }

    let fid = match ensure_mesh_frame() {
        Some(f) => f,
        None => return false,
    };

    // Update frame scene to current active scene.
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        if !restore_minimized_frame(fid) {
            return false;
        }
        static mut MESH_RESTORE_BUDGET: u32 = 8;
        let b = &mut MESH_RESTORE_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.mesh.lifecycle.restore] frame={}", fid); }
    } else if frame_is_zoomed(fid) {
        // Already visible and zoomed — ensure focus.
    } else {
        // Already visible in tiling — ensure focus and re-tile.
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (MESH_BOOT_Y as u64) << 32 | MESH_BOOT_X as u64,
                (MESH_BOOT_H as u64) << 32 | MESH_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[mesh.placeholder.focus] frame={} sid={}", fid, sid);
    }

    // N4: Render Mesh fact list (replaces old single-fill placeholder).
    mesh_render_fact_list();

    serial_println!("[mesh.placeholder.open] frame={}", fid);
    // J6: Emit diagnostic link facts every time Mesh opens.
    mesh_emit_linen_quil_links();
    snap_capture_layout();
    static mut MESH_OPEN_BUDGET: u32 = 4;
    let b = &mut MESH_OPEN_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.mesh.open] frame={}", fid); }
    true
}

/// Focus Mesh if it is already open (frame exists and not minimized in active
/// scene). If Mesh is not open, call open_mesh_in_active_scene().
unsafe fn focus_or_open_mesh() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(MESH_FRAME_ID) {
                    if try_set_focus(sid) {
                        static mut MESH_FOCUS_BUDGET: u32 = 4;
                        let b = &mut MESH_FOCUS_BUDGET;
                        if *b > 0 { *b -= 1; serial_println!("[shell.mesh.focus] frame={}", MESH_FRAME_ID); }
                        return true;
                    }
                }
            }
        }
    }
    open_mesh_in_active_scene()
}

/// Toggle Mesh visibility in the active scene. If Mesh frame exists and is
/// not minimized, minimize it. Otherwise open/un-minimize it.
unsafe fn toggle_mesh() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if minimize_frame(MESH_FRAME_ID) {
                    static mut MESH_TOGGLE_BUDGET: u32 = 4;
                    let b = &mut MESH_TOGGLE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.mesh.lifecycle.minimize] frame={}", MESH_FRAME_ID); }
                    return true;
                }
                return false;
            }
        }
    }
    open_mesh_in_active_scene()
}

// ── Collar Surface Control Helpers ─────────────────────────────────────────
// Collar = authority wallet placeholder.
// Mirrors Linen/Quil/Mesh placeholder pattern. No real grants yet.

/// Frame ID reserved for Collar's ShellFrame.
const COLLAR_FRAME_ID: u32 = 5;
/// Boot geometry for Collar when first opened.
const COLLAR_BOOT_X: i32 = 300;
const COLLAR_BOOT_Y: i32 = 100;
const COLLAR_BOOT_W: u32 = 640;
const COLLAR_BOOT_H: u32 = 480;

/// Fill color for the Collar visual placeholder surface (muted teal/authority).
const COLLAR_PLACEHOLDER_COLOR: u32 = 0x00204038;

/// Ensure a ShellFrame exists for Collar in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
unsafe fn ensure_collar_frame() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COLLAR_FRAME_ID {
                return Some(COLLAR_FRAME_ID);
            }
        }
    }
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: COLLAR_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_COLLAR,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR,
                normal_x: COLLAR_BOOT_X,
                normal_y: COLLAR_BOOT_Y,
                normal_w: COLLAR_BOOT_W,
                normal_h: COLLAR_BOOT_H,
            });
            serial_println!("[collar.placeholder.attach.frame] frame={} scene={} slot={}", COLLAR_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[collar.placeholder.attach.tab] frame={} tab=0 surface={}", COLLAR_FRAME_ID, SURFACE_ID_COLLAR);
            static mut COLLAR_CREATE_BUDGET: u32 = 4;
            let b = &mut COLLAR_CREATE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.collar.frame.create] frame={} slot={}", COLLAR_FRAME_ID, slot_idx); }
            return Some(COLLAR_FRAME_ID);
        }
    }
    static mut COLLAR_NOSLOT_BUDGET: u32 = 4;
    let b = &mut COLLAR_NOSLOT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.collar.frame.reject] reason=no_slot"); }
    None
}

/// Open Collar in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Collar is already visible in the active scene, focuses it.
unsafe fn open_collar_in_active_scene() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COLLAR_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[collar.placeholder.reject.duplicate] frame={} scene={}", COLLAR_FRAME_ID, ACTIVE_SCENE_IDX);
                if let Some(sid) = active_surface_for_frame(COLLAR_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[collar.placeholder.focus] frame={} sid={}", COLLAR_FRAME_ID, sid);
                    }
                }
                return true;
            }
        }
    }

    let fid = match ensure_collar_frame() {
        Some(f) => f,
        None => return false,
    };

    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        if !restore_minimized_frame(fid) {
            return false;
        }
        static mut COLLAR_RESTORE_BUDGET: u32 = 8;
        let b = &mut COLLAR_RESTORE_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.collar.lifecycle.restore] frame={}", fid); }
    } else if frame_is_zoomed(fid) {
    } else {
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (COLLAR_BOOT_Y as u64) << 32 | COLLAR_BOOT_X as u64,
                (COLLAR_BOOT_H as u64) << 32 | COLLAR_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[collar.placeholder.focus] frame={} sid={}", fid, sid);
    }

    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_COLLAR, 0,
        (COLLAR_PLACEHOLDER_COLOR as u64) << 32 | ((SURFACE_203_H as u64) << 16) | SURFACE_203_W as u64);

    serial_println!("[collar.placeholder.open] frame={}", fid);
    snap_capture_layout();
    static mut COLLAR_OPEN_BUDGET: u32 = 4;
    let b = &mut COLLAR_OPEN_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.collar.open] frame={}", fid); }
    true
}

/// Focus Collar if it is already open. If not, open it.
unsafe fn focus_or_open_collar() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COLLAR_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(COLLAR_FRAME_ID) {
                    if try_set_focus(sid) {
                        static mut COLLAR_FOCUS_BUDGET: u32 = 4;
                        let b = &mut COLLAR_FOCUS_BUDGET;
                        if *b > 0 { *b -= 1; serial_println!("[shell.collar.focus] frame={}", COLLAR_FRAME_ID); }
                        return true;
                    }
                }
            }
        }
    }
    open_collar_in_active_scene()
}

/// Toggle Collar visibility. Minimize if visible, open if not.
unsafe fn toggle_collar() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COLLAR_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if minimize_frame(COLLAR_FRAME_ID) {
                    static mut COLLAR_TOGGLE_BUDGET: u32 = 4;
                    let b = &mut COLLAR_TOGGLE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.collar.lifecycle.minimize] frame={}", COLLAR_FRAME_ID); }
                    return true;
                }
                return false;
            }
        }
    }
    open_collar_in_active_scene()
}

/// Return Collar's frame_id, if its frame exists.
unsafe fn collar_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COLLAR_FRAME_ID {
                return Some(COLLAR_FRAME_ID);
            }
        }
    }
    None
}

/// Return Mesh's frame_id, if its frame exists.
unsafe fn mesh_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == MESH_FRAME_ID {
                return Some(MESH_FRAME_ID);
            }
        }
    }
    None
}

// ── Bell Placeholder Surface Control Helpers ──────────────────────────────
// Bell = attention firewall placeholder.
// Mirrors Linen/Quil/Mesh/Collar placeholder pattern. No real notifications yet.

/// Frame ID reserved for Bell's ShellFrame.
const BELL_FRAME_ID: u32 = 6;
/// Boot geometry for Bell when first opened.
const BELL_BOOT_X: i32 = 400;
const BELL_BOOT_Y: i32 = 100;
const BELL_BOOT_W: u32 = 640;
const BELL_BOOT_H: u32 = 480;

/// Fill color for the Bell visual placeholder surface (attention red-orange).
const BELL_PLACEHOLDER_COLOR: u32 = 0x00402020;
/// Header bar height for the Bell event list, in pixels.
const BELL_LIST_HEADER_H: u32 = 28;
/// Height of each event row in the Bell list, in pixels.
const BELL_LIST_ROW_H: u32 = 26;
/// Gap between row fill rects in the Bell list, in pixels.
const BELL_LIST_ROW_GAP: u32 = 2;
/// Max rows with visual fill rects. Header takes rect_index=0; rows get 1-7.
const BELL_LIST_ROW_RECTS: u8 = 7;

/// Currently selected visible row index in the Bell event list.
/// 0 = newest event row. Repaired during render if ring shrinks.
static mut BELL_SELECTED_ROW: u8 = 0;

// ── K11: Command Palette Stub ────────────────────────────────────────────
// Shell-owned action router. No text input, no fuzzy search, no app manifests.

/// Frame ID reserved for the Command Palette's ShellFrame.
const COMMAND_PALETTE_FRAME_ID: u32 = 7;
/// Boot geometry for Command Palette when first opened.
const COMMAND_PALETTE_BOOT_X: i32 = 400;
const COMMAND_PALETTE_BOOT_Y: i32 = 200;
const COMMAND_PALETTE_BOOT_W: u32 = 480;
const COMMAND_PALETTE_BOOT_H: u32 = 240;

/// Header bar height for command palette, in pixels.
const PALETTE_LIST_HEADER_H: u32 = 28;
/// Height of each command row in the palette, in pixels.
const PALETTE_LIST_ROW_H: u32 = 24;
/// Vertical gap between row fill rects, in pixels.
const PALETTE_LIST_ROW_GAP: u32 = 2;
/// Width of the left accent bar per command row, in pixels.
const PALETTE_ACCENT_BAR_W: u32 = 5;
/// Background color for the command palette list area (behind all rows).
const PALETTE_LIST_BG_COLOR: u32 = 0x00101820; // dark slate

// ── Spindle Terminal Constants ───────────────────────────────────────────────
// Spindle = native SexOS terminal/command console. Shell-local for 0.2.
// Toggle via Scroll Lock (scancode 0x46). No POSIX/PTY/TTY.
const SPINDLE_FRAME_ID: u32 = 9;
const SPINDLE_BOOT_X: i32 = 200;
const SPINDLE_BOOT_Y: i32 = 200;
const SPINDLE_BOOT_W: u32 = 500;
const SPINDLE_BOOT_H: u32 = 300;

/// Fill color for the Spindle surface placeholder (dark terminal teal).
const SPINDLE_PLACEHOLDER_COLOR: u32 = 0x00182028;
/// Header bar height for the Spindle terminal status line, in pixels.
const SPINDLE_HEADER_H: u32 = 28;
/// Height of each output line/row in the Spindle display, in pixels.
const SPINDLE_ROW_H: u32 = 26;
/// Gap between row fill rects in the Spindle display, in pixels.
const SPINDLE_ROW_GAP: u32 = 2;
/// Max rows with visual fill rects. Header takes rect_index=0; rows get 1-7.
const SPINDLE_ROW_RECTS: u8 = 7;
/// Maximum characters in a single Yarn command line buffer.
const YARN_CMD_BUF_CAP: usize = 256;
/// Maximum output lines tracked in the Yarn session ring.
const YARN_OUTPUT_LINES: usize = 20;
/// Maximum characters per output line (trimmed to fit).
const YARN_OUTPUT_LINE_CAP: usize = 32;
/// Maximum command history entries.
const YARN_HISTORY_CAP: usize = 16;

/// YarnSession — bounded input/output/state for the Spindle terminal.
/// No heap, no strings, no POSIX/PTY/TTY.
struct YarnSession {
    cmd_buf: [u8; YARN_CMD_BUF_CAP],
    cmd_len: usize,
    output_lines: [[u8; YARN_OUTPUT_LINE_CAP]; YARN_OUTPUT_LINES],
    output_count: usize,
    history: [[u8; YARN_CMD_BUF_CAP]; YARN_HISTORY_CAP],
    history_count: usize,
    history_pos: i64,
}

static mut YARN: YarnSession = YarnSession {
    cmd_buf: [0u8; YARN_CMD_BUF_CAP],
    cmd_len: 0,
    output_lines: [[0u8; YARN_OUTPUT_LINE_CAP]; YARN_OUTPUT_LINES],
    output_count: 0,
    history: [[0u8; YARN_CMD_BUF_CAP]; YARN_HISTORY_CAP],
    history_count: 0,
    history_pos: -1,
};

/// Shell commands exposed via the command palette.
/// Each command routes to an existing SurfaceAction via the normal dispatch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Command {
    OpenSelectedInQuil = 0,
    FocusLinen = 1,
    FocusQuil = 2,
    SceneNext = 3,
    OpenAtlas = 4,
}

/// Static display info for each command in the palette.
struct CommandDef {
    command: Command,
    name: &'static str,
}

/// The five commands available in the command palette.
const COMMAND_LIST: [CommandDef; 5] = [
    CommandDef { command: Command::OpenSelectedInQuil, name: "Open in Quil" },
    CommandDef { command: Command::FocusLinen, name: "Focus Linen" },
    CommandDef { command: Command::FocusQuil, name: "Focus Quil" },
    CommandDef { command: Command::SceneNext, name: "Next Scene" },
    CommandDef { command: Command::OpenAtlas, name: "Open Atlas" },
];

/// Whether the command palette overlay is currently open.
static mut COMMAND_PALETTE_OPEN: bool = false;
/// Index into COMMAND_LIST of the currently selected command.
static mut COMMAND_PALETTE_SELECTED: u8 = 0;

// ── Spindle Surface Control Helpers ─────────────────────────────────────────
// Spindle = native SexOS terminal/command console (0.2: shell-local).
// Toggle via Scroll Lock (scancode 0x46). No sexdisplay protocol changes.

/// Ensure a ShellFrame exists for Spindle in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
unsafe fn ensure_spindle_frame() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == SPINDLE_FRAME_ID {
                return Some(SPINDLE_FRAME_ID);
            }
        }
    }
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: SPINDLE_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_SPINDLE,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR, // top bar ON by default
                normal_x: SPINDLE_BOOT_X,
                normal_y: SPINDLE_BOOT_Y,
                normal_w: SPINDLE_BOOT_W,
                normal_h: SPINDLE_BOOT_H,
            });
            serial_println!("[spindle.placeholder.attach.frame] frame={} scene={} slot={}", SPINDLE_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[spindle.placeholder.attach.tab] frame={} tab=0 surface={}", SPINDLE_FRAME_ID, SURFACE_ID_SPINDLE);
            return Some(SPINDLE_FRAME_ID);
        }
    }
    serial_println!("[spindle.frame.reject] reason=no_slot");
    None
}

/// Open Spindle in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Spindle is already visible in the active scene, focuses it.
/// Returns true if Spindle became visible/focused.
unsafe fn open_spindle_in_active_scene() -> bool {
    // Duplicate guard — if Spindle already visible in active scene, reject open.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == SPINDLE_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[spindle.placeholder.reject.duplicate] frame={} scene={}", SPINDLE_FRAME_ID, ACTIVE_SCENE_IDX);
                if let Some(sid) = active_surface_for_frame(SPINDLE_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[spindle.placeholder.focus] frame={} sid={}", SPINDLE_FRAME_ID, sid);
                    }
                }
                return true;
            }
        }
    }

    let fid = match ensure_spindle_frame() {
        Some(f) => f,
        None => return false,
    };

    // Update frame scene to current active scene.
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        if !restore_minimized_frame(fid) {
            return false;
        }
    } else if frame_is_zoomed(fid) {
        // Already visible and zoomed — ensure focus.
    } else {
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (SPINDLE_BOOT_Y as u64) << 32 | SPINDLE_BOOT_X as u64,
                (SPINDLE_BOOT_H as u64) << 32 | SPINDLE_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[spindle.placeholder.focus] frame={} sid={}", fid, sid);
    }

    // Render Spindle output bands.
    spindle_render();

    serial_println!("[spindle.placeholder.open] frame={}", fid);
    true
}

/// Toggle Spindle visibility in the active scene. If frame exists and is
/// not minimized, minimize it. Otherwise open/un-minimize it.
unsafe fn toggle_spindle() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == SPINDLE_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if minimize_frame(SPINDLE_FRAME_ID) {
                    return true;
                }
                return false;
            }
        }
    }
    open_spindle_in_active_scene()
}

/// Append an output line to the Yarn output ring buffer.
unsafe fn yarn_append_output(text: &[u8]) {
    let yarn = &mut YARN;
    if yarn.output_count < YARN_OUTPUT_LINES {
        yarn.output_count += 1;
    }
    // Shift lines up (discard oldest).
    for i in 1..YARN_OUTPUT_LINES {
        yarn.output_lines[i - 1] = yarn.output_lines[i];
    }
    // Write new line (trim to fit).
    let line = &mut yarn.output_lines[YARN_OUTPUT_LINES - 1];
    for (i, &b) in text.iter().enumerate() {
        if i >= YARN_OUTPUT_LINE_CAP - 1 { break; }
        line[i] = b;
    }
    serial_println!("[spindle.output.append] len={}", text.len().min(YARN_OUTPUT_LINE_CAP - 1));
}

/// Yarn built-in: help — list available commands.
unsafe fn yarn_cmd_help() {
    serial_println!("[spindle.command.help]");
    yarn_append_output(b"Commands: help clear echo about time pd scene routes faults");
}

/// Yarn built-in: clear — reset output ring.
unsafe fn yarn_cmd_clear() {
    serial_println!("[spindle.command.clear]");
    for i in 0..YARN_OUTPUT_LINES {
        YARN.output_lines[i] = [0u8; YARN_OUTPUT_LINE_CAP];
    }
    YARN.output_count = 0;
}

/// Yarn built-in: echo — echo arguments back.
unsafe fn yarn_cmd_echo(args: &[u8]) {
    let trimmed = trim_ascii(args);
    serial_println!("[spindle.command.echo] args={:?}", trimmed);
    if trimmed.is_empty() {
        yarn_append_output(b"");
    } else {
        yarn_append_output(trimmed);
    }
}

/// Yarn built-in: about — system info.
unsafe fn yarn_cmd_about() {
    serial_println!("[spindle.command.about]");
    yarn_append_output(b"SexOS 0.2 - Spindle terminal");
    yarn_append_output(b"No POSIX, no TTY, no PTY");
}

/// Yarn built-in: time — show system boot time or clock reading.
unsafe fn yarn_cmd_time() {
    serial_println!("[spindle.command.time]");
    // Read boot seconds from kernel (sys_tick or similar).
    // For V1, display a placeholder.
    yarn_append_output(b"System time: TBD (no RTC read in V1)");
}

/// Yarn built-in: pd — list active PDs.
unsafe fn yarn_cmd_pd() {
    serial_println!("[spindle.command.pd]");
    // PD list from known shell slots.
    // For V1, display known PD IDs.
    yarn_append_output(b"PD slots: 5=display 6=shell 7=silkbar 10=store 11=quil 12=bell");
}

/// Yarn built-in: scene — list scenes.
unsafe fn yarn_cmd_scene() {
    serial_println!("[spindle.command.scene]");
    // List active scene and frame counts.
    let active = ACTIVE_SCENE_IDX;
    let mut count = 0u8;
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.scene_id == active as u8 {
                count += 1;
            }
        }
    }
    // Simple output for V1.
    yarn_append_output(b"Active scene: 0");
}

/// Yarn built-in: routes — surface route map.
unsafe fn yarn_cmd_routes() {
    serial_println!("[spindle.command.routes]");
    yarn_append_output(b"200=linen 201=quil 202=mesh");
    yarn_append_output(b"203=collar 204=bell 0x98=palette 0x99=spindle");
}

/// Yarn built-in: faults — error counters.
unsafe fn yarn_cmd_faults() {
    serial_println!("[spindle.command.faults]");
    yarn_append_output(b"No fault counters tracked in V1");
}

/// Trim leading/trailing whitespace (space, tab, newline) from a byte slice.
fn trim_ascii(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ' && b != b'\t' && b != b'\n').unwrap_or(s.len());
    let end = s.iter().rposition(|&b| b != b' ' && b != b'\t' && b != b'\n').map(|i| i + 1).unwrap_or(start);
    &s[start..end]
}

/// Yarn session: dispatch a command from cmd_buf.
unsafe fn spindle_dispatch() {
    let yarn = &mut YARN;
    let cmd = yarn.cmd_buf;
    let len = yarn.cmd_len;
    if len == 0 {
        return;
    }
    // Save to history.
    if yarn.history_count < YARN_HISTORY_CAP {
        yarn.history[yarn.history_count] = cmd;
        yarn.history_count += 1;
    } else {
        // Shift history.
        for i in 1..YARN_HISTORY_CAP {
            yarn.history[i - 1] = yarn.history[i];
        }
        yarn.history[YARN_HISTORY_CAP - 1] = cmd;
    }
    yarn.history_pos = -1;

    // Echo command to output.
    let mut echo_buf = [0u8; YARN_OUTPUT_LINE_CAP];
    if len > 0 {
        let copy_len = len.min(YARN_OUTPUT_LINE_CAP - 1);
        echo_buf[..copy_len].copy_from_slice(&cmd[..copy_len]);
    }
    yarn_append_output(&echo_buf[..len.min(YARN_OUTPUT_LINE_CAP - 1)]);

    // Match first whitespace-delimited token.
    let args = trim_ascii(&cmd[..len]);
    let first_space = args.iter().position(|&b| b == b' ').unwrap_or(args.len());
    let token = &args[..first_space];
    let rest = if first_space < args.len() { &args[first_space + 1..] } else { &[] };

    match token {
        b"help" => yarn_cmd_help(),
        b"clear" => yarn_cmd_clear(),
        b"echo" => yarn_cmd_echo(rest),
        b"about" => yarn_cmd_about(),
        b"time" => yarn_cmd_time(),
        b"pd" => yarn_cmd_pd(),
        b"scene" => yarn_cmd_scene(),
        b"routes" => yarn_cmd_routes(),
        b"faults" => yarn_cmd_faults(),
        _ => {
            serial_println!("[spindle.command.unknown] cmd={:?}", token);
            yarn_append_output(b"Unknown command. Type 'help'.");
        }
    }

    // Clear command buffer after dispatch.
    yarn.cmd_buf = [0u8; YARN_CMD_BUF_CAP];
    yarn.cmd_len = 0;

    // Render output bands.
    spindle_render();
}

/// Render Spindle output using existing 0xEF fill rects (colored bands).
/// No text rendering, no sexdisplay protocol changes.
unsafe fn spindle_render() {
    let w = SURFACE_0x99_W;
    let h = SURFACE_0x99_H;
    if w == 0 || h == 0 { return; }

    serial_println!("[spindle.render.band] w={} h={}", w, h);

    // Draw header bar at top of surface (rect_index=0).
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_SPINDLE,
        (0u64 << 32) | 0u64,
        ((SPINDLE_PLACEHOLDER_COLOR as u64) << 32)
            | ((SPINDLE_HEADER_H as u64) << 16)
            | w as u64);

    // Draw output lines as colored fill-rect bands.
    let yarn = &YARN;
    let output_start = if yarn.output_count > SPINDLE_ROW_RECTS as usize {
        yarn.output_count - SPINDLE_ROW_RECTS as usize
    } else {
        0
    };
    let visible_count = yarn.output_count - output_start;
    for i in 0..visible_count.min(SPINDLE_ROW_RECTS as usize) {
        let line_idx = output_start + i;
        if line_idx >= YARN_OUTPUT_LINES { break; }
        let rect_index = (i as u64 + 1) & 0xF;
        let row_y = SPINDLE_HEADER_H + i as u32 * (SPINDLE_ROW_H + SPINDLE_ROW_GAP);
        // Simple semantic coloring: command lines get accent, others get dim.
        let color = if yarn.output_lines[line_idx][0] == b'>' {
            0x007AAFA4u32  // teal accent for echo/prompt lines
        } else if i == visible_count - 1 {
            0x00386050u32  // green tint for latest output
        } else {
            0x00202830u32  // dim default
        };
        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_SPINDLE,
            (row_y as u64) << 32 | 0u64,
            (rect_index << 56)
                | ((color as u64) << 32)
                | ((SPINDLE_ROW_H as u64) << 16)
                | w as u64);
        serial_println!("[spindle.render.band] index={} color={:#010x}", rect_index, color);
    }
    serial_println!("[spindle.render.done] lines={}", visible_count);
}

// ── Bell Surface Control Helpers ─────────────────────────────────────────────
// Bell = attention/event placeholder surface. No real Bell PD yet.

/// Ensure a ShellFrame exists for Bell in an empty FRAMES slot, assigned to
/// the active scene. Returns the frame_id if created/found, or 0 if no slot.
unsafe fn ensure_bell_frame() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID {
                return Some(BELL_FRAME_ID);
            }
        }
    }
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: BELL_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_BELL_PLACEHOLDER,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR,
                normal_x: BELL_BOOT_X,
                normal_y: BELL_BOOT_Y,
                normal_w: BELL_BOOT_W,
                normal_h: BELL_BOOT_H,
            });
            serial_println!("[bell.placeholder.attach.frame] frame={} scene={} slot={}", BELL_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[bell.placeholder.attach.tab] frame={} tab=0 surface={}", BELL_FRAME_ID, SURFACE_ID_BELL_PLACEHOLDER);
            static mut BELL_CREATE_BUDGET: u32 = 4;
            let b = &mut BELL_CREATE_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.bell.frame.create] frame={} slot={}", BELL_FRAME_ID, slot_idx); }
            return Some(BELL_FRAME_ID);
        }
    }
    static mut BELL_NOSLOT_BUDGET: u32 = 4;
    let b = &mut BELL_NOSLOT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.bell.frame.reject] reason=no_slot"); }
    None
}

/// Open Bell in the active scene: ensure frame exists, un-minimize, position,
/// focus, and tile. If Bell is already visible in the active scene, focuses it.
unsafe fn open_bell_in_active_scene() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                serial_println!("[bell.placeholder.reject.duplicate] frame={} scene={}", BELL_FRAME_ID, ACTIVE_SCENE_IDX);
                if let Some(sid) = active_surface_for_frame(BELL_FRAME_ID) {
                    if try_set_focus(sid) {
                        serial_println!("[bell.placeholder.focus] frame={} sid={}", BELL_FRAME_ID, sid);
                    }
                }
                return true;
            }
        }
    }

    let fid = match ensure_bell_frame() {
        Some(f) => f,
        None => return false,
    };

    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == fid {
                frame.scene_id = ACTIVE_SCENE_IDX;
                break;
            }
        }
    }

    if frame_is_minimized(fid) {
        if !restore_minimized_frame(fid) {
            return false;
        }
        static mut BELL_RESTORE_BUDGET: u32 = 8;
        let b = &mut BELL_RESTORE_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.bell.lifecycle.restore] frame={}", fid); }
    } else if frame_is_zoomed(fid) {
    } else {
        let sid = match active_surface_for_frame(fid) {
            Some(s) => s,
            None => return false,
        };
        if surface_is_alive(sid) {
            pdx_call(SLOT_DISPLAY, 0xEC, sid,
                (BELL_BOOT_Y as u64) << 32 | BELL_BOOT_X as u64,
                (BELL_BOOT_H as u64) << 32 | BELL_BOOT_W as u64);
        }
        tile_active_scene_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
        serial_println!("[bell.placeholder.focus] frame={} sid={}", fid, sid);
    }

    bell_render_event_list();

    serial_println!("[bell.placeholder.open] frame={}", fid);
    snap_capture_layout();
    static mut BELL_OPEN_BUDGET: u32 = 4;
    let b = &mut BELL_OPEN_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.bell.open] frame={}", fid); }
    true
}

/// Focus Bell if it is already open. If not, open it.
unsafe fn focus_or_open_bell() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(BELL_FRAME_ID) {
                    if try_set_focus(sid) {
                        static mut BELL_FOCUS_BUDGET: u32 = 4;
                        let b = &mut BELL_FOCUS_BUDGET;
                        if *b > 0 { *b -= 1; serial_println!("[shell.bell.focus] frame={}", BELL_FRAME_ID); }
                        return true;
                    }
                }
            }
        }
    }
    open_bell_in_active_scene()
}

/// Toggle Bell visibility. Minimize if visible, open if not.
unsafe fn toggle_bell() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if minimize_frame(BELL_FRAME_ID) {
                    static mut BELL_TOGGLE_BUDGET: u32 = 4;
                    let b = &mut BELL_TOGGLE_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.bell.lifecycle.minimize] frame={}", BELL_FRAME_ID); }
                    return true;
                }
                return false;
            }
        }
    }
    open_bell_in_active_scene()
}

/// Return Bell's frame_id, if its frame exists.
unsafe fn bell_frame_id() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID {
                return Some(BELL_FRAME_ID);
            }
        }
    }
    None
}

/// Return a deterministic accent color for a Bell event row based on event kind.
/// V1 only supports ObjectLinkedToBuffer. Color derived from linked object kind.
unsafe fn bell_row_color(ev: &BellEvent) -> u32 {
    match ev.kind {
        BellEventKind::ObjectLinkedToBuffer => {
            // Derive color from the linked object's kind if available.
            if let Some(obj) = linen_object_by_id(ev.object_id) {
                linen_kind_color(obj.kind)
            } else {
                0x00404060 // fallback muted blue-grey
            }
        }
        _ => 0x00404060, // fallback for unimplemented event kinds
    }
}

/// Count events visible in the Bell list (capped at BELL_LIST_ROW_RECTS).
unsafe fn bell_visible_event_count() -> u8 {
    let count = bell_ring_count();
    if count == 0 { return 0; }
    core::cmp::min(count as u8, BELL_LIST_ROW_RECTS)
}

/// Brighten a 0x00RRGGBB color for selected row highlighting.
/// Adds 0x40 (~25%) to each RGB component with per-channel clamping.
fn bell_selected_row_highlight(color: u32) -> u32 {
    let r = core::cmp::min(((color >> 16) & 0xFF).wrapping_add(0x40), 0xFF);
    let g = core::cmp::min(((color >> 8) & 0xFF).wrapping_add(0x40), 0xFF);
    let b = core::cmp::min((color & 0xFF).wrapping_add(0x40), 0xFF);
    (r << 16) | (g << 8) | b
}

/// Advance Bell selection to the next visible event row. Wraps around.
unsafe fn bell_select_next_row() {
    let count = bell_visible_event_count();
    if count <= 1 {
        serial_println!("[bell.selection.reject] reason=single_or_empty count={}", count);
        return;
    }
    let current = BELL_SELECTED_ROW;
    let next = if current + 1 >= count { 0 } else { current + 1 };
    BELL_SELECTED_ROW = next;
    serial_println!("[bell.selection.next] prev={} next={}", current, next);
    bell_render_event_list();
}

/// Move Bell selection to the previous visible event row. Wraps around.
unsafe fn bell_select_prev_row() {
    let count = bell_visible_event_count();
    if count <= 1 {
        serial_println!("[bell.selection.reject] reason=single_or_empty count={}", count);
        return;
    }
    let current = BELL_SELECTED_ROW;
    let prev = if current == 0 { count - 1 } else { current - 1 };
    BELL_SELECTED_ROW = prev;
    serial_println!("[bell.selection.prev] prev={} next={}", current, prev);
    bell_render_event_list();
}

/// Return a copy of the Bell event at the currently selected visible row.
/// Iterates the ring newest-first (same order as bell_for_each_event)
/// to map BELL_SELECTED_ROW to the corresponding event. Returns None if
/// the ring is empty or the selected index has no event.
unsafe fn bell_selected_event_snapshot() -> Option<BellEvent> {
    let total = BELL_RING_WRITE_INDEX;
    let count = bell_ring_count();
    if count == 0 { return None; }
    let start = (total as usize).wrapping_sub(1) % BELL_RING_CAP;
    for i in 0..count {
        let idx = (start + BELL_RING_CAP - i) % BELL_RING_CAP;
        if let Some(ev) = BELL_EVENTS[idx] {
            if (i as u8) == BELL_SELECTED_ROW {
                return Some(ev);
            }
        }
    }
    None
}

/// Emit proof markers for the currently selected Bell event.
/// No action, no ack, no delete, no Bell PD. Proof-marker-only stub.
unsafe fn bell_emit_selected_event_detail_proof() {
    if FOCUSED_SURFACE_ID != SURFACE_ID_BELL_PLACEHOLDER {
        serial_println!("[bell.detail.reject] reason=not_focused");
        return;
    }
    let ev = match bell_selected_event_snapshot() {
        Some(e) => e,
        None => {
            serial_println!("[bell.detail.reject] reason=no_event");
            return;
        }
    };
    serial_println!("[bell.detail.open] event_id={} kind={:?}", ev.event_id, ev.kind);
    match ev.kind {
        BellEventKind::ObjectLinkedToBuffer => {
            serial_println!("[bell.detail.event] event_id={} kind=ObjectLinkedToBuffer object_id={} buffer_id={}",
                ev.event_id, ev.object_id, ev.buffer_id);
            serial_println!("[bell.detail.object_link] object_id={} buffer_id={}", ev.object_id, ev.buffer_id);
        }
        _ => {
            serial_println!("[bell.detail.reject] reason=unsupported_kind kind={:?}", ev.kind);
        }
    }
    serial_println!("[bell.detail.done] event_id={}", ev.event_id);
}

/// Check whether the Bell surface is currently visible in the active scene.
/// Returns true only when the Bell frame exists in the active scene,
/// is not minimized, and the surface is alive/focusable.
unsafe fn bell_is_visible_in_active_scene() -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == BELL_FRAME_ID
                && frame.scene_id == ACTIVE_SCENE_IDX
                && (frame.flags & FRAME_FLAG_MINIMIZED) == 0
            {
                if let Some(sid) = active_surface_for_frame(BELL_FRAME_ID) {
                    if surface_is_alive(sid) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Render the Bell event list as row fill rects inside the Bell placeholder surface.
/// Uses existing multi-rect pattern (header + event rows). Read-only over ring.
unsafe fn bell_render_event_list() {
    let w = SURFACE_204_W;
    let h = SURFACE_204_H;
    if w == 0 || h == 0 { return; }

    // Clamp selected row to valid range after ring changes.
    let visible = bell_visible_event_count();
    if visible == 0 {
        BELL_SELECTED_ROW = 0;
    } else if BELL_SELECTED_ROW >= visible {
        let old = BELL_SELECTED_ROW;
        BELL_SELECTED_ROW = visible.wrapping_sub(1);
        serial_println!("[bell.selection.repair] old={} new={} count={}", old, BELL_SELECTED_ROW, visible);
    }
    serial_println!("[bell.selection.current] row={} visible={}", BELL_SELECTED_ROW, visible);

    serial_println!("[bell.event_list.render] w={} h={} count={}", w, h, bell_ring_count());

    // Draw header bar at top of surface (rect_index=0).
    // arg2 format: (rect_index<<56)|(color_rgb<<32)|(sh<<16)|sw
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_BELL_PLACEHOLDER,
        (0u64 << 32) | 0u64,  // position (0,0)
        ((BELL_PLACEHOLDER_COLOR as u64) << 32)
            | ((BELL_LIST_HEADER_H as u64) << 16)
            | w as u64);

    // Emit row markers and visual fill rects, newest-first.
    let mut rows_emitted: u8 = 0;
    let mut rects_sent: u8 = 0;
    bell_for_each_event(|ev| {
        if rows_emitted >= BELL_LIST_ROW_RECTS {
            serial_println!("[bell.event_list.skip] event_id={} reason=max_rows", ev.event_id);
            return;
        }
        let kind_name = match ev.kind {
            BellEventKind::ObjectLinkedToBuffer => "ObjectLinkedToBuffer",
            _ => "Unknown",
        };
        serial_println!("[bell.event_list.row] event_id={} kind={} object_id={} buffer_id={}",
            ev.event_id, kind_name, ev.object_id, ev.buffer_id);

        // Send visual row rect if within fill-rect slot budget (slots 1-7; slot 0 = header).
        if rows_emitted < BELL_LIST_ROW_RECTS {
            let rect_index = (rows_emitted as u64 + 1) & 0xF;
            let row_y = BELL_LIST_HEADER_H
                + rows_emitted as u32 * (BELL_LIST_ROW_H + BELL_LIST_ROW_GAP);
            let base_color = bell_row_color(ev);
            let row_color = if rows_emitted == BELL_SELECTED_ROW {
                let highlighted = bell_selected_row_highlight(base_color);
                serial_println!("[bell.selection_visual.row] event_id={} index={} base={:#010x} highlight={:#010x}",
                    ev.event_id, rows_emitted, base_color, highlighted);
                highlighted
            } else {
                base_color
            };
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_BELL_PLACEHOLDER,
                (row_y as u64) << 32 | 0u64,
                (rect_index << 56)
                    | ((row_color as u64) << 32)
                    | ((BELL_LIST_ROW_H as u64) << 16)
                    | w as u64);
            serial_println!("[bell.row_visual.rect] index={} event_id={} kind={} color={:#010x}",
                rect_index, ev.event_id, kind_name, row_color);
            rects_sent += 1;
        } else {
            serial_println!("[bell.row_visual.skip] event_id={} reason=rect_budget", ev.event_id);
        }
        rows_emitted += 1;
    });
    serial_println!("[bell.event_list.done] count={} rows={} rects={}", bell_ring_count(), rows_emitted, rects_sent);
}

// ── K11: Command Palette Helpers ─────────────────────────────────────────────
// Shell-owned action router. Routes to existing SurfaceAction handlers.

/// Ensure a ShellFrame exists for the command palette in an empty FRAMES slot.
unsafe fn ensure_command_palette_frame() -> Option<u32> {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == COMMAND_PALETTE_FRAME_ID {
                return Some(COMMAND_PALETTE_FRAME_ID);
            }
        }
    }
    for (slot_idx, slot) in FRAMES.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: COMMAND_PALETTE_FRAME_ID,
                active_tab: 0,
                tab_count: 1,
                tabs: {
                    let mut t: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] =
                        [None; MAX_TABS_PER_FRAME as usize];
                    t[0] = Some(ShellTab {
                        surface_id: SURFACE_ID_COMMAND_PALETTE,
                        title_id: 0,
                        flags: 0,
                    });
                    t
                },
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR,
                normal_x: COMMAND_PALETTE_BOOT_X,
                normal_y: COMMAND_PALETTE_BOOT_Y,
                normal_w: COMMAND_PALETTE_BOOT_W,
                normal_h: COMMAND_PALETTE_BOOT_H,
            });
            serial_println!("[command_palette.attach.frame] frame={} scene={} slot={}", COMMAND_PALETTE_FRAME_ID, ACTIVE_SCENE_IDX, slot_idx);
            serial_println!("[command_palette.attach.tab] frame={} tab=0 surface={}", COMMAND_PALETTE_FRAME_ID, SURFACE_ID_COMMAND_PALETTE);
            return Some(COMMAND_PALETTE_FRAME_ID);
        }
    }
    None
}

/// Return a deterministic muted color for a specific Command (not necessarily selected).
/// Used for non-selected row visuals in the palette list.
fn command_kind_color(cmd: Command) -> u32 {
    match cmd {
        Command::OpenSelectedInQuil => 0x00605020, // muted amber
        Command::FocusLinen => 0x00206040,         // muted green
        Command::FocusQuil => 0x00206060,          // muted cyan
        Command::SceneNext => 0x00303060,          // muted indigo
        Command::OpenAtlas => 0x00503060,          // muted violet
    }
}

/// Return the accent color for the currently selected command in the palette.
/// Each command gets a distinctive color to provide visual feedback on selection.
fn command_palette_selected_accent() -> u32 {
    unsafe {
        let idx = COMMAND_PALETTE_SELECTED as usize;
        if idx >= COMMAND_LIST.len() {
            return 0x00404060; // default muted blue-grey
        }
        match COMMAND_LIST[idx].command {
            Command::OpenSelectedInQuil => 0x00C0A040, // amber (matching CodeFile)
            Command::FocusLinen => 0x0040C080,         // green (matching Document)
            Command::FocusQuil => 0x0040C0C0,          // cyan (matching QuilWorkspaceRef)
            Command::SceneNext => 0x006060C0,          // indigo (matching Reference)
            Command::OpenAtlas => 0x00A060C0,          // violet (matching MeshDiagnosticRef)
        }
    }
}

/// Render the command palette as a placeholder overlay.
/// Uses 0xEF fill rect with rect_index packing.
/// rect_index allocation (fits within sexdisplay MAX_RECTS=8):
///   0: header bar (selected command accent)
///   1: shared list background (neutral dark slate)
///   2: selected row highlight (full-width bright accent)
///   3-7: per-row left accent bars (5px wide, kind-colored)
unsafe fn palette_render_list() {
    let w = COMMAND_PALETTE_BOOT_W;
    let h = COMMAND_PALETTE_BOOT_H;
    if w == 0 || h == 0 { return; }

    serial_println!("[command_palette.render] w={} h={}", w, h);

    // Determine header color based on currently selected command.
    let header_color = command_palette_selected_accent();
    serial_println!("[command_palette.selection_visual.header] command={} index={} color={:#010x}",
        COMMAND_LIST[COMMAND_PALETTE_SELECTED as usize].command as u8,
        COMMAND_PALETTE_SELECTED, header_color);

    // Draw header bar at top of surface using selected command accent color (rect_index=0).
    // arg2 format: (rect_index<<56)|(color_rgb<<32)|(sh<<16)|sw
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_COMMAND_PALETTE,
        (0u64 << 32) | 0u64,
        ((header_color as u64) << 32)
            | ((PALETTE_LIST_HEADER_H as u64) << 16)
            | w as u64);

    let selected = COMMAND_PALETTE_SELECTED;
    let count = COMMAND_LIST.len();

    // ── List background (rect_index=1) ───────────────────────────────────────
    // Single neutral rect behind all rows. Dark slate provides contrast for
    // accent bars and selected row highlight.
    let list_bg_h = count as u32 * (PALETTE_LIST_ROW_H + PALETTE_LIST_ROW_GAP) - PALETTE_LIST_ROW_GAP;
    let list_bg_y = PALETTE_LIST_HEADER_H;
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_COMMAND_PALETTE,
        (list_bg_y as u64) << 32 | 0u64,
        (1u64 << 56)
            | ((PALETTE_LIST_BG_COLOR as u64) << 32)
            | ((list_bg_h as u64) << 16)
            | w as u64);
    serial_println!("[command_palette.bg_rect] y={} h={}", list_bg_y, list_bg_h);

    // ── Selected row highlight (rect_index=2) ───────────────────────────────
    // Full-width bright accent bar at the selected command row.
    // Suppressed (with reject marker) if selected index is out of bounds.
    if (selected as usize) >= count {
        serial_println!("[quil.palette.row.reject] index={} reason=out_of_bounds", selected);
    } else {
        let sel_y = PALETTE_LIST_HEADER_H
            + selected as u32 * (PALETTE_LIST_ROW_H + PALETTE_LIST_ROW_GAP);
        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_COMMAND_PALETTE,
            (sel_y as u64) << 32 | 0u64,
            (2u64 << 56)
                | ((header_color as u64) << 32)
                | ((PALETTE_LIST_ROW_H as u64) << 16)
                | w as u64);
        serial_println!("[command_palette.row_visual.selected] index={} color={:#010x}",
            selected, header_color);
    }

    // ── Per-row left accent bars (rect_indices 3-7) ─────────────────────────
    // Each command row gets a 5px-wide accent bar at the left edge.
    // Non-selected rows use muted command_kind_color; selected row uses header_color.
    for i in 0..count {
        let cmd = &COMMAND_LIST[i];
        let is_selected = i as u8 == selected;
        let sel = if is_selected { "true" } else { "false" };
        serial_println!("[command_palette.row] index={} cmd={} name={} selected={}",
            i, cmd.command as u8, cmd.name, sel);

        let accent_index = i as u64 + 3; // maps to rect_indices 3,4,5,6,7
        let row_y = PALETTE_LIST_HEADER_H
            + i as u32 * (PALETTE_LIST_ROW_H + PALETTE_LIST_ROW_GAP);
        let accent_color = if is_selected { header_color } else { command_kind_color(cmd.command) };
        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_COMMAND_PALETTE,
            (row_y as u64) << 32 | 0u64,
            (accent_index << 56)
                | ((accent_color as u64) << 32)
                | ((PALETTE_LIST_ROW_H as u64) << 16)
                | PALETTE_ACCENT_BAR_W as u64);
        serial_println!("[command_palette.row_visual.accent] index={} cmd={} color={:#010x} selected={}",
            i, cmd.command as u8, accent_color, sel);
    }
    serial_println!("[command_palette.done] count={} selected={}", count, selected);
}

/// Render and call the 0xEC upsert for the command palette surface geometry.
unsafe fn palette_show() {
    if ensure_command_palette_frame().is_none() { return; }
    // Position the palette via 0xEC geometry upsert.
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_COMMAND_PALETTE,
        ((COMMAND_PALETTE_BOOT_Y as u64) << 32) | (COMMAND_PALETTE_BOOT_X as u64 as u64),
        ((COMMAND_PALETTE_BOOT_H as u64) << 32) | COMMAND_PALETTE_BOOT_W as u64);
    palette_render_list();
}

/// Toggle the command palette open/closed.
unsafe fn toggle_command_palette() -> bool {
    if COMMAND_PALETTE_OPEN {
        // Close palette — minimize the frame.
        COMMAND_PALETTE_OPEN = false;
        if let Some(_) = bell_frame_id() { // use hide pattern
            if minimize_frame(COMMAND_PALETTE_FRAME_ID) {
                serial_println!("[command_palette.close]");
                return true;
            }
        }
        // If frame doesn't exist yet, just mark closed.
        serial_println!("[command_palette.close]");
        true
    } else {
        // Open palette — show the frame and render.
        COMMAND_PALETTE_OPEN = true;
        COMMAND_PALETTE_SELECTED = 0;
        palette_show();
        serial_println!("[command_palette.open]");
        true
    }
}

/// Advance selection to next command in the palette.
unsafe fn palette_select_next() {
    let count = COMMAND_LIST.len() as u8;
    if count <= 1 { return; }
    let next = if COMMAND_PALETTE_SELECTED + 1 >= count { 0 } else { COMMAND_PALETTE_SELECTED + 1 };
    COMMAND_PALETTE_SELECTED = next;
    serial_println!("[command_palette.select] index={}", next);
    palette_render_list();
}

/// Move selection to previous command in the palette.
unsafe fn palette_select_prev() {
    let count = COMMAND_LIST.len() as u8;
    if count <= 1 { return; }
    let prev = if COMMAND_PALETTE_SELECTED == 0 { count - 1 } else { COMMAND_PALETTE_SELECTED - 1 };
    COMMAND_PALETTE_SELECTED = prev;
    serial_println!("[command_palette.select] index={}", prev);
    palette_render_list();
}

/// Execute the currently selected command by routing to its SurfaceAction.
unsafe fn palette_execute_selected() -> bool {
    let idx = COMMAND_PALETTE_SELECTED as usize;
    if idx >= COMMAND_LIST.len() { return false; }
    let cmd = COMMAND_LIST[idx].command;
    serial_println!("[command_palette.execute] cmd={} name={}", cmd as u8, COMMAND_LIST[idx].name);

    // Route to existing SurfaceAction handler paths.
    // Each of these reuses the same match arms as keyboard-triggered actions.
    match cmd {
        Command::OpenSelectedInQuil => {
            if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
                let obj_id = linen_selected_object_id();
                if obj_id != 0 && open_linen_object_in_quil(obj_id) {
                    return true;
                }
            }
            serial_println!("[command_palette.reject] cmd={} reason=not_focused", cmd as u8);
            false
        }
        Command::FocusLinen => {
            open_linen_in_active_scene()
        }
        Command::FocusQuil => {
            open_quil_in_active_scene()
        }
        Command::SceneNext => {
            // Cycle to next scene.
            let total = 3; // hardcoded scene count
            let next = if ACTIVE_SCENE_IDX + 1 >= total { 0 } else { ACTIVE_SCENE_IDX + 1 };
            switch_scene(next);
            true
        }
        Command::OpenAtlas => {
            atlas_toggle();
            true
        }
    }
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
        SURFACE_ID_CURSOR | SURFACE_ID_LAUNCHER | SURFACE_ID_STATUS
        | SURFACE_ID_CLOCK | SURFACE_ID_BELL
        | SURFACE_ID_SCENE_SETTINGS => false,
        _ => {
            // Registry lookup: app surfaces use their closeable field
            if let Some(spec) = app_surface_spec(surface_id) {
                serial_println!("[shell.app_registry.lookup] closeable sid={} val={}", surface_id, spec.closeable);
                return spec.closeable;
            }
            surface_is_alive(surface_id)
        }
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
    // A6: Reject close if surface already in Closing/Tombstoned/Destroyed state.
    if let Some(state) = lifecycle_state(surface_id) {
        match state {
            LifecycleState::Closing | LifecycleState::Tombstoned | LifecycleState::Destroyed => {
                serial_println!("[tombstone.close.reject.dead] sid={} state={:?}", surface_id, state);
                serial_println!("[lifecycle.destroy.reject] sid={} state={:?} reason=already_dead", surface_id, state);
                return false;
            }
            _ => {}
        }
    }
    // A5: Check drag before lifecycle transition. Cancel drag on target surface.
    if let InteractionState::Dragging { surface_id: drag_sid, .. } = INTERACTION {
        if drag_sid == surface_id {
            serial_println!("[frame.light.close.reject.drag] sid={} cancel_drag", surface_id);
            // A6: Record tombstone for drag cancelled before close.
            let st = lifecycle_state(surface_id).unwrap_or(LifecycleState::Allocated);
            record_tombstone_event(surface_id, st, st, TombstoneReason::DragCancelled);
            try_transition(InteractionState::Idle);
        }
    }
    // A5: Clear focus first if this surface was focused.
    if FOCUSED_SURFACE_ID == surface_id {
        clear_focus_if_dead();
    }
    match surface_id {
        SURFACE_ID_APP    => SURFACE_100_ALIVE = false,
        SURFACE_ID_STATIC => SURFACE_101_ALIVE = false,
        SURFACE_ID_TEST3  => SURFACE_102_ALIVE = false,
        SURFACE_ID_TEST4  => SURFACE_103_ALIVE = false,
        _ => return false, // unknown or non-closeable surface
    }
    // A3/A6: Track lifecycle state transition: live -> Closing -> Tombstoned.
    // Record tombstone events for each stage.
    let old_state = lifecycle_state(surface_id).unwrap_or(LifecycleState::Visible);
    set_lifecycle_state(surface_id, LifecycleState::Closing);
    record_tombstone_event(surface_id, old_state, LifecycleState::Closing, TombstoneReason::CloseRequested);
    set_lifecycle_state(surface_id, LifecycleState::Tombstoned);
    record_tombstone_event(surface_id, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::CloseRequested);
    // A6: Complete the lifecycle FSM: Tombstoned -> Destroyed.
    set_lifecycle_state(surface_id, LifecycleState::Destroyed);
    record_tombstone_event(surface_id, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
    serial_println!("[lifecycle.destroy.record] sid={}", surface_id);
    serial_println!("[frame.light.close.fsm] sid={}", surface_id);
    // Deactivate surface on display (active=false). Sexdisplay does not free resources;
    // the shell's lifecycle FSM (Tombstoned) prevents reuse without generation safety.
    pdx_call(SLOT_DISPLAY, OP_SURFACE_DEACTIVATE, surface_id, 0, 0);
    // Focus fallback: clear remaining stale focus and drag.
    clear_focus_if_dead();
    clear_drag_if_dead();
    clear_hover_if_dead();
    clear_hover_if_wrong_scene();
    // Clear hover if the closed surface's frame is no longer valid.
    clear_hover_if_wrong_scene();
    // Re-tile remaining visible frames.
    tile_active_scene_frames();
    snap_capture_layout();
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

/// Returns true if the given frame should receive pointer/keyboard input.
/// Guards: must be in active scene, non-minimized, and have alive/non-tombstoned active tab.
unsafe fn frame_accepts_input(frame_id: u32) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if frame.scene_id != ACTIVE_SCENE_IDX { return false; }
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { return false; }
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    if !surface_is_alive(tab.surface_id) {
                        serial_println!("[tab.focus.reject.dead] frame={} tab={} surface={}",
                            frame_id, frame.active_tab, tab.surface_id);
                        return false;
                    }
                    if is_tombstoned(tab.surface_id) {
                        serial_println!("[tab.focus.reject.dead] frame={} tab={} surface={} reason=tombstoned",
                            frame_id, frame.active_tab, tab.surface_id);
                        return false;
                    }
                }
                return true;
            }
        }
    }
    false
}

/// B4: Derive tab chrome visibility for a frame.
/// multi-tab frame → always visible
/// single-tab + hover/rim hit → visible
/// otherwise → hidden
/// inactive/dead/minimized/tombstoned frames never show chrome.
unsafe fn frame_chrome_visible(frame_id: u32) -> bool {
    // Inactive scene frames never show chrome.
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if frame.scene_id != ACTIVE_SCENE_IDX { return false; }
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { return false; }
                break;
            }
        }
    }
    // Check active tab surface is alive and not tombstoned.
    if let Some(sid) = active_surface_for_frame(frame_id) {
        if !surface_is_alive(sid) { return false; }
        if is_tombstoned(sid) { return false; }
    } else {
        return false;
    }
    let tab_count = frame_tab_count(frame_id);
    if tab_count > 1 {
        return true; // multi-tab always visible
    }
    // Single-tab: visible only when hovered by pointer.
    HOVERED_FRAME_ID == frame_id && HOVER_KIND != HOVER_NONE
}

/// If the hovered frame no longer accepts input (wrong scene, minimized, etc.),
/// clear hover state to avoid stale highlights. Call after scene switch or minimize.
unsafe fn clear_hover_if_wrong_scene() {
    if HOVERED_FRAME_ID != 0 && !frame_accepts_input(HOVERED_FRAME_ID) {
        serial_println!("[shell.hover.clear_dead] frame={} reason=invalid_target", HOVERED_FRAME_ID);
        HOVERED_FRAME_ID = 0;
        HOVER_KIND = HOVER_NONE;
        HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
        serial_println!("[shell.frame.hover.clear.wrong-scene]");
    }
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
    // A3: Track lifecycle state transition: Visible/Hidden -> Minimized.
    set_lifecycle_state(surface_id, LifecycleState::Minimized);
    // Hide surface on display (deactivate). Restore uses 0xEC to re-activate.
    // Same 0xEE opcode as close, but lifecycle state differs (Minimized vs Tombstoned).
    pdx_call(SLOT_DISPLAY, OP_SURFACE_DEACTIVATE, surface_id, 0, 0);
    // Clear drag if dragging this surface.
    clear_drag_if_dead();
    // Clear hover if the minimized frame was hovered.
    if HOVERED_FRAME_ID == frame_id {
        HOVERED_FRAME_ID = 0;
        HOVER_KIND = HOVER_NONE;
        HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
        serial_println!("[shell.frame.minimize.hover.clear] frame={}", frame_id);
    }
    // Fall back focus if this surface was focused.
    clear_focus_if_dead();
    unsafe {
        static mut FRAME_MINIMIZE_BUDGET: u32 = 8;
        let b = &mut FRAME_MINIMIZE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[frame.light.minimize.fsm] frame={} surface={}", frame_id, surface_id);
        }
    }
    // A8: Re-tile after minimize — frame removed from visible set.
    tile_active_scene_frames();
    serial_println!("[shell.interact.tile.return] source=minimize frame={}", frame_id);
    serial_println!("[shell.interact.minimize] frame={} sid={}", frame_id, surface_id);
    static mut TILE_AFTER_MINIMIZE_BUDGET: u32 = 8;
    let b = &mut TILE_AFTER_MINIMIZE_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.tile.after_minimize] frame={}", frame_id); }
    snap_capture_layout();
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
    // A6: Reject restore for Tombstoned/Destroyed/Closing lifecycle states.
    if let Some(state) = lifecycle_state(surface_id) {
        if matches!(state, LifecycleState::Tombstoned | LifecycleState::Destroyed | LifecycleState::Closing) {
            serial_println!("[lifecycle.tombstone.reject_restore] sid={} state={:?}", surface_id, state);
            return false;
        }
    }
    // Clear minimized flag.
    set_frame_minimized(frame_id, false);
    // A3: Track lifecycle state transition: Minimized -> Visible.
    set_lifecycle_state(surface_id, LifecycleState::Visible);
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
    // A5: Focus restored surface and emit restore marker.
    try_set_focus(surface_id);
    serial_println!("[frame.light.restore.fsm] frame={} surface={}", frame_id, surface_id);
    // Re-tile to include the restored frame.
    tile_active_scene_frames();
    serial_println!("[shell.interact.tile.return] source=restore frame={}", frame_id);
    serial_println!("[shell.interact.restore] frame={} sid={}", frame_id, surface_id);
    unsafe {
        static mut FRAME_RESTORE_BUDGET: u32 = 8;
        let b = &mut FRAME_RESTORE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.restore] frame={} surface={}", frame_id, surface_id);
        }
    }
    snap_capture_layout();
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

/// Returns true if the given frame has the top bar enabled (default mode).
/// When true, hit targets use FRAME_TOP_BAR_HEIGHT_PX for the top chrome band.
/// When false, hit targets use the 4px rim model (minimal mode).
unsafe fn frame_has_top_bar(frame_id: u32) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id && (frame.flags & FRAME_FLAG_TOP_BAR) != 0 {
                return true;
            }
        }
    }
    false
}

/// Set or clear the top bar flag on the given frame.
unsafe fn set_frame_top_bar(frame_id: u32, enabled: bool) {
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                if enabled {
                    frame.flags |= FRAME_FLAG_TOP_BAR;
                } else {
                    frame.flags &= !FRAME_FLAG_TOP_BAR;
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
            if let Some(wnd) = WINDOWS.get_mut(1) {
                wnd.desc.x = x;
                wnd.desc.y = y;
                wnd.desc.width = w;
                wnd.desc.height = h;
            }
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
        SURFACE_ID_LINEN => {
            SURFACE_200_X = x; SURFACE_200_Y = y;
            SURFACE_200_W = w; SURFACE_200_H = h;
        }
        SURFACE_ID_QUIL => {
            SURFACE_201_X = x; SURFACE_201_Y = y;
            SURFACE_201_W = w; SURFACE_201_H = h;
        }
        SURFACE_ID_MESH => {
            SURFACE_202_X = x; SURFACE_202_Y = y;
            SURFACE_202_W = w; SURFACE_202_H = h;
        }
        SURFACE_ID_COLLAR => {
            SURFACE_203_X = x; SURFACE_203_Y = y;
            SURFACE_203_W = w; SURFACE_203_H = h;
        }
        SURFACE_ID_BELL_PLACEHOLDER => {
            SURFACE_204_X = x; SURFACE_204_Y = y;
            SURFACE_204_W = w; SURFACE_204_H = h;
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
    // Clear stale hover light — zoom changes surface geometry completely,
    // invalidating any light position from the previous (non-zoomed) chrome.
    HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
    // Preserve focus (zoom does not change focus).
    unsafe {
        static mut FRAME_ZOOM_BUDGET: u32 = 8;
        let b = &mut FRAME_ZOOM_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.zoom] frame={} surface={}", frame_id, surface_id);
        }
    }
    snap_capture_layout();
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
    // Clear stale hover light — unzoom restores normal geometry, which has
    // different chrome than the zoomed full-content-area geometry.
    HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
    // Preserve focus.
    unsafe {
        static mut FRAME_UNZOOM_BUDGET: u32 = 8;
        let b = &mut FRAME_UNZOOM_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.unzoom] frame={} surface={}", frame_id, surface_id);
        }
    }
    // A8: Re-tile after unzoom — frame returns to tiled layout.
    tile_active_scene_frames();
    static mut TILE_AFTER_UNZOOM_BUDGET: u32 = 8;
    let b = &mut TILE_AFTER_UNZOOM_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.tile.after_unzoom] frame={}", frame_id); }
    snap_capture_layout();
    true
}

/// Toggle zoom state for the given frame. If zoomed, unzoom. If not zoomed, zoom.
/// Returns true if the state changed.
unsafe fn toggle_zoom_frame(frame_id: u32) -> bool {
    // A5: Reject zoom for Closing/Tombstoned/Destroyed surfaces.
    let surface_id = active_surface_for_frame(frame_id);
    if let Some(sid) = surface_id {
        match lifecycle_state(sid) {
            Some(LifecycleState::Closing) | Some(LifecycleState::Tombstoned)
            | Some(LifecycleState::Destroyed) => {
                serial_println!("[frame.light.zoom.fsm.reject] frame={} surface={} lifecycle=invalid", frame_id, sid);
                return false;
            }
            _ => {}
        }
    }
    let result = if frame_is_zoomed(frame_id) {
        unzoom_frame(frame_id)
    } else {
        zoom_frame(frame_id)
    };
    if result {
        serial_println!("[frame.light.zoom.fsm] frame={}", frame_id);
    }
    result
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

    // Dispatch on chrome mode: top bar (default) vs minimal (4px rim).
    if frame_has_top_bar(frame_id) {
        // Default mode: lights in 16px top bar band.
        let band_bottom = sy + FRAME_TOP_BAR_HEIGHT_PX;
        if y < sy || y >= band_bottom {
            return FRAME_LIGHT_NONE;
        }
        let lx = x - sx;
        // CLOSE: gap from left edge.
        if lx >= FRAME_TOP_BAR_LIGHT_GAP_PX
            && lx < FRAME_TOP_BAR_LIGHT_GAP_PX + FRAME_TOP_BAR_LIGHT_SIZE_PX
        {
            return FRAME_LIGHT_CLOSE;
        }
        // MINIMIZE: gap + size + gap.
        let l2_start = FRAME_TOP_BAR_LIGHT_GAP_PX + FRAME_TOP_BAR_LIGHT_SIZE_PX
            + FRAME_TOP_BAR_LIGHT_GAP_PX;
        if lx >= l2_start && lx < l2_start + FRAME_TOP_BAR_LIGHT_SIZE_PX {
            return FRAME_LIGHT_MINIMIZE;
        }
        // ZOOM: gap + size + gap + size + gap.
        let l3_start = l2_start + FRAME_TOP_BAR_LIGHT_SIZE_PX + FRAME_TOP_BAR_LIGHT_GAP_PX;
        if lx >= l3_start && lx < l3_start + FRAME_TOP_BAR_LIGHT_SIZE_PX {
            return FRAME_LIGHT_ZOOM;
        }
        FRAME_LIGHT_NONE
    } else {
        // Minimal mode: lights in 4px rim band (existing behavior).
        let top_rim_bottom = sy + FRAME_RIM_PX;
        if y < sy || y >= top_rim_bottom {
            return FRAME_LIGHT_NONE;
        }
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
}

// ── Frame Tab Strip Helpers ─────────────────────────────────────────────────

/// Returns the number of valid tabs for the given frame.
unsafe fn frame_tab_count(frame_id: u32) -> u32 {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                return frame.tab_count as u32;
            }
        }
    }
    0
}

/// Returns the active tab index for the given frame, or 0 if no tabs.
unsafe fn frame_active_tab_index(frame_id: u32) -> u32 {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                return frame.active_tab as u32;
            }
        }
    }
    0
}

/// Detect which tab the pointer is over in the tab strip band.
/// Returns Some(tab_index) if the pointer is within a tab block in the
/// top rim band and outside the Frame Lights exclusion zone.
/// Tab blocks are sized as equal-width slots filling the available width.
/// Returns None if no tab is hit (lights zone, rim zone, content area).
unsafe fn frame_tab_at(frame_id: u32, x: i32, y: i32) -> Option<u32> {
    // Guard: only interactive tabs on frames that accept input.
    if !frame_accepts_input(frame_id) {
        return None;
    }
    let surface_id = active_surface_for_frame(frame_id)?;
    let bounds = get_surface_bounds(surface_id)?;
    let (sx, sy, sw, _sh) = bounds;

    // Determine chrome band height and light exclusion zone based on mode.
    let (band_height, exclusion) = if frame_has_top_bar(frame_id) {
        (FRAME_TOP_BAR_HEIGHT_PX, FRAME_TOP_BAR_LIGHT_EXCLUSION_PX)
    } else {
        (FRAME_TAB_STRIP_PX, FRAME_TAB_LIGHT_EXCLUSION_PX)
    };

    // Must be in the chrome band (top bar or top rim).
    if y < sy || y >= sy + band_height {
        return None;
    }

    // Must be outside the Frame Lights exclusion zone.
    let tab_strip_start = sx + exclusion;
    if x < tab_strip_start {
        return None;
    }

    // Must not extend into the right rim edge band.
    let right_rim_start = sx + sw as i32 - FRAME_RIM_PX;
    if x >= right_rim_start {
        return None;
    }

    // Compute tab slot layout: equal-width slots.
    let tab_count = frame_tab_count(frame_id);
    if tab_count == 0 {
        return None;
    }

    let available_width = (right_rim_start - tab_strip_start).max(0);
    if available_width < FRAME_TAB_MIN_WIDTH_PX {
        // Available width too small for even one tab — treat entire area as tab 0.
        return Some(0);
    }

    let slot_w = available_width / tab_count as i32;
    let lx = x - tab_strip_start;
    let tab_index = (lx / slot_w.max(1)).min(tab_count as i32 - 1);
    Some(tab_index as u32)
}

/// Send current tab metadata for the given frame to sexdisplay via OP_SURFACE_TAB_INFO.
/// Called after frame init and on tab changes (future).
unsafe fn send_frame_tab_info(frame_id: u32) {
    let surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return,
    };
    let tab_count = frame_tab_count(frame_id);
    let active_tab = frame_active_tab_index(frame_id);
    let top_bar: u64 = if frame_has_top_bar(frame_id) { 1 } else { 0 };
    // B4: Derive tab chrome visibility from frame state + hover.
    let tab_chrome_visible: u64 = if frame_chrome_visible(frame_id) { 1 } else { 0 };
    // Pack: low 8 bits = active_tab, bit 8 = top_bar, bit 9 = chrome_visible.
    let arg2 = (active_tab as u64) | (top_bar << 8) | (tab_chrome_visible << 9);
    pdx_call(SLOT_DISPLAY, OP_SURFACE_TAB_INFO, surface_id, tab_count as u64, arg2);
    unsafe {
        static mut SHELL_TAB_INFO_SEND_BUDGET: u32 = 8;
        let b = &mut SHELL_TAB_INFO_SEND_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.tab.info.send] frame={} surface={} tabs={} active={} top_bar={} chrome_visible={}",
                frame_id, surface_id, tab_count, active_tab, top_bar, tab_chrome_visible);
        }
    }
}

/// Toggle the top bar flag on the frame containing the currently focused surface.
/// Resolves the frame via selected_frame_id(), flips FRAME_FLAG_TOP_BAR,
/// and notifies sexdisplay via send_frame_tab_info() with updated chrome_flags.
/// Does not change focus, surface geometry, drag state, minimize, or zoom state.
/// Returns true if the toggle was applied.
unsafe fn toggle_top_bar_for_active_frame() -> bool {
    let frame_id = match selected_frame_id() {
        Some(fid) => fid,
        None => {
            unsafe {
                static mut TOP_BAR_TOGGLE_REJECT_BUDGET: u32 = 4;
                let b = &mut TOP_BAR_TOGGLE_REJECT_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[shell.frame.topbar.toggle.reject] reason=no_active_frame");
                }
            }
            return false;
        }
    };

    let new_state = !frame_has_top_bar(frame_id);
    set_frame_top_bar(frame_id, new_state);
    send_frame_tab_info(frame_id);
    // Chrome mode changed (top bar ↔ minimal) — all light positions have
    // shifted. Clear hover light to prevent stale light from a different
    // chrome geometry. Hover is re-evaluated on the next pointer event.
    HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;

    unsafe {
        static mut TOP_BAR_TOGGLE_BUDGET: u32 = 8;
        let b = &mut TOP_BAR_TOGGLE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.topbar.toggle] frame={} enabled={}",
                frame_id, new_state as u32);
        }
    }
    snap_capture_layout();
    true
}

/// Switch the active tab of the given frame to `tab_index`.
/// Hides the old tab's surface via 0xEE, shows the new tab's surface via 0xEC
/// at the current frame geometry, updates focus, and notifies sexdisplay.
/// Returns true if the switch succeeded.
unsafe fn switch_to_tab(frame_id: u32, tab_index: u32) -> bool {
    // Guard: frame must accept input (active scene, non-minimized, alive, non-tombstoned).
    // The mouse-click path (via frame_tab_at) checks this earlier, but keyboard
    // paths (focus_next_tab, focus_prev_tab) call switch_to_tab directly.
    if !frame_accepts_input(frame_id) {
        return false;
    }
    // Validate frame and tab_index.
    let frame = match FRAMES.iter_mut().find_map(|f| {
        if let Some(frame) = f {
            if frame.frame_id == frame_id { Some(frame) } else { None }
        } else { None }
    }) {
        Some(f) => f,
        None => return false,
    };
    if tab_index as u8 >= frame.tab_count {
        return false;
    }
    if tab_index as u8 == frame.active_tab {
        return true; // already on this tab — not an error
    }

    // Get old and new surface IDs.
    let old_surface_id = match active_surface_for_frame(frame_id) {
        Some(sid) => sid,
        None => return false,
    };
    let new_surface_id = match &frame.tabs[tab_index as usize] {
        Some(tab) => tab.surface_id,
        None => return false,
    };
    if new_surface_id == old_surface_id {
        return true;
    }

    // Capture current geometry from old surface before hiding.
    let bounds = get_surface_bounds(old_surface_id);
    let (sx, sy, sw, sh) = match bounds {
        Some(b) => b,
        None => (frame.normal_x, frame.normal_y, frame.normal_w, frame.normal_h),
    };

    // Update active_tab before sending 0xEE/0xEC so that sexdisplay
    // renders the correct highlight when tab info is sent below.
    frame.active_tab = tab_index as u8;
    // Drop mutable borrow before calling other helpers.
    drop(frame);

    // Hide old surface on display.
    if surface_is_alive(old_surface_id) {
        pdx_call(SLOT_DISPLAY, 0xEE, old_surface_id, 0, 0);
    }

    // Show new surface at captured geometry.
    pdx_call(SLOT_DISPLAY, 0xEC, new_surface_id,
        (sy as u64) << 32 | sx as u64,
        (sh as u64) << 32 | sw as u64);
    update_local_geometry(new_surface_id, sx, sy, sw, sh);

    // Set focus to new surface.
    try_set_focus(new_surface_id);

    // Clear any stale drag targeting the old surface.
    clear_drag_if_dead();
    // Clear stale hover light — the new active surface may have different
    // chrome geometry, making the old light position invalid.
    HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;

    // Notify sexdisplay of updated tab metadata.
    send_frame_tab_info(frame_id);

    unsafe {
        static mut TAB_SWITCH_BUDGET: u32 = 8;
        let b = &mut TAB_SWITCH_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.tab.switch] frame={} old={} new={} tab={}",
                frame_id, old_surface_id, new_surface_id, tab_index);
        }
    }
    snap_capture_layout();
    true
}

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
            static mut INTERACT_HOVER_BUDGET: u32 = 8;
            let b = &mut INTERACT_HOVER_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[shell.interact.hover] frame={} kind={} light={} x={} y={}",
                    new_frame_id, new_kind, new_light, x, y);
            }
        }
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

        // B4: Update tab chrome visibility on hover change.
        // Only the hovered frame's chrome state may change (multi-tab is stable;
        // single-tab toggles on hover enter/leave).
        if new_frame_id != 0 && frame_tab_count(new_frame_id) <= 1 {
            let chrome_on = frame_chrome_visible(new_frame_id);
            if chrome_on {
                unsafe {
                    static mut TAB_CHROME_SHOW_BUDGET: u32 = 8;
                    let b = &mut TAB_CHROME_SHOW_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[tab.chrome.show] frame={}", new_frame_id);
                    }
                }
            } else {
                unsafe {
                    static mut TAB_CHROME_HIDE_BUDGET: u32 = 4;
                    let b = &mut TAB_CHROME_HIDE_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[tab.chrome.hide] frame={}", new_frame_id);
                    }
                }
            }
            // Notify sexdisplay of chrome visibility change.
            send_frame_tab_info(new_frame_id);
        }
        // Multi-tab frames emit persist.multi on first hover if not already set.
        if new_frame_id != 0 && frame_tab_count(new_frame_id) > 1 && new_kind != HOVER_NONE {
            unsafe {
                static mut TAB_CHROME_PERSIST_BUDGET: u32 = 8;
                let b = &mut TAB_CHROME_PERSIST_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[tab.chrome.persist.multi] frame={}", new_frame_id);
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
        serial_println!("[shell.interact.reject] op=focus sid={} reason=nonfocusable", sid);
        serial_println!("[shell.focus.reject.nonfocusable] id={}", sid);
        return false;
    }
    if !surface_is_alive(sid) {
        serial_println!("[shell.interact.reject] op=focus sid={} reason=dead", sid);
        serial_println!("[shell.focus.reject.dead] id={}", sid);
        return false;
    }
    if is_tombstoned(sid) {
        serial_println!("[shell.interact.reject] op=focus sid={} reason=tombstoned", sid);
        serial_println!("[shell.focus.reject.tombstoned] id={}", sid);
        serial_println!("[lifecycle.tombstone.reject_focus] sid={} reason=tombstoned", sid);
        return false;
    }
    // A4: Reject if lifecycle state does not allow focus (Visible or Mapped only).
    if !surface_is_lifecycle_focusable(sid) {
        serial_println!("[shell.interact.reject] op=focus sid={} reason=lifecycle", sid);
        serial_println!("[focus.lifecycle.reject] id={}", sid);
        return false;
    }
    // A4: Verify generation is current before committing focus.
    if let Some(fr) = make_focus_ref(sid) {
        if !focus_ref_is_current(&fr) {
            serial_println!("[shell.interact.reject] op=focus sid={} reason=stale_generation", sid);
            serial_println!("[focus.generation.reject] id={}", sid);
            serial_println!("[lifecycle.generation.stale_reject] sid={} gen={:?}", sid, surface_generation(sid));
            return false;
        }
    }
    // B2: Reject focus if surface belongs to a frame in a non-active scene.
    // Panels, cursor, and non-frame surfaces have no scene association and are always eligible.
    if let Some(scene) = surface_scene_id(sid) {
        if scene != ACTIVE_SCENE_IDX {
            serial_println!("[shell.interact.reject] op=focus sid={} reason=inactive_scene", sid);
            serial_println!("[scene.focus.reject.inactive] id={} sid_scene={} active={}", sid, scene, ACTIVE_SCENE_IDX);
            return false;
        }
    }
    FOCUSED_SURFACE_ID = sid;
    // A4: Sync FocusRef shadow and emit commit marker.
    sync_focus_ref();
    serial_println!("[focus.ref.commit] id={}", sid);
    serial_println!("[shell.focus.set] id={}", sid);
    serial_println!("[shell.interact.focus] sid={}", sid);
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

    // D2: Focus description marker — fired on every successful focus change.
    // Logs role and label derived from the focused surface.
    unsafe {
        let mut label_str = "unknown";
        let mut role_str = "Surface";
        if sid == SURFACE_ID_QUIL { label_str = "Quil"; role_str = "AppPlaceholder"; }
        else if sid == SURFACE_ID_MESH { label_str = "Mesh"; role_str = "AppPlaceholder"; }
        else if sid == SURFACE_ID_COLLAR { label_str = "Collar"; role_str = "AppPlaceholder"; }
        else if sid == SURFACE_ID_BELL_PLACEHOLDER { label_str = "Bell"; role_str = "AppPlaceholder"; }
        else if sid == SURFACE_ID_LINEN { label_str = "Linen"; role_str = "AppPlaceholder"; }
        else if sid == SURFACE_ID_APP { label_str = "App"; role_str = "Frame"; }
        else if sid == SURFACE_ID_STATIC { label_str = "Test2"; role_str = "Frame"; }
        else if sid == SURFACE_ID_TEST3 { label_str = "Test3"; role_str = "Frame"; }
        else if sid == SURFACE_ID_TEST4 { label_str = "Test4"; role_str = "Frame"; }
        else if sid == SURFACE_ID_CURSOR { label_str = "Cursor"; role_str = "Desktop"; }
        else if let Some(frame_id) = frame_for_surface(sid) {
            if let Some(spec) = app_surface_spec_by_frame(frame_id) {
                label_str = spec.name;
            }
        }
        static mut ACCESS_FOCUS_DESCRIBE_BUDGET: u32 = 32;
        let b = &mut ACCESS_FOCUS_DESCRIBE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[access.focus.describe] target={} role={} label={}", sid, role_str, label_str);
        }
    }

    // D4: Structured numeric focus description via D2 semantic tree.
    // Pure logging — never mutates focus, lifecycle, or frame state.
    // Emits role token, state flags, action flags, target ids, label hash.
    access_describe_focus();

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
            unsafe {
                static mut INTERACT_DRAG_MOVE_BUDGET: u32 = 24;
                let b = &mut INTERACT_DRAG_MOVE_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[shell.interact.drag.move] sid={} dx={} dy={}", surface_id, dx, dy);
                }
            }
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

    // Guard: only provide chrome hit-targets for frames that accept input
    // (active scene, non-minimized, alive, non-tombstoned).
    if !frame_accepts_input(frame_id) {
        return None;
    }

    // Determine chrome mode: top bar (default) vs minimal (4px rim).
    let top_bar = frame_has_top_bar(frame_id);
    let band_height = if top_bar { FRAME_TOP_BAR_HEIGHT_PX } else { FRAME_RIM_PX };
    let tab_exclusion = if top_bar { FRAME_TOP_BAR_LIGHT_EXCLUSION_PX } else { FRAME_TAB_LIGHT_EXCLUSION_PX };

    // Tab strip (top band): highest priority. Gated on FRAME_TAB_STRIP_PX > 0.
    // In default mode, the tab strip uses the full top bar height.
    // The tab strip excludes the Frame Lights zone and the right rim band.
    // Lights are handled separately with higher priority in the click handler.
    if FRAME_TAB_STRIP_PX > 0 || top_bar {
        let strip_bot = sy + if top_bar { FRAME_TOP_BAR_HEIGHT_PX } else { FRAME_TAB_STRIP_PX };
        let tab_strip_start = sx + tab_exclusion;
        let right_rim_start = sx + sw as i32 - FRAME_RIM_PX;
        if y >= sy && y < strip_bot
            && x >= tab_strip_start
            && x < right_rim_start
        {
            // Verify a tab exists at this position (not empty gap).
            if frame_tab_at(frame_id, x, y).is_some() {
                return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
            }
        }
    }

    // Rim (edge band): check all four edges of the surface.
    // In default mode, the top edge uses band_height (=top bar height) instead of FRAME_RIM_PX.
    let right = sx + sw as i32 - 1;
    let bottom = sy + sh as i32 - 1;
    let in_rim =
        (x >= sx && x < sx + FRAME_RIM_PX)                            // left edge
        || (x > right - FRAME_RIM_PX && x <= right)                   // right edge
        || (y >= sy && y < sy + band_height)                          // top edge (or top bar)
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
        // Content-area hit on focused surface: verify frame accepts input.
        if let Some(fid) = frame_for_surface(focused) {
            if frame_accepts_input(fid) {
                return HitTarget::Surface(focused);
            }
            // Frame doesn't accept input — fall through to z-order.
        } else {
            // Non-frame surface (linen, standalone) — always hittable.
            return HitTarget::Surface(focused);
        }
    }
    let z_order = [SURFACE_ID_QUIL, SURFACE_ID_MESH, SURFACE_ID_COLLAR, SURFACE_ID_BELL_PLACEHOLDER, SURFACE_ID_LINEN, SURFACE_ID_TEST4,
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
            // Content-area hit on z-order surface: verify frame accepts input.
            if let Some(fid) = frame_for_surface(sid) {
                if frame_accepts_input(fid) {
                    return HitTarget::Surface(sid);
                }
                // Frame doesn't accept input — skip to next z-order.
                continue;
            } else {
                // Non-frame surface (linen, standalone) — always hittable.
                return HitTarget::Surface(sid);
            }
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
    // ── Atlas intercept: if Atlas mode is enabled, hit-test scene cards ──
    // Consumes all clicks while Atlas is open — hits switch scene and exit,
    // misses keep Atlas open. No SilkBar or frame chrome interaction in Atlas mode.
    if ATLAS_MODE_ENABLED {
        if let Some(scene_idx) = atlas_scene_at_point(px, py) {
            // Hit a scene card: destroy overlay, switch to scene, exit Atlas.
            pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_ATLAS_OVERLAY, 0, 0);
            // A3: Track Atlas overlay lifecycle exit.
            set_lifecycle_state(SURFACE_ID_ATLAS_OVERLAY, LifecycleState::Allocated);
            let already_active = scene_idx == ACTIVE_SCENE_IDX;
            if !already_active {
                switch_scene(scene_idx);
            } else {
                // Clicked active scene card: restore normal rendering without switching.
                sync_scene_visibility();
                clear_focus_if_dead();
                clear_drag_if_dead();
                clear_hover_if_dead();
                clear_hover_if_wrong_scene();
                tile_active_scene_frames();
                snap_capture_layout();
            }
            ATLAS_MODE_ENABLED = false;
            static mut ATLAS_SELECT_BUDGET: u32 = 4;
            let b = &mut ATLAS_SELECT_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.select] id={}", scene_idx); }
            serial_println!("[shell.interact.atlas.select] scene={}", scene_idx);
            return (HitTarget::None, true);
        } else {
            // Click missed all cards — keep Atlas open, consume click.
            static mut ATLAS_MISS_BUDGET: u32 = 4;
            let b = &mut ATLAS_MISS_BUDGET;
            if *b > 0 { *b -= 1; serial_println!("[shell.atlas.miss]"); }
            return (HitTarget::None, true);
        }
    }
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
                // B4: Reject Frame Light actions on inactive/dead/minimized/tombstoned frames.
                {
                    let mut reject = false;
                    let mut reason = "";
                    for f in FRAMES.iter() {
                        if let Some(frame) = f {
                            if frame.frame_id == frame_id {
                                if frame.scene_id != ACTIVE_SCENE_IDX { reject = true; reason = "inactive_scene"; break; }
                                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { reject = true; reason = "minimized"; break; }
                                if let Some(sid) = active_surface_for_frame(frame_id) {
                                    if !surface_is_alive(sid) { reject = true; reason = "dead"; break; }
                                    if is_tombstoned(sid) { reject = true; reason = "tombstoned"; break; }
                                }
                                break;
                            }
                        }
                    }
                    if reject {
                        serial_println!("[frame.light.reject.inactive] frame={} reason={}", frame_id, reason);
                        // Still handle rim drag for valid frames below.
                        // If the frame is inactive/minimized but not dead/tombstoned, allow rim drag.
                        if reason == "inactive_scene" || reason == "minimized" {
                            // Fall through to rim drag logic.
                        } else {
                            return (HitTarget::None, true);
                        }
                    }
                }
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
            } else if kind == FRAME_CHROME_TAB_STRIP {
                // Tab strip click: switch to tab at pointer position.
                if let Some(tab_index) = frame_tab_at(frame_id, px, py) {
                    if !switch_to_tab(frame_id, tab_index) {
                        unsafe {
                            static mut TAB_SWITCH_REJECT_BUDGET: u32 = 4;
                            let b = &mut TAB_SWITCH_REJECT_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[shell.frame.tab.switch.reject] frame={} tab={} reason=switch_failed",
                                    frame_id, tab_index);
                            }
                        }
                    }
                }
            } else {
                // Other non-rim chrome (reserved): capture/no-op.
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
        serial_println!("[shell.interact.drag.begin] sid={} x={} y={}", FOCUSED_SURFACE_ID, px, py);
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
        // A3: Track panel open -> Mapped lifecycle state.
        set_lifecycle_state(surface_id, LifecycleState::Mapped);
        try_transition(InteractionState::PanelActive { panel: kind });
    } else {
        serial_println!("[shell.{}.close.start] id={:#x}", label, surface_id);
        pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
        serial_println!("[shell.{}.close.ok] id={:#x}", label, surface_id);
        *active = false;
        // A3: Track panel close -> Allocated lifecycle state.
        set_lifecycle_state(surface_id, LifecycleState::Allocated);
        try_transition(InteractionState::Idle);
    }
    true
}

/// Toggle the Scene Settings quick panel (surface 0x96) via F7.
/// Reuses the same 0xEC/0xEE show/hide pattern as OS panels.
/// No text labels in V1 — shaped/colored rect affordances only.
unsafe fn toggle_scene_settings_panel() {
    // Budgeted marker for panel toggle
    static mut PANEL_BUDGET: u32 = 16;
    let budget = &mut PANEL_BUDGET;

    if !SCENE_SETTINGS_ACTIVE {
        serial_println!("[shell.scene.settings.panel.open.start] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_SCENE_SETTINGS,
            (SCENE_SETTINGS_PANEL_Y as u64) << 32 | SCENE_SETTINGS_PANEL_X as u64,
            (SCENE_SETTINGS_PANEL_H as u64) << 32 | SCENE_SETTINGS_PANEL_W as u64);
        serial_println!("[shell.scene.settings.panel.open.ok] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        SCENE_SETTINGS_ACTIVE = true;
        // A3: Track panel open -> Mapped lifecycle state.
        set_lifecycle_state(SURFACE_ID_SCENE_SETTINGS, LifecycleState::Mapped);
        try_transition(InteractionState::PanelActive { panel: PanelKind::Settings });
    } else {
        serial_println!("[shell.scene.settings.panel.close.start] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0);
        serial_println!("[shell.scene.settings.panel.close.ok] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        SCENE_SETTINGS_ACTIVE = false;
        // A3: Track panel close -> Allocated lifecycle state.
        set_lifecycle_state(SURFACE_ID_SCENE_SETTINGS, LifecycleState::Allocated);
        try_transition(InteractionState::Idle);
    }
    if *budget > 0 {
        *budget -= 1;
        serial_println!("[shell.scene.settings.panel] visible={}", SCENE_SETTINGS_ACTIVE as u8);
    }
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
            unsafe {
                let prev = ACTIVE_SCENE_IDX;
                if prev != ws_idx as u8 {
                    ACTIVE_SCENE_IDX = ws_idx as u8;
                    sync_scene_visibility();
                    clear_focus_if_wrong_scene();
                    clear_drag_if_dead();
                    clear_drag_if_wrong_scene();
                    clear_hover_if_wrong_scene();
                    tile_active_scene_frames();
                    snap_capture_layout();
                    atlas_capture_snapshot();
                    static mut SCENE_SWITCH_BUDGET: u32 = 8;
                    let b = &mut SCENE_SWITCH_BUDGET;
                    if *b > 0 { *b -= 1; serial_println!("[shell.scene.switch] from={} to={}", prev, ACTIVE_SCENE_IDX); }
                }
            }
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

// ── Scene Layout Snapshot (SCENE_PERSISTENCE_V1A) ─────────────────────────
// In-memory snapshot/restore for Scene layout. NO disk, NO sexstore, NO filesystem.
// Lives only in static memory for the current boot/session.
// Versioned fixed-size structs with bounded arrays and XOR checksum validation.

/// Magic bytes: "SC" (Scene Capture)
const SNAP_MAGIC: u8 = b'S'; // 0x53 — 'S' for Scene snapshot
/// Snapshot format version.
const SNAP_VERSION: u8 = 0x01;

/// Per-frame data in a snapshot. Fixed-size, no heap.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct FrameSnapshot {
    /// 0 = slot empty/invalid; 1 = valid entry.
    present: u8,
    frame_id: u32,
    scene_id: u8,
    active_tab: u8,
    tab_count: u8,
    flags: u32,
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
    /// Surface IDs for each tab (only first tab_count entries valid on restore).
    tab_surfaces: [u64; MAX_TABS_PER_FRAME as usize],
}

/// Top-level snapshot: versioned, fixed-size, checksummed.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SceneLayoutSnapshot {
    magic: u8,
    version: u8,
    active_scene: u8,
    frame_count: u8,
    /// XOR checksum of all bytes in the struct (magic..checksum excluded from XOR).
    checksum: u8,
    /// Padding to ensure deterministic layout (reserved, zero).
    _reserved: [u8; 3],
    frames: [FrameSnapshot; MAX_FRAMES],
}

/// Global snapshot storage — single copy, no queue, no history.
static mut SCENE_SNAPSHOT: SceneLayoutSnapshot = SceneLayoutSnapshot {
    magic: 0,
    version: 0,
    active_scene: 0,
    frame_count: 0,
    checksum: 0,
    _reserved: [0u8; 3],
    frames: [FrameSnapshot {
        present: 0, frame_id: 0, scene_id: 0, active_tab: 0, tab_count: 0,
        flags: 0, normal_x: 0, normal_y: 0, normal_w: 0, normal_h: 0,
        tab_surfaces: [0u64; MAX_TABS_PER_FRAME as usize],
    }; MAX_FRAMES],
};

/// Compute XOR checksum over the layout fields of a SceneLayoutSnapshot.
/// Skips the checksum byte itself (offset 4). Used for validate-after-restore
/// and defensive integrity checks.
fn snap_compute_checksum(snap: &SceneLayoutSnapshot) -> u8 {
    let ptr = snap as *const SceneLayoutSnapshot as *const u8;
    let len = core::mem::size_of::<SceneLayoutSnapshot>();
    let mut chk: u8 = 0u8;
    for i in 0..len {
        // Skip the checksum byte at offset 4.
        if i == 4 { continue; }
        chk ^= unsafe { *ptr.add(i) };
    }
    chk
}

/// Capture current FRAMES layout into the global SCENE_SNAPSHOT.
/// Called after layout mutations to keep the snapshot current.
unsafe fn snap_capture_layout() {
    let snap = &mut SCENE_SNAPSHOT;
    snap.magic = SNAP_MAGIC;
    snap.version = SNAP_VERSION;
    snap.active_scene = ACTIVE_SCENE_IDX;
    snap._reserved = [0u8; 3];

    let mut fcount: u8 = 0;
    for (i, f_opt) in FRAMES.iter().enumerate() {
        let fs = &mut snap.frames[i];
        if let Some(frame) = f_opt {
            fs.present = 1;
            fs.frame_id = frame.frame_id;
            fs.scene_id = frame.scene_id;
            fs.active_tab = frame.active_tab;
            fs.tab_count = frame.tab_count;
            fs.flags = frame.flags;
            fs.normal_x = frame.normal_x;
            fs.normal_y = frame.normal_y;
            fs.normal_w = frame.normal_w;
            fs.normal_h = frame.normal_h;
            // Copy tab surface IDs (only valid entries).
            for t in 0..MAX_TABS_PER_FRAME as usize {
                fs.tab_surfaces[t] = match &frame.tabs[t] {
                    Some(tab) => tab.surface_id,
                    None => 0u64,
                };
            }
            fcount += 1;
        } else {
            fs.present = 0;
            fs.frame_id = 0;
            fs.scene_id = 0;
            fs.active_tab = 0;
            fs.tab_count = 0;
            fs.flags = 0;
            fs.normal_x = 0;
            fs.normal_y = 0;
            fs.normal_w = 0;
            fs.normal_h = 0;
            for t in 0..MAX_TABS_PER_FRAME as usize {
                fs.tab_surfaces[t] = 0u64;
            }
        }
    }
    snap.frame_count = fcount;
    snap.checksum = snap_compute_checksum(snap);
}

/// Validate a SceneLayoutSnapshot: magic, version, counts, checksum.
/// Returns true if the snapshot is structurally valid.
/// Does NOT validate surface liveness — that is done in snap_restore_layout.
fn snap_validate(snap: &SceneLayoutSnapshot) -> bool {
    if snap.magic != SNAP_MAGIC { return false; }
    if snap.version != SNAP_VERSION { return false; }
    if snap.frame_count as usize > MAX_FRAMES { return false; }
    if snap.active_scene as usize >= WORKSPACE_COUNT as usize { return false; }
    // Verify each present frame has valid tab_count and flags.
    for i in 0..snap.frame_count as usize {
        let fs = &snap.frames[i];
        if fs.present == 0 { continue; }
        if fs.tab_count as usize > MAX_TABS_PER_FRAME as usize { return false; }
        if fs.active_tab as usize >= fs.tab_count as usize { return false; }
        // Flags must not contain reserved bits (only MINIMIZED | ZOOMED | TOP_BAR).
        let known_flags = FRAME_FLAG_MINIMIZED | FRAME_FLAG_ZOOMED | FRAME_FLAG_TOP_BAR;
        if fs.flags & !known_flags != 0 { return false; }
    }
    // Verify checksum.
    if snap.checksum != snap_compute_checksum(snap) { return false; }
    true
}

/// Restore FRAMES and ACTIVE_SCENE_IDX from a validated snapshot.
/// Safe: skips tombstoned/dead surfaces, clamps geometry, clears invalid state.
/// If snapshot is invalid, returns false and leaves current layout unchanged.
unsafe fn snap_restore_layout() -> bool {
    let snap = &SCENE_SNAPSHOT;
    if !snap_validate(snap) {
        static mut SNAP_RESTORE_REJECT_BUDGET: u32 = 1;
        let b = &mut SNAP_RESTORE_REJECT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.snapshot.restore.reject] reason=invalid"); }
        return false;
    }

    let mut restored_count: u8 = 0;
    for i in 0..MAX_FRAMES {
        let fs = &snap.frames[i];
        if fs.present == 0 {
            FRAMES[i] = None;
            continue;
        }
        // Build tabs array: skip tombstoned/dead surfaces.
        let mut tabs: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize] = [None; MAX_TABS_PER_FRAME as usize];
        let mut valid_count: u8 = 0;
        for t in 0..fs.tab_count as usize {
            let sid = fs.tab_surfaces[t];
            if sid == 0 { continue; }
            if is_tombstoned(sid) { continue; }
            if !surface_is_alive(sid) { continue; }
            tabs[valid_count as usize] = Some(ShellTab { surface_id: sid, title_id: 0, flags: 0 });
            valid_count += 1;
        }
        if valid_count == 0 { continue; } // skip frame with no valid tabs

        // Clamp geometry using existing helpers.
        let (cx, cy) = clamp_position(fs.normal_x.max(0), fs.normal_y.max(0), fs.normal_w.max(1), fs.normal_h.max(1));
        let (cw, ch) = clamp_surface_size(cx, cy, fs.normal_w.max(1), fs.normal_h.max(1));

        FRAMES[i] = Some(ShellFrame {
            frame_id: fs.frame_id,
            active_tab: fs.active_tab.min(valid_count - 1),
            tab_count: valid_count,
            tabs,
            scene_id: fs.scene_id.min(WORKSPACE_COUNT - 1),
            flags: fs.flags & (FRAME_FLAG_MINIMIZED | FRAME_FLAG_ZOOMED | FRAME_FLAG_TOP_BAR),
            normal_x: cx,
            normal_y: cy,
            normal_w: cw,
            normal_h: ch,
        });
        restored_count += 1;
    }

    if restored_count == 0 {
        static mut SNAP_RESTORE_EMPTY_BUDGET: u32 = 1;
        let b = &mut SNAP_RESTORE_EMPTY_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.snapshot.restore.reject] reason=no_valid_frames"); }
        return false;
    }

    // Restore active scene.
    ACTIVE_SCENE_IDX = snap.active_scene;
    // Notify silkbar of active scene.
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, ACTIVE_SCENE_IDX as u64, 0, 0);

    // Sync visibility: show surfaces in active scene, hide others.
    sync_scene_visibility();

    // Clear stale focus/hover/drag.
    clear_focus_if_dead();
    clear_focus_if_wrong_scene();
    clear_drag_if_dead();
    clear_drag_if_wrong_scene();
    clear_hover_if_wrong_scene();

    // Re-tile visible frames.
    tile_active_scene_frames();

    static mut SNAP_RESTORE_OK_BUDGET: u32 = 1;
    let b = &mut SNAP_RESTORE_OK_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.snapshot.restore.ok] frames={} scene={}",
        restored_count, ACTIVE_SCENE_IDX); }
    true
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

        // Initialize frame chrome model: one frame, two tabs wrapping surfaces 100 and 101.
        // normal_* initialized to boot geometry of surface 100 for zoom restore.
        let boot_x: i32 = 100;
        let boot_y: i32 = 100;
        let boot_w: u32 = 800;
        let boot_h: u32 = 500;
        FRAMES[0] = Some(ShellFrame {
            frame_id: 1,
            active_tab: 0,
            tab_count: 2,
            tabs: [
                Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
                Some(ShellTab { surface_id: SURFACE_ID_STATIC, title_id: 0, flags: 0 }),
                None, None, None, None, None, None,
            ],
            scene_id: 0,
            flags: FRAME_FLAG_TOP_BAR, // top bar ON by default
            normal_x: boot_x,
            normal_y: boot_y,
            normal_w: boot_w,
            normal_h: boot_h,
        });
        serial_println!("[shell.frame.model.init] frames=1 tabs=1");
        serial_println!("[frame.core.attach] frame=1 scene=0 tabs=2");
        serial_println!("[tab.core.attach] frame=1 tab=0 surface={}", SURFACE_ID_APP);
        serial_println!("[tab.core.attach] frame=1 tab=1 surface={}", SURFACE_ID_STATIC);

        // A3: Initialize lifecycle metadata for all known surfaces.
        lifecycle_init_all();

        // B1: Initialize scene metadata array from FRAMES state.
        scene_init_all();

        // J1: Initialize Linen object table with seed objects.
        linen_object_table_init();

        // J3: Initialize Quil buffer table with seed buffers.
        quil_buffer_table_init();

        // K2C: Synchronize Linen linked_surface_id for seed pre-links.
        linen_quil_seed_coherence_init();

        // C2: Initialize Collar auto-grants for seed objects.
        collar_init_grants();

        // Initial snapshot after frames are set up.
        snap_capture_layout();

        // Validate app surface registry at boot.
        app_surface_registry_validate();

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

    // Frame Tab Strip model: prove constants and helpers exist.
    unsafe {
        static mut FRAME_TAB_MODEL_BUDGET: u32 = 1;
        if FRAME_TAB_MODEL_BUDGET > 0 {
            FRAME_TAB_MODEL_BUDGET -= 1;
            let tab_count = {
                let mut c = 0u32;
                for f in FRAMES.iter() {
                    if let Some(frame) = f {
                        c = frame.tab_count as u32;
                        break;
                    }
                }
                c
            };
            let has_tab = frame_tab_at(1, 140, 101).is_some(); // sx=100 + exclusion=20, tab 0 at x=140
            serial_println!("[shell.frame.tab.model] tabs={} has_tab={} strip_px={}",
                tab_count, has_tab, FRAME_TAB_STRIP_PX);
        }
    }

    // Frame Top Bar model: prove flag, constants, and helper exist.
    unsafe {
        static mut FRAME_TOPBAR_MODEL_BUDGET: u32 = 1;
        if FRAME_TOPBAR_MODEL_BUDGET > 0 {
            FRAME_TOPBAR_MODEL_BUDGET -= 1;
            let enabled = frame_has_top_bar(1) as u32;
            serial_println!("[shell.frame.topbar.model] frame=1 enabled={} height={}",
                enabled, FRAME_TOP_BAR_HEIGHT_PX);
        }
    }

    // Stage: cursor surface — created first so it occupies SURFACES slot 0,
    // winning composite Pass 1 over all other non-focused surfaces.
    serial_println!("[shell.cursor_surface.create.start] id={:#x}", SURFACE_ID_CURSOR);
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_CURSOR,
        ((P.height / 2) as u64) << 32 | (P.width / 2) as u64,
        (18u64 << 32) | 12u64);
    serial_println!("[shell.cursor_surface.create.ok]");

    // Legacy demo surfaces (100..103) are intentionally not created at boot in
    // the two-surface startup path to avoid transient fullscreen overlays.
    serial_println!("[silk-shell.boot.legacy_surfaces.skip] ids=100,101,102,103");

    // ── UI Readiness: deterministic boot content layout (below SilkBar) ──
    let content_x: i32 = 0;
    let content_y: i32 = P.bar_height.max(0);
    let content_w: u32 = (P.width.max(0)) as u32;
    let content_h: u32 = (P.height - content_y).max(0) as u32;
    if content_w == 0 || content_h == 0 {
        serial_println!(
            "[silk-shell.boot.layout.reject] reason=invalid_content_rect x={} y={} w={} h={}",
            content_x,
            content_y,
            content_w,
            content_h
        );
    } else {
        serial_println!(
            "[silk-shell.boot.layout.content] x={} y={} w={} h={}",
            content_x,
            content_y,
            content_w,
            content_h
        );
    }

    // Dynamic non-overlapping tiled demo layout for boot:
    // Quil = left/main tile, Linen = right/secondary tile.
    let gutter: u32 = 16;
    let min_w = P.min_width as u32;
    let min_h = P.min_height as u32;
    let tile_h: u32 = content_h.max(min_h);
    let main_w_target = content_w.saturating_mul(72) / 100;
    let side_w_target = content_w.saturating_sub(main_w_target).saturating_sub(gutter);
    let side_w: u32 = side_w_target.max(min_w).min(content_w.saturating_sub(min_w).saturating_sub(gutter));
    let main_w: u32 = content_w.saturating_sub(side_w).saturating_sub(gutter).max(min_w);
    let boot_quil_x = content_x;
    let boot_quil_y = content_y;
    let boot_quil_w = main_w;
    let boot_quil_h = tile_h;
    let boot_linen_x = content_x + boot_quil_w as i32 + gutter as i32;
    let boot_linen_y = content_y;
    let linen_w: u32 = side_w;
    let linen_h: u32 = tile_h;

    unsafe {
        SURFACE_201_X = boot_quil_x;
        SURFACE_201_Y = boot_quil_y;
        SURFACE_201_W = boot_quil_w;
        SURFACE_201_H = boot_quil_h;
        SURFACE_200_X = boot_linen_x;
        SURFACE_200_Y = boot_linen_y;
        SURFACE_200_W = linen_w;
        SURFACE_200_H = linen_h;
    }

    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_QUIL,
        (boot_quil_y as u64) << 32 | boot_quil_x as u64,
        (boot_quil_h as u64) << 32 | boot_quil_w as u64);
    serial_println!("[silk-shell] Boot 0xEC surface 201 (Quil) created");
    serial_println!("[silk-shell.boot.surface.create] sid={} owner=quil", SURFACE_ID_QUIL);
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_LINEN,
        (boot_linen_y as u64) << 32 | boot_linen_x as u64,
        (linen_h as u64) << 32 | linen_w as u64);
    serial_println!("[silk-shell] Boot 0xEC surface 200 (Linen) created");
    serial_println!("[silk-shell.boot.surface.create] sid={} owner=linen", SURFACE_ID_LINEN);
    serial_println!("[silk-shell.ui.ready] surfaces=2");
    serial_println!("[silk-shell.boot.layout.tiled] mode=2pane main_sid={} side_sid={} gutter={}",
        SURFACE_ID_QUIL, SURFACE_ID_LINEN, gutter);

    // Deterministic boot-readiness proof for Linen/Quil visibility ordering.
    unsafe {
        static mut UI_BOOT_PROOF_BUDGET: u32 = 1;
        if UI_BOOT_PROOF_BUDGET > 0 {
            UI_BOOT_PROOF_BUDGET -= 1;

            // bounds checks
            if boot_quil_w > 0 && boot_quil_h > 0 {
                serial_println!("[silk-shell.boot.surface.bounds] sid={} x={} y={} w={} h={}",
                    SURFACE_ID_QUIL, boot_quil_x, boot_quil_y, boot_quil_w, boot_quil_h);
            } else {
                serial_println!("[silk-shell.boot.reject] sid={} reason=zero_bounds", SURFACE_ID_QUIL);
            }
            if linen_w > 0 && linen_h > 0 {
                serial_println!("[silk-shell.boot.surface.bounds] sid={} x={} y={} w={} h={}",
                    SURFACE_ID_LINEN, boot_linen_x, boot_linen_y, linen_w, linen_h);
            } else {
                serial_println!("[silk-shell.boot.reject] sid={} reason=zero_bounds", SURFACE_ID_LINEN);
            }

            // composition truth: sexdisplay composites non-focused first, then focused on top.
            serial_println!("[silk-shell.compose.order] focused_top=1 focus={}", SURFACE_ID_QUIL);

            // visibility/liveness checks
            let q_visible = surface_is_alive(SURFACE_ID_QUIL) && !is_tombstoned(SURFACE_ID_QUIL);
            let l_visible = surface_is_alive(SURFACE_ID_LINEN) && !is_tombstoned(SURFACE_ID_LINEN);
            serial_println!("[silk-shell.boot.surface.visible] sid={} visible={}", SURFACE_ID_QUIL, if q_visible { 1 } else { 0 });
            serial_println!("[silk-shell.boot.surface.visible] sid={} visible={}", SURFACE_ID_LINEN, if l_visible { 1 } else { 0 });

            // Visible boot stack truth: focused Quil is top, Linen remains visible (non-overlap).
            if q_visible && l_visible {
                serial_println!("[silk-shell.boot.zorder] visible_count=2 first={} second={}", SURFACE_ID_QUIL, SURFACE_ID_LINEN);
            } else {
                serial_println!("[silk-shell.boot.zorder.reject] reason=boot_pair_not_visible q={} l={}",
                    if q_visible { 1 } else { 0 }, if l_visible { 1 } else { 0 });
            }
        }
    }

    // Initialize focus on surface 201 (syncs sexdisplay z-order + color)
    pdx_call(SLOT_DISPLAY, 0xED, SURFACE_ID_QUIL, 0, 0);
    serial_println!("[silk-shell] Boot focus set to surface 201 (Quil)");
    if surface_is_alive(SURFACE_ID_QUIL) && !unsafe { is_tombstoned(SURFACE_ID_QUIL) } {
        serial_println!("[silk-shell.boot.focus] sid={} valid=1", SURFACE_ID_QUIL);
    } else {
        serial_println!("[silk-shell.boot.reject] sid={} reason=focus_invalid", SURFACE_ID_QUIL);
    }
    // A3: Sync initial FocusRef from boot focus.
    unsafe { sync_focus_ref(); }
    serial_println!("[silk-shell.boot.ui.ready] surfaces=2 focus={}", SURFACE_ID_QUIL);

    // Send initial tab metadata for frame 1 (surface 100: 1 tab, active tab 0)
    unsafe { send_frame_tab_info(1); }
    serial_println!("[silk-shell] Boot tab info sent to sexdisplay");

    // Push default scene render tokens to sexdisplay (establishes DISPLAY_TOKENS baseline)
    unsafe { send_scene_render_tokens(); }
    serial_println!("[silk-shell] Boot scene render tokens sent to sexdisplay");

    // Fire GET to sexstore for persisted scene appearance settings.
    // Reply arrives asynchronously in main loop via type_id == 0x1.
    unsafe { boot_load_scene_settings(); }
    serial_println!("[silk-shell.ready]");

    loop {
        // Runtime containment: park without syscall while null-jump root cause is isolated.
        if !SHELL_USB_MOUSE_RECEIVE_UNPARK_PROOF_V1 {
            core::hint::spin_loop();
            continue;
        }

        // ── Scene Settings Protocol synthetic proof ──
        if SCENE_SETTINGS_PROTOCOL_PROOF_ENABLED {
            unsafe {
                let stage = SCENE_SETTINGS_PROTOCOL_PROOF_STAGE;
                if stage < 5 {
                    SCENE_SETTINGS_PROTOCOL_PROOF_STAGE = stage + 1;
                    serial_println!("[shell.scene.settings.cmd.proof] stage={}", stage);
                    match stage {
                        0 => handle_scene_settings_cmd(CMD_SET_PRESET, 1, 0),
                        1 => handle_scene_settings_cmd(CMD_CYCLE_TINT, 0, 0),
                        2 => handle_scene_settings_cmd(CMD_TOGGLE_TOP_BAR, 0, 0),
                        3 => handle_scene_settings_cmd(CMD_RESET_DEFAULTS, 0, 0),
                        4 => handle_scene_settings_cmd(99, 0, 0),
                        _ => {}
                    }
                    sys_yield();
                    continue;
                }
            }
        }

        // ── App Surface Request synthetic proof ──
        if APP_SURFACE_REQ_PROOF_ENABLED {
            unsafe {
                let stage = APP_SURFACE_REQ_PROOF_STAGE;
                if stage < 4 {
                    APP_SURFACE_REQ_PROOF_STAGE = stage + 1;
                    serial_println!("[shell.app_surface.proof] stage={}", stage);
                    let accepted = match stage {
                        0 => handle_app_surface_req(300, 42, 0), // valid: sid=300, title=42
                        1 => handle_app_surface_req(0, 42, 0),   // reject: zero sid
                        2 => handle_app_surface_req(301, 0, 0),  // reject: zero title
                        3 => handle_app_surface_req(300, 99, 0), // reject: duplicate sid
                        _ => false,
                    };
                    serial_println!("[shell.app_surface.proof] stage={} accepted={}", stage, accepted);
                    sys_yield();
                    continue;
                }
            }
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
                        clear_hover_if_dead();
                        clear_hover_if_wrong_scene();
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
                                    serial_println!("[shell.interact.drag.end] sid={} x={} y={}", surface_id, POINTER_X, POINTER_Y);
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
                }
                OP_HID_EVENT => {
                    let scancode = msg.arg0 as u8;
                    let value = msg.arg1; // 1=pressed, 0=released
                    let event_class = msg.arg2; // EV_KEY, EV_REL, EV_ABS, EV_BTN

                    unsafe {
                        // ── Event-class dispatch ──
                        if event_class == EV_KEY && value == 1 {
                            // Track C2: key routing proof
                            if FOCUSED_SURFACE_ID == SURFACE_ID_QUIL {
                                unsafe {
                                    static mut KEY_ROUTE_BUDGET: u32 = 16;
                                    let b = &mut KEY_ROUTE_BUDGET;
                                    if *b > 0 {
                                        *b -= 1;
                                        serial_println!("[silk-shell.key.route] owner=quil sid={} scancode={:#x}", SURFACE_ID_QUIL, scancode);
                                    }
                                }
                                pdx_call(SLOT_QUIL, OP_HID_EVENT, scancode as u64, value, EV_KEY);
                                mutated = true;
                            } else if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
                                unsafe {
                                    static mut KEY_ROUTE_BUDGET_LINEN: u32 = 16;
                                    let b = &mut KEY_ROUTE_BUDGET_LINEN;
                                    if *b > 0 {
                                        *b -= 1;
                                        serial_println!("[silk-shell.key.route] owner=linen sid={} scancode={:#x}", SURFACE_ID_LINEN, scancode);
                                    }
                                }
                                pdx_call(sex_pdx::SLOT_LINEN, OP_HID_EVENT, scancode as u64, value, EV_KEY);
                                mutated = true;
                            }

                            // ── Scene Settings panel key intercept ──────────────
                            // When panel visible, route 1/2/3/Esc to panel commands.
                            // [shell.scene.settings.panel.key] budget 16.
                            // F7 (0x41) falls through to normal dispatch unchanged.
                            let mut panel_consumed = false;
                            if SCENE_SETTINGS_ACTIVE {
                                static mut PANEL_KEY_BUDGET: u32 = 16;
                                let b = &mut PANEL_KEY_BUDGET;
                                match scancode {
                                    0x01 => { // Esc → close panel; consumed to prevent AccessZoomToggle
                                        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0);
                                        SCENE_SETTINGS_ACTIVE = false;
                                        try_transition(InteractionState::Idle);
                                        mutated = true;
                                        panel_consumed = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=close"); }
                                    }
                                    0x02 => { // Key 1 → cycle preset (like F5)
                                        cycle_scene_render_token_preset();
                                        mutated = true;
                                        panel_consumed = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=preset"); }
                                    }
                                    0x03 => { // Key 2 → cycle tint (like F6)
                                        cycle_custom_tint();
                                        mutated = true;
                                        panel_consumed = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=tint"); }
                                    }
                                    0x04 => { // Key 3 → toggle top bar (like F4)
                                        if toggle_top_bar_for_active_frame() {
                                            mutated = true;
                                        }
                                        panel_consumed = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=topbar"); }
                                    }
                                    // All other scancodes (including F7=0x41) fall through to normal dispatch
                                    _ => {}
                                }
                            }
                            // ── Command palette keyboard intercept: consume keys when palette open ──
                            if !panel_consumed && COMMAND_PALETTE_OPEN {
                                match scancode {
                                    0x24 => { palette_select_next(); mutated = true; } // J - next
                                    0x25 => { palette_select_prev(); mutated = true; } // K - prev
                                    0x1C => { // Enter - execute
                                        let _ = palette_execute_selected();
                                        toggle_command_palette(); // close after execute
                                        mutated = true;
                                    }
                                    0x01 => { // Escape - close
                                        toggle_command_palette();
                                        mutated = true;
                                    }
                                    0x29 => { // backtick - close
                                        toggle_command_palette();
                                        mutated = true;
                                    }
                                    _ => {} // pass through to normal dispatch
                                }
                                if mutated { panel_consumed = true; }
                            }
                            // ── Atlas keyboard intercept: consume non-F10 keys when Atlas active ──
                            if panel_consumed {
                                // panel or palette handled key; skip Atlas and action dispatch
                            } else if ATLAS_MODE_ENABLED && scancode != 0x44 /* F10 falls through to ToggleAtlas */ {
                                handle_atlas_keyboard(scancode);
                                mutated = true;
                            // ── Bell focused-surface navigation: J/K nav + Enter detail proof ──
                            } else if FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER
                                && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C)
                            {
                                match scancode {
                                    0x24 => {
                                        serial_println!("[bell.keyboard.next] sid={}", FOCUSED_SURFACE_ID);
                                        bell_select_next_row();
                                    }
                                    0x25 => {
                                        serial_println!("[bell.keyboard.prev] sid={}", FOCUSED_SURFACE_ID);
                                        bell_select_prev_row();
                                    }
                                    0x1C => {
                                        serial_println!("[bell.keyboard.enter] sid={}", FOCUSED_SURFACE_ID);
                                        bell_emit_selected_event_detail_proof();
                                    }
                                    _ => {}
                                }
                                mutated = true;
                            // ── Mesh focused-surface navigation: J/K nav + Enter detail proof ──
                            } else if FOCUSED_SURFACE_ID == SURFACE_ID_MESH
                                && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C || scancode == 0x59)
                            {
                                match scancode {
                                    0x24 => {
                                        serial_println!("[mesh.keyboard.next] sid={}", FOCUSED_SURFACE_ID);
                                        mesh_select_next_row();
                                    }
                                    0x25 => {
                                        serial_println!("[mesh.keyboard.prev] sid={}", FOCUSED_SURFACE_ID);
                                        mesh_select_prev_row();
                                    }
                                    0x1C => {
                                        serial_println!("[mesh.keyboard.enter] sid={}", FOCUSED_SURFACE_ID);
                                        // N8: Emit detail proof markers.
                                        if mesh_emit_selected_fact_detail_proof() {
                                            // N11: Focus Linen at selected fact after successful proof.
                                            if let Some(fact) = mesh_selected_fact_snapshot() {
                                                mesh_focus_linen_at_selected_fact(&fact);
                                            }
                                        }
                                    }
                                    // N14: Open selected fact's linked object in Quil via PrintScreen.
                                    // Reuses existing open_linen_object_in_quil() which contains the
                                    // Collar gate (LinkObjectToBuffer → grant table lookup). Mesh cannot bypass
                                    // Collar because the gate is inside the callee, not at the call site.
                                    0x59 => {
                                        serial_println!("[mesh.keyboard.open_in_quil] sid={}", FOCUSED_SURFACE_ID);
                                        if let Some(fact) = mesh_selected_fact_snapshot() {
                                            open_linen_object_in_quil(fact.subject_id);
                                        }
                                    }
                                    _ => {}
                                }
                                mutated = true;
                            // ── Spindle focused-surface: capture printable input + dispatch commands ──
                            // Consumes: Enter, Backspace, Escape, Space, alphanumeric.
                            // All other keys fall through to scancode_to_action unchanged.
                            } else if FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE
                                && (scancode == 0x1C || scancode == 0x0E || scancode == 0x01
                                    || scancode == 0x0F || scancode == 0x39
                                    || (scancode >= 0x02 && scancode <= 0x0B)
                                    || (scancode >= 0x10 && scancode <= 0x19)
                                    || (scancode >= 0x1E && scancode <= 0x26)
                                    || scancode == 0x2C || scancode == 0x2D || scancode == 0x2E
                                    || scancode == 0x2F || scancode == 0x30 || scancode == 0x31
                                    || scancode == 0x32)
                            {
                                match scancode {
                                    0x1C => { // Enter — dispatch command
                                        serial_println!("[spindle.enter] len={}", YARN.cmd_len);
                                        spindle_dispatch();
                                    }
                                    0x0E => { // Backspace — edit command buffer
                                        if YARN.cmd_len > 0 {
                                            YARN.cmd_len -= 1;
                                            YARN.cmd_buf[YARN.cmd_len] = 0;
                                            serial_println!("[spindle.key.backspace] len={}", YARN.cmd_len);
                                        }
                                    }
                                    0x01 => { // Escape — clear command buffer
                                        if YARN.cmd_len > 0 {
                                            YARN.cmd_buf = [0u8; YARN_CMD_BUF_CAP];
                                            YARN.cmd_len = 0;
                                            serial_println!("[spindle.key.escape]");
                                        }
                                    }
                                    0x0F => { // Tab — future completion (consume for now)
                                        serial_println!("[spindle.key.tab] deferred");
                                    }
                                    0x39 => { // Space
                                        if YARN.cmd_len < YARN_CMD_BUF_CAP - 1 {
                                            YARN.cmd_buf[YARN.cmd_len] = b' ';
                                            YARN.cmd_len += 1;
                                            serial_println!("[spindle.key.char] ch=' '");
                                        }
                                    }
                                    // Numbers 1-9, 0 (evdev KEY_1 through KEY_0).
                                    s if s >= 0x02 && s <= 0x0B => {
                                        static SCAN2NUM: [u8; 10] = [b'1',b'2',b'3',b'4',b'5',b'6',b'7',b'8',b'9',b'0'];
                                        let ch = SCAN2NUM[(s - 0x02) as usize];
                                        if YARN.cmd_len < YARN_CMD_BUF_CAP - 1 {
                                            YARN.cmd_buf[YARN.cmd_len] = ch;
                                            YARN.cmd_len += 1;
                                            serial_println!("[spindle.key.char] ch={}", ch as char);
                                        }
                                    }
                                    // Lowercase letters (unshifted V1). evdev KEY_Q through KEY_P (row 1).
                                    s if s >= 0x10 && s <= 0x19 => {
                                        static SCAN2ROW1: [u8; 10] = [b'q',b'w',b'e',b'r',b't',b'y',b'u',b'i',b'o',b'p'];
                                        let ch = SCAN2ROW1[(s - 0x10) as usize];
                                        if YARN.cmd_len < YARN_CMD_BUF_CAP - 1 {
                                            YARN.cmd_buf[YARN.cmd_len] = ch;
                                            YARN.cmd_len += 1;
                                            serial_println!("[spindle.key.char] ch={}", ch as char);
                                        }
                                    }
                                    // Lowercase letters A-L (row 2). evdev KEY_A through KEY_L.
                                    s if s >= 0x1E && s <= 0x26 => {
                                        static SCAN2ROW2: [u8; 9] = [b'a',b's',b'd',b'f',b'g',b'h',b'j',b'k',b'l'];
                                        let ch = SCAN2ROW2[(s - 0x1E) as usize];
                                        if YARN.cmd_len < YARN_CMD_BUF_CAP - 1 {
                                            YARN.cmd_buf[YARN.cmd_len] = ch;
                                            YARN.cmd_len += 1;
                                            serial_println!("[spindle.key.char] ch={}", ch as char);
                                        }
                                    }
                                    // Remaining letters: Z, X, C, V, B, N, M.
                                    s if s == 0x2C || s == 0x2D || s == 0x2E || s == 0x2F
                                        || s == 0x30 || s == 0x31 || s == 0x32 => {
                                        static SCAN2ROW3: [u8; 7] = [b'z',b'x',b'c',b'v',b'b',b'n',b'm'];
                                        let idx = match s {
                                            0x2C => 0, 0x2D => 1, 0x2E => 2, 0x2F => 3,
                                            0x30 => 4, 0x31 => 5, 0x32 => 6,
                                            _ => 0,
                                        };
                                        let ch = SCAN2ROW3[idx];
                                        if YARN.cmd_len < YARN_CMD_BUF_CAP - 1 {
                                            YARN.cmd_buf[YARN.cmd_len] = ch;
                                            YARN.cmd_len += 1;
                                            serial_println!("[spindle.key.char] ch={}", ch as char);
                                        }
                                    }
                                    _ => {}
                                }
                                mutated = true;
                            } else if let Some(action) = scancode_to_action(scancode) {
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

                                    // D3: Accessibility keyboard actions via semantic node tree.
                                    // These dispatch through access_handle_keyboard_action() which uses the
                                    // D2 semantic node tree for focus traversal and activate (minimize/restore).
                                    // All paths are lifecycle-safe: try_set_focus, minimize_frame,
                                    // restore_minimized_frame. No direct state mutation.
                                    SurfaceAction::AccessFocusNext |
                                    SurfaceAction::AccessFocusPrev |
                                    SurfaceAction::AccessActivate => {
                                        if access_handle_keyboard_action(action) {
                                            mutated = true;
                                        }
                                    }

                                    // D3B: Additional keyboard accessibility actions.
                                    // Close (F11) → close_surface_from_frame_light()
                                    // Zoom toggle (Esc) → toggle_zoom_frame()
                                    // Scene next/prev → next_scene()/prev_scene() (bindings deferred)
                                    SurfaceAction::AccessClose |
                                    SurfaceAction::AccessZoomToggle |
                                    SurfaceAction::AccessSceneNext |
                                    SurfaceAction::AccessScenePrev => {
                                        if access_handle_keyboard_action(action) {
                                            mutated = true;
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
                                            // A6: lifecycle transitions + tombstone for keyboard destroy.
                                            let os_100 = lifecycle_state(target).unwrap_or(LifecycleState::Visible);
                                            set_lifecycle_state(target, LifecycleState::Closing);
                                            record_tombstone_event(target, os_100, LifecycleState::Closing, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Tombstoned);
                                            record_tombstone_event(target, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Destroyed);
                                            record_tombstone_event(target, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
                                            serial_println!("[lifecycle.destroy.record] sid={}", target);
                                        } else if target == SURFACE_ID_STATIC && SURFACE_101_ALIVE {
                                            SURFACE_101_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 101");
                                            // A6: lifecycle transitions + tombstone for keyboard destroy.
                                            let os_101 = lifecycle_state(target).unwrap_or(LifecycleState::Visible);
                                            set_lifecycle_state(target, LifecycleState::Closing);
                                            record_tombstone_event(target, os_101, LifecycleState::Closing, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Tombstoned);
                                            record_tombstone_event(target, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Destroyed);
                                            record_tombstone_event(target, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
                                            serial_println!("[lifecycle.destroy.record] sid={}", target);
                                        } else if target == SURFACE_ID_TEST3 && SURFACE_102_ALIVE {
                                            SURFACE_102_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 102");
                                            // A6: lifecycle transitions + tombstone for keyboard destroy.
                                            let os_102 = lifecycle_state(target).unwrap_or(LifecycleState::Visible);
                                            set_lifecycle_state(target, LifecycleState::Closing);
                                            record_tombstone_event(target, os_102, LifecycleState::Closing, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Tombstoned);
                                            record_tombstone_event(target, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Destroyed);
                                            record_tombstone_event(target, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
                                            serial_println!("[lifecycle.destroy.record] sid={}", target);
                                        } else if target == SURFACE_ID_TEST4 && SURFACE_103_ALIVE {
                                            SURFACE_103_ALIVE = false;
                                            pdx_call(SLOT_DISPLAY, 0xEE, target, 0, 0);
                                            destroyed = true;
                                            serial_println!("[silk-shell] Destroyed surface 103");
                                            // A6: lifecycle transitions + tombstone for keyboard destroy.
                                            let os_103 = lifecycle_state(target).unwrap_or(LifecycleState::Visible);
                                            set_lifecycle_state(target, LifecycleState::Closing);
                                            record_tombstone_event(target, os_103, LifecycleState::Closing, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Tombstoned);
                                            record_tombstone_event(target, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::DestroyCommand);
                                            set_lifecycle_state(target, LifecycleState::Destroyed);
                                            record_tombstone_event(target, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
                                            serial_println!("[lifecycle.destroy.record] sid={}", target);
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
                                            clear_drag_if_dead();
                                            clear_hover_if_wrong_scene();
                                            snap_capture_layout();
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
                                            if let Some(w) = WINDOWS.get_mut(1) {
                                                w.desc.x = rx; w.desc.y = ry;
                                                w.desc.width = rw; w.desc.height = rh;
                                            }
                                            SURFACE_100_W = rw; SURFACE_100_H = rh;
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
                                            if let Some(w) = WINDOWS.get_mut(1) {
                                                w.desc.x = rx; w.desc.y = ry;
                                                w.desc.width = rw; w.desc.height = rh;
                                            }
                                            SURFACE_100_W = rw; SURFACE_100_H = rh;
                                            try_set_focus(SURFACE_ID_APP);
                                            mutated = true;
                                            serial_println!("[silk-shell] Recreated surface 100 (fallback)");
                                        }
                                        snap_capture_layout();
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

                                    SurfaceAction::ToggleTopBar => {
                                        if toggle_top_bar_for_active_frame() {
                                            mutated = true;
                                        }
                                    }

                                    SurfaceAction::ToggleLinen => {
                                        if toggle_linen() {
                                            mutated = true;
                                            serial_println!("[shell.action.linen] toggle");
                                        }
                                    }

                                    SurfaceAction::ToggleQuil => {
                                        if toggle_quil() {
                                            mutated = true;
                                            serial_println!("[shell.action.quil] toggle");
                                        }
                                    }

                                    SurfaceAction::ToggleMesh => {
                                        if toggle_mesh() {
                                            mutated = true;
                                            serial_println!("[shell.action.mesh] toggle");
                                        }
                                    }

                                    SurfaceAction::ToggleCollar => {
                                        if toggle_collar() {
                                            mutated = true;
                                            serial_println!("[shell.action.collar] toggle");
                                        }
                                    }

                                    SurfaceAction::ToggleBell => {
                                        if toggle_bell() {
                                            mutated = true;
                                            serial_println!("[shell.action.bell] toggle");
                                        }
                                    }

                                    SurfaceAction::ToggleSpindle => {
                                        if toggle_spindle() {
                                            mutated = true;
                                            serial_println!("[shell.action.spindle] toggle");
                                        }
                                    }

                                    // K9: Open selected Linen object into a Quil buffer — scoped to Linen focus.
                                    SurfaceAction::OpenObjectInQuil => {
                                        if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
                                            let obj_id = linen_selected_object_id();
                                            if obj_id == 0 {
                                                serial_println!("[linen.quil.open.reject.no_selection]");
                                            } else if open_linen_object_in_quil(obj_id) {
                                                mutated = true;
                                                serial_println!("[shell.action.open_object_in_quil] object_id={}", obj_id);
                                            }
                                        } else {
                                            serial_println!("[linen.quil.open.reject] reason=not_focused");
                                        }
                                    }

                                    // K4: Cycle Linen selection forward — gated to Linen-focused state.
                                    SurfaceAction::SelectNextLinenObject => {
                                        if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
                                            linen_select_next_object();
                                            linen_render_object_list();
                                            mutated = true;
                                            serial_println!("[shell.action.select_next_linen] id={}", SELECTED_LINEN_OBJECT_ID);
                                        } else {
                                            serial_println!("[linen.object_select.reject] reason=not_focused");
                                        }
                                    }

                                    // K4: Cycle Linen selection backward — gated to Linen-focused state.
                                    SurfaceAction::SelectPrevLinenObject => {
                                        if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
                                            linen_select_prev_object();
                                            linen_render_object_list();
                                            mutated = true;
                                            serial_println!("[shell.action.select_prev_linen] id={}", SELECTED_LINEN_OBJECT_ID);
                                        } else {
                                            serial_println!("[linen.object_select.reject] reason=not_focused");
                                        }
                                    }

                                    SurfaceAction::ToggleAtlas => {
                                        unsafe { atlas_toggle(); }
                                        mutated = true;
                                    }

                                    SurfaceAction::ToggleCommandPalette => {
                                        unsafe { toggle_command_palette(); }
                                        mutated = true;
                                    }

                                    SurfaceAction::ToggleSceneSettingsPanel => {
                                        mutated = true;
                                        unsafe { toggle_scene_settings_panel(); }
                                    }

                                    SurfaceAction::CycleRenderTokenPreset => {
                                        unsafe { cycle_scene_render_token_preset(); }
                                    }

                                    SurfaceAction::CycleCustomTint => {
                                        unsafe { cycle_custom_tint(); }
                                    }

                                    SurfaceAction::ResetAll => {
                                        let (rx, ry, rw, rh) = P.boot_rect_100;
                                        SURFACE_100_ALIVE = true;
                                        if let Some(w) = WINDOWS.get_mut(1) {
                                            w.desc.x = rx; w.desc.y = ry;
                                            w.desc.width = rw; w.desc.height = rh;
                                        }
                                        SURFACE_100_W = rw; SURFACE_100_H = rh;

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
                                        snap_capture_layout();

                                        mutated = true;
                                        serial_println!("[silk-shell] Reset all surfaces to boot state");
                                    }

                                    SurfaceAction::SnapLeft |
                                    SurfaceAction::SnapRight |
                                    SurfaceAction::SnapHome |
                                    SurfaceAction::SnapEnd |
                                    SurfaceAction::Maximize |
                                    SurfaceAction::Center => {
                                        mutated = true;
                                        tile_active_scene_frames();
                                        snap_capture_layout();
                                    }

                                    SurfaceAction::ShrinkWidth => {
                                        let focused = FOCUSED_SURFACE_ID;
                                        if focused == SURFACE_ID_APP && SURFACE_100_ALIVE {
                                            let (wx, wy) = WINDOWS.get(1).map_or((0, 0), |w| (w.desc.x, w.desc.y));
                                            let new_w = SURFACE_100_W.saturating_sub(P.resize_step);
                                            let (new_w, _) = clamp_surface_size(wx, wy, new_w, SURFACE_100_H);
                                            if new_w != SURFACE_100_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (wy as u64) << 32 | wx as u64,
                                                    (SURFACE_100_H as u64) << 32 | new_w as u64);
                                                SURFACE_100_W = new_w;
                                                if let Some(w) = WINDOWS.get_mut(1) {
                                                    w.desc.width = new_w;
                                                }
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
                                            let (wx, wy) = WINDOWS.get(1).map_or((0, 0), |w| (w.desc.x, w.desc.y));
                                            let new_w = SURFACE_100_W + P.resize_step;
                                            let (new_w, _) = clamp_surface_size(wx, wy, new_w, SURFACE_100_H);
                                            if new_w != SURFACE_100_W {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (wy as u64) << 32 | wx as u64,
                                                    (SURFACE_100_H as u64) << 32 | new_w as u64);
                                                SURFACE_100_W = new_w;
                                                if let Some(w) = WINDOWS.get_mut(1) {
                                                    w.desc.width = new_w;
                                                }
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
                                            let (wx, wy) = WINDOWS.get(1).map_or((0, 0), |w| (w.desc.x, w.desc.y));
                                            let new_h = SURFACE_100_H.saturating_sub(P.resize_step);
                                            let (_, new_h) = clamp_surface_size(wx, wy, SURFACE_100_W, new_h);
                                            if new_h != SURFACE_100_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (wy as u64) << 32 | wx as u64,
                                                    (new_h as u64) << 32 | SURFACE_100_W as u64);
                                                SURFACE_100_H = new_h;
                                                if let Some(w) = WINDOWS.get_mut(1) {
                                                    w.desc.height = new_h;
                                                }
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
                                            let (wx, wy) = WINDOWS.get(1).map_or((0, 0), |w| (w.desc.x, w.desc.y));
                                            let new_h = SURFACE_100_H + P.resize_step;
                                            let (_, new_h) = clamp_surface_size(wx, wy, SURFACE_100_W, new_h);
                                            if new_h != SURFACE_100_H {
                                                pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_APP,
                                                    (wy as u64) << 32 | wx as u64,
                                                    (new_h as u64) << 32 | SURFACE_100_W as u64);
                                                SURFACE_100_H = new_h;
                                                if let Some(w) = WINDOWS.get_mut(1) {
                                                    w.desc.height = new_h;
                                                }
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
                            unsafe {
                                static mut SILK_SHELL_POINTER_RECV_BUDGET: u32 = 2048;
                                let rem = &mut SILK_SHELL_POINTER_RECV_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[silk-shell.pointer.recv] class={} a0={} a1={}", event_class, msg.arg0 as i32, msg.arg1 as i32);
                                }
                            }
                            POINTER_X = (msg.arg0 as i32).clamp(0, P.width - 1);
                            POINTER_Y = (msg.arg1 as i32).clamp(0, P.height - 1);
                            serial_println!("[silk-shell] Pointer ABS ({}, {})", POINTER_X, POINTER_Y);
                            
                            // Budgeted marker: shell sends cursor surface update to display.
                            unsafe {
                                static mut SHELL_CURSOR_SURFACE_UPDATE_BUDGET_ABS: u32 = 16;
                                let rem = &mut SHELL_CURSOR_SURFACE_UPDATE_BUDGET_ABS;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[shell.cursor.surface.update] n=0 x={} y={}", POINTER_X, POINTER_Y);
                                }
                            }
                            // Move cursor surface to updated pointer position.
                            serial_println!("[shell.cursor_surface.move.start] id={:#x} x={} y={}", SURFACE_ID_CURSOR, POINTER_X, POINTER_Y);
                            pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, POINTER_X as u64, POINTER_Y as u64);
                            serial_println!("[shell.cursor_surface.move.ok]");
                            unsafe {
                                static mut SILK_SHELL_CURSOR_UPDATE_BUDGET_ABS: u32 = 16;
                                let rem = &mut SILK_SHELL_CURSOR_UPDATE_BUDGET_ABS;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[silk-shell.cursor.update] x={} y={}", POINTER_X, POINTER_Y);
                                }
                            }
                        } else if event_class == EV_REL {
                            let dx = msg.arg0 as i32;
                            let dy = msg.arg1 as i32;
                            unsafe {
                                static mut SILK_SHELL_POINTER_RECV_BUDGET: u32 = 2048;
                                let rem = &mut SILK_SHELL_POINTER_RECV_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[silk-shell.pointer.recv] class={} a0={} a1={}", event_class, dx, dy);
                                }
                            }
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
                            unsafe {
                                static mut SILK_SHELL_CURSOR_UPDATE_BUDGET: u32 = 16;
                                let rem = &mut SILK_SHELL_CURSOR_UPDATE_BUDGET;
                                if *rem > 0 {
                                    *rem -= 1;
                                    serial_println!("[silk-shell.cursor.update] x={} y={}", POINTER_X, POINTER_Y);
                                }
                            }
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
                            clear_hover_if_dead();
                            clear_hover_if_wrong_scene();

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
                                            serial_println!("[shell.interact.drag.end] sid={} x={} y={}", surface_id, POINTER_X, POINTER_Y);
                                            try_transition(InteractionState::Idle);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            OP_SCENE_SETTINGS_CMD => {
                // Scene Settings protocol command.
                // Dispatch is synchronous, non-blocking, no reply wait.
                unsafe {
                    handle_scene_settings_cmd(msg.arg0, msg.arg1, msg.arg2);
                }
                mutated = true;
            }
            0x1 => {
                // Reply from sexstore (GET result or PUT ack).
                // type_id == 0x1 is the kernel's IpcReply marker for
                // syscall 29 (SYSCALL_PDX_REPLY) pushes to incoming_replies.
                unsafe {
                    if SEXSTORE_LOAD_PENDING {
                        SEXSTORE_LOAD_PENDING = false;
                        handle_sexstore_get_reply(msg.arg0);
                    }
                    // PUT acks are fire-and-forget — ignored.
                }
            }
            0xFA => { // OP_APP_SURFACE_REQ
                unsafe {
                    let accepted = handle_app_surface_req(msg.arg0, msg.arg1, msg.caller_pd);
                    if accepted {
                        mutated = true;
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

        // A3: Sync FocusRef from FOCUSED_SURFACE_ID after all state changes.
        unsafe { sync_focus_ref(); }

        sys_yield();
    }
}
