# KEYBOARD_GUI_DAILY_DRIVER_V1

Date: 2026-05-14
Scope:
- `servers/silk-shell/src/main.rs`
- Keyboard-only daily-driver observability improvements
- No `sexusb`, `sexinput`, `sexdisplay`, ABI, or kernel edits

## Current Keymap (`scancode_to_action`)
- `Esc (0x01)` -> `AccessZoomToggle`
- `Tab (0x0F)` -> `AccessFocusNext`
- `Backspace (0x0E)` -> `AccessFocusPrev`
- `Enter (0x1C)` -> `AccessActivate` (minimize/restore via semantic action)
- `PageUp (0x49)` -> `RestoreMinimized`
- `F8 (0x42)` -> `ToggleLinen`
- `F9 (0x43)` -> `ToggleQuil`
- `F10 (0x44)` -> `ToggleAtlas`
- `F11 (0x57)` -> `AccessClose`
- `F12 (0x58)` -> `ToggleMesh`
- `ScrollLock (0x46)` -> `ToggleSpindle`
- `PageDown (0x51)` -> `ToggleBell`
- `Insert (0x52)` -> `ToggleCollar`
- `` ` `` `(0x29)` -> `ToggleCommandPalette`

## Coverage: Daily-Driver Actions
- Focus next: covered (`Tab`)
- Focus previous: covered (`Backspace`)
- Zoom toggle: covered (`Esc`)
- Minimize/restore focused: covered (`Enter` activate toggles min/restore)
- Restore minimized: covered (`PageUp`)
- Close focused safely: covered (`F11`)
- Open/toggle main apps: covered (`F8/F9/F12/ScrollLock/PageDown/Insert/F10/backtick`)
- Current focus visibility/logical proof: covered via focus markers and new `shell.kbd.ui.*` markers

No missing core daily-driver action was found in keymap. No new bindings added.

## Markers Added
Added at the keyboard action dispatch site (same path as `shell.key.action`):
- `[shell.kbd.ui.action] scancode=N action=NAME focused=N frame=N sid=N`
- `[shell.kbd.ui.focus] old=N new=N frame=N reason=NAME` (emits only on focus change)
- `[shell.kbd.ui.result] action=NAME ok=N reason=ok|noop_or_reject frame=N sid=N`

These are additive diagnostics only; behavior/bindings unchanged.

## Build / Runtime
- Build command: `./scripts/entrypoint_build.sh`
- Runtime grep:
```bash
grep -E "shell.kbd.ui|shell.key.action|shell.window.action|silk-shell.key.recv|silk-shell.key.route|shell.focus.set|focus.ref.commit|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|shell.frame.close|spindle.placeholder|linen|quil|atlas|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1600
```

## KEYBOARD_GUI_AUTOPILOT_V1 Fix (2026-05-14)
### Problem
Reserved UI keys (Tab, Esc, Enter, etc.) were routed to Quil BEFORE shell
could consume them. The `handle_hid_event` drain path (called from
`linen_sync_reply` and input-first drain) routed ALL EV_KEY events to the
focused app without checking `scancode_to_action()`.

### Fix
Added `scancode_to_action()` check + `access_handle_keyboard_action()` dispatch
in `handle_hid_event()`, before app routing. Reserved keys are now consumed
and dispatched as shell actions in the drain path, matching the main
`OP_HID_EVENT` dispatch behavior.

### Runtime Proof
Enter key via USB keyboard → Quil minimized (frame 3, sid 201), focus
switched to surface 100. All shell.kbd.ui.* markers verified.

See: `docs/handoff/KEYBOARD_GUI_AUTOPILOT_V1.md`

## Notes
- This pass intentionally avoids pointer and USB slot2 work.
- Keyboard-first usability is supported by existing actions; this patch makes manual operation auditable and repeatable.
- KEYBOARD_GUI_AUTOPILOT_V1 applied the dispatch-priority fix to the drain path.
