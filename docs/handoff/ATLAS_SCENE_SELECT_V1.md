# ATLAS_SCENE_SELECT_V1

**Status:** Active  
**Purpose:** Click an Atlas scene card to switch to that Scene and exit Atlas. Click-only — no drag, no thumbnails, no rename.  
**Scope:** `servers/silk-shell/src/main.rs` only.  
**Prerequisites:** ATLAS_RENDER_STUB_V1 (c5a7b8c)

---

## 1. Hit-Test Geometry

### `atlas_scene_at_point(px: i32, py: i32) -> Option<u8>`

Convert screen coords to overlay-local coords (overlay starts at `y = P.bar_height`), then iterate scene cards using `atlas_card_pos()` and test `(px, local_y)` against each card rect.

```rust
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
```

### Coordinate system

| Coordinate | Source | Notes |
|------------|--------|-------|
| `px`, `py` | Screen coordinates (from pointer input) | As used by all click handlers |
| `local_y` | `py - P.bar_height` | Overlay surface starts below SilkBar |
| `cw` | `P.width` | Content area width, matches overlay width |
| Card rect | `atlas_card_pos(scene_idx, cw)` | Same layout as render stub |

---

## 2. Selection Behavior

### Click on card → switch + exit

```
Atlas open → user clicks Scene 2 card →
  1. atlas_scene_at_point(px, py) returns Some(2)
  2. pdx_call(0xEE, ATLAS_OVERLAY) — destroy overlay
  3. switch_scene(2) — sync visibility, tile, clear state, capture
  4. ATLAS_MODE_ENABLED = false
  5. [shell.atlas.select] id=2
  6. Return to normal shell mode
```

### Click on active scene card → exit only

```
Atlas open → user clicks currently active scene card →
  1. atlas_scene_at_point(px, py) returns Some(idx)
  2. scene_idx == ACTIVE_SCENE_IDX → skip switch_scene()
  3. pdx_call(0xEE, ATLAS_OVERLAY) — destroy overlay
  4. sync_scene_visibility() + tile_visible_frames() — restore rendering
  5. ATLAS_MODE_ENABLED = false
  6. [shell.atlas.select] id=N (same scene)
  7. Return to normal shell mode
```

### Click misses all cards → keep Atlas open

```
Atlas open → user clicks empty area →
  1. atlas_scene_at_point(px, py) returns None
  2. [shell.atlas.miss]
  3. Return (HitTarget::None, true) — click consumed, no fallthrough
```

### F10 exit after missed click

```
Atlas open → miss → F10 →
  1. atlas_toggle() → ATLAS_MODE_ENABLED true
  2. atlas_clear_stub() → 0xEE + restore visibility/tiling
  3. ATLAS_MODE_ENABLED = false
  4. [shell.atlas.exit]
```

---

## 3. Patch Summary

### New function: `atlas_scene_at_point()` (+20 lines)

Inserted after `atlas_card_pos()`, before `atlas_render_stub()`. Pure function, no unsafe needed (uses const `P`).

### Atlas intercept in `click_hit_test_and_focus()` (+25 lines)

Inserted at the top of the function, after the `[shell.click_focus.down]` marker and before `let target = hit_test_at(px, py)`.

```rust
if ATLAS_MODE_ENABLED {
    if let Some(scene_idx) = atlas_scene_at_point(px, py) {
        // Hit: destroy overlay, switch scene, exit Atlas.
        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_ATLAS_OVERLAY, 0, 0);
        let already_active = scene_idx == ACTIVE_SCENE_IDX;
        if !already_active {
            switch_scene(scene_idx);
        } else {
            sync_scene_visibility();
            clear_focus_if_dead();
            clear_drag_if_dead();
            clear_hover_if_wrong_scene();
            tile_visible_frames();
            snap_capture_layout();
        }
        ATLAS_MODE_ENABLED = false;
        // [shell.atlas.select] id=N
        return (HitTarget::None, true);
    } else {
        // Miss: keep Atlas open, consume click.
        // [shell.atlas.miss]
        return (HitTarget::None, true);
    }
}
```

### Return value

Both hit and miss return `(HitTarget::None, true)`. The `true` silkbar_handled flag prevents normal drag-start logic from executing (the body after `// SilkBar intercept:` checks `!silkbar_handled`).

---

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — +45 lines (hit-test function + Atlas intercept in click handler)
- `docs/handoff/ATLAS_SCENE_SELECT_V1.md` — new handoff doc

---

## 5. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

Warning count: 212 (unchanged — all pre-existing).

---

## 6. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Click empty scene card | Switches to empty scene, exits Atlas | ✅ |
| Click active scene card | Exits Atlas, restores current scene | ✅ |
| Click outside all cards (miss) | Atlas stays open, click consumed | ✅ |
| Clisk in SilkBar area while Atlas open | Consumed by Atlas intercept (miss) — SilkBar unreachable | ✅ (design choice) |
| F10 exit after miss | Exits Atlas normally via atlas_clear_stub() | ✅ |
| Switch to scene with hover/drag/focus | switch_scene() clears all stale state | ✅ |
| F8/F9 while Atlas open | F8/F9 not affected — Atlas intercepts only clicks, not keyboard | ✅ |
| Click on overlay but y < bar_height | `local_y < 0` → early None return → miss | ✅ |
| Zero-width content area | `cw == 0` → early None return → miss | ✅ |
| Atlas not enabled | Normal click handling unchanged | ✅ |
| Click during Drag state | Atlas intercept only triggers when ATLAS_MODE_ENABLED; drag state doesn't affect it | ✅ |

---

## 7. Deferred for Later Phases

- Keyboard scene selection in Atlas (`ATLAS_KEYBOARD_SELECT_V1`)
- Atlas card hover highlighting
- Atlas card drag-to-reorder
- Atlas scene rename from overview
- Frame thumbnails inside Atlas cards

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add atlas_scene_at_point() hit-test, Atlas intercept in click_hit_test_and_focus(), selection + exit path | ATLAS_SCENE_SELECT_V1 |
