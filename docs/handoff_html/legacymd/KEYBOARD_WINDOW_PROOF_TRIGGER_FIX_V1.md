# KEYBOARD_WINDOW_PROOF_TRIGGER_FIX_V1

Date: 2026-05-14

## Root cause
`SEXOS_KEYBOARD_WINDOW_PROOF=1` proof function existed and was called, but it returned silently when prerequisites were not ready (`no_focus` / `no_frame`) and emitted no diagnostics. This looked like "proof never ran" despite strings being present in ISO.

## Fix applied
Patched `servers/silk-shell/src/main.rs` proof trigger path only.

### 1) Retry-safe gating with bounded skip markers
Added bounded logs:
- `[shell.keyboard.window.proof.skip] reason=disabled`
- `[shell.keyboard.window.proof.skip] reason=no_focus`
- `[shell.keyboard.window.proof.skip] reason=no_frame focused=N`
- `[shell.keyboard.window.proof.skip] reason=already_done`

Behavior:
- If enabled but not ready, proof retries later (does not consume/done early).
- DONE state is only reached after stage 5 runs.

### 2) Trigger/stage/done markers
Added:
- `[shell.keyboard.window.proof.trigger] focused=N`
- `[shell.keyboard.window.proof.stage] stage=N action=NAME ok=N`
- `[shell.keyboard.window.proof.done] ok=N`

Stages are non-destructive:
1. Focus next
2. Zoom toggle
3. Zoom toggle (restore)
4. Activate (minimize)
5. Activate (restore)

## Build result
Built with proof enabled as requested:
- `SEXOS_KEYBOARD_WINDOW_PROOF=1 ./scripts/entrypoint_build.sh`
- Result: success.

## Runtime grep
```bash
grep -E "shell.keyboard.window.proof|shell.key.action|shell.window.action|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

## Expected runtime outcomes
- Trigger/skip/stage/done markers appear with exact reason when not ready.
- Once focus+frame are ready, stage markers execute.
- `shell.key.action` and `shell.window.action` appear from normal action path.
- Fault count remains 0.
