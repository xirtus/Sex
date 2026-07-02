# L2: Multi-Rect Display — Existing Op Implementation

**Status:** Complete  
**Phase:** L2 — implements L1 design  
**Files changed:** `servers/sexdisplay/src/main.rs`, `servers/silk-shell/src/main.rs`  
**Build:** `[SEXOS ENTRYPOINT] success`

---

## What Changed

### sexdisplay — Surface struct

`MAX_RECTS = 8` added. Fill rect storage changed from a single scalar set to arrays:

| Old field | New field | Type |
|-----------|-----------|------|
| `fill_sx: i32` | `fill_sx: [i32; MAX_RECTS]` | per-slot x offset |
| `fill_sy: i32` | `fill_sy: [i32; MAX_RECTS]` | per-slot y offset |
| `fill_sw: u32` | `fill_sw: [u32; MAX_RECTS]` | per-slot width |
| `fill_sh: u32` | `fill_sh: [u32; MAX_RECTS]` | per-slot height |
| `fill_color: u32` | `fill_color: [u32; MAX_RECTS]` | per-slot ARGB |
| `fill_active: bool` | `fill_count: u8` | 0 = no rects; N = highest set index + 1 |

`SURFACE_EMPTY` updated to zero-initialize all arrays.  
Both 0xEC surface creation paths updated.

### sexdisplay — 0xEF arg2 encoding (backward compatible)

**New format:** `arg2 = (rect_index << 56) | (color_rgb << 32) | (sh << 16) | sw`

`rect_index` lives in bits 56-59 — the top nibble of the `color` field's alpha byte.
All existing callers use `0x00RRGGBB` colors, so bits 56-63 = 0 → `rect_index = 0`.
No existing call site changed.

New extraction in handler:
```rust
let color      = ((msg.arg2 >> 32) & 0x00FF_FFFF) as u32;
let rect_index = ((msg.arg2 >> 56) & 0xF) as usize;
```

If `rect_index >= MAX_RECTS`: emit `[display.fill_rect.reject.index]`, break.  
Otherwise write `slot.fill_sx[rect_index]` etc. and bump `fill_count` to `max(fill_count, rect_index+1)`.

### sexdisplay — fill_rect_color() (painter's order)

Iterates `0..fill_count` rects. Slots with `sw=0` or `sh=0` skipped.
Last matching rect per pixel wins (painter's algorithm). `base_color` returned if no rect matches.

Bounds checks unchanged: L1 design's two-level clamping still applies per call.

### Proof markers (sexdisplay)

| Marker | Condition |
|--------|-----------|
| `[display.fill_rect.set] sid=N index=N` | Rect successfully written to slot |
| `[display.fill_rect.reject.index] sid=N index=N` | rect_index >= MAX_RECTS |

### silk-shell — linen_render_object_list()

Added `LINEN_LIST_ROW_RECTS = 7` constant (MAX_RECTS=8 minus header at slot 0).

Header rect unchanged: slot 0, accent color, `(sy=0, sx=0, sh=LINEN_LIST_HEADER_H, sw=w)`.

Per-object row rect (slots 1-7):
- y offset: `LINEN_LIST_HEADER_H + row_idx * (LINEN_LIST_ROW_H + LINEN_LIST_ROW_GAP)`
- color: selected → `header_color`; otherwise → `linen_kind_color(obj.kind)`
- Falls through to `[linen.row_visual.skip]` if row_idx >= LINEN_LIST_ROW_RECTS

Existing proof markers (`[linen.object_list.row]`, `[linen.object_list.skip]`, etc.) preserved.

### Proof markers (silk-shell)

| Marker | Condition |
|--------|-----------|
| `[linen.row_visual.rect] index=N id=N kind=K color=C selected=T/F` | Row rect sent |
| `[linen.row_visual.skip] id=N reason=rect_budget` | Row beyond slot budget |

---

## Rect-Index Packing (shell side)

```rust
// New multi-rect 0xEF call pattern:
let arg2 = (rect_index << 56)
    | ((row_color as u64) << 32)
    | ((sh as u64) << 16)
    | (sw as u64);
pdx_call(SLOT_DISPLAY, 0xEF, surface_id, (sy << 32) | sx, arg2);
```

Backward compatibility: existing single-rect calls use `(color << 32) | (sh << 16) | sw`
which has bits 56-63 = 0 → rect_index = 0. Behavior identical to old single-rect path.

---

## Sexdisplay Bounds Proof (unchanged from L1)

Three levels of clamping remain intact:
1. `sw = sw.min(slot.w); sh = sh.min(slot.h)` — rect can't exceed surface dimensions
2. `fill_sx = sx.clamp(0, max_sx)` — rect position clamped so rect fits in surface
3. Surface geometry clamped to framebuffer in `clamp_surface()`

All three levels apply per-rect-index call, identical to old single-rect behavior.

---

## Linen Row Visual Behavior

| Row index | Rect slot | Color | Height |
|-----------|-----------|-------|--------|
| Header | 0 | `linen_selected_object_accent()` | `LINEN_LIST_HEADER_H` = 28px |
| Row 0 | 1 | accent if selected, else `linen_kind_color()` | `LINEN_LIST_ROW_H` = 24px |
| Row 1 | 2 | accent if selected, else kind color | 24px |
| … | … | … | … |
| Row 6 | 7 | accent if selected, else kind color | 24px |
| Row 7+ | — | `[linen.row_visual.skip]` proof only | — |

Total fill rects per render: 1 (header) + min(object_count, 7) row rects.

---

## No-Touch Invariants (all preserved)

- No new PDX opcode  
- No `crates/sex-pdx/` edit  
- No kernel edit  
- sexdisplay remains dumb renderer (stores and composites rects; no row semantics)  
- Framebuffer bounds checks unchanged  
- Quil/Command palette/Bell/Mesh/Collar 0xEF calls unmodified (existing single-rect behavior)  
- Atlas overlay 0xEF calls unmodified (rect_index=0, colors are 0x00RRGGBB)

---

## Remaining L-series Work

| Phase | Scope |
|-------|-------|
| L3 audit | Audit L2 implementation for correctness before adding Quil/command rows |
| L3/L4 | Quil buffer row highlights (same pattern as Linen) |
| L5 | Command palette row highlights |
| L6+ | Bell/Mesh/Collar row highlights |
