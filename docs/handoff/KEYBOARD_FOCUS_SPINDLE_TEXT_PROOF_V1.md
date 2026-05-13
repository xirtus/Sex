# KEYBOARD_FOCUS_SPINDLE_TEXT_PROOF_V1

Date: 2026-05-13

## Result
No code changes required. Existing focus and route paths already support Spindle text proof.

## Findings
1. Spindle focus path already exists:
- Scroll Lock maps to `ToggleSpindle` (`scancode 0x46`).
- `toggle_spindle()` -> `open_spindle_in_active_scene()` -> `try_set_focus(sid)`.
- Focus marker emitted by existing focus path:
  - `[shell.focus.set] id=<sid>`
- Spindle-specific focus marker also emitted:
  - `[spindle.placeholder.focus] frame=9 sid=153`

2. Spindle route path already exists:
- In EV_KEY dispatch when `FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE`:
  - shell emits `[silk-shell.key.route] target=spindle sid=153 code=N down=N`
  - forwards `OP_HID_EVENT` to `SLOT_SPINDLE`.

3. Spindle text markers already exist:
- `[spindle.key.recv] code=N down=N mod=N`
- `[spindle.text.append] ch=N`
- `[spindle.text.backspace]`
- `[spindle.key.enter]`

4. Manual click focus is possible:
- Pointer click path uses `click_hit_test_and_focus(...)` -> `try_set_focus(sid)`.
- Spindle surface is registered as focusable app surface.

## GTK proof steps (no code changes)
1. Boot with USB keyboard + USB tablet.
2. Press `Scroll Lock` once to open/focus Spindle.
3. Confirm focus markers in log:
- `[shell.action.spindle] toggle`
- `[spindle.placeholder.focus] frame=9 sid=153`
- `[shell.focus.set] id=153`
4. Type: `ab` `Backspace` `Enter`.
5. Confirm keyboard/text markers:
- `[silk-shell.key.recv] ...`
- `[silk-shell.key.route] target=spindle sid=153 ...`
- `[spindle.key.recv] ...`
- `[spindle.text.append] ...`
- `[spindle.text.backspace]`
- `[spindle.key.enter]`

## Grep
`grep -E "focus.*spindle|spindle.*focus|silk-shell.focus|shell.focus|silk-shell.key.recv|silk-shell.key.route|spindle.key.recv|spindle.text.append|spindle.text.backspace|spindle.key.enter|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1000`

## Build proof
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning about missing `x86_64-sex` target remains unchanged.

## Files changed
- `docs/handoff/KEYBOARD_FOCUS_SPINDLE_TEXT_PROOF_V1.md`

## Backups
- `/tmp/silk-shell.main.rs.pre_keyboard_focus_spindle_text_proof_v1.bak`
- `/tmp/spindle.main.rs.pre_keyboard_focus_spindle_text_proof_v1.bak`
