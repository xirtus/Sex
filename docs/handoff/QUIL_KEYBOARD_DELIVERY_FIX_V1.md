# QUIL_KEYBOARD_DELIVERY_FIX_V1

## Result
STOP FIRST

## First dead hop
Shell delivery send path reports success, but Quil receives zero matching messages.

## Evidence
Runtime log: `/tmp/sexos_quil_keyboard_delivery_fix_v1.log`

Shell send markers (new):
- `[silk-shell.key.route.send] target=quil sid=201 scancode=0x24 slot=11 type=0x202 status=0 err=0`
- `[silk-shell.key.route.send] target=quil sid=201 scancode=0x25 slot=11 type=0x202 status=0 err=0`
- `[silk-shell.key.route.send] target=quil sid=201 scancode=0x1c slot=11 type=0x202 status=0 err=0`
- `silk-shell.key.route.fail`: 0

Quil liveness markers:
- `[quil.init.start]` present
- `[quil.ready]` present

Quil receive markers (new, earliest receive point):
- `quil.pdx.recv`: 0
- `quil.key.recv`: 0

Conclusion: shell->quil send is accepted (`status=0`), Quil is alive, but delivery does not surface in Quil receive loop.

## Scope of changes made
- `servers/silk-shell/src/main.rs`
  - Added structured send-status marker:
    `[silk-shell.key.route.send] ... status=... err=...`
  - Added fail marker:
    `[silk-shell.key.route.fail] ...`
- `servers/quil/src/main.rs`
  - Added earliest receive marker in main loop:
    `[quil.pdx.recv] type=... caller=... a0=... a1=... a2=...`
  - Kept existing key marker in required format:
    `[quil.key.recv] code=... down=... mod=...`

## Build status
- `SEXOS_QUIL_KEYBOARD_NAV_PROOF=1 ./scripts/entrypoint_build.sh` -> pass
- `./scripts/entrypoint_build.sh` -> pass

## Runtime count summary
- `silk-shell.key.route.send`: 3
- `silk-shell.key.route.fail`: 0
- `quil.pdx.recv`: 0
- `quil.key.recv`: 0
- `quil.keyboard.nav*`: 0
- faults (`fault.kill|#PF|#GP|panic|KERNEL PANIC`): 0

## Why STOP FIRST
Further progress would require changing delivery semantics outside local shell/quil userland flow (likely kernel/router/slot plumbing or message-lane ownership assumptions). That crosses the mission rule boundary (`no kernel/ABI edits unless STOP FIRST`).
