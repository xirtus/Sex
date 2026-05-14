# QUIL_KEYBOARD_BUFFER_NAV_V1

## Result
FAIL (blocked by Quil receive/proof trigger gap)

## What is proven
- Shell focuses Quil and emits route markers to Quil slot:
  - `[silk-shell.key.route] target=quil sid=201 code=36 down=1`
  - `[silk-shell.key.route] target=quil sid=201 code=37 down=1`
  - `[silk-shell.key.route] target=quil sid=201 code=28 down=1`
- No route rejection markers observed (`route_reject=0`, `route_defer=0`).
- faults=0.

## Blocker (repeated twice)
Despite route markers, Quil-side markers remain absent in runtime log:
- `quil.key.recv = 0`
- `quil.nav.move = 0`
- `quil.select = 0`
- `quil.open.request = 0`
- `quil.delete.proof = 0`
- `quil.keyboard.nav.proof = 0`
- `quil.keyboard.nav.proof.done = 0`

This repeated across two proof runs, so stopped per autopilot rule.

## Reserved-key priority status
- Preserved. No change to shell reserved-key dispatch ordering.
- Proof injector used J/K/Enter only and did not alter Tab/Esc/F-key handling paths.

## Changes attempted
- Added `SEXOS_QUIL_KEYBOARD_NAV_PROOF` gate and Quil nav/select/open/delete proof markers in `servers/quil/src/main.rs`.
- Added shell-side Quil route proof injector with readiness checks in `servers/silk-shell/src/main.rs`.

## Build
- `SEXOS_QUIL_KEYBOARD_NAV_PROOF=1 ./scripts/entrypoint_build.sh` -> pass
- `./scripts/entrypoint_build.sh` -> pass

## Runtime counts
Log: `/tmp/sexos_quil_keyboard_buffer_nav_v1.log`
- `quil.key.recv`: 0
- `quil.nav.move`: 0
- `quil.select`: 0
- `quil.open.request`: 0
- `quil.delete.proof`: 0
- `quil.keyboard.nav.proof`: 0
- `quil.keyboard.nav.proof.done`: 0
- `silk-shell.key.route target=quil`: 3
- `silk-shell.key.route.reject target=quil`: 0
- `silk-shell.key.route.defer target=quil`: 0
- fault markers (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Next safe step
Investigate Quil server receive lane in isolation (slot binding / message class / caller expectations) before further feature work; no ABI/opcode change done here.
