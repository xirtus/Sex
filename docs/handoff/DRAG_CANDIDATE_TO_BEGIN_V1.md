# DRAG_CANDIDATE_TO_BEGIN_V1

Date: 2026-05-14

## Summary
Added diagnostics to prove the dead hop between click candidate and drag begin.

No interaction behavior changes were made.

## What was added
File changed:
- `servers/silk-shell/src/main.rs`

New markers:
- `[shell.drag.pending] target=N kind=N start_x=N start_y=N buttons=N`
- `[shell.drag.threshold] dx=N dy=N dist=N required=N buttons=N pass=N`
- `[shell.drag.begin.reject] reason=... target=N kind=N buttons=N dx=0 dy=0`

Existing markers preserved:
- `shell.drag.begin`
- `shell.drag.update`
- `shell.drag.end`
- `shell.interact.drag.*`
- `shell.drag.candidate`
- `shell.drag.hit_test`

## Why candidate may not begin drag
Current model starts drag in click path only when all are true:
- not SilkBar-handled
- content hit (`Surface`/`None`)
- focused surface is shell-managed (`is_shell_surface`) -> only 100/101/102/103
- pointer is inside focused surface

So `result=app draggable=1` can still fail begin if focused surface is not shell-managed (e.g. Quil sid=201).

## Build
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning for missing `x86_64-sex` target remains unchanged.

## Runtime grep
`grep -E "shell.pointer.button|shell.drag.hit_test|shell.drag.candidate|shell.drag.pending|shell.drag.threshold|shell.drag.begin|shell.drag.begin.reject|shell.drag.update|shell.drag.end|shell.interact.drag|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200`

## Expected diagnostic interpretation
- If candidate is app but begin rejected with `focused_not_shell_surface`, dead hop is policy gate, not transport.
- If threshold passes (`pass=1`) but begin still absent, check reject reason and buttons.
- If begin appears, follow with update/end markers for full lifecycle proof.

## Backup
- `/tmp/silk-shell.main.rs.pre_drag_candidate_to_begin_v1.bak`
