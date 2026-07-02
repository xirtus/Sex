# ATLAS_SCENE_OVERVIEW_MODEL_V1

**Status:** LOCKED
**Date:** 2026-05-06
**Files changed:** 1 (+38 / -0 lines)

---

## Model Shape

The Atlas overview model is a fixed, bounded, shell-owned representation of the workspace/scene topology. No heap allocation, no IPC protocol, no renderer policy — purely silk-shell internal state.

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `ATLAS_MAX_SCENES` | 5 | Maximum scenes (workspaces) tracked |
| `ATLAS_MAX_FRAMES_PER_SCENE` | 9 | Maximum frames per scene descriptor |
| `MAX_FRAMES` | 9 | Maximum frames in FRAMES array |
| `MAX_TABS_PER_FRAME` | 8 | Maximum tabs per frame |
| `ATLAS_LABEL_LEN` | 16 | Fixed-size scene label length |
| `WORKSPACE_COUNT` | 5 | Workspace count (== ATLAS_MAX_SCENES) |

### Data Structures

```
SceneId(u8)       — type-safe scene index (0..4)
FrameId(u32)      — type-safe frame identifier
TabIndex(u8)      — type-safe tab index within frame
```

**ShellTab** — single tab holding a surface reference:
```rust
struct ShellTab {
    surface_id: u64,
    title_id: u64,
    flags: u32,
}
```

**ShellFrame** — tiled container owning up to 8 tabs:
```rust
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,
    tab_count: u8,
    tabs: [Option<ShellTab>; 8],
    scene_id: u8,
    flags: u32,
    normal_x, normal_y, normal_w, normal_h: geometry,
}
```

**Scene** — runtime per-scene tracking state:
```rust
struct Scene {
    flags: u8,
    label: [u8; 16],
    accent: u8,
    pinned: bool,
}
```

**SceneDescriptor** — snapshot of a scene for Atlas rendering:
```rust
struct SceneDescriptor {
    scene_id: u32,
    label: [u8; 16],
    flags: u8,
    accent: u8,
    pinned: bool,
    focused_frame_id: u32,
    frame_count: u8,
    frame_ids: [u32; 9],
}
```

**AtlasSnapshot** — full derived overview of all scenes:
```rust
struct AtlasSnapshot {
    active_scene_id: u32,
    scene_count: u8,
    scenes: [SceneDescriptor; 5],
}
```

### Global State

| Variable | Type | Description |
|----------|------|-------------|
| `SCENES` | `[Scene; 5]` | Runtime scene tracking state |
| `FRAMES` | `[Option<ShellFrame>; 9]` | Frame storage array |
| `ACTIVE_SCENE_IDX` | `u8` | Current active scene index |
| `ATLAS_MODE_ENABLED` | `bool` | Atlas overview mode flag |
| `ATLAS_SELECTED_SCENE` | `u8` | Keyboard-nav selected scene index |
| `ATLAS_SNAPSHOT` | `AtlasSnapshot` | Cached derived overview |

### Scene Flags

| Flag | Bit | Description |
|------|-----|-------------|
| `SCENE_FLAG_ACTIVE` | 0 | This scene is the active workspace |
| `SCENE_FLAG_EMPTY` | 1 | Scene has no frames |
| `SCENE_FLAG_HAS_FOCUS` | 2 | Scene contains focused surface |
| `SCENE_FLAG_HAS_MINIMIZED` | 3 | Scene has at least one minimized frame |
| `SCENE_FLAG_HAS_ZOOMED` | 4 | Scene has at least one zoomed frame |

---

## Operations

### Scene Initialization (`scene_init_all`)

Called once at boot. Initializes all 5 scenes with default labels ("Scene 0"-"Scene 4") and cycling accent tokens (Clear, Warm, Cool, Coral, Gold). Emits `[scene.core.init]` and `[atlas.scene.settings.init]`.

### Scene Switch (`switch_scene`)

Clamps to `WORKSPACE_COUNT - 1`. Updates `ACTIVE_SCENE_IDX`, syncs visibility, clears stale focus/drag/hover, re-tiles, captures Atlas snapshot. Emits `[scene.switch]` and `[shell.interact.scene.switch]`.

### Atlas Toggle (`atlas_toggle`)

Opens/closes Atlas overview mode. On enter: renders card overlay, captures snapshot, clears stale hover/drag. On exit: destroys overlay, restores tiling. Emits `[atlas.view.enter]` / `[atlas.view.exit]`.

### Atlas Snapshot Capture (`atlas_capture_snapshot`)

Derives `AtlasSnapshot` from current `FRAMES` and `SCENES` state. Filters out dead/tombstoned/minimized frames and surfaces in non-tileable lifecycle states. Updates `ATLAS_SNAPSHOT`. Emits `[atlas.snapshot.*]` per scene.

### Scene Flag Update (`scene_update_flags`)

Recomputes `SCENE_FLAG_EMPTY`, `SCENE_FLAG_HAS_MINIMIZED`, `SCENE_FLAG_HAS_ZOOMED` from frame state. `SCENE_FLAG_ACTIVE` and `SCENE_FLAG_HAS_FOCUS` are set during snapshot capture.

### Keyboard Navigation (`handle_atlas_keyboard`)

Routes scancodes in Atlas mode: arrow keys move selected card in 3+2 grid layout, number keys 1-5 switch directly to scene, Enter confirms selection, Esc exits Atlas.

### Render Stub (`atlas_render_stub`)

Full card-based overview renderer using fill-rect ops (0xEF). Draws:
- Dark overlay background
- Card backgrounds with accent/active/empty colors
- Frame indicator blocks at card bottom
- Pinned indicator dot
- Selection border (bright cyan) for selected card
- Active rim for active scene
- Focus marker (green dot) for scenes with focused surface
- Tile-count accent bar for scenes with >1 frame

---

## Proof Markers (gated by `SEXOS_ATLAS_OVERVIEW_PROOF=1`)

Five synthetic proof stages run at boot:

| Stage | Operation | Marker |
|-------|-----------|--------|
| 0 | Switch to scene 1 | `[shell.atlas.proof.switch] from=0 to=1 ok=true` |
| 1 | Capture and list Atlas snapshot | `[shell.atlas.proof.list] scenes=5 active=1` |
| 2 | Switch to invalid index 99 (clamped) | `[shell.atlas.proof.clamp] clamped=true idx=4` |
| 3 | Count FRAMES, verify bounded | `[shell.atlas.proof.frames] count=4 max=9 valid=true` |
| 4 | Capture snapshot, verify scene flags | `[shell.atlas.proof.flags] flags=... active=true empty=...` |

All stages emit `[shell.atlas.proof] stage=N` at entry.

### Rejection Proofs

| Condition | Marker |
|-----------|--------|
| Invalid scene accent token | `[atlas.scene.visual.reject] reason=accent_oob` |
| Invalid scene accent set | `[atlas.scene.settings.reject] fn=accent` |
| Invalid scene pin set | `[atlas.scene.settings.reject] fn=pinned` |
| Invalid scene label set | `[atlas.scene.settings.reject] fn=label` |
| Invalid accent in UI | `[atlas.scene.settings.ui.reject] fn=accent` |
| Invalid pin in UI | `[atlas.scene.settings.ui.reject] fn=pin` |

---

## Build / Runtime

- Build: `SEXOS_ATLAS_OVERVIEW_PROOF=1 ./scripts/entrypoint_build.sh` — **PASS**
- No kernel edits. No ABI changes. No sexdisplay changes. No sex-pdx changes.
- No USB/pointer dependency. Atlas works entirely via keyboard (F10 toggle, arrow nav).

## STOP FIRST Conditions

None triggered. No sex-pdx ABI change, no kernel edit, no renderer policy, no broad shell rewrite.

## Next Atlas Render Step

Atlas overview rendering is already implemented in `atlas_render_stub()` — card layout, accent colors, frame blocks, selection border, focus marker, tile-count bar, pinned indicator, active rim. The next phase would be:

1. **Click-to-select**: The `atlas_scene_at_point()` function already exists for hit-testing, but is only wired in the click handler. Ensure it reliably selects the clicked scene card.
2. **Drag-to-reorder**: Allow dragging scene cards to reorder scenes.
3. **Scene thumbnail previews**: Replace fill-rect cards with surface snapshots.

## Files Changed

```
servers/silk-shell/src/main.rs  +38 / -0  (ATLAS_OVERVIEW_PROOF_ENABLED + 5 proof stages)
```

No sex-pdx ABI changes. No kernel edits. No renderer primitives.
