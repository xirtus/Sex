# KEYBOARD_WINDOW_CONTROL_V1

Date: 2026-05-14

## 1) Existing bindings found
Already present in `servers/silk-shell/src/main.rs` via `scancode_to_action(...)` + access handlers:

- Focus next: `Tab (0x0F)` -> `AccessFocusNext`
- Focus previous: `Backspace (0x0E)` -> `AccessFocusPrev`
- Minimize/restore toggle: `Enter (0x1C)` -> `AccessActivate`
  - frame visible => minimize
  - frame minimized => restore
- Close focused window: `F11 (0x57)` -> `AccessClose`
- Zoom/unzoom toggle: `Esc (0x01)` -> `AccessZoomToggle`
- Restore first minimized frame: `PageUp (0x49)` -> `RestoreMinimized`
- Toggle Linen/Quil/Spindle/Atlas:
  - `F8` Linen
  - `F9` Quil
  - `ScrollLock (0x46)` Spindle
  - `F10` Atlas
- Tile/cycle actions already exist via snap/maximize/center and access focus traversal.

## 2) Missing bindings / STOP-FIRST decisions
No new binding was required for requested keyboard-first window control.
No key stealing performed.

## 3) Actions proven / proof mode added
Added runtime markers:
- `[shell.key.action] scancode=N action=NAME focused=N`
- `[shell.window.action] action=NAME frame=N sid=N ok=N reason=...`

Added default-off synthetic keyboard window proof gate:
- `SEXOS_KEYBOARD_WINDOW_PROOF=1`
- One-shot staged sequence (non-destructive):
  1. `AccessFocusNext`
  2. `AccessZoomToggle`
  3. `AccessZoomToggle` (restore)
  4. `AccessActivate` (minimize)
  5. `AccessActivate` (restore)

This avoids destructive close of the only working window while proving control path.

## 4) Files changed
- `servers/silk-shell/src/main.rs`
- `docs/handoff/KEYBOARD_WINDOW_CONTROL_V1.md`

## 5) Build result
- `./scripts/entrypoint_build.sh` completed successfully.

## 6) Runtime grep
```bash
grep -E "shell.key.action|shell.window.action|silk-shell.key.recv|silk-shell.key.route|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|shell.frame.close|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

Optional proof-mode grep additions:
```bash
grep -E "shell.keyboard.window.proof|shell.key.action|shell.window.action|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

## 7) Notes
- Pointer precision is no longer required for core window operations.
- Keyboard path is now the recommended control lane for routine GUI proofs under current QEMU input quality limits.
