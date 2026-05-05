# I2: Collar Placeholder Surface

**Status:** Handoff (code + doc)
**Commit:** _(to be committed)_
**Build:** ISO produced

## 1. Purpose

Attach a Collar placeholder surface through the proven Scene/Frame/Tab lifecycle
path, mirroring the D1 (Linen), E1 (Quil), and I1 (Mesh) pattern. Collar is a
placeholder surface with no real grants, secrets, prompts, or authority enforcement.

### What Collar IS (I2)
- Placeholder surface (Surface ID 203, Frame ID 5)
- Toggle via Insert key
- Lifecycle-registered as Visible
- Attached to active Scene as Frame/Tab through B1 model
- Can be closed/minimized/restored by existing A5/A6/B3 paths

### What Collar IS NOT (I2)
- No grant/revoke enforcement
- No secret/key storage
- No prompt/security UI
- No Collar authority control yet
- No cross-PD pointers
- No renderer changes

## 2. Changes to `servers/silk-shell/src/main.rs`

### Constants and arrays
- `SURFACE_ID_COLLAR: u64 = 203`
- `COLLAR_FRAME_ID: u32 = 5`
- `MAX_FRAMES`: 5 → 6
- `ATLAS_MAX_FRAMES_PER_SCENE`: 5 → 6
- `APP_SURFACES`: 3 → 4 (added Collar entry)
- Boot geometry: (300, 100, 640, 480)
- Placeholder color: 0x00204038 (muted teal/authority)
- `SURFACE_203_X/Y/W/H` static vars

### SurfaceAction and scancode
- `ToggleCollar` variant added
- Insert key (0x52) mapped to `ToggleCollar`

### Lifecycle
- `lifecycle_register(SURFACE_ID_COLLAR, LifecycleState::Visible)`

### Wire-up points
- `surface_is_alive`, `surface_in_active_scene`, `get_surface_bounds`
- `point_in_surface`, `update_local_geometry`
- Both tile functions (geometry + placeholder fill rect)
- `OP_SURFACE_UPDATE` position update
- Both `z_order` arrays
- Focus description in `try_set_focus`

### Helper functions
- `ensure_collar_frame()`
- `open_collar_in_active_scene()`
- `toggle_collar()`
- `focus_or_open_collar()`
- `collar_frame_id()`

### Keyboard handler
- `SurfaceAction::ToggleCollar` → `toggle_collar()`

## 3. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[collar.placeholder.attach.frame]` | ensure_collar_frame() | Frame created |
| `[collar.placeholder.attach.tab]` | ensure_collar_frame() | Tab attached |
| `[collar.placeholder.reject.duplicate]` | open_collar_in_active_scene() | Already open |
| `[collar.placeholder.open]` | open_collar_in_active_scene() | Successfully opened |
| `[collar.placeholder.focus]` | open_collar_in_active_scene() | Focused |

## 4. Invariants

1. Collar surface follows existing lifecycle FSM
2. No duplicate Collar frame in same active scene
3. Focus goes through try_set_focus() lifecycle + scene guards
4. Close/minimize/tombstone use existing proven paths
5. No real grants, secrets, or authority enforcement in I2
6. Toggle via Insert key

## 5. STOP FIRST Triggers

- Grant/revoke enforcement before F2 implementation
- Secret/key storage
- Prompt/security UI
- Collar PDX ops before F2
- Renderer changes
- Kernel/sex-pdx ABI edits
- Cross-PD raw pointers
