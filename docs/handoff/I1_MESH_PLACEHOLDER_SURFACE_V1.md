# I1: Mesh Placeholder Surface

**Status:** Handoff (code + doc)
**Commit:** _(to be committed)_
**Build:** ISO produced

## 1. Purpose

Attach a Mesh placeholder surface through the proven Scene/Frame/Tab lifecycle
path, mirroring the D1 (Linen) and E1 (Quil) pattern. Mesh is a placeholder
surface with no live graph, no authority changes, and no Collar interaction
yet. It provides visual presence for the Mesh diagnostic graph concept.

### What Mesh IS (I1)
- Placeholder surface (Surface ID 202, Frame ID 4)
- Toggle via F12 key
- Lifecycle-registered as Visible
- Attached to active Scene as Frame/Tab through B1 model
- Can be closed/minimized/restored by existing A5/A6/B3 paths

### What Mesh IS NOT (I1)
- Not a live capability graph
- No authority grant/revoke
- No Collar authority control
- No PDX route enumeration
- No cross-PD pointers
- No renderer changes

## 2. Changes to `servers/silk-shell/src/main.rs`

### Surface ID and Frame ID (Change 1)
```rust
pub const SURFACE_ID_MESH: u64 = 202;
```
```rust
const MESH_FRAME_ID: u32 = 4;
```

Increase `MAX_FRAMES` from 4 to 5.
Increase `ATLAS_MAX_FRAMES_PER_SCENE` from 4 to 5.

### AppSurfaceSpec registry (Change 2)
Add Mesh entry to `APP_SURFACES` (increase array size from 2 to 3):
```rust
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
```

### Static geometry variables (Change 3)
```rust
static mut SURFACE_202_X: i32 = MESH_BOOT_X;
static mut SURFACE_202_Y: i32 = MESH_BOOT_Y;
static mut SURFACE_202_W: u32 = MESH_BOOT_W;
static mut SURFACE_202_H: u32 = MESH_BOOT_H;
```

### SurfaceAction enum (Change 4)
Add `ToggleMesh` variant.

### Scancode mapping (Change 5)
Map F12 (0x58) to `ToggleMesh`.

### Lifecycle init (Change 6)
Add `lifecycle_register(SURFACE_ID_MESH, LifecycleState::Visible)`.

### surface_is_alive (Change 7)
Add `SURFACE_ID_MESH => true`.

### point_in_surface / get_surface_bounds (Change 8)
Add `SURFACE_ID_MESH` geometry entries.

### surface_in_active_scene (Change 9)
Add `SURFACE_ID_MESH` to the frame-owner check alongside Linen/Quil.

### update_local_geometry (Change 10)
Add `SURFACE_ID_MESH` match arm.

### tile_active_scene_frames / layout_tile_visible (Change 11)
Add `SURFACE_ID_MESH` match arms for geometry update.

### z_order arrays (Change 12)
Add `SURFACE_ID_MESH` to z_order arrays.

### Frame/tab access nodes (Change 13)
Add Mesh to access node label/role emission.

### Keyboard handler (Change 14)
Handle `SurfaceAction::ToggleMesh` → call `toggle_mesh()`.

### Helper functions (Change 15)
- `ensure_mesh_frame()` — creates ShellFrame if not exists
- `open_mesh_in_active_scene()` — open Mesh in active scene
- `toggle_mesh()` — toggle visibility
- `focus_or_open_mesh()` — focus or open
- `mesh_frame_id()` — query frame_id

### Proof markers (Change 16)
- `[mesh.placeholder.open]` — on successful open
- `[mesh.placeholder.attach.frame]` — on frame creation
- `[mesh.placeholder.attach.tab]` — on tab attach
- `[mesh.placeholder.focus]` — on focus
- `[mesh.placeholder.reject.duplicate]` — on duplicate reject

## 3. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.placeholder.attach.frame]` | ensure_mesh_frame() | Frame created |
| `[mesh.placeholder.attach.tab]` | ensure_mesh_frame() | Tab attached |
| `[mesh.placeholder.reject.duplicate]` | open_mesh_in_active_scene() | Already open |
| `[mesh.placeholder.open]` | open_mesh_in_active_scene() | Successfully opened |
| `[mesh.placeholder.focus]` | open_mesh_in_active_scene() | Focused |

## 4. Invariants

1. Mesh surface follows existing lifecycle FSM (Visible, Minimized, Closing, etc.)
2. No duplicate Mesh frame in same active scene
3. Focus goes through try_set_focus() lifecycle + scene guards
4. Close/minimize/tombstone use existing proven paths (A5/A6/B3)
5. No Collar, no authority, no live graph in I1
6. Toggle via F12 works like F8/F9 for Linen/Quil

## 5. Relationship to Existing Work

| Existing Work | Relationship |
|---------------|--------------|
| **D1/E1** | Exact pattern match for placeholder surface lifecycle |
| **B1** | Frame/Tab attach through existing FRAMES model |
| **B3** | Participates in tiling via tile_active_scene_frames |
| **A5/A6** | Close/minimize/tombstone through proven FSM |
| **F1** | Mesh diagnostic model defined; I1 is visual placeholder |
| **F2** | Collar authority map; no Collar in I1 |

## 6. Future Implementation

| Phase | Scope | Type |
|-------|-------|------|
| I1 | Placeholder surface (this) | Code |
| I2 | Collar placeholder surface | Code |
| I3 | Bell placeholder surface | Code |
| I4 | Runtime proof for Mesh/Collar/Bell together | Code + Docs |
| Mesh v2 | Live graph nodes via PDX | Code |

## 7. STOP FIRST Triggers

- Live graph implementation before I4
- Authority grant/revoke before F2 implementation
- Collar PDX ops before F2
- Renderer changes
- Kernel/sex-pdx ABI edits
- Cross-PD raw pointers
