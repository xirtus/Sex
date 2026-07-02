# D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1

**Status:** Complete — audit/spec only, no code changes.
**Build:** ISO produced, no errors.

---

## Summary

Audit of the current silk-shell UI element inventory, keyboard dispatch,
focus model, and interaction state. Defines the minimal accessibility
semantics model for D2 (semantic node emitter).

**Verdict:** All semantic elements are shell-owned and deterministically
derivable from the existing model state. No STOP FIRST conditions triggered.
Ready for D2.

---

## Files Inspected

| File | Lines | Role |
|------|-------|------|
| `servers/silk-shell/src/main.rs` | ~6600 | Shell policy owner, all UI elements |
| `crates/silkbar-model/src/lib.rs` | ~210 | SilkBar module/chip model |
| `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` | 280 | Track D plan |
| `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` | — | Lifecycle proof |
| `docs/handoff/QUIL_STUB_CONSOLIDATION_AUDIT_V1.md` | — | Quil stub audit |
| `docs/handoff/ATLAS_FRAME_PREVIEW_REFRESH_V1.md` | — | Atlas lifecycle filtering |
| `docs/handoff/ATLAS_SCENE_SETTINGS_UI_V1.md` | — | Atlas keyboard controls |

---

## 1. Shell Semantic Elements Inventory

### Element: SilkBar (top bar)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell (uses silkbar-model crate for layout/model) |
| **Model source** | `silkbar_model::SILK_BAR` (static) + `DEFAULT_SILK_BAR` |
| **Sub-elements** | Workspaces (3), Launcher button, Status chips (3: net/wifi/battery), Clock, Bell |
| **Actions** | `OpenLauncher`, `SwitchWorkspace(u8)`, `ToggleModule(Module)`, `OpenClock`, `OpenBell` |
| **Hit test** | `handle_silkbar_click()` → `hit_test_action()` |
| **Keyboard accessible?** | ❌ **No keyboard path to trigger SilkBar actions.** SilkBar actions are pointer-only (click dispatch). |
| **Focus relation** | SilkBar has no focus state — it's a static chrome element rendered by sexdisplay via model share. |

### Element: Atlas (scene overview)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Toggle key** | F10 (`0x44`, scancode mapping) |
| **Keyboard navigation** | Arrow keys (left/right/up/down), Enter (select), Escape (cancel), A (accent cycle), P (pin toggle) |
| **Selected scene** | `ATLAS_SELECTED_SCENE` (static u8) |
| **Visible/hidden** | `ATLAS_MODE_ENABLED` bool toggle |
| **Focus relation** | Atlas replaces normal focus; on exit, focus returns to previous surface |
| **Scope** | All scenes (1..5), each with flag summary (empty/minimized/zoomed) |
| **Missing keyboard** | No keyboard path to navigate within Atlas cards (card selection is arrow-only but cards are not individually named/narrated). |

### Element: Scenes (workspaces)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Switch keys** | `SurfaceAction::Focus100` (`0x02`), `Focus101` (`0x03`), `Focus102` (`0x04`), `Focus103` (`0x05`), `Focus200` (`0x06`) |
| **Active scene** | `ACTIVE_SCENE_IDX` (static u8, 0..4) |
| **Count** | `ATLAS_MAX_SCENES` = 5 |
| **Scene flags** | `SCENE_FLAG_EMPTY`, `SCENE_FLAG_HAS_MINIMIZED`, `SCENE_FLAG_HAS_ZOOMED` |
| **Scene settings** | `accent` (u8, 0..5), `pinned` (bool), `label` (`[u8; ATLAS_LABEL_LEN]`) |
| **Keyboard accessible?** | ✅ Key bindings for direct scene switch exist (1-5 number keys). Alt+[1-9] not implemented (key bindings are single-key without modifier). |
| **Missing** | Scene name narration, scene state narration (empty/minimized/zoomed). |

### Element: Frames (app windows)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Storage** | `FRAMES` static array (`[Option<Frame>; MAX_FRAMES]`, 32 slots) |
| **Frame ID** | u32, allocated by `frame_allocate()` |
| **Fields** | `frame_id`, `scene_id`, `surface_id`, `flags` (minimized/zoomed), `lifecycle_state`, `boot_x/y/w/h`, `tile_x/y/w/h`, `chrome_flags` |
| **Lifecycle states** | Allocated, Mapped, Visible, Hidden (never set), Minimized, Closing, Tombstoned, Destroyed |
| **Keyboard actions** | FocusToggle, Focus100-200, DestroyFocused, RestoreMinimized, SnapLeft/Right/Maximize/Center etc. |
| **Focus relation** | Each frame can have a focused tab surface. `FOCUSED_SURFACE_ID` tracks current focus. FocusRef with generation protects stale references. |
| **Missing keyboard** | No Tab/Shift+Tab cycling through frames. No keyboard path to cycle frames by order (only by direct surface ID targeting). |

### Element: Frame Tabs

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Storage** | Per-frame tab tracking (surface_id per frame, tab_count) |
| **Tab switching** | Via frame lights interaction (click on tab area in frame chrome) |
| **Keyboard accessible?** | ❌ **No dedicated keyboard tab cycling.** `SurfaceAction` has no "cycle tab forward/backward" action. Tab switching is pointer-only. |
| **Focus relation** | Each tab is a surface within a frame. Focus targets the active tab surface. |

### Element: Frame Lights (close/minimize/zoom)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Storage** | Part of frame chrome (reserved geometry in frame model, not independently enumerated in V1) |
| **Actions** | Close (0x3C DestroyFocused), Minimize (0x49 RestoreMinimized), Zoom (SurfaceAction::Maximize) |
| **Keyboard accessible?** | ✅ Partially. Close/DestroyFocused at `0x3C`, RestoreMinimized at `0x49`, Maximize at `0x32`. **No keyboard path to target a specific frame's lights** — actions target the focused surface. |
| **Individual light targeting** | ❌ Lights are clickable per-frame via pointer hit-test on `FrameChrome` region, but no keyboard path selects "close frame 3's light" vs "close frame 1's light". |

### Element: Quil Stub Surface

| Property | Value |
|----------|-------|
| **Owner** | silk-shell (surface lifecycle + placeholder) |
| **Toggle key** | F9 (`0x43`, scancode mapping) |
| **Surface ID** | `SURFACE_ID_QUIL` = 201 |
| **Frame ID** | `QUIL_FRAME_ID` = 3 |
| **Keyboard accessible?** | ✅ F9 toggles Quil open/close. **No alternative key binding** (only F9). |
| **Missing** | Quil surface has no semantic label or role accessible to a11y model. |

### Element: Frame Chrome (rim, resize handles)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Hit test target** | `HitTarget::FrameChrome { frame_id, kind }` |
| **Kind values** | Undefined in V1 (placeholder) |
| **Keyboard accessible?** | ❌ Resize/move actions exist (`ShrinkWidth`, `GrowWidth`, `MoveLeft/Right/Up/Down`, `SnapLeft/Right`) but they apply to the focused surface — there is no keyboard path to select a resize handle on a non-focused frame. |

### Element: SilkBar Chips (net/wifi/battery)

| Property | Value |
|----------|-------|
| **Owner** | silkbar-model (model) + silk-shell (click dispatch) |
| **Kind** | `ChipKind::Net`, `ChipKind::Wifi`, `ChipKind::Battery`, `ChipKind::Clock` |
| **Visible** | `ChipState.visible` bool |
| **Keyboard accessible?** | ❌ **No keyboard path to interact with chips.** Chips are pointer-only. |

### Element: SilkBar Panels (launcher, status, clock, bell)

| Property | Value |
|----------|-------|
| **Owner** | silk-shell |
| **Panel kinds** | `PanelKind::Launcher`, `PanelKind::Status`, `PanelKind::Clock`, `PanelKind::Bell`, `PanelKind::Settings` |
| **Toggle** | `try_transition(InteractionState::PanelActive { panel })` |
| **Keyboard accessible?** | ❌ **No keyboard path to open/close panels.** Panel toggles are pointer-only. F7 toggles the scene settings panel as a SurfaceAction but is the only panel with keyboard access. |

---

## 2. Existing Keyboard Access Inventory

### Direct Key Bindings (scancode_to_action)

| Scancode | Key | SurfaceAction | Has alternative? |
|----------|-----|---------------|------------------|
| `0x02` | 1 | Focus100 (Scene 0) | ✅ Single-key |
| `0x03` | 2 | Focus101 (Scene 1) | ✅ Single-key |
| `0x04` | 3 | Focus102 (Scene 2) | ✅ Single-key |
| `0x05` | 4 | Focus103 (Scene 3) | ✅ Single-key |
| `0x06` | 5 | Focus200 (Scene 4) | ✅ Single-key |
| `0x0F` | Tab | FocusToggle | ⚠️ **Cycles surfaces, not semantic elements.** No forward/backward distinction. |
| `0x3C` | F2 | DestroyFocused | ✅ |
| `0x3D` | F3 | RecreateFocused | ✅ |
| `0x3E` | F4 | ToggleTopBar | ✅ |
| `0x3F` | F5 | CycleRenderTokenPreset | ✅ |
| `0x40` | F6 | CycleCustomTint | ✅ |
| `0x41` | F7 | ToggleSceneSettingsPanel | ✅ |
| `0x42` | F8 | ToggleLinen | ✅ |
| `0x43` | F9 | ToggleQuil | ✅ |
| `0x44` | F10 | ToggleAtlas | ✅ |
| `0x49` | PageUp | RestoreMinimized | ✅ |
| `0x13` | R | ResetAll | ✅ |
| Various | Arrow/WASD | Move/Snap/Resize | ✅ |

### Atlas-Internal Keys

| Scancode | Key | Action |
|----------|-----|--------|
| `0x4B` | Left arrow | Navigate left |
| `0x4D` | Right arrow | Navigate right |
| `0x48` | Up arrow | Navigate up |
| `0x50` | Down arrow | Navigate down |
| `0x1C` | Enter | Confirm selection |
| `0x01` | Escape | Cancel, exit Atlas |
| `0x1E` | A | Cycle accent token |
| `0x19` | P | Toggle pinned flag |

### Missing Keyboard Alternatives

| Action | Current input | Keyboard alternative needed for D3 |
|--------|---------------|-------------------------------------|
| Tab cycle forward | `0x0F` (Tab) — cycles surface IDs | ✅ Exists but cycles surface IDs, not semantic elements. Needs re-scoping for D3. |
| Tab cycle backward | None | ❌ No Shift+Tab equivalent |
| SilkBar workspace switch | Pointer click only | ❌ Missing |
| SilkBar launcher open | Pointer click only | ❌ Missing |
| SilkBar chip interact | Pointer click only | ❌ Missing |
| SilkBar clock/bell open | Pointer click only | ❌ Missing |
| Frame tab cycling | Pointer click only | ❌ Missing |
| Frame light targeting (non-focused) | Pointer click only | ❌ Missing |
| Panel open (launcher/status/clock/bell) | Pointer click only | ❌ Missing (F7 exists for settings panel only) |
| Atlas card selection | Arrow keys | ✅ Exists but no Tab/Shift+Tab to enter/exit Atlas card grid |
| Scene name narration | None | ❌ Missing |

---

## 3. Minimal Semantic Node Model for D2

```rust
/// Unique identifier for a semantic node within the shell.
/// Flat ID space — no hierarchy in V1.
/// 0 is reserved (invalid). u32 matches surface_id/frame_id range.
type NodeId = u32;

/// SemanticRole categorizes a shell chrome element.
/// V1: shell chrome only. App content roles reserved for future.
#[repr(u8)]
enum SemanticRole {
    /// SilkBar top bar as a container
    SilkBar = 1,
    /// Workspace toggle chip in SilkBar
    SceneChip,
    /// SilkBar launcher button
    LauncherButton,
    /// Status chip (net/wifi/battery)
    StatusChip,
    /// Clock display area
    ClockDisplay,
    /// Bell notification area
    BellIndicator,
    /// App/content frame
    Frame,
    /// Tab within a frame
    Tab,
    /// Frame light: close/minimize/zoom
    FrameLight,
    /// Frame close light
    FrameLightClose,
    /// Frame minimize light
    FrameLightMinimize,
    /// Frame zoom light
    FrameLightZoom,
    /// Atlas card (scene preview in overview mode)
    AtlasCard,
    /// Scene settings panel
    SettingsPanel,
    /// Panel overlay (launcher, status, clock, bell)
    Panel,
    /// App surface placeholder (Quil/Linen stub)
    AppPlaceholder,
}

/// Bitmask of node state flags.
/// u16 fits in register, no heap.
type NodeStateFlags = u16;
const NODE_FOCUSED:    NodeStateFlags = 1 << 0;
const NODE_SELECTED:   NodeStateFlags = 1 << 1;
const NODE_VISIBLE:    NodeStateFlags = 1 << 2;
const NODE_HIDDEN:     NodeStateFlags = 1 << 3;
const NODE_MINIMIZED:  NodeStateFlags = 1 << 4;
const NODE_ZOOMED:     NodeStateFlags = 1 << 5;
const NODE_DISABLED:   NodeStateFlags = 1 << 6;
// Bits 7-15 reserved.

/// Bitmask of available actions on a node.
/// u16 fits in register, no heap.
type NodeActionFlags = u16;
const ACT_FOCUS:        NodeActionFlags = 1 << 0;
const ACT_ACTIVATE:     NodeActionFlags = 1 << 1;
const ACT_CLOSE:        NodeActionFlags = 1 << 2;
const ACT_MINIMIZE:     NodeActionFlags = 1 << 3;
const ACT_RESTORE:      NodeActionFlags = 1 << 4;
const ACT_ZOOM:         NodeActionFlags = 1 << 5;
const ACT_UNZOOM:       NodeActionFlags = 1 << 6;
const ACT_SWITCH_SCENE: NodeActionFlags = 1 << 7;
const ACT_CYCLE_ACCENT: NodeActionFlags = 1 << 8;
const ACT_TOGGLE_PIN:   NodeActionFlags = 1 << 9;
// Bits 10-15 reserved.

/// A semantic node. Fixed-size, no heap, no String.
/// Total: 8 + 2 + 2 + 2 + 4 + 4 = 22 bytes.
/// Label is stored as fixed byte array (no heap).
/// Scene/frame/surface target IDs are stored as u32 (0 = N/A).
#[repr(C)]
struct SemanticNode {
    /// Unique node ID within the shell session
    node_id: NodeId,
    /// Semantic role of this element
    role: SemanticRole,
    /// Current state flags
    state: NodeStateFlags,
    /// Available actions
    actions: NodeActionFlags,
    /// Target surface_id (0 = N/A)
    target_surface: u32,
    /// Target frame_id (0 = N/A)
    target_frame: u32,
    /// Target scene_id (0xFF = N/A)
    target_scene: u8,
    /// Pinned flag (valid only for scenes)
    pinned: u8,
    /// Acride token (valid only for scenes)
    accent: u8,
    /// Label bytes (null-terminated, no heap)
    /// Max 32 bytes for V1 shell chrome labels.
    label: [u8; 32],
}
```

**Constraint: No heap, no `String`, no `Vec`.** The SemanticNode array is
a fixed-size `[Option<SemanticNode>; MAX_SEMANTIC_NODES]` in the shell.
V1 flat list (not tree). Tree hierarchy reserved for future app content.

---

## 4. Roles with Label Sources and Target IDs

| Role | Label source | Target ID | State sources |
|------|-------------|-----------|---------------|
| `SceneChip` | `scene_label_token(scene_id)` | scene_id | active scene flag |
| `Frame` | "Frame N" (derived from frame_id) | frame_id, surface_id | focused, minimized, zoomed, tombstoned |
| `Tab` | App name (future) or "Tab N" | surface_id, frame_id | focused, visible, hidden |
| `FrameLightClose` | "Close" | frame_id | enabled/disabled (based on lifecycle) |
| `FrameLightMinimize` | "Minimize" | frame_id | enabled/disabled |
| `FrameLightZoom` | "Zoom" | frame_id | disabled when zoomed (toggle) |
| `AtlasCard` | Scene label | scene_id | selected, empty flag |
| `SilkBar` | "Top Bar" | 0 | always visible |
| `AppPlaceholder` | Surface name (Linen/Quil) | surface_id | visible when open |

---

## 5. States with Derivation Rules

| State | Derivation |
|-------|-----------|
| `focused` | `surface_id == FOCUSED_SURFACE_ID` (after focus guard passes) |
| `selected` | `scene_id == ATLAS_SELECTED_SCENE` (Atlas mode only) |
| `visible` | `surface_is_alive(sid)` && `surface_in_active_scene(sid)` |
| `hidden` | Scene inactive but surface alive |
| `minimized` | `frame.flags & FRAME_FLAG_MINIMIZED` |
| `zoomed` | `frame_is_zoomed(frame_id)` |
| `disabled` | Surface tombstoned/closing/destroyed, or action not available for lifecycle |

### States explicitly excluded from V1 semantic model

| State | Reason |
|-------|--------|
| `tombstoned` | Dead surface — excluded by `surface_is_alive()` check before node creation |
| `closing` | Surface mid-close — excluded by `surface_is_alive()` returning false |
| `destroyed` | Dead — excluded before node creation |
| `allocated` | Not a user-facing state — excluded |

---

## 6. Actions with Guard Rules

| Action | Target | Guard |
|--------|--------|-------|
| `focus` | Any surface | `surface_is_alive()`, `!is_tombstoned()`, `surface_is_lifecycle_focusable()` |
| `activate` | Any surface | Same as focus + Enter dispatch |
| `close` | Frame surface | `surface_is_alive()`, not already closing/destroyed |
| `minimize` | Frame surface | `(frame.flags & FRAME_FLAG_MINIMIZED) == 0` |
| `restore` | Frame surface | `(frame.flags & FRAME_FLAG_MINIMIZED) != 0`, `surface_is_alive()` |
| `zoom` | Frame surface | `surface_is_alive()`, not already zoomed |
| `unzoom` | Frame surface | `frame_is_zoomed(frame_id)` |
| `switch_scene` | Scene | `scene_id != ACTIVE_SCENE_IDX` |
| `cycle_accent` | Scene | Atlas open, `validate_scene_id(sel)` |
| `toggle_pin` | Scene | Atlas open, `validate_scene_id(sel)` |

---

## 7. Proof Markers Needed for D2

| Marker | Budget | When | What it proves |
|--------|--------|------|----------------|
| `[access.node.emit]` | 8 | On SemanticNode construction | Node is valid and included in the tree |
| `[access.node.skip_dead]` | 8 | When dead surface excluded from tree | Dead surfaces never become semantic nodes |
| `[access.action.allow]` | 16 | On successful action dispatch | Action was permitted by lifecycle/focus guards |
| `[access.action.reject]` | 8 | On denied action dispatch | Action was blocked (reason logged) |
| `[access.keyboard.alt]` | 8 | On keyboard alternative for gesture | Input alternative reached valid target |
| `[access.focus.describe]` | 32 | On focus change with description | Focused element has valid role+label+state |

The `[access.focus.describe]` marker is the workhorse of D2 — it fires on
every focus change and emits role+label+target_id+state flags. This is the
V1 "narration" equivalent (event-log-only, no speech).

---

## 8. STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Requires app memory scraping | ✅ Not needed — all semantics are shell-model-derived |
| Requires sexdisplay semantics ownership | ✅ Not needed — shell owns all semantic elements |
| Requires speech/audio engine | ✅ Not needed — V1 is event-log-only (proof markers) |
| Requires POSIX/dbus/AT-SPI | ✅ Not needed — no external accessibility protocol |
| Requires kernel/ABI change | ✅ Not needed — no PDX ABI edits |
| Requires heap/String/broad refactor | ✅ Not needed — SemanticNode is fixed-size, `[u8; 32]` label |
| Requires persistent storage | ✅ Not needed — V1 semantics are runtime-only |
| Requires new caps or PD spawn | ✅ Not needed — all within silk-shell |
| Requires sexdisplay protocol change | ✅ Not needed — no framebuffer changes |
| Requires lifecycle/FSM changes | ✅ Not needed — semantic tree reads existing model state |

**No STOP FIRST conditions triggered.**

---

## 9. Ownership and Topology

```
SemanticNode ownership:            silk-shell exclusively
SemanticTree construction:         silk-shell reads model state (FRAMES, SCENES,
                                   FocusRef, SilkBar layout, panel state)
Action dispatch:                   silk-shell through existing SurfaceAction dispatch
Narration (V1):                    proof markers only — [access.focus.describe]
SilkBar model source:              silkbar-model crate (read by shell)
Sexdisplay involvement:            NONE — no semantic inference from pixels
Quil/Linen involvement:            NONE — shell owns their placeholders
Keyboard dispatch:                 existing scancode_to_action() extended in D3
Focus validity:                    Track A A4 guards (unchanged)
```

The semantic model is entirely shell-side. No other PD needs awareness.

---

## 10. D2 Node Construction Strategy

For D2 (semantic node emitter), the shell will iterate:

1. **Frames loop**: For each live frame (skip dead/tombstoned/destroyed):
   - Emit `Frame` node with actions: focus, close, minimize, restore, zoom/unzoom
   - For each tab on frame: emit `Tab` node if tab has a surface
   - For each frame light: emit `FrameLight*` node with state (enabled/disabled)
   - Note: V1 frame lights are **not individually enumerated** in the model — they
     exist as clickable regions defined by the hit-test geometry (`FrameChrome`).
     D2 will derive them from frame state + chrome geometry constants.

2. **Scenes loop**: Emit `SceneChip` nodes (not rendered in V1 SilkBar as
   clickable chips, but present in the model for scene switching actions).

3. **SilkBar**: Emit `SilkBar` container node, with child action stubs for
   launcher, workspace switches, chips. V1 stubs are present but may not have
   keyboard dispatch yet (that's D3+).

4. **Atlas cards**: Emit `AtlasCard` nodes when Atlas is open. Each card has
   scene label + accent + pinned state. Actions: select, cycle_accent, toggle_pin.

5. **App placeholders**: Emit `AppPlaceholder` for Quil (and Linen) when
   their surfaces exist. Label: "Quil" / "Linen".

**No heap allocation.** Nodes are written into a fixed-size array
`[Option<SemanticNode>; 64]` (64 is enough for V1 chrome — ~32 frames + tabs
+ SilkBar + scenes + Atlas + placeholders).

---

## 11. Build Verification

```sh
$ ./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
ISO produced: sexos-v1.0.0.iso
Warnings: only pre-existing (unused import in sexstore, unnecessary unsafe blocks)
```

No code changes in this audit — build is for baseline confirmation.

---

## 12. Ready for D2

**Yes.** All semantic elements are identified. The node model is defined
(fixed-size, no heap). Label sources are deterministic from model state.
No STOP FIRST conditions block D2.

### D2 Implementation Scope

| Task | File | Change |
|------|------|--------|
| Add `SemanticRole` enum | `servers/silk-shell/src/main.rs` | New enum |
| Add `NodeStateFlags` / `NodeActionFlags` | `servers/silk-shell/src/main.rs` | New consts |
| Add `SemanticNode` struct | `servers/silk-shell/src/main.rs` | New struct |
| Add `semantic_emit_tree()` function | `servers/silk-shell/src/main.rs` | Iterates frames/scenes/silkbar/atlas |
| Add `[access.node.emit]` / `[access.node.skip_dead]` markers | `servers/silk-shell/src/main.rs` | Inside emit function |
| Add `[access.focus.describe]` marker | `servers/silk-shell/src/main.rs` | On focus change |
| Wire tree emit into focus change path | `servers/silk-shell/src/main.rs` | Call after try_set_focus() |

---

## References

- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — Track D plan
- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — lifecycle proof
- `docs/handoff/ATLAS_SCENE_SETTINGS_UI_V1.md` — Atlas keyboard controls
- `docs/handoff/ATLAS_FRAME_PREVIEW_REFRESH_V1.md` — Atlas lifecycle filtering
- `servers/silk-shell/src/main.rs` — all UI elements and dispatch
- `crates/silkbar-model/src/lib.rs` — SilkBar model definitions
