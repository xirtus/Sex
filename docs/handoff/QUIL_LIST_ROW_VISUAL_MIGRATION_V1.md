# QUIL_LIST_ROW_VISUAL_MIGRATION_V1

**Status:** Complete — built, committed.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.

---

## Summary

Migrated Quil buffer list rows from the old per-row full-width kind fill pattern to the Silk list row visual canon (shared list background + left accent bars). No sexdisplay changes. No ABI/storage/text/editor changes.

### Visual change

| Before | After |
|--------|-------|
| rect 0: header | rect 0: header (unchanged) |
| rect 1-7: per-row full-width kind fills | rect 1: shared list background (`0x000C1420` dark slate) |
| — | rect 2: selected row highlight — **suppressed** (no Quil selection model) |
| — | rect 3-7: left accent bars (5px wide, buffer-kind-colored) |

### rect_index allocation (before → after)

```
Before (MAX_RECTS=8):           After (MAX_RECTS=8):
  0: header                       0: header
  1-7: full-width kind fills      1: shared list background
                                  2: selected row highlight — suppressed
                                  3-7: left accent bars
```

### Selected row behavior

Quil has **no buffer selection model** — no equivalent of `SELECTED_LINEN_OBJECT_ID` or `COMMAND_PALETTE_SELECTED`. The `quil_render_buffer_list()` function draws a static header and all rows use muted kind colors.

Per the canon: rect 2 (selected row highlight) is always suppressed, emitting `[quil.row.reject] reason=no_selection_model`. This is acceptable — the canon explicitly allows suppression when no selection exists. No silent clamping occurs.

When a future phase adds buffer selection to Quil, rect 2 must be un-suppressed with a proper OOB guard.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | ~25 lines — constants updated + render function rewritten |
| `docs/handoff/QUIL_LIST_ROW_VISUAL_MIGRATION_V1.md` | New handoff doc |

---

## Changes Detail

### 1. Constants (lines 648-663)

| Constant | Old | New | Change |
|----------|-----|-----|--------|
| `QUIL_LIST_ROW_RECTS` | `7` | — | Removed |
| `QUIL_LIST_ACCENT_BARS` | — | `5` | Added |
| `QUIL_LIST_BG_COLOR` | — | `0x000C1420` | Added (dark slate, matching Linen) |
| `QUIL_ACCENT_BAR_W` | — | `5` | Added |

Surface height unchanged (`SURFACE_201_H = 480` — Quil surface is larger than the buffer list, no height adjustment needed).

### 2. Render function (lines 856-928)

Rewritten `quil_render_buffer_list()` to implement the canon:

- **rect 0 (header):** Unchanged — static `QUIL_LIST_HEADER_COLOR` (blue-purple).
- **rect 1 (list background):** New — single `0x000C1420` dark slate rect behind all rows.
- **rect 2 (selected highlight):** Suppressed — `[quil.row.reject] reason=no_selection_model`.
- **rect 3-7 (accent bars):** New — 5px-wide bars at left edge. All rows use `quil_buffer_kind_color()` (muted kind color, since no selection exists).

### 3. Proof markers

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[quil.buffer_list.render]` | 1 | Top of function | Always |
| `[quil.bg_rect]` | 1 | After list background | Always |
| `[quil.row.reject]` | 1 | Selected highlight block | Always (no selection model) |
| `[quil.buffer_list.row]` | 8 | Per-buffer loop | Each visible buffer |
| `[quil.buffer_list.skip]` | 8 | Per-buffer loop | Past MAX_ROWS limit |
| `[quil.row_visual.accent]` | 5 | Accent bar call | Within accent budget |
| `[quil.row_visual.skip]` | 8 | Accent bar else | Past accent budget |
| `[quil.buffer_list.done]` | 1 | End of function | Always |

**Removed:** `[quil.row_visual.rect]` (old full-width fills), `rects_sent` accumulator.

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Quil cannot fit canon within MAX_RECTS=8 | 8 rects used: 0+1+2+3+4+5+6+7 = 8. Fits exactly (rect 2 suppressed but slot reserved). | ❌ Not triggered |
| S2 | Selected-row state absent would require model work | Rect 2 suppressed via reject marker. No model expansion needed. Canon explicitly allows suppression when no selection exists. | ❌ Not triggered |
| S3 | Bounds checks weakened | Same pattern as Linen/Command Palette. sexdisplay clamps remain active. | ❌ Not triggered |
| S4 | Migration requires sexdisplay changes | Only `servers/silk-shell/src/main.rs` changed. No sexdisplay edits. | ❌ Not triggered |
| S5 | Migration requires storage/editor redesign | No sexstore/sexshop/sex-pdx changes. No real Quil editor touched. | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Full Canon Compliance Status

| Surface | Status | Selected highlight | Notes |
|---------|--------|-------------------|-------|
| Command Palette | ✅ Canonical | ✅ Active (OOB-guarded) | First implementation |
| Linen object list | ✅ Canonical | ✅ Active (OOB-guarded) | Migrated this session |
| Quil buffer list | ✅ Canonical | ⛔ Suppressed (no selection model) | Migrated this session |
| Bell inbox | Not implemented | — | Must adopt canon on first implementation |

---

## References

- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — the canon this migration implements
- `QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md` — first canonical implementation
- `LINEN_LIST_ROW_VISUAL_MIGRATION_V1.md` — Linen migration (same pattern)
- `servers/silk-shell/src/main.rs` — all list renderers live here

---

*End of QUIL_LIST_ROW_VISUAL_MIGRATION_V1.md*
