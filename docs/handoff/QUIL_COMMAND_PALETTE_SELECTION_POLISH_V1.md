# QUIL_COMMAND_PALETTE_SELECTION_POLISH_V1

Date: 2026-05-06

## Current Quil UI Model
Quil currently draws directly via `SLOT_DISPLAY` + `0xEF` fill ops on `SURFACE_ID_QUIL`.
No text pipeline is used. Input arrives via `OP_HID_EVENT` (0x202) and carries scancode/value.

## Changes Applied
- Added a minimal internal palette-row model (5 rows).
- Added row visuals using existing fill primitives only:
  - inactive row background
  - selected row highlight
  - left accent strip for selected row
- Preserved boot draw path by replacing one-shot flat fill with one-shot `draw_palette(selected=0)`.

## Keyboard Behavior
Implemented when `value == 1` (press event):
- Up: move selection up (wrap)
- Down: move selection down (wrap)
- Enter: emit metadata action marker for current row
- Esc: clear palette (base fill), set palette inactive
- Unmapped key: keep prior liveness fallback color toggle and emit reject marker

If key routing is absent at runtime, behavior remains blocked by input path (not Quil draw path).

## Proof Markers
Added:
- `[quil.palette.draw]`
- `[quil.palette.row]`
- `[quil.palette.selected]`
- `[quil.palette.key]`
- `[quil.palette.action]`
- `[quil.palette.reject]`

Existing kept:
- `[quil.boot.draw.ok]`
- `[quil.key.recv]`

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Quil UI Gaps
- No text labels yet (intentionally out of scope).
- Palette active/inactive state is local-only and not integrated with a broader shell command model.
- Selection action is marker-only (no command execution path in this patch).
