# POINTER_SMOOTHNESS_CONTROL_V1

Date: 2026-05-14

## 1) Root cause of poor feel
Main issue was not USB liveness; it was control quality in ABS path:
- after V3, post-ready ABS accepted almost everything, including host escape/corner poison jumps,
- these sudden corner/edge samples can yank cursor and make toolbar/chrome targeting feel unstable,
- repeated identical ABS samples also add noisy send churn without visible movement gain.

## 2) Exact fixes applied
File changed:
- `servers/silk-shell/src/main.rs`

Changes (narrow, ABS-only):
1. Added post-ready poison guard (default behavior unchanged otherwise):
- reject near top-left poison jump after ready when no button held and jump is large:
  - reason=`corner_poison_after_ready`
- reject edge-max poison jump after ready when no button held and jump is large:
  - reason=`edge_poison_after_ready`

2. Added duplicate ABS coalescing:
- reject identical `(sx, sy)` sample as `duplicate_sample`
- avoids redundant cursor updates from stale repeat samples

3. Added diagnostics:
- `[shell.abs.sample] raw_x raw_y sx sy dt accepted reason`
- `[shell.cursor.delta] old/new/dx/dy`
- `[shell.cursor.rate] samples sends draws`

Kept constraints:
- no acceleration added to ABS path
- no drag/hitbox policy changes
- no display/renderer/kernel/ABI/opcode changes

## 3) Files changed
- `servers/silk-shell/src/main.rs`
- `docs/handoff/POINTER_SMOOTHNESS_CONTROL_V1.md`

## 4) Build result
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning for missing `x86_64-sex` target remains unchanged.

## 5) Runtime proof instructions
Run GTK usb-tablet lane and perform:
- slow left/right across center
- slow up/down to topbar
- small circles near toolbar
- click once in app body

Grep:
`grep -E "shell.abs.sample|shell.abs.normalize|shell.abs.reject|shell.cursor.delta|shell.cursor.rate|shell.cursor.final.send|sexdisplay.cursor.draw|shell.pointer.button|shell.click.real.target|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200`

Expected:
- mostly accepted ABS samples with stable deltas
- poison reasons (`corner_poison_after_ready` / `edge_poison_after_ready`) only on host escape artifacts
- no repetitive stale corner jumps after ready
- better control in y=50–80 topbar zone

## 6) Handoff path
- `docs/handoff/POINTER_SMOOTHNESS_CONTROL_V1.md`

## Backup
- `/tmp/silk-shell.main.rs.pre_pointer_smoothness_control_v1.bak`
- `/tmp/sexinput.main.rs.pre_pointer_smoothness_control_v1.bak`
