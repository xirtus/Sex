# B1: Scene/Frame/Tab Core Model

**Status:** Approved
**Commit:** `6627e0b`
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive model only)

## Purpose

Define shell-local Scene/Frame/Tab abstractions using existing safe opcode
primitives, proven lifecycle invariants from A3-A7, and the A7 opcode audit
result (0xEE = deactivate, not destroy). Atlas renderer, sexdisplay, and
kernel/PDX ABI are untouched.

## Changes to `servers/silk-shell/src/main.rs`

### Type-safe wrapper types (Change 1)
- `SceneId(u8)` — scene index, repr(transparent)
- `FrameId(u32)` — frame identifier, repr(transparent)
- `TabIndex(u8)` — tab index within frame, repr(transparent)
- Added after `struct ShellFrame`, before `static mut WINDOWS`

### Scene runtime state (Change 2)
- `struct Scene { flags: u8, label: [u8; ATLAS_LABEL_LEN] }` — after `SceneDescriptor`
- `static mut SCENES: [Scene; ATLAS_MAX_SCENES]` — after `ATLAS_SNAPSHOT`
  - Initialized with `SCENE_FLAG_EMPTY` (semantically true default)
- `scene_init_all()` — initializes SCENES from `atlas_default_label()` + `scene_update_flags()`
  - Proof marker: `[scene.core.init]`
- `scene_update_flags(scene_idx)` — derives flags from FRAMES frame state
  - Sets `SCENE_FLAG_MINIMIZED` / `SCENE_FLAG_ZOOMED` / `SCENE_FLAG_EMPTY`

### atlas_capture_snapshot() uses SCENES cache (Change 3)
- Refresh scene flags via `scene_update_flags()` before capture
- `sd.label = SCENES[scene_idx].label` (cached, not re-derived)
- `sd.flags = SCENES[scene_idx].flags` (cached, not re-derived)
- Old inline flag derivation removed

### switch_scene() updates (Change 4)
- `scene_update_flags(idx)` called after scene switch
- `[scene.switch]` proof marker

### clear_focus_if_wrong_scene() update (Change 5)
- `[scene.focus.reject.inactive]` proof marker when focused surface is in wrong scene

### frame_accepts_input() update (Change 6)
- `[tab.focus.reject.dead]` proof markers for dead + tombstoned checks

### Boot sequence proof markers (Change 7)
- `[frame.core.attach]` — boot frame 1 attached to scene 0
- `[tab.core.attach]` — boot tabs 0,1 with surface_ids

### Boot call sequence (Change 8)
- `scene_init_all()` added after `lifecycle_init_all()`, before `snap_capture_layout()`

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[scene.core.init]` | scene_init_all() | Boot SCENES initialization |
| `[scene.switch]` | switch_scene() | Scene transition |
| `[scene.focus.reject.inactive]` | clear_focus_if_wrong_scene() | Focused surface not in active scene |
| `[tab.focus.reject.dead]` | frame_accepts_input() | Dead/tombstoned tab surface |
| `[frame.core.attach]` | Boot init | Frame attached to scene |
| `[tab.core.attach]` | Boot init | Tab attached to frame |

## Invariants

1. Scene flags are derived from FRAMES state, never stale
2. SCENES array is initialized before any IPC/message loop (called in boot)
3. SceneDescriptor (sent to renderer via AtlasSnapshot) derives from SCENES + FRAMES
4. No Scene/FD/tab state is persisted across scene switches (V1: memory-only)
5. All frame operations go through existing FRAMES model unchanged

## Deferred

- B2: Active-scene focus + switch proof (formal FocusRef + generation)
- B3: Deterministic tiling (frame layout from scene geometry)
- B4: Tab strip/hover/frame-light behavior
- Atlas renderer (scene card rendering in overview mode)
- User-settable scene labels (currently "Scene N" defaults)

## Dependencies

- **Requires:** A3 (lifecycle FSM), A4 (focus guards), A6 (tombstone events),
  A7 (opcode audit)
- **Blocks:** B2 (active-scene focus), C1 (Atlas)
