#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_listen_raw, pdx_reply, sys_yield, sys_set_state, serial_println, WindowDescriptor,
    SLOT_DISPLAY, SLOT_SILKBAR, SLOT_SEXSTORE, OP_SILKBAR_WORKSPACE_ACTIVE, OP_SILKBAR_FOCUS_STATE,
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
pub const SURFACE_ID_APP: u64 = 100;
pub const SURFACE_ID_STATIC: u64 = 101;
pub const SURFACE_ID_TEST3: u64 = 102;
pub const SURFACE_ID_TEST4: u64 = 103;
pub const SURFACE_ID_LINEN: u64 = 200;
pub const SURFACE_ID_QUIL: u64 = 201;
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
pub const SURFACE_ID_SCENE_SETTINGS: u64 = 0x96;
pub const OP_SURFACE_DESTROY: u64 = 0xEE;

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
const APP_SURFACES: [AppSurfaceSpec; 2] = [
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
];

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
///   Byte 7: checksum  = XOR(byte0 .. byte6)
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
    let chk: u8 = b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6];
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
    let expected: u8 = b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6];
    if b[7] != expected {
        return None;
    }
    Some((b[2], b[3], b[4]))
}

/// Handle a validated GET reply: apply persisted fields, reset ephemeral
/// state to defaults, re-send tokens to sexdisplay.
unsafe fn handle_sexstore_get_reply(value: u64) {
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
    ToggleAtlas,       // F10 — toggle Atlas overview mode
    ToggleSceneSettingsPanel,
    CycleRenderTokenPreset,
    CycleCustomTint,
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
        0x3E => Some(SurfaceAction::ToggleTopBar),           // F4
        0x3F => Some(SurfaceAction::CycleRenderTokenPreset), // F5
        0x40 => Some(SurfaceAction::CycleCustomTint),        // F6
        0x41 => Some(SurfaceAction::ToggleSceneSettingsPanel), // F7
        0x3B => Some(SurfaceAction::LegacyFocusToggle),
        0x42 => Some(SurfaceAction::ToggleLinen),    // F8
        0x43 => Some(SurfaceAction::ToggleQuil),     // F9
        0x44 => Some(SurfaceAction::ToggleAtlas),    // F10
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
    let mut tiles: [u64; MAX_FRAMES] = [0; MAX_FRAMES];
    let mut count: usize = 0;
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
            if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { continue; }
            // Zoomed frames are excluded from tiling — their surface occupies the
            // full content area via layout_maximize(). Tiling them would overwrite
            // the zoomed position with a tiled position, corrupting the zoom state.
            if (frame.flags & FRAME_FLAG_ZOOMED) != 0 { continue; }
            if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                if count < MAX_FRAMES {
                    tiles[count] = tab.surface_id;
                    count += 1;
                }
            }
        }
    }
    if count == 0 { return; }

    let cw: u32 = P.width as u32;
    let ch: u32 = (P.height - P.bar_height) as u32;

    for i in 0..count {
        let sid = tiles[i];
        if !surface_is_alive(sid) { continue; }

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

        match sid {
            SURFACE_ID_APP => {
                WINDOWS[1].desc.x = rx; WINDOWS[1].desc.y = ry;
                WINDOWS[1].desc.width = rw; WINDOWS[1].desc.height = rh;
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
            _ => {}
        }
        pdx_call(SLOT_DISPLAY, 0xEC, sid,
            (ry as u64) << 32 | rx as u64,
            (rh as u64) << 32 | rw as u64);
        // Quil visual placeholder: set distinctive fill rect after geometry update.
        if sid == SURFACE_ID_QUIL {
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0,
                (QUIL_PLACEHOLDER_COLOR as u64) << 32 | ((rh as u64) << 16) | rw as u64);
        }
    }
    static mut TILE_BUDGET: u32 = 8;
    let b = &mut TILE_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.tile] count={}", count); }
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

// ── Atlas Overview Model ─────────────────────────────────────────────────────
/// Atlas is Silk's shell-owned map of all Scenes.
/// It sits above Scene in the abstraction stack:
///   Silk → Atlas → Scene → Frame → Tab → Surface
/// V1 is data/model only: no rendering, no sexdisplay changes.
/// Future phases add Atlas toggle action, card rendering, scene select, previews.

/// Maximum scenes tracked by Atlas (equals WORKSPACE_COUNT).
const ATLAS_MAX_SCENES: usize = 5;
/// Maximum frames tracked per scene descriptor (equals MAX_FRAMES).
const ATLAS_MAX_FRAMES_PER_SCENE: usize = 4;
/// Length of fixed-size scene label byte array (no heap strings).
const ATLAS_LABEL_LEN: usize = 16;

/// SceneDescriptor flags
const SCENE_FLAG_ACTIVE: u8         = 1 << 0;  // this scene is active
const SCENE_FLAG_EMPTY: u8          = 1 << 1;  // scene has no frames
const SCENE_FLAG_HAS_FOCUS: u8      = 1 << 2;  // scene contains focused surface
const SCENE_FLAG_HAS_MINIMIZED: u8  = 1 << 3;  // scene has at least one minimized frame
const SCENE_FLAG_HAS_ZOOMED: u8     = 1 << 4;  // scene has at least one zoomed frame

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
    /// Focused frame_id in this scene, or 0 if none.
    focused_frame_id: u32,
    /// Number of valid entries in frame_ids[].
    frame_count: u8,
    /// Fixed-size array of frame IDs present in this scene.
    frame_ids: [u32; ATLAS_MAX_FRAMES_PER_SCENE],
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
static mut FOCUSED_SURFACE_ID: u64 = SURFACE_ID_APP;
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
        focused_frame_id: 0,
        frame_count: 0,
        frame_ids: [0u32; ATLAS_MAX_FRAMES_PER_SCENE],
    }; ATLAS_MAX_SCENES],
};
/// Atlas mode enabled: when true, the shell is in overview mode (no rendering yet in V1).
/// Toggled by F10 (ToggleAtlas). State-only — no visual behavior changes in V1.
static mut ATLAS_MODE_ENABLED: bool = false;
/// Bounded tombstone list for recently-closed surface IDs.
/// Prevents immediate reuse of freed IDs. Circular insertion.
static mut TOMBSTONES: [u64; 8] = [0; 8];
static mut TOMBSTONE_NEXT: usize = 0;
static mut TOMBSTONE_COUNT: usize = 0;
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
static mut SURFACE_200_H: u32 = 150;
// Quil surface 201 position tracking
static mut SURFACE_201_X: i32 = 100;
static mut SURFACE_201_Y: i32 = 100;
static mut SURFACE_201_W: u32 = 640;
static mut SURFACE_201_H: u32 = 480;

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
        // Linen surface 200 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_LINEN, SURFACE_200_X as u64, SURFACE_200_Y as u64);
        // Quil surface 201 position update
        pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_QUIL, SURFACE_201_X as u64, SURFACE_201_Y as u64);
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
        SURFACE_ID_QUIL   => Some((SURFACE_201_X, SURFACE_201_Y, SURFACE_201_W, SURFACE_201_H)),
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
            SURFACE_ID_QUIL   => (SURFACE_201_X, SURFACE_201_Y, SURFACE_201_W, SURFACE_201_H),
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

/// Record a surface ID as permanently closed. Prevents immediate reuse
/// via tombstone checks in focus/drag/hover paths.
/// Circular buffer: oldest entry dropped when full.
unsafe fn tombstone_surface(sid: u64) {
    let idx = TOMBSTONE_NEXT;
    TOMBSTONES[idx] = sid;
    TOMBSTONE_NEXT = (idx + 1) % TOMBSTONES.len();
    if TOMBSTONE_COUNT < TOMBSTONES.len() {
        TOMBSTONE_COUNT += 1;
    }
}

/// Returns true if `sid` is in the tombstone set (recently closed, must
/// not be focused, dragged, hovered, or restored as live).
unsafe fn is_tombstoned(sid: u64) -> bool {
    for i in 0..TOMBSTONE_COUNT {
        if TOMBSTONES[i] == sid {
            return true;
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
    if focused != 0 && !surface_is_alive(focused) {
        serial_println!("[shell.surface.focus.clear.dead] id={}", focused);
        let z_order = [SURFACE_ID_QUIL, SURFACE_ID_LINEN, SURFACE_ID_TEST4,
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
    if sid == SURFACE_ID_LINEN || sid == SURFACE_ID_QUIL {
        return false;
    }
    true // panels/cursor always visible
}

/// If the focused surface belongs to a frame in a non-active scene,
/// clear focus to a surface in the active scene.
unsafe fn clear_focus_if_wrong_scene() {
    let focused = FOCUSED_SURFACE_ID;
    if focused != 0 && !surface_in_active_scene(focused) {
        serial_println!("[shell.scene.focus.clear.wrong-scene] id={}", focused);
        // Try to focus the first alive surface in the active scene.
        let mut found = false;
        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id != ACTIVE_SCENE_IDX { continue; }
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    if surface_is_alive(tab.surface_id) && !is_tombstoned(tab.surface_id) {
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

/// Hide surfaces belonging to non-active scenes, show surfaces belonging
/// to the active scene. Called after ACTIVE_SCENE_IDX changes.
unsafe fn sync_scene_visibility() {
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
        }; ATLAS_MAX_SCENES],
    };

    // Derive focused frame for active scene.
    let active_focused_frame = selected_frame_id().unwrap_or(0);

    for scene_idx in 0..ATLAS_MAX_SCENES {
        let sd = &mut snapshot.scenes[scene_idx];
        sd.scene_id = scene_idx as u32;
        sd.label = atlas_default_label(scene_idx as u32);

        let mut frame_count: u8 = 0;
        let mut has_minimized = false;
        let mut has_zoomed = false;

        for f in FRAMES.iter() {
            if let Some(frame) = f {
                if frame.scene_id as usize != scene_idx { continue; }
                if frame_count >= ATLAS_MAX_FRAMES_PER_SCENE as u8 { break; }
                sd.frame_ids[frame_count as usize] = frame.frame_id;
                frame_count += 1;
                if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 { has_minimized = true; }
                if (frame.flags & FRAME_FLAG_ZOOMED) != 0 { has_zoomed = true; }
            }
        }

        sd.frame_count = frame_count;
        if frame_count == 0 {
            sd.flags |= SCENE_FLAG_EMPTY;
        }
        if has_minimized { sd.flags |= SCENE_FLAG_HAS_MINIMIZED; }
        if has_zoomed { sd.flags |= SCENE_FLAG_HAS_ZOOMED; }

        // Focus: only the active scene has a tracked focused frame.
        if scene_idx == ACTIVE_SCENE_IDX as usize {
            sd.flags |= SCENE_FLAG_ACTIVE;
            if active_focused_frame != 0 {
                sd.focused_frame_id = active_focused_frame;
                sd.flags |= SCENE_FLAG_HAS_FOCUS;
            }
        }
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
    ATLAS_MODE_ENABLED = !ATLAS_MODE_ENABLED;
    // Capture fresh snapshot on toggle.
    atlas_capture_snapshot();
    if ATLAS_MODE_ENABLED {
        // Entering Atlas: clear stale hover/drag to prevent interaction
        // state from a previous mode bleeding into Atlas awareness.
        clear_hover_if_wrong_scene();
        clear_drag_if_dead();
        static mut ATLAS_ENTER_BUDGET: u32 = 4;
        let b = &mut ATLAS_ENTER_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.atlas.enter]"); }
    } else {
        static mut ATLAS_EXIT_BUDGET: u32 = 4;
        let b = &mut ATLAS_EXIT_BUDGET;
        if *b > 0 { *b -= 1; serial_println!("[shell.atlas.exit]"); }
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
    tile_visible_frames();
    snap_capture_layout();
    // Capture Atlas snapshot after scene switch.
    atlas_capture_snapshot();
    static mut SCENE_SWITCH_SHORTCUT_BUDGET: u32 = 4;
    let b = &mut SCENE_SWITCH_SHORTCUT_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.scene.shortcut.switch] from={} to={}", prev, ACTIVE_SCENE_IDX); }
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
        tile_visible_frames();
        try_set_focus(sid);
    }

    // Focus Linen's surface.
    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
    }

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
        tile_visible_frames();
        try_set_focus(sid);
    }

    if let Some(sid) = active_surface_for_frame(fid) {
        try_set_focus(sid);
    }

    // Ensure Quil placeholder fill rect is set on every open (covers the
    // restore-from-minimized path where tile_visible_frames() is not called).
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0,
        (QUIL_PLACEHOLDER_COLOR as u64) << 32 | ((SURFACE_201_H as u64) << 16) | SURFACE_201_W as u64);

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
                    if *b > 0 { *b -= 1; serial_println!("[shell.quil.toggle.minimize] frame={}", QUIL_FRAME_ID); }
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
    match surface_id {
        SURFACE_ID_APP    => SURFACE_100_ALIVE = false,
        SURFACE_ID_STATIC => SURFACE_101_ALIVE = false,
        SURFACE_ID_TEST3  => SURFACE_102_ALIVE = false,
        SURFACE_ID_TEST4  => SURFACE_103_ALIVE = false,
        _ => return false, // unknown or non-closeable surface
    }
    tombstone_surface(surface_id);
    pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
    // Focus fallback: if the closed surface was focused, clear_focus_if_dead
    // will auto-switch to the next alive surface in z-order.
    clear_focus_if_dead();
    // Clear drag if the closed surface was being dragged (surface is now dead).
    clear_drag_if_dead();
    // Clear hover if the closed surface's frame is no longer valid.
    clear_hover_if_wrong_scene();
    // Re-tile remaining visible frames.
    tile_visible_frames();
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
                    if !surface_is_alive(tab.surface_id) { return false; }
                    if is_tombstoned(tab.surface_id) { return false; }
                }
                return true;
            }
        }
    }
    false
}

/// If the hovered frame no longer accepts input (wrong scene, minimized, etc.),
/// clear hover state to avoid stale highlights. Call after scene switch or minimize.
unsafe fn clear_hover_if_wrong_scene() {
    if HOVERED_FRAME_ID != 0 && !frame_accepts_input(HOVERED_FRAME_ID) {
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
    // Hide surface on display.
    pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
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
            serial_println!("[shell.frame.minimize] frame={} surface={}", frame_id, surface_id);
        }
    }
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
    // Re-tile to include the restored frame.
    tile_visible_frames();
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
        SURFACE_ID_LINEN => {
            SURFACE_200_X = x; SURFACE_200_Y = y;
            SURFACE_200_W = w; SURFACE_200_H = h;
        }
        SURFACE_ID_QUIL => {
            SURFACE_201_X = x; SURFACE_201_Y = y;
            SURFACE_201_W = w; SURFACE_201_H = h;
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
    snap_capture_layout();
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
    let chrome_flags: u64 = if frame_has_top_bar(frame_id) { 1 } else { 0 };
    // Pack chrome_flags into arg2 bit 8 (low 8 bits = active_tab).
    let arg2 = (active_tab as u64) | (chrome_flags << 8);
    pdx_call(SLOT_DISPLAY, OP_SURFACE_TAB_INFO, surface_id, tab_count as u64, arg2);
    unsafe {
        static mut SHELL_TAB_INFO_SEND_BUDGET: u32 = 8;
        let b = &mut SHELL_TAB_INFO_SEND_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.tab.info.send] frame={} surface={} tabs={} active={} chrome={}",
                frame_id, surface_id, tab_count, active_tab, chrome_flags);
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
    if is_tombstoned(sid) {
        serial_println!("[shell.focus.reject.tombstoned] id={}", sid);
        return false;
    }
    // Guard: reject focus for surfaces belonging to frames not in the active scene.
    // Panels, cursor, and other non-frame surfaces are always eligible.
    if !surface_in_active_scene(sid) {
        serial_println!("[shell.focus.reject.wrong-scene] id={}", sid);
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
    let z_order = [SURFACE_ID_QUIL, SURFACE_ID_LINEN, SURFACE_ID_TEST4,
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
        try_transition(InteractionState::PanelActive { panel: PanelKind::Settings });
    } else {
        serial_println!("[shell.scene.settings.panel.close.start] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0);
        serial_println!("[shell.scene.settings.panel.close.ok] id={:#x}", SURFACE_ID_SCENE_SETTINGS);
        SCENE_SETTINGS_ACTIVE = false;
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
                    tile_visible_frames();
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
    tile_visible_frames();

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

    // Send initial tab metadata for frame 1 (surface 100: 1 tab, active tab 0)
    unsafe { send_frame_tab_info(1); }
    serial_println!("[silk-shell] Boot tab info sent to sexdisplay");

    // Push default scene render tokens to sexdisplay (establishes DISPLAY_TOKENS baseline)
    unsafe { send_scene_render_tokens(); }
    serial_println!("[silk-shell] Boot scene render tokens sent to sexdisplay");

    // Fire GET to sexstore for persisted scene appearance settings.
    // Reply arrives asynchronously in main loop via type_id == 0x1.
    unsafe { boot_load_scene_settings(); }

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
                            // ── Scene Settings panel key intercept ──────────────
                            // When panel visible, route 1/2/3/Esc to panel commands.
                            // [shell.scene.settings.panel.key] budget 16.
                            // F7 (0x41) falls through to normal dispatch unchanged.
                            if SCENE_SETTINGS_ACTIVE {
                                static mut PANEL_KEY_BUDGET: u32 = 16;
                                let b = &mut PANEL_KEY_BUDGET;
                                match scancode {
                                    0x01 => { // Esc → close panel
                                        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0);
                                        SCENE_SETTINGS_ACTIVE = false;
                                        try_transition(InteractionState::Idle);
                                        mutated = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=close"); }
                                    }
                                    0x02 => { // Key 1 → cycle preset (like F5)
                                        cycle_scene_render_token_preset();
                                        mutated = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=preset"); }
                                    }
                                    0x03 => { // Key 2 → cycle tint (like F6)
                                        cycle_custom_tint();
                                        mutated = true;
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=tint"); }
                                    }
                                    0x04 => { // Key 3 → toggle top bar (like F4)
                                        if toggle_top_bar_for_active_frame() {
                                            mutated = true;
                                        }
                                        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.panel.key] cmd=topbar"); }
                                    }
                                    // All other scancodes (including F7=0x41) fall through to normal dispatch
                                    _ => {}
                                }
                            }
                            // ── Normal make-code dispatch via policy lookup ──────
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
                                            tombstone_surface(target);
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

                                    SurfaceAction::ToggleAtlas => {
                                        unsafe { atlas_toggle(); }
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
                                        tile_visible_frames();
                                        snap_capture_layout();
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
            OP_SCENE_SETTINGS_CMD => {
                // Scene Settings protocol command.
                // Dispatch is synchronous, non-blocking, no reply wait.
                unsafe {
                    handle_scene_settings_cmd(msg.arg0, msg.arg1, msg.arg2);
                }
                pdx_reply(0);
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
