# RELATIVE_POINTER_TRANSFER_V2

Date: 2026-05-14

## Goal
Adjust REL transfer after V1: keep safety, improve speed/feel by raising large-delta cap.

## Change scope
Patched only:
- `servers/silk-shell/src/main.rs`

Added handoff:
- `docs/handoff/RELATIVE_POINTER_TRANSFER_V2.md`

## Constants tuned
In `apply_rel_pointer()` transfer function:
- `abs <= 3` unchanged (1:1)
- `4..16` unchanged (half)
- `>=17` cap changed from `12` to `18`

Marker reason updated:
- `large_cap12` -> `large_cap18`

No transfer-shape redesign, no acceleration added.

## Expected effect
- large raw bursts (including saturated ±127) map to ±18 instead of ±12.
- motion should feel faster/easier than V1 while still bounded.
- fine micro-control remains unchanged.

## Build result
- `./scripts/entrypoint_build.sh` completed successfully.

## Runtime proof
Use usb-mouse lane and grep:

```bash
grep -E "silk-shell.rel.recv|shell.rel.transfer|shell.cursor.final.send|sexdisplay.cursor.draw|shell.pointer.button|shell.click.real.target|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

Expected evidence:
- `[shell.rel.transfer] ... reason=large_cap18`
- raw ±127 produces out ±18
- `[shell.cursor.final.send] source=rel` still present
- no faults

## Non-changes (explicit)
- no kernel/ABI/sex-pdx/sexdisplay/opcode changes
- no ABS/tablet path changes
- no drag/hitbox policy changes
