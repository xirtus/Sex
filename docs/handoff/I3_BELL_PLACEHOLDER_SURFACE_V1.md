# I3: Bell Placeholder Surface

**Status:** Handoff (code + doc)
**Commit:** _(to be committed)_
**Build:** ISO produced

## 1. Purpose

Attach a Bell placeholder surface through the proven Scene/Frame/Tab lifecycle
path, mirroring D1 (Linen), E1 (Quil), I1 (Mesh), and I2 (Collar). Bell is a
placeholder surface with no real notifications, attention enforcement, or
event routing.

### What Bell IS (I3)
- Placeholder surface (Surface ID 204, Frame ID 6)
- Toggle via PageDown key
- Lifecycle-registered as Visible
- Attached to active Scene as Frame/Tab through B1 model
- Can be closed/minimized/restored by existing A5/A6/B3 paths

### What Bell IS NOT (I3)
- No real notification delivery
- No attention budget enforcement
- No capability-gated urgency
- No Collar authority control
- No cross-PD pointers
- No renderer changes

## 2. Changes to `servers/silk-shell/src/main.rs`

### Constants and arrays
- `SURFACE_ID_BELL_PLACEHOLDER: u64 = 204`
- `BELL_FRAME_ID: u32 = 6`
- `MAX_FRAMES`: 6 → 7
- `ATLAS_MAX_FRAMES_PER_SCENE`: 6 → 7
- `APP_SURFACES`: 4 → 5 (added Bell entry)
- Boot geometry: (400, 100, 640, 480)
- Placeholder color: 0x00402020 (attention red-orange)
- `SURFACE_204_X/Y/W/H` static vars

### SurfaceAction and scancode
- `ToggleBell` variant added
- PageDown key (0x51) mapped to `ToggleBell`

### Lifecycle
- `lifecycle_register(SURFACE_ID_BELL_PLACEHOLDER, LifecycleState::Visible)`

### Wire-up points
- `surface_is_alive`, `surface_in_active_scene`, `get_surface_bounds`
- `point_in_surface`, `update_local_geometry`
- Both tile functions (geometry + placeholder fill rect)
- `OP_SURFACE_UPDATE` position update
- Both `z_order` arrays
- Focus description in `try_set_focus`

### Helper functions
- `ensure_bell_frame()`
- `open_bell_in_active_scene()`
- `toggle_bell()`
- `focus_or_open_bell()`
- `bell_frame_id()`

### Keyboard handler
- `SurfaceAction::ToggleBell` → `toggle_bell()`

## 3. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[bell.placeholder.attach.frame]` | ensure_bell_frame() | Frame created |
| `[bell.placeholder.attach.tab]` | ensure_bell_frame() | Tab attached |
| `[bell.placeholder.reject.duplicate]` | open_bell_in_active_scene() | Already open |
| `[bell.placeholder.open]` | open_bell_in_active_scene() | Successfully opened |
| `[bell.placeholder.focus]` | open_bell_in_active_scene() | Focused |

## 4. Invariants

1. Bell surface follows existing lifecycle FSM
2. No duplicate Bell frame in same active scene
3. Focus goes through try_set_focus() lifecycle + scene guards
4. Close/minimize/tombstone use existing proven paths
5. No real notifications, attention budget, or event routing in I3
6. Toggle via PageDown key
7. Bell panel (0x95) and Bell placeholder (204) are distinct surfaces

## 5. Relationship to Existing Work

| Existing Work | Relationship |
|---------------|--------------|
| **G1 Bell event contract** | Defines event model; I3 provides visual placeholder |
| **PHASE_09 Bell** | Rapid source defines attention firewall; I3 is placeholder only |
| **D1/E1/I1/I2** | Exact pattern match for placeholder lifecycle |

## 6. STOP FIRST Triggers

- Notification delivery before event contract implementation
- Attention budget enforcement before G1 implementation
- Capability-gated urgency before Collar (F2)
- Bell PDX ops before F2/G1
- Renderer changes
- Kernel/sex-pdx ABI edits
- Cross-PD raw pointers
