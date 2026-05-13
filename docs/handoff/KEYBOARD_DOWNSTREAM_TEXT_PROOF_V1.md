# KEYBOARD_DOWNSTREAM_TEXT_PROOF_V1

Date: 2026-05-13
Mode: diagnostics-only / alias-marker-only

## Scope honored
- Changed only:
  - `servers/sexinput/src/main.rs`
  - `servers/silk-shell/src/main.rs`
  - `apps/spindle/src/main.rs`
- No routing/focus/opcode/ABI/behavior changes.
- No kernel/sex-pdx/sexdisplay/renderer/build-script edits.
- Existing markers preserved; aliases added only.

## Added alias markers
- `sexinput`:
  - `[sexinput.key.emit] code=N down=N mod=N`
- `silk-shell`:
  - `[silk-shell.key.recv] code=N down=N mod=N focused=N`
  - `[silk-shell.key.route] target=spindle sid=N code=N down=N`
- `spindle`:
  - `[spindle.key.recv] code=N down=N mod=N`
  - `[spindle.text.append] ch=N`
  - `[spindle.text.backspace]`
  - `[spindle.key.enter]`

## Build result
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Notes:
  - ABI/PKRU/FSM guard completed.
  - Host warning in optional gate: missing `x86_64-sex` target for `cargo check -p sex-pdx` in this environment.
  - Full pipeline still completed and packaged ISO successfully.

## First dead hop / pass state
- Static code audit indicates path is present:
  - `sexinput` emits EV_KEY to shell.
  - `silk-shell` receives EV_KEY and routes spindle-focused keys to `SLOT_SPINDLE`.
  - `spindle` receives OP_HID_EVENT and logs text append/backspace/enter.
- Runtime proof is **pending**. No GTK USB keyboard log captured in this run, so PASS is not claimed.

## Required runtime proof lane (not yet executed here)
- Boot with:
  - `-device nec-usb-xhci,id=xhci`
  - `-device usb-kbd,bus=xhci.0`
  - `-device usb-tablet,bus=xhci.0`
- Manual type sequence:
  - `abc123`
  - `Shift+A`
  - `Backspace`
  - `Enter`
- Grep:
  - `grep -E "sexusb.*kbd|sexusb.*key|sexinput.key|sexinput.kbd|silk-shell.key|shell.key|spindle.key|spindle.text|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1000`

## Expected proof lines
- `[sexinput.key.emit] ...`
- `[silk-shell.key.recv] ...`
- `[silk-shell.key.route] target=spindle ...`
- `[spindle.key.recv] ...`
- `[spindle.text.append] ...`
- `[spindle.text.backspace]`
- `[spindle.key.enter]`

If runtime shows missing marker, first missing marker in that order is the first dead hop.
