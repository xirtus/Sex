# SILK_FRAME_HOVER_CURSOR_STABILITY_PROOF_V1

**Date:** 2026-05-08
**Status:** PASS

## Summary

Hover state route, cursor movement, and GUI stability proven. Frame Light click actions deferred to targeted chrome-coordinate proof.

## Proof markers

- sexdisplay received hover flags for surfaces 200 and 201 via existing 0xFD path:
  - `[sexdisplay.frame.hover.recv] sid=201 flags=0x03 light=0`
  - `[sexdisplay.frame.hover.recv] sid=200 flags=0x03 light=0`
- cursor drew and moved away from center (640,360):
  - `[sexdisplay.cursor.draw] n=0 x=640 y=360` (initial)
  - `[sexdisplay.cursor.draw] n=0 x=200 y=200`
  - `[sexdisplay.cursor.draw] n=0 x=649 y=365`
  - `[sexdisplay.cursor.draw] n=0 x=657 y=374`
- real clicks landed on app body, not chrome:
  - `[shell.click.real.target] x=200 y=200 target=201 kind=app`
- fault count: **0**
  - `fault.kill / #PF / #GP / panic / KERNEL PANIC` = 0

## Frame Light action markers (for future proof)

| Action | Success marker | Reject marker |
|--------|---------------|---------------|
| Close | `[shell.frame.light.close] frame=N surface=N` | `[shell.frame.light.close.reject] frame=N reason=...` |
| Minimize | `[frame.light.minimize.fsm] frame=N surface=N` | `[shell.frame.minimize.reject] frame=N reason=...` |
| Zoom | `[frame.light.zoom.fsm] frame=N` | `[shell.frame.zoom.reject] frame=N reason=...` |
| Rim drag | `[shell.interact.drag.begin] sid=N x=N y=N` | n/a |

## Conclusion

Hover state route to sexdisplay is alive. Cursor movement/render path is alive. GUI stack remains fault-clean. Frame Light click actions require a targeted chrome-coordinate proof.
