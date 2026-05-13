# DRAG_HITBOX_BOUNDS_V1

Date: 2026-05-14

## Goal
Diagnostics-only proof of real hit-test and draggable bounds used by shell drag/chrome logic.

## What was added (no behavior change)
File changed:
- `servers/silk-shell/src/main.rs`

Added bounded markers:
1. Bounds marker inside chrome hit-test path:
- `[shell.drag.bounds] sid=N frame=N sx=N sy=N sw=N sh=N topbar_y0=N topbar_y1=N rim_y0=N rim_y1=N draggable_x0=N draggable_x1=N`

2. Per-click candidate classifier marker:
- `[shell.drag.hit_test] x=N y=N result=<none|app|rim|tab|light_close|light_min|light_zoom|chrome> draggable=<0|1>`

Existing markers preserved:
- `shell.click.real.target`
- `shell.drag.candidate`
- `shell.hit_target.chrome`
- `shell.frame.rim.drag.start`

No hit-test logic changed.
No geometry/hitbox values changed.
No pointer calibration/renderer/display/ABI/opcode changes.

## Build
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning for missing `x86_64-sex` target remains unchanged.

## Runtime proof command
`grep -E "shell.drag.bounds|shell.drag.hit_test|shell.pointer.button|shell.click.real.target|shell.drag.candidate|shell.hit_target.chrome|shell.drag.begin|shell.drag.update|shell.drag.end|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1000`

## How to use the output
- Use `[shell.drag.hit_test]` to identify what the click resolved to at each x/y probe.
- Use `[shell.drag.bounds]` to get exact active surface/frame rectangle and topbar/rim ranges used by code.
- If clicks keep resolving `result=app` and `draggable=1` but no drag markers appear, focus on button-hold movement path.
- If `result=tab` or `light_*`, clicks are intentionally not rim-drag starts.

## Backup
- `/tmp/silk-shell.main.rs.pre_drag_hitbox_bounds_v1.bak`
