# QUIL_F9_TOGGLE_DEBOUNCE_V1

## Root Cause
Repeated raw F9 press scancodes (`0x43`) were delivered into shell key handling.
`shell` maps `0x43 -> ToggleQuil` and runs toggle on `EV_KEY value==1`, so repeated key-downs repeatedly called open/minimize/restore on Quil.

## Fix Scope
- Shell-local only.
- No kernel, ABI, sex-pdx, sexinput, sexdisplay, or Quil renderer changes.
- No global key repeat suppression.

## Implementation
File touched:
- `servers/silk-shell/src/main.rs`

Edge guard for `ToggleQuil` only:
- Added `F9_TOGGLE_DOWN` latch.
- On `EV_KEY` release (`scancode=0x43`, `value=0`): clear latch.
- On `ToggleQuil` press path:
  - if latch is already set: suppress action and emit marker
  - else: set latch, emit accept marker, execute existing toggle path

Markers:
- `[shell.key.repeat.suppressed] scancode=0x43 action=ToggleQuil`
- `[shell.key.edge.accept] scancode=0x43 action=ToggleQuil`

## Proof Steps
1. Build:
   - `./scripts/entrypoint_build.sh`
2. Boot and exercise F9 (hold or repeated bursts).
3. Verify markers:
   - `rg -n "shell.key.repeat.suppressed|shell.key.edge.accept|shell.action.quil|shell.quil.lifecycle.minimize|shell.quil.lifecycle.restore|frame.light.minimize.fsm|frame.light.restore.fsm" /tmp/sexos.log`
4. Visual expectation:
   - Quil should toggle once per press edge.
   - No rapid open/close loop from repeated F9 down events.

## Not Attempted
- No debounce for other keys.
- No sexinput-level filtering.
- No lifecycle/minimize/restore logic changes.
