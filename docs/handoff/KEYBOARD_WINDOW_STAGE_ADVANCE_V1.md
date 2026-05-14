# KEYBOARD_WINDOW_STAGE_ADVANCE_V1

## Goal
Fix keyboard window proof progression when runtime showed only:
- `[shell.keyboard.window.proof.stage] stage=0 action=Begin ok=1`

and no stages `1..5` / `done`.

## Root Cause
The stage machine itself incremented correctly, but progression depended on subsequent loop cadence/conditions to re-enter per-stage execution. In practice this caused proof to appear stuck at stage 0 in runtime logs.

## Fix
File changed:
- `servers/silk-shell/src/main.rs`

Updated `maybe_run_keyboard_window_synthetic_proof()` to:

1. Add bounded state marker each call:
- `[shell.keyboard.window.proof.state] stage=N in_progress=N done=N focused=N`

2. Add bounded defer markers on early exits:
- `[shell.keyboard.window.proof.defer] stage=N reason=disabled|no_focus|no_frame`

3. Advance proof deterministically in one trigger call (bounded loop):
- executes remaining non-destructive stages 0..5 in-order
- emits existing stage markers and done marker
- avoids relying on later loop passes for stage continuation

No destructive close action was added.
No kernel/ABI/opcode/sexdisplay changes.

## Backup
Branch backup creation was blocked in this environment because `.git` refs are read-only.
Used writable snapshot backup instead:
- `/tmp/pre_keyboard_window_stage_advance.diff`
- `/tmp/pre_keyboard_window_stage_advance_main.rs`

## Build
Command:
- `SEXOS_KEYBOARD_WINDOW_PROOF=1 ./scripts/entrypoint_build.sh`

Result:
- PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime Proof Grep
```bash
grep -E "shell.keyboard.window.proof|shell.window.action|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

## Expected Success
- stages `0..5` appear
- `[shell.keyboard.window.proof.done] ok=1` appears
- window action / frame action markers appear
- fault count remains 0
