# ATLAS_RENDER_STUB_V1

**Status:** Active  
**Purpose:** Render Atlas overview scene cards using the existing sexdisplay fill-rect protocol (0xEF). Shell-owned overlay surface, no sexdisplay changes, no new ABI, no thumbnails.  
**Scope:** `servers/silk-shell/src/main.rs` only.  
**Prerequisites:** ATLAS_TOGGLE_ACTION_V1 (e5750d8)

---

## 1. Rendering Approach

Atlas renders as a **shell-owned overlay surface** (`SURFACE_ID_ATLAS_OVERLAY = 0x97`), created and drawn entirely via the existing sexdisplay protocol:

| Step | Call | What it does |
|------|------|-------------|
| Enter Atlas | `0xEC` create overlay | Full content area (below SilkBar) |
| | `0xEF` fill background | Dark navy (`0x00182850`) covering entire surface |
| | `0xEF` fill cards | One `0xEF` call per scene card + one per frame block |
| Exit Atlas | `0xEE` destroy overlay | Removes surface, restores normal rendering |

This follows the same pattern as existing shell-owned panels (0x92-0x96).

### Why no new geometry surface

Each card is drawn as a positioned fill-rect via `0xEF` with `(sx, sy)` offset in arg1. The 0xEF handler in sexdisplay supports position offsets within the surface — no need for per-card surfaces. A single overlay surface hosts all cards as colored rectangles.

---

## 2. Overlay Surface

```
SURFACE_ID_ATLAS_OVERLAY = 0x97  (151 decimal)
```

Created on Atlas enter with geometry matching the content area (full width, height minus SilkBar). Positioned with y-offset equal to `P.bar_height` so it sits below the SilkBar.

Destroyed on Atlas exit via `0xEE`, which is the standard hide/destroy for shell-owned surfaces.

### Surface ID allocation

| ID | Hex | Owner | Purpose |
|----|-----|-------|---------|
| 146 | 0x92 | Shell (Silk) | Linen panel overlay |
| 147 | 0x93 | Shell (Silk) | Quil panel overlay |
| 148 | 0x94 | Shell (Silk) | Bell panel overlay |
| 149 | 0x95 | Shell (Silk) | Brightness panel overlay |
| 150 | 0x96 | Shell (Silk) | Scene Settings panel overlay |
| **151** | **0x97** | Shell (Silk) | **Atlas overview overlay (NEW)** |

---

## 3. Card Layout

Five scene cards arranged in two centered rows:

```
┌─────────────────────────────────────────────────┐
│                                                 │
│         ┌──────┐  ┌──────┐  ┌──────┐           │
│         │Scene │  │Scene │  │Scene │           │  ← Row 0: scenes 0,1,2
│         │  0   │  │  1   │  │  2   │           │
│         │[█ █] │  │[█ █ █]│  │[ ]   │           │
│         └──────┘  └──────┘  └──────┘           │
│                                                 │
│                  ┌──────┐  ┌──────┐             │
│                  │Scene │  │Scene │             │  ← Row 1: scenes 3,4
│                  │  3   │  │  4   │             │
│                  │[ ]   │  │[ ]   │             │
│                  └──────┘  └──────┘             │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Card dimensions

| Constant | Value | Notes |
|----------|-------|-------|
| `ATLAS_CARD_W` | 220px | Card width |
| `ATLAS_CARD_H` | 150px | Card height |
| `ATLAS_CARD_GAP` | 24px | Gap between cards |
| `ATLAS_CARDS_ROW0` | 3 | Scene indices 0,1,2 in top row |
| `ATLAS_CARDS_ROW1` | 2 | Scene indices 3,4 in bottom row |

### Card visual structure

Each card has two stacked colored regions:

1. **Top area** (100px): Scene color block — card color fills this region
2. **Bottom area** (50px): Frame indicator blocks — small rects showing each frame in the scene

```
┌──────────────┐
│              │
│  Scene color │  ← 100px (ATLAS_CARD_TOP_H)
│              │
├──────┬───────┤
│  ██  │  ██   │  ← 28px frame blocks (ATLAS_FRAME_BLOCK_H)
└──────┴───────┘
  12px padding from card edges
```

### Frame indicator blocks

| Constant | Value | Notes |
|----------|-------|-------|
| `ATLAS_FRAME_BLOCK_W` | 36px | Block width |
| `ATLAS_FRAME_BLOCK_H` | 28px | Block height |
| `ATLAS_FRAME_BLOCK_GAP` | 8px | Gap between blocks |
| `ATLAS_FRAME_PAD` | 12px | Padding from card edge |

Up to 4 blocks per card (matching `ATLAS_MAX_FRAMES_PER_SCENE`), centered horizontally below the top area.

---

## 4. Color Palette

| Constant | ARGB | Usage |
|----------|------|-------|
| `ATLAS_COLOR_BG` | `0x00182850` | Dark navy — overlay background |
| `ATLAS_COLOR_CARD_ACTIVE` | `0x004468c0` | Brighter blue — active scene card |
| `ATLAS_COLOR_CARD_SCENE` | `0x00284878` | Medium blue — non-active scene card |
| `ATLAS_COLOR_CARD_EMPTY` | `0x00182850` | Dim — empty scene (matches BG, invisible) |
| `ATLAS_COLOR_FRAME_NORMAL` | `0x003860a0` | Blue-gray — normal frame block |
| `ATLAS_COLOR_FRAME_ZOOMED` | `0x0048c080` | Teal — zoomed frame block |
| `ATLAS_COLOR_FRAME_MINIMIZED` | `0x00304060` | Muted — minimized frame block |

All colors are dim, non-saturated to keep Atlas as an overview layer rather than competing with content.

---

## 5. Wire-up

### `atlas_toggle()` — rewritten

```rust
unsafe fn atlas_toggle() {
    if ATLAS_MODE_ENABLED {
        // Exiting Atlas: clear overlay, restore normal rendering.
        atlas_clear_stub();
        ATLAS_MODE_ENABLED = false;
        // [shell.atlas.exit]
    } else {
        // Entering Atlas: render overlay, clear stale hover/drag.
        ATLAS_MODE_ENABLED = true;
        atlas_render_stub();
        clear_hover_if_wrong_scene();
        clear_drag_if_dead();
        // [shell.atlas.enter]
    }
}
```

**Key change from V1 toggle-only**: On enter, `atlas_render_stub()` creates the overlay and draws cards. On exit, `atlas_clear_stub()` destroys the overlay and restores normal scene rendering.

### `atlas_render_stub()` — new

1. Calls `atlas_capture_snapshot()` for fresh data
2. Creates overlay surface via `0xEC` (full content area, below SilkBar)
3. Fills background via `0xEF` (dark navy)
4. For each scene (0..4):
   - Computes card position via `atlas_card_pos()`
   - Selects card color based on flags (active/empty/scene)
   - Draws top region via `0xEF` with position offset
   - Draws frame indicator blocks via `0xEF` with position offset
5. Budgeted `[shell.atlas.render]` marker (4-budget)

### `atlas_clear_stub()` — new

1. Destroys overlay via `0xEE`
2. Calls `sync_scene_visibility()` to restore normal surface visibility
3. Clears stale focus/drag/hover state
4. Re-tiles visible frames via `tile_visible_frames()`
5. Captures layout snapshot via `snap_capture_layout()`
6. Budgeted `[shell.atlas.clear] restore` marker (4-budget)

### `atlas_card_pos()` — new helper

Pure function computing `(x, y, w, h)` for a scene index:

- Row 0 contains scenes 0, 1, 2 (3 cards)
- Row 1 contains scenes 3, 4 (2 cards)
- Each row is centered horizontally: `start_x = (cw - total_row_width) / 2`
- Y offset: row 0 at 30px from overlay top, row 1 below with gap

---

## 6. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Atlas toggle when already in Atlas | Exits Atlas, clears overlay | ✅ |
| Atlas toggle when not in Atlas | Enters Atlas, creates overlay, draws cards | ✅ |
| Toggle Atlas with 0-width/0-height content area | Early return in `atlas_render_stub()`, no 0xEC call | ✅ |
| Empty scene (no frames) | Card drawn with BG color, no frame blocks | ✅ |
| Scene with multiple frames | Up to 4 frame indicator blocks drawn | ✅ |
| Scene with zoomed frame | Frame block uses teal color | ✅ |
| Scene with minimized frame | Frame block uses muted color | ✅ |
| All scenes empty | 5 empty-colored cards (visually blank overlay) | ✅ |
| Exit Atlas with no overlay | `0xEE` on non-existent surface — sexdisplay handles gracefully | ✅ |
| Frame block count exceeds MAX_FRAMES_PER_SCENE | Capped by `.min(ATLAS_MAX_FRAMES_PER_SCENE)` | ✅ |
| Budget marker exhausts | After 4 calls, marker stops printing (no overflow) | ✅ |
| Concurrent with other overlay panels | Atlas overlay (0x97) is independent of existing panels | ✅ |
| Active scene change while in Atlas | Atlas stays open; stale snapshot until next toggle | ✅ (V1 limitation) |

---

## 7. Files Changed

- `servers/silk-shell/src/main.rs` — +145 lines (1 surface ID, 14 render constants, `atlas_card_pos()`, `atlas_render_stub()`, `atlas_clear_stub()`, rewritten `atlas_toggle()`)

---

## 8. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

Warning count: 212 (unchanged from ATLAS_TOGGLE_ACTION_V1 — all pre-existing).

---

## 9. Future Phases

| Phase | What | Status |
|-------|------|--------|
| ATLAS_MODEL_V1 | AtlasSnapshot + SceneDescriptor | ✅ Done |
| ATLAS_TOGGLE_ACTION_V1 | F10 toggle + state | ✅ Done |
| **ATLAS_RENDER_STUB_V1** | **Overlay + scene cards via 0xEF** | ✅ **Done** |
| ATLAS_SCENE_SELECT_V1 | Click Atlas card → switch to that Scene | Next |
| ATLAS_FRAME_PREVIEW_PLAN_V1 | Show Frame mini-layouts inside Atlas cards | Later |
| ATLAS_SNAPSHOT_REFRESH_V1 | Refresh Atlas view periodically or on scene change | Later |

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add Atlas overlay surface (0x97), card layout, fill-rect rendering, enter/exit wire-up | ATLAS_RENDER_STUB_V1 |
