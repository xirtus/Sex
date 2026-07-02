# VISIBLE_FOCUS_TOPBAR_REGRESSION_V1

**Date**: 2026-05-14
**Status**: PASS — root cause found and fixed in 1 attempt

## Summary

Keyboard-driven GUI was usable but suffered a visual regression: after focus/zoom/
minimize/restore cycles, the teal window topbar reverted to a "tiny" 4px rim instead
of the correct 28px top bar. Root cause was a missing `send_frame_tab_info()` call
after `restore_minimized_frame()` reactivates the surface via 0xEC.

## Root Cause

When a frame is minimized:
1. `minimize_frame()` calls `pdx_call(SLOT_DISPLAY, 0xEE, ...)` → sexdisplay sets
   `slot.active = false`
2. Later, `restore_minimized_frame()` calls `pdx_call(SLOT_DISPLAY, 0xEC, ...)` to
   reactivate

In sexdisplay's 0xEC handler:
- The **upsert path** looks for `slot.active && slot.surface_id == surface_id` —
  since active=false, it misses
- The **create path** looks for `!slot.active` — finds the deactivated slot and
  creates a fresh `Surface` with `chrome_flags: 0`
- `SURFACE_CHROME_TOP_BAR` bit is lost → sexdisplay renders the 4px minimal rim
  instead of the 28px glass top bar

`restore_minimized_frame()` did NOT call `send_frame_tab_info()` after 0xEC, so the
correct chrome_flags (including top_bar=1) were never re-sent to sexdisplay.

Zoom/unzoom were NOT affected because they use 0xEC while the surface is still
active → upsert path preserves chrome_flags.

## Fix

**File**: `servers/silk-shell/src/main.rs`

Added `send_frame_tab_info(frame_id);` call in `restore_minimized_frame()` after
the 0xEC reactivation block and before `try_set_focus()`. This re-sends the
frame's chrome metadata (top_bar flag, tab count, active tab) to sexdisplay,
restoring the full 28px top bar immediately after restore.

```rust
// After 0xEE deactivate + 0xEC reactivate, sexdisplay creates a fresh
// Surface slot with chrome_flags=0.  Re-send tab info so the top-bar
// chrome bit (and any hover state) is restored immediately.
send_frame_tab_info(frame_id);
```

## Diagnostics Added

Gated behind `SEXOS_VISIBLE_FOCUS_TOPBAR_PROOF=1` (default OFF):

1. **`[shell.focus.visible]`** — emitted on every `try_set_focus()` success.
   Shows old focus, new focus, frame ID, surface ID, active-scene flag, reason.

2. **`[shell.frame.chrome.size]`** — emitted after restore/zoom/unzoom/focus_set.
   Shows topbar_h, tab_h, toolbar_h, zoomed, minimized, focused, reason.

3. **`[shell.frame.chrome.state]`** — paired with chrome.size.
   Shows surface x/y/w/h, focused, active flags.

4. **`emit_chrome_diagnostics(frame_id, reason)`** — helper called from
   `restore_minimized_frame`, `zoom_frame`, `unzoom_frame`, and `try_set_focus`
   (when VISIBLE_FOCUS_TOPBAR_PROOF is enabled).

## Proof Mode

`SEXOS_VISIBLE_FOCUS_TOPBAR_PROOF=1` drives this sequence through the existing
keyboard action path:

| Stage | Action              | Verifies                          |
|-------|---------------------|-----------------------------------|
| 1     | AccessFocusNext     | Focus next frame, topbar_h=28     |
| 2     | AccessFocusPrev     | Focus prev frame, topbar_h=28     |
| 3     | AccessZoomToggle    | Zoom frame, topbar_h=28, zoomed=1 |
| 4     | AccessZoomToggle    | Unzoom frame, topbar_h=28         |
| 5     | AccessActivate      | Minimize focused frame            |
| 6     | RestoreMinimized    | Restore, topbar_h=28, minimized=0 |

Also added `SurfaceAction::RestoreMinimized` handling to `access_handle_keyboard_action()`
so the proof can drive restore through the access action path.

## Build

```
SEXOS_VISIBLE_FOCUS_TOPBAR_PROOF=1 ./scripts/entrypoint_build.sh  # proof build
./scripts/entrypoint_build.sh                                      # normal build
```

Both pass.

## Runtime Proof

Serial log grep confirms all stages pass with topbar_h=28 throughout:
- Focus cycle: topbar_h=28 ✓
- Zoom: topbar_h=28, zoomed=1 ✓
- Unzoom: topbar_h=28, zoomed=0 ✓
- Minimize + Restore: topbar_h=28 after restore ✓
- sexdisplay receives chrome=3 (bit 0 = top_bar) after restore ✓
- Faults: 0

## Files Changed

- `servers/silk-shell/src/main.rs` — fix + diagnostics + proof mode

Not touched:
- sexdisplay
- sexusb
- sexinput
- kernel
- ABI

## Autopilot Result

- Attempts: 1
- Root cause: clear (missing send_frame_tab_info after restore reactivation)
- Fix: local, minimal, single-line addition
- Build: PASS
- Runtime proof: PASS
- No STOP FIRST needed
