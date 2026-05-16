# WEBSTUB_TEXT_GLYPH_RENDER_V1

**Status:** STOP FIRST — No safe bounded glyph helper exists for shell-side surface text.
**Date:** 2026-05-16

---

## Blocker

Font glyph→fill-rect rendering pipeline exists ONLY in Quil (`draw_text_lines()`). The shell has font awareness (5×7 ASCII bitmap) but no reusable `draw_str(surface_id, text)` helper. Options:

| Option | Blocked By |
|--------|-----------|
| Duplicate Quil font pipeline in shell | Broad refactor — ~400 lines of glyph logic + font table |
| Cross-PD text rendering (Quil renders for WebStub) | New protocol — requires Quil to accept text render requests for other surfaces |
| Sexdisplay font subsystem | Renderer redesign — sexdisplay has no font rendering |

## Current State

WebStub has 4 colored fill-rect bands (from WEBSTUB_STATIC_TEXT_RENDER_V1). This is the best available within current constraints. Actual glyph text requires a shared font rendering helper — future architectural work.

## Recommendation

**SHELL_FONT_GLYPH_HELPER_SPEC_V1** — design a minimal shared font rendering helper in sexdisplay or silk-shell that any PD can use to render text on any surface via `pdx_call(SLOT_DISPLAY, 0xEF, ...)` glyph fill-rects.
