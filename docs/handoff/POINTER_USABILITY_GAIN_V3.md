# POINTER_USABILITY_GAIN_V3

Date: 2026-05-14

## Summary
Pointer usability issue was caused by over-aggressive ABS rejection logic and duplicated ABS handling paths.

Fix applied:
- Unified ABS handling into one shared function used by both:
  - pre-linen/drain path (`handle_hid_event`)
  - main event loop EV_ABS path
- Kept ABS as direct 1:1 mapping (no acceleration added).
- Reduced rejection policy to minimal pre-ready poison guards only:
  - reject near-zero init (`sx<=1 && sy<=1`) before ready
  - reject max-edge sentinel before ready
- After first valid ABS sample (`ABS_SEEN_VALID`), accept all normalized ABS coordinates and immediately update cursor.

## What changed
File changed:
- `servers/silk-shell/src/main.rs`

Added helper:
- `process_abs_tablet(raw_x, raw_y)`

Added markers:
- `[shell.abs.normalize] raw_x=N raw_y=N sx=N sy=N accepted=N reason=N`
- `[shell.abs.reject] reason=N raw_x=N raw_y=N last_x=N last_y=N`

Behavior preserved:
- No kernel/ABI/opcode/display/render changes.
- REL path still disabled once ABS is valid (`apply_rel_pointer` returns 0,0 when `ABS_SEEN_VALID`).
- Button/click/drag logic unchanged.

## Why this should improve usability
- Removes jump-distance and near-corner post-ready rejection that could drop legitimate tablet positions and cause sticky/slow targeting.
- Ensures both dispatch paths apply identical ABS normalization/accept logic.
- Ensures accepted ABS always updates `POINTER_X/Y` and sends cursor immediately.

## Build result
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning for missing `x86_64-sex` target remains unchanged.

## Runtime proof command/grep
Move cursor to center/topbar/frame-light area/app body and click each:

`grep -E "shell.abs.normalize|shell.abs.reject|shell.cursor.final.send|sexdisplay.cursor.draw|shell.pointer.button|shell.click.real.target|shell.drag.candidate|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200`

Expected:
- frequent `[shell.abs.normalize ... accepted=1 reason=ok]` matching movement
- only occasional early rejects (`zero_init`/`edge_before_ready`) at startup
- no recurring stale corner poison pattern after ready
- cursor reaches topbar/chrome zones predictably
- fault count 0

## Backup
- `/tmp/silk-shell.main.rs.pre_pointer_usability_gain_v3.bak`
- `/tmp/sexinput.main.rs.pre_pointer_usability_gain_v3.bak`
