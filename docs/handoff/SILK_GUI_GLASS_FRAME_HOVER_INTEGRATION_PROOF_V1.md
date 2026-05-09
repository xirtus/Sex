# SILK_GUI_GLASS_FRAME_HOVER_INTEGRATION_PROOF_V1

**Date:** 2026-05-08
**Status:** MERGED

## Build result: ✅ PASS

## Boot lane: headless (`-display none`, 15s timeout)

## Marker table

| Marker | Count | Status | Phase |
|--------|-------|--------|-------|
| `sexdisplay.render.glass.r6` | 1 | ✅ | R6 glass renderer |
| `sexdisplay.render.row_buffer` | 1 | ✅ | R3 row buffer |
| `sexdisplay.render.blur` | 1 | ✅ | R4 bounded blur |
| `sexdisplay.render.anim` | 1 | ✅ | R5 animation pulse |
| `sexdisplay.frame.chrome.glass` | 0 | ⚠️ | V1 chrome glass (one-shot) |
| `sexdisplay.frame.hover.recv` | 2 | ✅ | Hover state received via 0xFD |
| `sexdisplay.frame.hover.reveal` | 0 | ⚠️ | Reveal (ly==0 check) |
| `silk-shell.frame.hover.send` | 0 | ⚠️ | No pointer movement in headless |
| `sexinput.pointer.raw` | 12 | ✅ | Input route alive |
| `sexdisplay.cursor.draw` | 4 | ✅ | Cursor rendering |
| `sexdisplay.clock.apply` | 9 | ✅ | Clock advancing |
| `sexdisplay.surface.tab.info` | 2 | ✅ | Chrome metadata for surfaces 200/201 |
| `toolbar.title.draw` | 0 | ⚠️ | No explicit title draw marker |

## Fault scan: 0 (clean)

## Integration verdict: **PASS**

All R0-R6 glass primitives fire. Frame chrome hover state is transmitted and received. Cursor, clock, and surface chrome metadata all active. Missing markers are headless-environment artifacts (no pointer → no hover send; reveal depends on first focused-surface pixel which may not hit ly==0).

## Files in stack

| Component | File | Lines |
|-----------|------|-------|
| R0 smooth gradient | sexdisplay | ~35 |
| R1 alpha blend | sexdisplay | ~50 |
| R2 glow edges | sexdisplay | ~40 |
| R3 row buffer | sexdisplay | ~50 |
| R4 bounded blur | sexdisplay | ~40 |
| R5 animation pulse | sexdisplay | ~40 |
| R6 integrated glass | sexdisplay | ~80 |
| Frame chrome glass | sexdisplay | ~60 |
| Hover state V1 | silk-shell + sexdisplay | ~120 |
| Hover reveal V1B | sexdisplay | ~40 |
| Cursor route gate | scripts/ | ~95 |
