# QUIL_MINIMAL_TEXT_SURFACE_V1

**Status:** Implemented (with blocker)
**Date:** 2026-05-06
**Files changed:** 1 (+185 / -30 lines)

---

## Route Chosen

Add a static text surface to the Quil workstation PD (`SURFACE_ID_QUIL = 201`) using only fill-rect visuals. Since sexdisplay has no font subsystem, text lines are represented as styled fill-rect rows with a title bar, text buffer area, and palette command panel.

### Surface Layout

| Element | rect_index | Description |
|---------|-----------|-------------|
| Palette | 0 | Command palette (existing, keeps selected row state) |
| Title bar | 1 | Deep blue-purple bar, 32px tall, full surface width |
| Text area bg | 2 | Dark slate background behind text lines |
| Text line 1 | 3 | Line fill + left accent (4px) |
| Text line 2 | 4 | Line fill + left accent |
| Text line 3 | 5 | Line fill + left accent |
| Text line 4 | 6 | Line fill + left accent |
| Text line 5 | 7 | Line fill + left accent |

Surface geometry: `SURFACE_W = 640`, `SURFACE_H = 480`.

### Text Buffer

- Static, inline, no heap: `QUIL_TEXT_BUFFER` (demo content, ~512 bytes max)
- Split on `\n` for visual lines, capped at `QUIL_MAX_VISIBLE_LINES = 6`
- No text rendering — fill-rect visuals only

### Validation

- `validate_title()`: checks len ≤ 32 and non-empty
- `validate_buffer()`: checks len ≤ 512 and non-empty
- Overflow detection: `[quil.text.buffer.overflow]` when lines > 6

### Drawing Functions

- `draw_title_bar()` — fills rect_index=1 with `QUIL_TITLE_BAR_COLOR` (0x00302E56)
- `draw_text_lines()` — draws text area background (rect_index=2), then up to 5 lines (rect_indices 3-7) with fill + left accent
- `draw_palette()` — draws command palette (rect_index=0) with selected row highlight

### Boot Flow

1. Title validation → `[quil.text.title]` or `[quil.text.title.invalid]`
2. Buffer validation → `[quil.text.buffer]` with byte/line count
3. `draw_title_bar()` → `[quil.text.title.bar]`
4. `draw_text_lines()` → `[quil.text.lines]`, `[quil.text.line]` per line
5. `draw_palette()` → existing `[quil.boot.draw.ok]`

---

## Blocker

**No font subsystem exists in sexdisplay.** All text surface elements are fill-rect placeholders. True text rendering requires:

- A glyph rasterizer (or bitmap font)
- A font data source (embedded or loaded)
- A new sexdisplay render primitive (glyph blit)
- Text shaping and line wrapping

See `docs/handoff/QUIL_MINIMAL_TEXT_SURFACE_BLOCKER_V1.md` for full blocker analysis.

---

## Proof Markers

All markers emitted at boot (no env gate — always runs):

```
[quil.text.surface] title=Quil
[quil.text.title] title=Quil len=4
[quil.text.buffer] bytes=... lines=... max_bytes=512
[quil.text.title.bar] w=640 h=32 color=0x302e56
[quil.text.lines] count=... bytes=...
[quil.text.bg] y=... h=...
[quil.text.line] index=0 rect=3 y=...
[quil.text.line] index=1 rect=4 y=...
...
[quil.boot.draw.ok]
```

---

## Build / Runtime

- Build: `./scripts/entrypoint_build.sh` — PASS
- No kernel edits. No ABI changes.

## Remaining Risks

1. **No text rendering**: All text lines are fill-rect placeholders. No glyphs, no characters, no text content visible to user.
2. **rect_index overlap risk**: rect_indices 1-7 overlap with any future multi-rect protocol additions. Must ensure sexdisplay doesn't reassign these indices.
3. **Static demo content**: `QUIL_TEXT_BUFFER` contains hardcoded demo text. Real text input requires a separate input path.
4. **Max 5 visible lines**: Lines beyond 5 are detected by overflow check but have no visual representation.

## Files Changed

```
servers/quil/src/main.rs  +185 / -30  (text surface V1: title bar, text lines, palette)
```

No sex-pdx ABI changes. No kernel edits. No renderer primitives.
