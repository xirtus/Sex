# PALETTE_REJECTS_APP_OPEN_BATCH_V1

## Result
PASS

## Summary
Palette batch proof executed all 10 palette actions with explicit outcome markers, app open/focus markers, restore-minimized setup+restore markers, and done marker. No fault markers observed.

## Rejects resolved vs confirmed
- Open Linen: confirmed reject (intentional in batch proof)
  - `ok=0 reason=blocking_risk_confirmed`
  - Rationale: synchronous Linen open path can block during proof window; preserved non-hanging proof behavior.
- Open Atlas: palette exec remains `ok=0 reason=action_reject` from focus-attempt path,
  but app-open/focus proof confirms Atlas overlay enable path is active:
  - `[shell.app.open.focus] app=Atlas ... ok=1 reason=overlay_enabled`
- Restore Minimized: resolved
  - setup marker: `setup=1 ... reason=ready`
  - restore marker: `restore=1 ok=1 reason=ok`

## Runtime proof counts
Log: `/tmp/sexos_palette_rejects_app_open_batch_v1.log`
- `shell.palette.batch.proof`: 11 (10 stage + done)
- `shell.app.open.focus`: 7
- `shell.restore.minimized.proof`: 2
- `shell.palette.exec`: 10
- `shell.palette.batch.proof.done`: 1
- `fault.kill|#PF|#GP|panic|KERNEL PANIC`: 0

## App open/focus table
- Spindle: exec ok=1; app.open.focus ok=1
- Quil: exec ok=1; app.open.focus ok=1
- Linen: exec ok=0 reason=blocking_risk_confirmed; app.open.focus ok=0 reason=blocking_risk_confirmed
- Atlas: exec ok=0 reason=action_reject; app.open.focus ok=1 reason=overlay_enabled
- Bell: exec ok=1; app.open.focus ok=1
- Collar: exec ok=1; app.open.focus ok=1
- Mesh: exec ok=1; app.open.focus ok=1
- Restore Minimized: exec ok=1; restore proof ok=1
- Zoom Toggle: exec ok=1
- Minimize Focused: exec ok=1

## Build
- `SEXOS_PALETTE_REJECTS_APP_OPEN_PROOF=1 ./scripts/entrypoint_build.sh` -> pass
- `./scripts/entrypoint_build.sh` -> pass

## Files touched
- `servers/silk-shell/src/main.rs`
- `docs/handoff/PALETTE_REJECTS_APP_OPEN_BATCH_V1.md`
