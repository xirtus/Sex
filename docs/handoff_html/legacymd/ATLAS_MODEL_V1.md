# ATLAS_MODEL_V1

**Status:** Active  
**Purpose:** Introduce Atlas as a shell-owned model of all Scenes for future overview/collage UI. V1 is data/model only — no rendering, no sexdisplay changes, no new ABI.  
**Scope:** `servers/silk-shell/src/main.rs` only.  
**Prerequisites:** FRAME_CHROME_STATE_V1 (11b95e7)

---

## 1. Atlas Abstraction

Atlas is Silk's shell-owned map of all Scenes. It sits above Scene in the abstraction stack:

```
Silk
├── SilkBar         → selected Scene/frame status + commands
├── Scene           → current working environment
├── Frame           → tiled container inside a Scene
├── Tab             → app surface inside a Frame
└── Atlas           → overview/map of all Scenes (NEW)
```

**Atlas is not:**
- A renderer (no sexdisplay changes)
- A workspace switcher (Scene switching is pre-existing)
- A SilkBar replacement
- An app surface owner
- A framebuffer/backing buffer

**Atlas knows:**
```
Scene 0: Browser + Terminal (active, zoomed)
Scene 1: Linen files (minimized)
Scene 2: Quil coding session
Scene 3: empty
Scene 4: empty
```

---

## 2. Struct Fields

```rust
/// Maximum scenes tracked by Atlas (equals WORKSPACE_COUNT).
const ATLAS_MAX_SCENES: usize = 5;
/// Maximum frames tracked per scene descriptor (equals MAX_FRAMES).
const ATLAS_MAX_FRAMES_PER_SCENE: usize = 4;
/// Length of fixed-size scene label byte array (no heap strings).
const ATLAS_LABEL_LEN: usize = 16;

/// SceneDescriptor flags
const SCENE_FLAG_ACTIVE: u8         = 1 << 0;
const SCENE_FLAG_EMPTY: u8          = 1 << 1;
const SCENE_FLAG_HAS_FOCUS: u8      = 1 << 2;
const SCENE_FLAG_HAS_MINIMIZED: u8  = 1 << 3;
const SCENE_FLAG_HAS_ZOOMED: u8     = 1 << 4;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SceneDescriptor {
    scene_id: u32,
    label: [u8; 16],            // "Scene N" padded with zeros
    flags: u8,                   // SCENE_FLAG_* bitmask
    focused_frame_id: u32,       // 0 if none
    frame_count: u8,             // valid entries in frame_ids[]
    frame_ids: [u32; 4],         // fixed array, no heap
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct AtlasSnapshot {
    active_scene_id: u32,
    scene_count: u8,             // always ATLAS_MAX_SCENES in V1
    scenes: [SceneDescriptor; 5],
}
```

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| Fixed arrays, no heap | Matches existing ShellFrame/ShellTab pattern. No allocator dependency. |
| `label: [u8; 16]` | No string allocation. V1: "Scene N\0". Future: user-settable via fixed-buffer protocol. |
| `flags: u8` | Compact. 5 flags defined, room for 3 more before expansion needed. |
| `frame_ids: [u32; 4]` | Matches MAX_FRAMES. Non-active scenes may have 0 frames. |

---

## 3. Capture invocation

`atlas_capture_snapshot()` is called from two low-frequency paths:

| Call site | Frequency | Rationale |
|-----------|-----------|-----------|
| `switch_scene()` (shortcut) | On keyboard scene switch | Atlas must reflect active scene change |
| `handle_silkbar_click()` → `Action::SwitchWorkspace` | On SilkBar workspace click | Same — covers both scene switch entry points |

The capture function:
- Derives state from existing `FRAMES`, `ACTIVE_SCENE_IDX`, `FOCUSED_SURFACE_ID` only
- No IPC, no sexdisplay calls, no allocation
- Budgeted `[shell.atlas.capture]` marker (8-budget)

---

## 4. Patch Summary

### New constants (after FRAME_FLAG_TOP_BAR, line ~1016)

```rust
const ATLAS_MAX_SCENES: usize = 5;
const ATLAS_MAX_FRAMES_PER_SCENE: usize = 4;
const ATLAS_LABEL_LEN: usize = 16;
const SCENE_FLAG_ACTIVE: u8 = 1 << 0;
const SCENE_FLAG_EMPTY: u8 = 1 << 1;
const SCENE_FLAG_HAS_FOCUS: u8 = 1 << 2;
const SCENE_FLAG_HAS_MINIMIZED: u8 = 1 << 3;
const SCENE_FLAG_HAS_ZOOMED: u8 = 1 << 4;
```

### New structs (after constants)

```rust
struct SceneDescriptor { ... }
struct AtlasSnapshot { ... }
```

### New static

```rust
static mut ATLAS_SNAPSHOT: AtlasSnapshot = AtlasSnapshot { ... };
```

### New functions

```rust
fn atlas_default_label(scene_id: u32) -> [u8; 16] { ... }
unsafe fn atlas_capture_snapshot() { ... }
```

### Call sites (2 additions)

- `switch_scene()` — after `snap_capture_layout()`, before budget marker
- `handle_silkbar_click()` → `Action::SwitchWorkspace` — after `snap_capture_layout()`, before budget marker

---

## 5. Files Changed

- `servers/silk-shell/src/main.rs` — +120 lines (constants, structs, static, capture function, 2 call sites)

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## 7. Deferred Phases

| Phase | What | When |
|-------|------|------|
| **ATLAS_TOGGLE_ACTION_V1** | Add `ToggleAtlas` SurfaceAction + keyboard shortcut | Next |
| **ATLAS_RENDER_STUB_V1** | Render simple Atlas cards via sexdisplay protocol (0xEC scene surfaces) | After toggle |
| **ATLAS_SCENE_SELECT_V1** | Click Atlas card → switch to that Scene | After render |
| **ATLAS_FRAME_PREVIEW_PLAN_V1** | Show Frame mini-layouts inside Atlas cards | Later |

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add AtlasSnapshot, SceneDescriptor, atlas_capture_snapshot(), call from scene switch paths | ATLAS_MODEL_V1 |
