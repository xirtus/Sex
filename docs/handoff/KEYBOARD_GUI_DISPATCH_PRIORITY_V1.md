# KEYBOARD_GUI_DISPATCH_PRIORITY_V1

Date: 2026-05-14
Scope:
- `servers/silk-shell/src/main.rs`
- Keyboard dispatch priority fix for reserved shell UI actions

## Problem
Runtime showed keyboard receive markers but no shell UI action markers:
- `silk-shell.key.recv` present
- `shell.kbd.ui.action=0`, `shell.key.action=0`, `shell.window.action=0`
- Focused Quil consumed keys first via:
  - `[silk-shell.key.route] owner=quil sid=201 scancode=0x1c`
  - `[silk-shell.key.route] owner=quil sid=201 scancode=0xe`

## Root Cause
In EV_KEY down dispatch, Quil/Linen (and other focused-surface intercepts) were evaluated before shell action dispatch (`scancode_to_action`).

## Fix
Implemented reserved-key priority for shell UI actions on key-down:
1. Compute `reserved_ui_action = scancode_to_action(scancode)` once.
2. Gate app/intercept routes behind `!reserved_ui_key`:
- Quil route
- Linen route
- Scene Settings panel intercept
- Command palette intercept
- Atlas intercept
- Bell/Mesh focused key handlers
- Spindle text route
- Linen Enter/Space open-intent route
3. Reserved actions now reach shell action branch first.
4. Added consume marker:
- `[shell.kbd.ui.consume] scancode=N action=NAME down=N consumed=N`

## Preserved Behavior
- Non-reserved keys still route to focused app unchanged.
- No kernel/ABI/display/USB/input/opcode changes.
- Existing modifier handling (`Ctrl`, `F9` edge suppression) preserved.

## Markers to Verify
- `[shell.kbd.ui.consume] ...`
- `[shell.kbd.ui.action] ...`
- `[shell.kbd.ui.result] ...`
- Existing `[shell.window.action] ...` and focus/zoom/minimize markers.

## Build
- `./scripts/entrypoint_build.sh`

## Runtime Grep
```bash
grep -E "shell.kbd.ui|shell.key.action|shell.window.action|silk-shell.key.recv|silk-shell.key.route|shell.focus.set|focus.ref.commit|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|shell.frame.close|spindle.placeholder|linen|quil|atlas|bell|collar|command.palette|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1800
```
