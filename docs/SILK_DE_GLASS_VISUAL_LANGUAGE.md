# SILK DE Glass Visual Language

## Purpose
Define the futuristic Apple-glass visual direction now, without renderer/compositor rewrites.
This is a design-target spec and a safe near-term styling guide.

## Core Visual Principles
- Deep blue-violet glass identity for the global bar.
- Cool desktop backdrop gradients imply depth behind the bar.
- Brightest values reserved for active/focused/clock layers.
- Semantic chip colors communicate status at a glance.
- Perceived depth from color temperature layering:
  cool background -> warm bar body -> bright foreground accents.

## Safe Current Subset (Now)
Top-strip only, flat ARGB fills, no new rendering primitives:
- Desktop backdrop as 4 flat horizontal bands:
  - deep navy
  - blue
  - violet
  - warm purple
- Global bar body fill updated to deep blue-violet.
- Workspace active state shown as lavender glow color.
- Chips use stable semantic colors (net/wifi/battery/clock).
- Clock digits use cool white-blue.
- Launcher uses subtle cyan (not neon green).

This subset is explicitly compatible with current safety constraints:
- top-strip redraw path
- no alpha compositing
- no blur
- no shadow passes that require full-frame redraw
- no dynamic text pipeline changes

## ARGB Palette (Flat)
Reference values for immediate style pass:
- Bar fill: `0x00182040`
- Workspace active: `0x00A8A0FF`
- Clock digits: `0x00C8D8FF`
- Launcher dot: `0x0070CCFF`

Recommended semantic chip palette (flat, safe):
- Net: `0x005A8DFF` (blue)
- Wifi: `0x004EC9B0` (teal)
- Battery: `0x00D8A24C` (amber)
- Clock chip body/accent: `0x006B7A96` (steel)

Backdrop band suggestions (flat desktop bands):
- Band 0 (top): `0x000A1026`
- Band 1: `0x0012203D`
- Band 2: `0x00221846`
- Band 3 (bottom of strip region): `0x0030224E`

## Dimensions and Spatial Intent
- Keep existing geometry/layout constants unchanged for now.
- Preserve current bar height, workspace/chip bounds, and clock placement.
- Any visual depth effect must come from color only in current stage.

## Forbidden Effects Until Later
Do not implement yet:
- true translucency / alpha blending against scene content
- background blur / frosted glass sampling
- multi-pass drop shadows
- wallpaper-aware adaptive contrast
- full-frame effect passes

## Later Renderer Requirements (Deferred)
When scheduler/yield and redraw/full-frame safety are complete:
- real translucency compositing
- blur kernel support for frosted panels
- layered shadows and glow falloff
- contrast management against live wallpaper/content
- animation timing polish tied to stable scheduling cadence

## Immediate Safe Style-Patch Candidate
Smallest safe visual upgrade:
- update color constants only in:
  - `crates/silkbar-model/src/lib.rs` (`DEFAULT_THEME`)
  - `servers/sexdisplay/src/main.rs` (hardcoded bar/clock/launcher colors)
- no layout changes
- no primitive changes
- no renderer path changes
- success signal: bar reads as dark glass rather than debug scaffolding.
