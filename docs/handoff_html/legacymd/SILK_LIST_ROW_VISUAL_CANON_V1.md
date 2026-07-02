# SILK_LIST_ROW_VISUAL_CANON_V1

**Status:** Docs-only canon. No code changed.
**Build:** `[SEXOS ENTRYPOINT] success` (no code changes).
**Date:** 2026-05-05

---

## Purpose

Define one shared visual language for Silk list-style surfaces (Linen, Quil, Command Palette, future Bell inbox). Prevent future agents from reintroducing per-row full-width kind fills or sexdisplay row semantics, and provide a migration target for surfaces that still use the old pattern.

---

## Canonical rect_index Allocation

All list-style surfaces MUST use the following rect_index allocation, which fits within sexdisplay `MAX_RECTS=8`:

| Index | Purpose | Description |
|-------|---------|-------------|
| **0** | Header / top chrome | Surface title bar, selected-object accent color |
| **1** | Shared list background | Single neutral rect behind all rows |
| **2** | Selected row highlight | Full-width bright accent rect at selected row's y position. **Suppressed with reject marker if selected index is out of bounds.** |
| **3..N** | Per-row left accent bars | 5px-wide bars at left edge (`sx=0`) of each row. Non-selected rows use muted kind color; selected row uses bright accent matching rect 2. |

### Slot Budget

```
MAX_RECTS = 8 (sexdisplay, immutable)
  rect 0: header        (1 slot)
  rect 1: list bg       (1 slot)
  rect 2: selected hl   (1 slot)
  rect 3-7: accent bars (5 slots)
  ─────────────────────────────
  Total: 8              (exactly fits)
  Spare: 0
```

For surfaces with >5 visible rows, rect_index 3-7 covers the first 5 rows. Rows beyond index 5 must emit `[surface.row_visual.skip]` markers and cannot get accent bars without a STOP FIRST review to increase `MAX_RECTS` in sexdisplay.

---

## Color Roles

| Role | Source | Example |
|------|--------|---------|
| **Header color** | Selected row's kind accent (bright) | `linen_selected_object_accent()`, `command_palette_selected_accent()` |
| **List background** | Surface-specific neutral dark | `PALETTE_LIST_BG_COLOR = 0x00101820` (command palette), Linen/Quil to adopt |
| **Non-selected accent bar** | Muted kind color | `linen_kind_color()`, `command_kind_color()`, `quil_buffer_kind_color()` |
| **Selected row highlight** | Same as header color (bright kind accent) | Derived from selected object/command/buffer |
| **Selected accent bar** | Same as header color (overlay on highlight) | Same bright accent |

### Color Derivation Rule

Colors MUST be derived from the shell's semantic model (object kind, command enum, buffer kind), NOT from sexdisplay. sexdisplay receives only opaque ARGB fill rect values via `0xEF` and must not infer row semantics.

---

## Selected-Row Behavior

1. If `selected_index >= visible_row_count`, **suppress rect 2 entirely** and emit `[surface.row.reject]`.
2. **Do not wrap or clamp silently.** A reject marker must be emitted every time suppression occurs.
3. The selected row highlight (rect 2) is full-width, using the bright kind accent color.
4. The selected row's accent bar (rect 3+sel_idx) also uses the bright accent, overlaying the highlight at the left 5px. Since both use the same color, there is no visible seam.

---

## Ownership Boundaries

| Responsibility | Owner | Constraint |
|---------------|-------|------------|
| Row semantics (kind, selection, count) | silk-shell (model) | sexdisplay must not read or infer |
| Color derivation | silk-shell (model → ARGB) | sexdisplay receives only opaque ARGB |
| Fill rect rendering | silk-shell → sexdisplay (0xEF) | Bounded by `MAX_RECTS=8` |
| Pixel compositing | sexdisplay (scanout) | Renderer-only; no policy decisions |
| Surface geometry | silk-shell (0xEC upsert) | sexdisplay applies clamped bounds |

sexdisplay remains **renderer-only**: it receives `0xEC` (surface create/geometry) and `0xEF` (fill rect) calls only. No row semantics, no selection state, no lifecycle logic, no storage reads.

---

## Current Surface Status

| Surface | Pattern | Follows Canon? | Migrate Needed? |
|---------|---------|---------------|-----------------|
| **Command Palette** (SURFACE_ID_COMMAND_PALETTE=0x98) | rect 0=header, rect 1=list bg, rect 2=selected hl, rect 3-7=accent bars | ✅ YES | No |
| **Linen object list** (SURFACE_ID_LINEN=200) | rect 0=header, rect 1-7=per-row full-width kind fills | ❌ NO — uses old pattern | Yes (see Migration Checklist) |
| **Quil buffer list** (SURFACE_ID_QUIL=201) | rect 0=header, rect 1-7=per-row full-width kind fills | ❌ NO — uses old pattern | Yes (see Migration Checklist) |
| **Bell inbox** (future) | Not yet implemented | — | Must adopt canon on first implementation |
| **Atlas overview** (SURFACE_ID_ATLAS=0x90) | Grid of cards, not list | N/A (not a list) | No — grid surface, separate pattern |

---

## Forbidden Changes

The following are **forbidden** without a STOP FIRST review and explicit approval:

1. **Per-row full-width kind fills as default.** The old pattern (Linen/Quil) where each non-selected row gets a full-width kind-colored background is deprecated. New list surfaces MUST use shared background + accent bars.
2. **sexdisplay inferring row semantics.** sexdisplay must not read, store, or act on command kinds, object kinds, buffer kinds, or selection state. It renders opaque rects.
3. **Adding text/string/heap requirements to the visual canon.** Row visuals are fill-rect-only. Text rendering requires a separate design gate.
4. **Silent wrap/clamp of selected row highlight.** If selected index is out of bounds, suppress rect 2 and emit a reject marker. No silent correction.
5. **Exceeding MAX_RECTS=8 without sexdisplay change review.** Increasing MAX_RECTS requires a STOP FIRST review covering sexdisplay surface struct size, scanout performance, and renderer-only constraint.
6. **Removing proof markers.** Each rect must have a corresponding proof marker for audit. Markers may be renamed/restructured but not removed without replacement.

---

## Recommended Colors for Shared Background (rect 1)

| Surface | Recommended Color | Rationale |
|---------|------------------|-----------|
| Command Palette | `0x00101820` (dark slate) | Already adopted — `PALETTE_LIST_BG_COLOR` |
| Linen | `0x000C1420` (slightly darker slate) | Distinct from command palette, complements Linen header teal-green |
| Quil | `0x000C1420` (slightly darker slate) | Same as Linen; Quil header purple provides surface distinction |
| Bell inbox | `0x00101820` (dark slate) or `0x000C1420` | TBD based on header color context |

These are RECOMMENDATIONS, not requirements. Each surface may tune its neutral background to complement its header color, as long as:
- It is visibly neutral/dark (not a kind color)
- It provides sufficient contrast for accent bars (5px wide, ~60% brightness kind colors)
- It is a flat ARGB value, not derived from sexdisplay state

---

## Migration Checklist (for future agents)

### Linen (`linen_render_object_list()`)

- [ ] Add `LINEN_LIST_BG_COLOR: u32 = 0x000C1420` constant
- [ ] Add `LINEN_ACCENT_BAR_W: u32 = 5` constant
- [ ] Remove `LINEN_LIST_ROW_RECTS` constant
- [ ] Rewrite render loop:
  - Old: rect 1-7 = per-row full-width fills (selected=bright, non-selected=muted)
  - New: rect 1 = list background, rect 2 = selected highlight, rect 3-7 = accent bars
- [ ] Update `LINEN_SURFACE_VISUAL_H` calculation if row count changes
- [ ] Proof markers:
  - Remove `[linen.row_visual.rect]`, `[linen.row_visual.skip]`
  - Add `[linen.bg_rect]`, `[linen.row_visual.selected]`, `[linen.row_visual.accent]`
  - Add `[linen.row.reject]` for OOB selection guard
- [ ] Verify `MAX_RECTS=8` budget

### Quil (`quil_render_buffer_list()`)

Same as Linen with `QUIL_` prefix:
- [ ] Add `QUIL_LIST_BG_COLOR`, `QUIL_ACCENT_BAR_W` constants
- [ ] Remove `QUIL_LIST_ROW_RECTS`
- [ ] Rewrite render loop (same pattern)
- [ ] Update proof markers (same pattern)
- [ ] Verify MAX_RECTS=8 budget

### Shared concerns

- [ ] Accent bar width (5px) consistent across all surfaces
- [ ] List background colors visually compatible with header colors
- [ ] Selected row highlight uses same color as header (bright kind accent)
- [ ] Build: `./scripts/entrypoint_build.sh`

---

## Proof / Audit Grep Commands

```bash
# Verify all list surfaces use rect_index pattern
rg "rect_index" servers/silk-shell/src/main.rs

# Verify no per-row full-width kind fills remain (after migration)
rg "row_visual\.rect" servers/silk-shell/src/main.rs

# Verify all surfaces have selected row highlight with OOB guard
rg "row_visual\.selected\|row\.reject" servers/silk-shell/src/main.rs

# Verify all surfaces have list background rect
rg "bg_rect" servers/silk-shell/src/main.rs

# Verify MAX_RECTS=8 unchanged in sexdisplay
rg "MAX_RECTS" servers/sexdisplay/src/main.rs

# Verify no sexdisplay row semantics
rg -n "kind\|selected\|row\|command\|linen\|quil" servers/sexdisplay/src/main.rs \
  | grep -v "fill_s\|AUTH\|owner\|redraw\|FOCUSED\|scanout\|composite\|FB\|surface\|display\."
```

---

## Future Surfaces That May Adopt This Canon

- **Bell inbox** (future): Inbox event list — should adopt canon from first implementation.
- **sexshop browser** (future G gate): Object/package browser — should adopt canon.
- **SexGemini compiler diagnostics** (future): Diagnostic list — should adopt canon.
- **Any Silk shell popover/list** (future): Must adopt canon unless STOP FIRST approves exception.

Non-list surfaces (Atlas grid, Bell waveform, fullscreen editor) are explicitly NOT covered by this canon. They have different layout needs (grid, waveform, freeform) that require separate visual designs.

---

## References

- `QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md` — first canonical implementation (command palette)
- `L3A_FIX_LINEN_ROW_VISUAL_HEIGHT_V1.md` — Linen current row pattern (pre-canon, needs migration)
- `L4_QUIL_BUFFER_ROW_VISUALS_V1.md` — Quil current row pattern (pre-canon, needs migration)
- `SILK_DE_GLASS_VISUAL_LANGUAGE.md` — broader visual language (not row-specific)
- `servers/silk-shell/src/main.rs` — all three list renderers live here
- `servers/sexdisplay/src/main.rs:24` — `MAX_RECTS=8` (immutable without STOP FIRST)

---

*End of SILK_LIST_ROW_VISUAL_CANON_V1.md*
