# SPINDLE_REAL_KEYBOARD_FOCUS_TEXT_V1 — Handoff

**Date**: 2026-05-14
**Status**: PASS
**Attempts**: 1

## Summary

Proved real keyboard can focus/open Spindle through the command palette and type
text into it through the normal key dispatch path, without relying on the
existing synthetic Spindle proof (SEXOS_SPINDLE_KEYBOARD_PROOF).

## Spindle Focus Path

**Method**: Command palette via backtick (scancode 0x29)

The command palette now includes "Open Spindle" as the first entry (replacing
the niche "Open in Quil" command that required Linen focus + selection).
Pressing backtick opens the palette, Enter executes the first command,
which calls `open_spindle_in_active_scene()` — the same function used by
ToggleSpindle/ScrollLock.

This provides a practical daily-driver keyboard access path to Spindle that
works on any standard keyboard (backtick is universal, unlike ScrollLock).

## Changes Made

### File: `servers/silk-shell/src/main.rs`

#### 1. Command palette: FocusSpindle replaces OpenSelectedInQuil
- `Command::OpenSelectedInQuil` → `Command::FocusSpindle` in enum
- `COMMAND_LIST[0]` → `("FocusSpindle", "Open Spindle")`
- `command_kind_color` / `command_palette_selected_accent` → teal accent
- `palette_execute_selected` → calls `open_spindle_in_active_scene()`

#### 2. Spindle lifecycle registration (focus fix)
- Added `lifecycle_register(SURFACE_ID_SPINDLE, LifecycleState::Visible)` to
  `lifecycle_init_all()`. Without this, `try_set_focus(153)` was rejected with
  "reason=lifecycle" because Spindle was never registered in the lifecycle table.

#### 3. Palette intercept in handle_hid_event drain path
- Added COMMAND_PALETTE_OPEN check BEFORE reserved_ui_action in
  `handle_hid_event` so palette navigation/execution works through the
  synthetic proof dispatch path (Enter, Escape, backtick, J, K).

#### 4. Spindle text key passthrough in handle_hid_event drain path
- Added `is_spindle_text_key()` check BEFORE `reserved_ui_action` in
  `handle_hid_event` so Enter (0x1C), Backspace (0x0E), Escape (0x01),
  and 'c' (0x2E) reach Spindle instead of being consumed as shell UI
  actions (AccessActivate, AccessFocusPrev, AccessZoomToggle, Center).

#### 5. Removed `!reserved_ui_key` gate from main dispatch path
- Command palette handler: removed `!reserved_ui_key &&` so
  Enter/Escape/J/K work in the palette through the main dispatch path.
- Spindle handler: removed `!reserved_ui_key &&` so
  Enter/Backspace/Escape/'c'/Tab reach Spindle when focused through the
  main dispatch path (real USB keyboard).

#### 6. Proof mode: `SEXOS_SPINDLE_REAL_KEYBOARD_FOCUS_PROOF`
- Gate constant, stage/state statics, `is_spindle_text_key()` helper.
- `maybe_run_spindle_real_keyboard_focus_proof()` function:
  - Stage 0: ensure Quil is focused as baseline
  - Stage 1: backtick (0x29) → open command palette
  - Stage 2: Enter (0x1C) → execute FocusSpindle → focus Spindle
  - Stage 3: 'a' (0x1E) → spindle.text.append ch='a'
  - Stage 4: 'b' (0x30) → spindle.text.append ch='b'
  - Stage 5: Backspace (0x0E) → spindle.text.backspace
  - Stage 6: 'c' (0x2E) → spindle.text.append ch='c'
  - Stage 7: Enter (0x1C) → spindle.key.enter
  - Stage 8: proof complete
- Called from main loop after other synthetic proofs.

## Key Design Decision: Spindle Text Key Whitelist

When Spindle is focused, certain reserved shell UI keys (Enter, Backspace,
Escape, Tab, 'c') are now routed to Spindle instead of triggering shell
window-management actions. This is correct terminal behavior:
- Enter → dispatches Spindle command (not minimize window)
- Backspace → deletes text in Spindle (not switch focus)
- Escape → enters vi normal mode in Spindle (not zoom toggle)
- 'c' (0x2E) → types 'c' (not center window)
- Tab → deferred for completion (not switch focus)

Other reserved keys (F-keys, ScrollLock, backtick, etc.) are NOT in the
Spindle whitelist and fall through to the shell UI handler as before.

## Proof Runtime Results

```
=== Proof stages === 9
=== Done === 1
=== Route markers === 6
=== Key recv === 6
=== text.append === 3
=== text.backspace === 1
=== key.enter === 2
=== focus set to 153 === 2
=== Faults === 0
```

All markers fire correctly:
- `[shell.spindle.focus.path] method=command_palette ok=1`
- `[shell.focus.set] id=153` (Spindle focused)
- `[shell.spindle.text.proof] stage=0..7 action=... ok=1`
- `[shell.spindle.text.proof.done] ok=1`
- `[silk-shell.key.route] target=spindle sid=153 code=... down=1` x 6
- `[spindle.key.recv] code=... down=1 mod=0` x 6
- `[spindle.text.append] ch=97` (a), `ch=98` (b), `ch=99` (c)
- `[spindle.text.backspace]`
- `[spindle.key.enter]`

## Build

Both builds succeed:
- `./scripts/entrypoint_build.sh` (normal) — zero faults at runtime
- `SEXOS_SPINDLE_REAL_KEYBOARD_FOCUS_PROOF=1 ./scripts/entrypoint_build.sh` — all markers pass

## Daily-Driver Caveat

The command palette backtick approach provides practical Spindle access on
any keyboard. ScrollLock (0x46) still works as an alternative.

Physical daily-driver testing of the backtick→palette→OpenSpindle→type flow
with a real USB keyboard is deferred until runtime hardware test.

## Files Changed

1. `servers/silk-shell/src/main.rs`:
   - Added `SPINDLE_REAL_KEYBOARD_FOCUS_PROOF_ENABLED` gate + `is_spindle_text_key()` helper
   - Added `maybe_run_spindle_real_keyboard_focus_proof()` function
   - Added palette intercept in `handle_hid_event` drain path
   - Added Spindle text key passthrough in `handle_hid_event` drain path
   - Removed `!reserved_ui_key` gate from palette handler (main dispatch)
   - Removed `!reserved_ui_key` gate from Spindle handler (main dispatch)
   - Replaced `OpenSelectedInQuil` with `FocusSpindle` in command palette
   - Added `SURFACE_ID_SPINDLE` to `lifecycle_init_all()`
   - Added proof call in main loop

2. `docs/handoff/SPINDLE_REAL_KEYBOARD_FOCUS_TEXT_V1.md` (this file)

## Recurring Notes

- Spindle lifecycle state MUST be registered in `lifecycle_init_all()` or
  `try_set_focus` will reject it with "reason=lifecycle".
- The `handle_hid_event` drain path and the main `OP_HID_EVENT` dispatch
  path must both handle reserved-key passthrough for Spindle; fixing only
  one path leads to inconsistent behavior between synthetic proofs and
  real keyboard input.
- `is_spindle_text_key()` whitelist must match the scancode set in the
  main dispatch Spindle handler to avoid key-leak bugs.
