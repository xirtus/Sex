# L4: Quil Buffer Row Visuals

**Status:** Handoff (code + docs)
**Date:** 2026-05-05
**Purpose:** Add visual row highlight fill rects to the Quil buffer list,
mirroring the Linen row visual pattern (L2/L3A). Each visible buffer row
gets its own 0xEF fill rect with color from `quil_buffer_kind_color()`.

## Verdict

```
╔══════════════════════════════════════════════════════════════════╗
║                    PASS_L4                                      ║
╠══════════════════════════════════════════════════════════════════╣
║ Build:                 PASSES (ISO produced)                     ║
║ Forbidden areas:       CLEAN                                    ║
║ Surface height change: NONE (640x480 already sufficient)        ║
║ Row rects:             7 (plus header = 8 total, MAX_RECTS=8)   ║
║ Sexdisplay changes:    NONE                                     ║
║ ABI changes:           NONE                                     ║
╚══════════════════════════════════════════════════════════════════╝
```

## Changes

### Constants (`servers/silk-shell/src/main.rs`)

| Constant | Value | Location |
|----------|-------|----------|
| `QUIL_LIST_ROW_RECTS` | `7` | After `QUIL_LIST_HEADER_H` |
| `QUIL_LIST_ROW_H` | `24` | After `QUIL_LIST_ROW_RECTS` |
| `QUIL_LIST_ROW_GAP` | `2` | After `QUIL_LIST_ROW_H` |

### Color Helper

Added `quil_buffer_kind_color()` — deterministic accent color per QuilBufferKind:

| Kind | Color | Hex |
|------|-------|-----|
| Text | Grey | `0x00808080` |
| Code | Green-teal | `0x0040A060` |
| DesignNote | Blue | `0x004060C0` |
| ReviewNote | Orange | `0x00C06040` |
| Diagnostic | Magenta | `0x00C04080` |
| BuildOutput | Brown | `0x00806040` |
| AgentTask | Steel blue | `0x006080C0` |
| LinenObjectView | Violet | `0x00A060C0` |

### Render Function

`quil_render_buffer_list()` updated to emit visual row fill rects:

- **Header** (rect_index=0): `QUIL_LIST_HEADER_COLOR` at (0,0), height `QUIL_LIST_HEADER_H`
- **Rows** (rect_index=1..7): Color from `quil_buffer_kind_color()`, positioned below header with `QUIL_LIST_ROW_H + QUIL_LIST_ROW_GAP` spacing
- **Budget check**: If more buffers than `QUIL_LIST_ROW_RECTS` (7), excess rows emit proof markers only

### Proof Markers

| Marker | Type | When |
|--------|------|------|
| `[quil.buffer_list.render]` | Existing | W/H measurement |
| `[quil.buffer_list.row]` | Existing | Per-buffer proof row |
| `[quil.buffer_list.skip]` | Existing | Row budget exceeded |
| `[quil.buffer_list.done]` | Updated | Now includes rect count |
| `[quil.row_visual.rect]` | **New** | Per-row fill rect emitted |
| `[quil.row_visual.skip]` | **New** | Row visual budget exceeded |

### Surface Sizing

**No height change needed.** Quil surface is 640×480 (480px tall). Total visual
height for header + 7 rows = 28 + 7(24+2) = 210px, well within 480px.

## Files Changed

- `servers/silk-shell/src/main.rs` — constants, color helper, render function
- `docs/handoff/L4_QUIL_BUFFER_ROW_VISUALS_V1.md` — this document

## Verification

- **Build:** `./scripts/entrypoint_build.sh` → `[SEXOS ENTRYPOINT] success`
- **No changes:** kernel/, sex-pdx/, sexdisplay/, linen/, quil/
- **No new selection model:** Quil has no selection state (unlike Linen's J/K)
- **Row colors:** Deterministic per `QuilBufferKind`, no selection highlight
- **rect_index pattern:** Mirrors Linen exactly (bits 56-63 of arg2, slot 0=header)
- **Backward compatible:** rect_index=0 for existing header callers
