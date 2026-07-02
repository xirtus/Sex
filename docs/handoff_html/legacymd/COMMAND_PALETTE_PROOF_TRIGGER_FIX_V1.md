# COMMAND_PALETTE_PROOF_TRIGGER_FIX_V1

## Result
PASS

## Root cause
Proof call-site was being starved by boot-time blocking/continue paths. The proof trigger existed in source but often ran too late in the loop to execute before stalls.

## Fix summary
- Added bounded trigger diagnostics and stage markers:
  - `[shell.palette.daily.proof.skip] ...`
  - `[shell.palette.daily.proof.trigger] ...`
  - `[shell.palette.daily.proof.stage] ...`
  - `[shell.palette.daily.proof.done] ...`
- Moved proof invocation to top loop region (before blocking paths).
- Changed proof progression to bounded burst execution in one call.
- Guarded drain-path Enter during proof (`command_palette.drain.execute.skip`) to prevent unwanted duplicate execution.
- Added explicit proof-mode reject for linen open if it risks blocking:
  - `[shell.palette.exec] idx=2 action=Open Linen ok=0 reason=proof_block_risk`

## ISO string proof
`strings sexos-v1.0.0.iso | grep -E "shell.palette.daily.proof|shell.palette.item|shell.palette.exec"`
shows all required markers present.

## Runtime counts
Log: `/tmp/sexos_command_palette_proof_trigger_fix_v1.log`
- `shell.palette.daily.proof.skip`: 0
- `shell.palette.daily.proof.trigger`: 1
- `shell.palette.item`: 10
- `shell.palette.exec`: 10
- `shell.palette.daily.proof.done`: 1
- `fault.kill|#PF|#GP|panic|KERNEL PANIC`: 0

Final done marker:
- `[shell.palette.daily.proof.done] ok=1 executed=7 rejected=3 skipped=0`

## Palette action outcomes
- 0 Open Spindle: ok=1
- 1 Open Quil: ok=1
- 2 Open Linen: ok=0 reason=proof_block_risk
- 3 Open Atlas: ok=0 reason=action_reject
- 4 Open Bell: ok=1
- 5 Open Collar: ok=1
- 6 Open Mesh: ok=1
- 7 Restore Minimized: ok=0 reason=action_reject
- 8 Zoom Toggle: ok=1
- 9 Minimize Focused: ok=1

## Build verification
- `SEXOS_COMMAND_PALETTE_DAILY_PROOF=1 ./scripts/entrypoint_build.sh` -> pass
- `./scripts/entrypoint_build.sh` -> pass
