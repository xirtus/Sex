# RELATIVE_POINTER_TRANSFER_V1

Date: 2026-05-14

## Goal
Tune usb-mouse REL transfer in `silk-shell` for smoother, controllable manual GTK use without changing decode/routing.

## Audit result
REL ingress path is unchanged and still proven at:
- `[silk-shell.rel.recv]`
- `[shell.cursor.final.send] source=rel`

The prior transfer in `apply_rel_pointer()` was effectively acceleration-heavy and amplified medium/large deltas (including saturated ±127 reports), making cursor control jumpy.

## Fix applied
File changed:
- `servers/silk-shell/src/main.rs`

Function changed:
- `apply_rel_pointer(dx_raw, dy_raw)` transfer axis function.

New conservative transfer (no acceleration):
- `abs <= 3` -> keep 1:1
- `4..16` -> halve (`abs/2`)
- `>= 17` -> saturate to `12` with sign

This preserves micro control, reduces mid-range jumps, and hard-caps large burst motion from host saturation.

## Diagnostics added
Bounded marker added:
- `[shell.rel.transfer] raw_dx=N raw_dy=N out_dx=N out_dy=N x=N y=N reason=...`

Reason classes:
- `zero`
- `micro_keep`
- `medium_half`
- `large_cap12`

Existing markers were preserved.

## Build result
- `./scripts/entrypoint_build.sh` completed successfully.

## Runtime proof command
Use usb-mouse GTK lane and grep:

```bash
grep -E "silk-shell.rel.recv|shell.rel.transfer|shell.cursor.final.send|sexdisplay.cursor.draw|shell.pointer.button|shell.click.real.target|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

Expected:
- `source=rel` remains present.
- `shell.rel.transfer` shows large raw inputs capped to `out=±12`.
- small raw deltas remain responsive (`±1..±3` mostly unchanged).
- fault count remains 0.

## Notes
- No kernel/ABI/opcode/display/renderer changes.
- Tablet ABS path was not modified.
