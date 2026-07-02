# QUIL_COMMAND_PALETTE_ROW_VISUALS_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Replaced per-row full-width background fills in the command palette with a shared list background + kind-colored left accent bars per row + full-width selected row highlight. Fits within sexdisplay `MAX_RECTS=8`. No sexdisplay changes. No sex-pdx edits. No storage. No text.

### Visual behavior

| Element | Rect index | Description |
|---------|------------|-------------|
| **Header bar** | 0 | Full-width bar at top, using selected command's bright accent (unchanged) |
| **List background** | 1 | Single neutral dark slate rect behind all command rows |
| **Selected row highlight** | 2 | Full-width bright accent rect at the selected command's row position |
| **Left accent bars** | 3-7 | 5px-wide bars at left edge of each row, using `command_kind_color()` for non-selected rows or `header_color` for selected row |

### Visual change from previous

| Before | After |
|--------|-------|
| Each row had full-width kind-colored background fill (selected=bright, non-selected=muted) | Shared neutral background + 5px kind-colored accent bar per row. Selected row additionally gets full-width bright highlight. |

### rect_index allocation

```
MAX_RECTS=8 (sexdisplay, unchanged)
 0: header bar
 1: list background (neutral dark slate)
 2: selected row highlight (full-width, bright accent)
 3: row 0 accent bar
 4: row 1 accent bar
 5: row 2 accent bar
 6: row 3 accent bar
 7: row 4 accent bar
Spare: 0 (all 8 slots used — exactly fits)
```

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | ~35 lines — new constants + rewritten `palette_render_list()` |
| `docs/handoff/QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md` | New handoff doc |

---

## Changes Detail

### 1. New constants (after `PALETTE_LIST_ROW_GAP`)

```rust
/// Width of the left accent bar per command row, in pixels.
const PALETTE_ACCENT_BAR_W: u32 = 5;
/// Background color for the command palette list area (behind all rows).
const PALETTE_LIST_BG_COLOR: u32 = 0x00101820; // dark slate
```

### 2. Removed constant

```rust
// REMOVED: PALETTE_LIST_ROW_RECTS — no longer needed (row fills eliminated)
```

### 3. Rewritten `palette_render_list()`

The function now renders 8 fill rects (fitting `MAX_RECTS=8`):

**Rect 0 (header):** Unchanged — full-width bar at top using `command_palette_selected_accent()`.

**Rect 1 (list background):** New. Single `0x00101820` dark slate rect spanning all rows. Provides neutral contrast for accent bars and highlight.

**Rect 2 (selected row highlight):** New. Full-width bright accent rect at the selected command's row position. Suppressed with `[quil.palette.row.reject]` if `selected >= count`.

**Rects 3-7 (accent bars):** New. 5px-wide bars at left edge (`sx=0`) of each row. Non-selected rows use `command_kind_color()` (muted kind-specific). Selected row uses `header_color` (bright).

### 4. Proof markers

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[command_palette.render]` | 1 | Top of `palette_render_list()` | Always |
| `[command_palette.selection_visual.header]` | 1 | After header color determination | Always |
| `[command_palette.bg_rect]` | 1 | After list background rect | Always |
| `[quil.palette.row.reject]` | 1 | Selected row highlight block | Selected index out of bounds |
| `[command_palette.row_visual.selected]` | 5 | Selected row highlight block | Selected index valid |
| `[command_palette.row]` | 5 | Per-row loop | Each command row |
| `[command_palette.row_visual.accent]` | 5 | Per-row accent bar | Each accent bar rendered |
| `[command_palette.done]` | 1 | End of `palette_render_list()` | Always |

**Removed markers:** `[command_palette.row_visual.rect]` (old per-row fills), `[command_palette.row_visual.skip]` (no longer needed — all rows get accent bars).

---

## Shell/Display Ownership Boundary

| Responsibility | Owner | Verification |
|---------------|-------|-------------|
| Render list visuals | silk-shell (`palette_render_list`) | ✅ Uses 0xEF fill rects only |
| Surface rendering | sexdisplay (fill rect via 0xEF) | ✅ Unchanged — sexdisplay is renderer-only |
| Command model + selection | silk-shell (`Command` enum, `COMMAND_LIST`) | ✅ Unchanged |
| Palette lifecycle | silk-shell (`palette_show`, `toggle_command_palette`) | ✅ Unchanged |
| HID dispatch | silk-shell (keyboard handler) | ✅ Unchanged |

sexdisplay remains **renderer-only**: it receives only `0xEC` (surface create) and `0xEF` (fill rect) calls. No command model, no selection state, no lifecycle logic, no storage reads.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: [SEXOS ENTRYPOINT] success — ISO produced
```

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Changes sexdisplay | Only silk-shell/src/main.rs touched. No sexdisplay code changes. MAX_RECTS=8 unchanged. | ❌ Not triggered |
| S2 | Changes sex-pdx | No sex-pdx edits. No new opcodes. No protocol changes. | ❌ Not triggered |
| S3 | Changes storage | No sexstore/sexshop/sex-pdx touched. | ❌ Not triggered |
| S4 | Adds text rendering | Uses only 0xEF fill rects (bounded rects only). No text, no font, no strings. | ❌ Not triggered |
| S5 | Changes kernel | No kernel edits. No syscall changes. | ❌ Not triggered |
| S6 | Reduces visual safety | Selected row highlight suppressed with reject marker if OOB. Accent bars bounded by count. No silent wrapping. | ❌ Not triggered |
| S7 | Exceeds rect budget | 8 rects total fits MAX_RECTS=8. Verified no overflow. | ❌ Not triggered |
| S8 | sexdisplay gains semantics | sexdisplay unchanged. Renderer-only preserved. | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Diff Summary

```
silk-shell/src/main.rs:
  + PALETTE_ACCENT_BAR_W: u32 = 5          new constant
  + PALETTE_LIST_BG_COLOR: u32 = 0x00101820  new constant
  - PALETTE_LIST_ROW_RECTS: u8 = 5          removed (unused after rewrite)
  ~ palette_render_list(): rewritten         rect allocation 0-7 instead of 0-5
    - Full-width per-row background fills (old rects 1-5)
    + List background rect (rect_index=1)
    + Selected row highlight (rect_index=2)
    + Per-row left accent bars (rect_indices 3-7)
    ~ Proof markers: added bg_rect, row_visual.selected, row_visual.accent, row.reject
    ~ Proof markers: removed row_visual.rect, row_visual.skip

Total: ~3 lines added, ~3 lines removed (constants) + ~35 lines rewritten (function)
```

---

## References

- `palette_render_list()` (line ~6086) — main render function
- `command_kind_color()` (line ~6049) — muted kind-specific colors
- `command_palette_selected_accent()` (line ~6061) — bright selection accent
- `PALETTE_LIST_ROW_H` (line ~5792) — row height (24px)
- `PALETTE_LIST_ROW_GAP` (line ~5794) — row gap (2px)
- `PALETTE_LIST_HEADER_H` (line ~5790) — header height (28px)
- `COMMAND_LIST` (line ~5818) — 5 command definitions
- sexdisplay `MAX_RECTS=8` (`servers/sexdisplay/src/main.rs:24`) — unchanged

---

*End of QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md*