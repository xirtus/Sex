# QUIL_PALETTE_PANEL_POLISH_V1

Date: 2026-05-06

## Scope
- `servers/quil/src/main.rs` only
- Visual polish via existing `0xEF` fill rects
- No ABI/kernel/renderer-policy changes

## Visual Polish Applied
- Darkened base fill to reduce flat fullscreen feel.
- Added bounded inner panel area (`x=24 y=24 w=760 h=520`).
- Kept 5-row model, now visually framed within panel.
- Kept selected row highlight and left accent strip.
- Preserved row spacing and conservative fixed margins.

## Keyboard Behavior
Preserved existing behavior:
- Up/Down selection
- Enter action marker
- Esc clear/deactivate
- Unmapped keys fallback toggle + reject marker

## Marker Changes
Added:
- `[quil.palette.panel]`

Kept:
- `[quil.palette.draw]`
- `[quil.palette.selected]`
- `[quil.palette.reject]`
- `[quil.key.recv]`
- `[quil.boot.draw.ok]`

Additional guard marker:
- `[quil.palette.reject] action=draw reason=row_overflow` when configured rows exceed panel height.

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Quil UI Gaps
- No text labels (out of scope by design).
- Panel dimensions are conservative constants, not live geometry queried from shell.
- Action markers are metadata-only (no command execution flow).
