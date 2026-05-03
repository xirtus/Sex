# SILKBAR_CLICKABLE_CONTROLS_V1

## Status: PASS (2026-05-03)

## Summary
SilkBar top-panel regions are now clickable at shell policy level. Hit-testing uses `silkbar-model` geometry constants + `hit_test_action()` to classify clicks as launcher, workspace, status chip, or clock. No renderer or ABI changes.

## Proof Markers
```
[shell.silkbar.click] target=launcher x=100 y=25
[shell.silkbar.click] target=workspace index=N x=600 y=25
[shell.silkbar.click] target=status x=940 y=25
[shell.silkbar.click] target=clock x=1100 y=25
```

## PASS Criteria Verified
- [x] `[shell.silkbar.click] target=launcher` - launcher region hit
- [x] `[shell.silkbar.click] target=workspace index=2` - workspace pill hit
- [x] `[shell.silkbar.click] target=status` - chip region hit
- [x] `[shell.silkbar.click] target=clock` - clock region hit
- [x] Drag proof still works (50 start/move/end sequences)
- [x] `[silk.contract.validate.ok] version=1`
- [x] `[silk.render_proof.top_strip.ok]`
- [x] No PF/GP/panic

## Files Changed

### servers/silk-shell/src/main.rs
- Added `silkbar_model` imports: `DEFAULT_SILK_BAR`, `hit_test_action`, `Action`, panel constants
- Added `handle_silkbar_click(px, py)` function that:
  - Guards: y < 50 (top strip), within panel bounds
  - Calls `hit_test_action(&DEFAULT_SILK_BAR, ux, uy)`
  - Dispatches: `SwitchWorkspace(n)` sends `OP_SILKBAR_WORKSPACE_ACTIVE`, others log
  - Returns `true` if panel click consumed, `false` otherwise
- Wired into USB path (OP_USB_MOUSE_REPORT) via `!handle_silkbar_click(...) &&` in drag guard
- Wired into HID_EVENT path (EV_BTN) via same pattern
- Updated proof markers to `[shell.silkbar.click] target=...` format

### servers/sexinput/src/main.rs
- Added `USB_PROOF_DISABLE_SYNTH_SILKBAR_CLICK` gate (default: false = enabled)
- Added `silkbar_click_stage` state machine (ticks 2-17):
  - Reset CLICK_ACTIVE from drag proof
  - EV_ABS + EV_BTN sequences for launcher, workspace, status, clock
- Sexinput-side markers: `[sexinput.synthetic.silkbar_click] target=...`

### servers/silk-shell/Cargo.toml
- Added `silkbar-model` dependency

## Architecture
- **No kernel edits, no PDX ABI changes, no sexdisplay edits, no framebuffer writes**
- Hit-test uses `hit_test_action()` from `silkbar-model` with `DEFAULT_SILK_BAR` layout
- Workspace clicks send `OP_SILKBAR_WORKSPACE_ACTIVE` to silkbar via existing ABI
- Launcher/status/clock clicks are classified and logged (no visual response yet)
- Bar area (y < 50) and surfaces (y >= 50) do not overlap, so click-focus + drag are naturally preserved
- Drag guard: `if !handle_silkbar_click(...) && is_shell_surface(...) && point_in_surface(...)` short-circuits on bar clicks

## Edge Cases
- CLICK_ACTIVE from drag proof must be reset before SilkBar proof (handled via EV_BTN(1,0) at tick 2)
- DRAG_ACTIVE remains true across SilkBar proof, drag resumes correctly at tick 120+
- Click-focus.miss for bar clicks (no surface at y < 50) — correct, no spurious focus changes
