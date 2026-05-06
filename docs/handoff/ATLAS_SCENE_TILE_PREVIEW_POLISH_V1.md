# ATLAS_SCENE_TILE_PREVIEW_POLISH_V1

**Status:** Implemented
**Date:** 2026-05-06
**Purpose:** Polish Atlas scene previews so tiled boot/shell state is easier to
understand visually. Shell-only visual metadata. No ABI changes.

---

## 1. Changes Applied

### Changed file: `servers/silk-shell/src/main.rs`

| Area | Lines | Change |
|------|-------|--------|
| New constants | 3684-3691 | Added `ATLAS_FOCUS_MARKER_COLOR`, `ATLAS_FOCUS_MARKER_SIZE`, `ATLAS_TILE_COUNT_BAR_COLOR`, `ATLAS_TILE_COUNT_BAR_H` |
| Active rim | 5169-5170 | Strengthened active scene rim from 1px→2px (matches selected card border width for visual parity) |
| Focus marker | 5192-5203 | Bright green 6×6 dot at top-left of cards whose scene contains focused surface |
| Tile count bar | 5205-5221 | Thin 3px light violet accent bar below card top when scene has `>1` visible frame |
| Polish marker | 5223-5231 | Budgeted `[atlas.preview.polish]` (max 8) per card per render |

### Diff summary

```
 servers/silk-shell/src/main.rs | 52 ++++++++++++++++++++++++++++++++++--
 1 file changed, 50 insertions(+), 2 deletions(-)
```

---

## 2. Atlas Preview Polish Applied

### 2a. Active scene stronger border

Before: 1px muted cyan rim (`ATLAS_CARD_ACTIVE_RIM_COLOR`).
After: 2px rim — same thickness as the selected card's selection border.

This makes the active scene visually distinct at a glance, especially when
no card is nav-selected (the active scene's 2px rim stands out against
inactive cards' 1px dim rim).

### 2b. Focus marker

When `sd.flags & SCENE_FLAG_HAS_FOCUS` is set (meaning this scene contains
the currently focused surface), a 6×6 bright green dot is drawn at offset
(4,4) from the card's top-left corner.

```
┌──●──────────────────┐
│                     │
│   (card content)    │
│                     │
└─────────────────────┘
```

Color: `0x0080FF80` — distinct from selection cyan (`0x0080e0ff`) and
active rim (`0x004090c0`). Only the active scene can have HAS_FOCUS in
current V1, but the flag is checked generically.

### 2c. Tile-count accent bar

When `sd.frame_count > 1` (scene has at least 2 visible, non-minimized,
non-dead frames), a 3px-tall light violet bar is drawn 2px below the card
top section, horizontally centered with 16px margin on each side.

```
┌─────────────────────┐
│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│  ← card top (ATLAS_CARD_TOP_H)
│ ═══════════════════  │  ← tile-count bar (3px, light violet)
│                     │
│ [frame] [frame]     │  ← frame indicator blocks at bottom
└─────────────────────┘
```

Color: `0x00C0C0FF` — light violet, distinct from frame block colors and
focus marker green. Only appears when multiple visible frames are tiled
in the same scene.

### 2d. Hidden/minimized/dead frame exclusion

The snapshot capture (`atlas_capture_snapshot`) already skips:
- Minimized frames (`FRAME_FLAG_MINIMIZED`)
- Dead surfaces (`!surface_is_alive`)
- Tombstoned surfaces (`is_tombstoned`)
- Closing/Destroyed/Hidden lifecycle states
- Stale generation surfaces

The `frame_count` value used for the tile-count bar naturally excludes
these — it only counts visible, alive, tiled frames.

---

## 3. Visible-Frame Counting Rule

```
frame_count = 0
for each frame in FRAMES:
    if frame.scene_id != current_scene:  skip
    if frame.flags & FRAME_FLAG_MINIMIZED: skip
    if !surface_is_alive(active_tab_surface): skip
    if is_tombstoned(active_tab_surface): skip
    if lifecycle_state in [Closing, Destroyed, Hidden]: skip
    if focus_ref_generation is stale: skip
    increment frame_count
```

This is unchanged from the existing `atlas_capture_snapshot` logic. The
polish just adds a visual bar when `frame_count > 1`.

---

## 4. Marker Changes

### New markers (budgeted)

| Marker | Budget | Condition |
|--------|--------|-----------|
| `[atlas.preview.focus_marker] scene=N` | 8 | Card has `SCENE_FLAG_HAS_FOCUS` |
| `[atlas.preview.tile_count] scene=N frames=M` | 8 | Card has `frame_count > 1` |

### New marker (budgeted)

| Marker | Budget | Condition |
|--------|--------|-----------|
| `[atlas.preview.polish] scene=N active=0/1 focus=0/1 frames=M` | 8 | Every card, every render (first 8 only) |

### Unchanged pre-existing markers

| Marker | Trigger |
|--------|---------|
| `[atlas.visual.active] scene=N` | Card is active scene |
| `[atlas.visual.selected] scene=N` | Card is nav-selected |
| `[atlas.visual.card] scene=N color=0x...` | Card fill color |
| `[atlas.visual.flags] scene=N flags=0x... frames=M` | Scene flags proof |
| `[atlas.visual.pinned] scene=N` | Card has pinned flag |
| `[atlas.visual.accent] scene=N accent=M` | Card has accent color |
| `[atlas.visual.reject] scene=N reason=...` | Accent OOB rejection |
| `[shell.atlas.render]` | Atlas render |
| `[atlas.preview.refresh] scenes=N` | Atlas refresh |

---

## 5. Build Result

**PASS.** `./scripts/entrypoint_build.sh` succeeds. No new warnings.
No ABI/kernel/renderer-policy changes.

---

## 6. Remaining Atlas Gaps

| Gap | Priority | Requires |
|-----|----------|----------|
| **No text rendering on cards** | Low — current color/blocks convey structure without text | Text/font rendering (blocked — no font renderer exists) |
| **Card labels are byte arrays, not drawn** | Low — V1 uses index-based labels for debugging | Text/font rendering or number→color mapping |
| **Frame blocks don't show which surface is focused** | Medium — focus marker per scene is sufficient for V1 | Per-frame focus indicator within a scene |
| **No accent color on frame blocks** | Low — frame blocks use zoom/minimized/normal colors | Per-frame accent query |
| **Atlas only opens on F10** | Low — V1 toggle is sufficient | Keybinding or gesture integration |
| **Polished only when Atlas overlay is visible** | N/A — polish is rendered on every `atlas_render_stub` call | No change needed |
