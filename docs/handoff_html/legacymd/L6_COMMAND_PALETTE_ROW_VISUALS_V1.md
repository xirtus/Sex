# L6: Command Palette Row Visuals

**Status:** Handoff (code + docs)
**Date:** 2026-05-05
**Purpose:** Add visual row highlight fill rects to the command palette,
mirroring the Linen (L2/L3A) and Quil (L4) row visual pattern. Selected
command row gets header accent color; non-selected rows get muted
per-command colors.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║                    PASS_L6                                      ║
╠══════════════════════════════════════════════════════════════════╣
║ Build:                 PASSES (ISO produced)                     ║
║ Forbidden areas:       CLEAN                                    ║
║ Surface height change: NONE (480x240 already sufficient)        ║
║ Row rects:             5 (plus header = 6 total, MAX_RECTS=8)   ║
║ Sexdisplay changes:    NONE                                     ║
║ ABI changes:           NONE                                     ║
║ Selected row:          Accent color (matching header)            ║
║ Non-selected rows:     Muted per-command color                  ║
╚══════════════════════════════════════════════════════════════════╝
```

## Changes

### Constants (`servers/silk-shell/src/main.rs`)

| Constant | Value | Location |
|----------|-------|----------|
| `PALETTE_LIST_HEADER_H` | `28` | After `COMMAND_PALETTE_BOOT_H` |
| `PALETTE_LIST_ROW_RECTS` | `5` | After `PALETTE_LIST_HEADER_H` |
| `PALETTE_LIST_ROW_H` | `24` | After `PALETTE_LIST_ROW_RECTS` |
| `PALETTE_LIST_ROW_GAP` | `2` | After `PALETTE_LIST_ROW_H` |

### Color Helper

Added `command_kind_color()` — muted color per Command for non-selected rows:

| Command | Color | Hex |
|---------|-------|-----|
| OpenSelectedInQuil | Muted amber | `0x00605020` |
| FocusLinen | Muted green | `0x00206040` |
| FocusQuil | Muted cyan | `0x00206060` |
| SceneNext | Muted indigo | `0x00303060` |
| OpenAtlas | Muted violet | `0x00503060` |

Selected row uses the same accent color as the header (from `command_palette_selected_accent()`),
mirroring the Linen pattern where the selected row = header accent color.

### Render Function

`palette_render_list()` updated to emit visual row fill rects:

- **Header** (rect_index=0): `command_palette_selected_accent()` at (0,0), height `PALETTE_LIST_HEADER_H`
- **Rows** (rect_index=1..5):
  - Selected row: accent color (bright, matching header)
  - Non-selected rows: muted color from `command_kind_color()`
  - Positioned below header with `PALETTE_LIST_ROW_H + PALETTE_LIST_ROW_GAP` spacing
- Header height replaced hardcoded 28 with `PALETTE_LIST_HEADER_H` constant

### Proof Markers

| Marker | Type | When |
|--------|------|------|
| `[command_palette.render]` | Existing | W/H measurement |
| `[command_palette.selection_visual.header]` | Existing | Header color |
| `[command_palette.row]` | Existing | Per-command proof row |
| `[command_palette.done]` | Updated | Now includes rect count |
| `[command_palette.row_visual.rect]` | **New** | Per-command fill rect emitted |
| `[command_palette.row_visual.skip]` | **New** | Row visual budget exceeded |

### Surface Sizing

**No height change needed.** Command palette is 480×240 (240px tall). Total visual
height for header + 5 rows = 28 + 5(24+2) = 158px, well within 240px.

## Files Changed

- `servers/silk-shell/src/main.rs` — constants, color helper, render function
- `docs/handoff/L6_COMMAND_PALETTE_ROW_VISUALS_V1.md` — this document

## Verification

- **Build:** `./scripts/entrypoint_build.sh` → `[SEXOS ENTRYPOINT] success`
- **No changes:** kernel/, sex-pdx/, sexdisplay/, linen/, quil/
- **Selected row highlight:** Bold accent color matching header
- **Non-selected rows:** Muted per-command color (distinct from header)
- **rect_index pattern:** Identical to Linen and Quil (bits 56-63 of arg2)
- **5 commands = 5 row rects:** Fits in MAX_RECTS=8 (1 header + 5 rows = 6 total)
