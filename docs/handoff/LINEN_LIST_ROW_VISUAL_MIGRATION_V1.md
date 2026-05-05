# LINEN_LIST_ROW_VISUAL_MIGRATION_V1

**Status:** Complete — built, committed.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.

---

## Summary

Migrated Linen object list rows from the old per-row full-width kind fill pattern to the Silk list row visual canon (shared list background + left accent bars + selected row highlight). No sexdisplay changes. No ABI/storage/text changes.

### Visual change

| Before | After |
|--------|-------|
| rect 0: header | rect 0: header (unchanged) |
| rect 1-7: per-row full-width kind fills (selected=bright, non-selected=muted) | rect 1: shared list background (`0x000C1420` dark slate) |
| — | rect 2: selected row highlight (full-width bright accent, OOB-guarded) |
| — | rect 3-7: left accent bars (5px wide, kind-colored) |

Number of visible rows with visual treatment: 7 → 5 (accent bar budget).
Rows beyond 5 emit `[linen.row_visual.skip]` but remain in the model.

### rect_index allocation (before → after)

```
Before (MAX_RECTS=8):           After (MAX_RECTS=8):
  0: header                       0: header
  1-7: full-width kind fills      1: shared list background
                                  2: selected row highlight
                                  3-7: left accent bars
```

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | ~25 lines — constants updated + render function rewritten |
| `docs/handoff/LINEN_LIST_ROW_VISUAL_MIGRATION_V1.md` | New handoff doc |

---

## Changes Detail

### 1. Constants (lines 471-490)

| Constant | Old | New | Change |
|----------|-----|-----|--------|
| `LINEN_LIST_ROW_RECTS` | `7` | — | Removed (no longer needed) |
| `LINEN_LIST_ACCENT_BARS` | — | `5` | Added |
| `LINEN_LIST_BG_COLOR` | — | `0x000C1420` | Added (dark slate) |
| `LINEN_ACCENT_BAR_W` | — | `5` | Added |
| `LINEN_SURFACE_VISUAL_H` | `220` (based on 7 rows) | `168` (based on 5 accent bars) | Updated |

### 2. Render function (lines 528-634)

Rewritten `linen_render_object_list()` to implement the canon:

- **rect 0 (header):** Unchanged — full-width bar using `linen_selected_object_accent()`.
- **rect 1 (list background):** New — single `0x000C1420` dark slate rect behind all rows.
- **rect 3-7 (accent bars):** New — 5px-wide bars at left edge. Non-selected rows use `linen_kind_color()`, selected row uses `header_color`. Only the first 5 visual rows get accent bars (budget-limited by `LINEN_LIST_ACCENT_BARS`).
- **rect 2 (selected highlight):** New — full-width bright accent at the selected row's position. Drawn via `Option<u32>` tracked during iteration. If the selected object is not found in the visible row set, emits `[linen.row.reject]`.

### 3. Proof markers

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[linen.object_list.render]` | 1 | Top of function | Always |
| `[linen.selection_visual.header]` | 1 | After header color | Always |
| `[linen.bg_rect]` | 1 | After list background | Always |
| `[linen.object_list.row]` | 8 | Per-object loop | Each visible object |
| `[linen.object_list.skip]` | 8 | Per-object loop | Past MAX_ROWS limit |
| `[linen.row_visual.accent]` | 5 | Accent bar call | Within accent budget |
| `[linen.row_visual.skip]` | 8 | Accent bar else | Past accent budget |
| `[linen.row_visual.selected]` | 1 | After iteration | Selected row found |
| `[linen.row.reject]` | 1 | After iteration | Selected row not found |
| `[linen.object_select.current]` | 1 | End of function | Always |
| `[linen.object_list.done]` | 1 | End of function | Always |

**Removed:** `[linen.row_visual.rect]` (old full-width fills).

---

## Selected Row OOB Behavior

The selected row highlight (rect 2) is suppressed if `SELECTED_LINEN_OBJECT_ID` does not match any visible row. This can happen if:
- The selected object was deleted/scrolled out of the visible set
- `SELECTED_LINEN_OBJECT_ID` is 0 (no selection)

On suppression, `[linen.row.reject]` is emitted with the selected object ID and reason. No silent clamping or wrapping occurs.

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Linen cannot fit canon within MAX_RECTS=8 | 8 rects used: 0+1+2+3+4+5+6+7 = 8. Fits exactly. | ❌ Not triggered |
| S2 | Selected-row state is unclear or unsafe | `selected_row_pos: Option<u32>` tracked during iteration. Only used if `Some`. | ❌ Not triggered |
| S3 | Bounds checks weakened | `LINEN_LIST_ACCENT_BARS = 5` is a compile-time constant. Surface height derived from it. sexdisplay clamps fill rects. | ❌ Not triggered |
| S4 | Migration requires sexdisplay changes | Only `servers/silk-shell/src/main.rs` changed. No sexdisplay edits. | ❌ Not triggered |
| S5 | Migration requires storage/model redesign | No sexstore/sexshop/sex-pdx changes. No data model changes. | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## References

- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — the canon this migration implements
- `QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md` — first canonical implementation (command palette)
- `L3A_FIX_LINEN_ROW_VISUAL_HEIGHT_V1.md` — previous Linen row fix (height adjustment)
- `servers/silk-shell/src/main.rs` — all list renderers live here

---

*End of LINEN_LIST_ROW_VISUAL_MIGRATION_V1.md*
