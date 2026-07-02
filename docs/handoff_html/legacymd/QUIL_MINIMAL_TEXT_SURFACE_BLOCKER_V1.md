# QUIL_MINIMAL_TEXT_SURFACE_BLOCKER_V1

**Status:** Active
**Commit:** (pending)
**Build:** Passed (quil server compiles)

## Purpose

Document the text rendering blocker that prevents Quil from displaying actual
text glyphs. Quil V1 uses fill-rect visual representation only.

## Blocker: No Font Subsystem in Sexdisplay

Sexdisplay (the sole framebuffer writer) supports only these visual primitives:
- **0xEC** — surface upsert (position/size/color)
- **0xEF** — fill rect (rect_index, color, position, size)
- **0xEE** — surface deactivate
- **0xEB** — surface move
- **0xEA** — cursor position

There is **no text/font/glyph rendering** capability. The only bitmap font
in sexdisplay is the 5×7 digit font `FONT` used exclusively for the SilkBar
clock digits (lines 460-471 of sexdisplay/src/main.rs).

## What Quil V1 Does Instead

| Feature | Implementation | Marker |
|---------|---------------|--------|
| Static title "Quil" | Title bar fill-rect (rect_index=1) | `[quil.text.title]` |
| Static text buffer | In-memory byte array, bounded (512 bytes) | `[quil.text.buffer]` |
| Visual "lines" | Fill-rects per line (rect_indices 2-7) | `[quil.text.line]` |
| Palette/command area | Existing palette (rect_index=0) | `[quil.palette.draw]` |
| Title validation | Bounds check (max 32 bytes) | `[quil.text.title.reject]` |
| Buffer validation | Bounds check (max 512 bytes) | `[quil.text.buffer.reject]` |

## What Is Needed For Real Text Rendering

1. **Font bitmap data** in sexdisplay (e.g., 8×13 or similar ASCII glyph table)
2. **New sexdisplay opcode** (STOP FIRST) for text rendering, e.g.:
   - `OP_TEXT_DRAW` — blit glyph string at (x,y) with color
   - Or extend 0xEF to carry a glyph index for fixed-width rendering
3. **sex-pdx ABI update** (STOP FIRST) for new opcode constant
4. **Renderer integration** — glyph blitting in sexdisplay's composite path

## Stopping Conditions For Next Step

Before implementing real text, these MUST be approved:
- [ ] New sexdisplay opcode design
- [ ] sex-pdx ABI update
- [ ] Font data format and storage location
- [ ] Backward compatibility with existing fill-rect surfaces

## Deferred For V1

- Text editing engine (edit deltas, cursor, selection)
- Syntax highlighting (language mode)
- Font size/face selection
- Unicode support
- Filesystem persistence (requires SEXFILES or sexstore)

## References

- `servers/quil/src/main.rs` — Quil text surface V1 implementation
- `servers/sexdisplay/src/main.rs` — No text rendering in render path
- `docs/handoff/H2_QUIL_WORKSTATION_MODEL_V1.md` — Quil workstation model
- `docs/handoff/J3_QUIL_BUFFER_TABLE_V1.md` — Quil buffer table (silk-shell side)
