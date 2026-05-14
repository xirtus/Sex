# LINEN_KEYBOARD_NAV_CRUD_V1

## Result
PASS

## Linen key route status
- Shell->Linen route proven via markers:
  - `[silk-shell.key.route] target=linen sid=200 code=36 down=1`
  - `[silk-shell.key.route] target=linen sid=200 code=37 down=1`
  - `[silk-shell.key.route] target=linen sid=200 code=28 down=1`
- Linen key receive proven:
  - `[linen.key.recv] code=... down=1 mod=0`

## What changed
- Added default-off proof gate in Linen:
  - `SEXOS_LINEN_KEYBOARD_NAV_PROOF`
- Added Linen keyboard proof markers:
  - `[linen.key.recv] code=N down=N mod=N`
  - `[linen.nav.move] old=N new=N count=N`
  - `[linen.select] idx=N object_id=N ok=N`
  - `[linen.open.request] object_id=N ok=N reason=blocking_risk_confirmed`
  - `[linen.delete.proof] object_id=N ok=N reason=no_safe_reversible_delete_path`
  - `[linen.keyboard.nav.proof] stage=N action=NAME ok=N reason=...`
  - `[linen.keyboard.nav.proof.done] ok=1`
- Added object sanity marker:
  - `[linen.object.sanity] count=N`
- Added shell-side proof injector (same gate) to prove route-to-Linen in headless:
  - focuses Linen and sends J/K/Enter via existing OP_HID_EVENT path.

## Safety behavior
- Open intent in this proof is explicitly non-blocking reject:
  - `ok=0 reason=blocking_risk_confirmed`
- Delete path is explicitly safe reject (no reversible delete path in current model):
  - `ok=0 reason=no_safe_reversible_delete_path`
- No ABI/opcode changes. No USB/pointer/sexdisplay changes.

## Runtime counts
Log: `/tmp/sexos_linen_keyboard_nav_crud_v1.log`
- `linen.key.recv`: 5
- `linen.nav.move`: 4
- `linen.select`: 2
- `linen.open.request`: 1
- `linen.delete.proof`: 1
- `linen.keyboard.nav.proof`: 6
- `linen.keyboard.nav.proof.done`: 1
- `silk-shell.key.route target=linen`: 3
- fault markers (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Nav/select/delete/open proof table
- move_next: ok=1
- move_prev: ok=1
- select: ok=1 (object_id=1)
- open_nonblocking: ok=0 reason=blocking_risk_confirmed
- delete_safe: ok=0 reason=no_safe_reversible_delete_path
- proof.done: ok=1

## Build
- `SEXOS_LINEN_KEYBOARD_NAV_PROOF=1 ./scripts/entrypoint_build.sh` -> pass
- `./scripts/entrypoint_build.sh` -> pass

## Files touched
- `servers/linen/src/main.rs`
- `servers/silk-shell/src/main.rs` (route diagnostics/proof injection only)
- `docs/handoff/LINEN_KEYBOARD_NAV_CRUD_V1.md`
