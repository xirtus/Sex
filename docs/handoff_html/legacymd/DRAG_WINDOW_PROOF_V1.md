# DRAG_WINDOW_PROOF_V1

## Status: PASS (2026-05-03)

## Summary
Shell-level drag-window behavior proven using existing HID_EVENT input route
and existing OP_SURFACE_UPDATE/OP_DISPLAY_SET_SNAPSHOT display path.

## Proof Markers Observed
```
[shell.drag.start] id=N x=N y=N
[shell.drag.move] id=N x=N y=N dx=N dy=N
[shell.drag.send.ok] id=N
[shell.drag.end] id=N x=N y=N
```

## PASS Criteria Verified
- [x] [shell.drag.start] id=100 x=200 y=200
- [x] [shell.drag.move] id=100 x=206 y=204 dx=6 dy=4
- [x] [shell.drag.send.ok] id=100
- [x] [shell.drag.end] id=100 x=206 y=204
- [x] [silk.contract.validate.ok] version=1
- [x] [silk.render_proof.top_strip.ok]
- [x] No PF/GP/panic

## Files Changed

### servers/silk-shell/src/main.rs
- Added drag state: `DRAG_SURFACE_ID`, `LAST_DRAG_X`, `LAST_DRAG_Y`
- Added proof markers to existing EV_BTN/EV_REL drag paths
- Added USB path drag handling (start, end, movement with proof markers)
- Added `[shell.drag.send.ok]` after drag movement

### servers/sexinput/src/main.rs
- Enabled `USB_PROOF_DISABLE_SYNTH_DRAG = false` to run synthetic drag proof
- Updated comment to describe drag proof behavior

## Architecture
- Drag is initiated on left-button down when pointer is over a shell-managed surface
- Drag movement applies delta to surface position using `wrapping_add` + `clamp_position`
- Surface position update propagated via `emit_snapshot()` -> `OP_SURFACE_UPDATE` to sexdisplay
- Drag ends on left-button release
- Click-to-focus behavior preserved (drag starts after focus is determined)
- No new ABI, no kernel edits, no sexdisplay edits, no PDX changes

## Synthetic Proof Sequence
The synthetic proof in sexinput runs every 120 ticks (tick % 120 == 0):
1. Stage 0: EV_ABS(200,200) + left button down (EV_BTN 1,1) -> cursor positioned in SURFACE_ID_APP, drag starts
2. Stage 1: EV_REL(6,4) with left held -> surface 100 moves by (6,4), drag move marker emitted
3. Stage 2: left button up (EV_BTN 1,0) -> drag ends

## Drag Routes Supported
1. **HID_EVENT path** (used by synthetic proof): EV_ABS/EV_REL/EV_BTN via OP_HID_EVENT
2. **USB path** (for physical mouse): OP_USB_MOUSE_REPORT with dx/dy/buttons

Both routes converge on the same surface position update -> emit_snapshot() path.

## Future Work
- Click-focus proof can be re-enabled by setting `USB_PROOF_DISABLE_SYNTH_CLICK = false`
- Draggable region detection could be refined with a title-bar model
- DPI-aware drag threshold could be added
