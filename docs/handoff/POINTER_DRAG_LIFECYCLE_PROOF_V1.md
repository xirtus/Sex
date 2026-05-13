# POINTER_DRAG_LIFECYCLE_PROOF_V1

Date: 2026-05-13

## 1) Exact draggable zone / manual gesture
From current `silk-shell` code:
- Drag can start in two ways:
  1. **Frame rim drag**: click/hold on frame rim (not frame-light icons), then move.
     - Existing marker: `[shell.frame.rim.drag.start] frame=... surface=... x=... y=...`
  2. **Content drag**: click/hold inside focused shell-managed surface content area (`point_in_surface(...)`), then move.
     - Existing marker: `[shell.interact.drag.begin] sid=... x=... y=...`
- Release with left button up to end drag.

Recommended GTK gesture:
1. Wait 5s after boot.
2. Move cursor onto a visible frame **rim band** (avoid close/minimize/zoom lights).
3. Left button down and hold.
4. Move 50-150 px.
5. Left button up.

## 2) Whether current code already supports drag
Yes. Drag state machine already exists and is active:
- `ClickPending -> Dragging -> Idle`
- drag move path via `drag_move_focused(dx, dy)`
- existing markers already included: `shell.interact.drag.begin/move/end`, `shell.frame.rim.drag.start`.

## 3) First dead hop if no drag observed
Likely proof-gap was marker/gesture mismatch, not missing code.
If still no drag, first hop to check is:
- left-button event seen (`shell.pointer.button down=1`) and candidate target kind (`shell.drag.candidate`).
- If candidate is chrome tab strip or SilkBar region, drag may be intentionally skipped.

## 4) Files changed
- `servers/silk-shell/src/main.rs`
- `docs/handoff/POINTER_DRAG_LIFECYCLE_PROOF_V1.md`

## 5) Build result
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host preflight warning for missing `x86_64-sex` target remains unchanged.

## 6) Runtime proof command/grep
Run GTK usb-tablet lane, perform the gesture above, then:

`grep -E "shell.pointer.button|shell.drag.candidate|shell.drag.begin|shell.drag.update|shell.drag.end|shell.interact.drag|shell.frame.rim.drag|shell.click.real.target|shell.hit_target.chrome|shell.cursor.final.send|sexusb.tablet.active|sexinput.pointer.raw|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200`

Expected chain:
- `[shell.pointer.button] btn=1 down=1 ...`
- `[shell.drag.candidate] target=... kind=...`
- `[shell.drag.begin] ...` (frame id nonzero for rim drag, frame=0 for content drag)
- `[shell.drag.update] ... dx=... dy=...`
- `[shell.drag.end] ...`

## Marker additions (diagnostic aliases only)
Added alias markers without behavior changes:
- `[shell.pointer.button] btn=N down=N x=N y=N`
- `[shell.drag.candidate] target=N kind=N x=N y=N`
- `[shell.drag.begin] sid=N frame=N x=N y=N`
- `[shell.drag.update] sid=N frame=N x=N y=N dx=N dy=N`
- `[shell.drag.end] sid=N frame=N x=N y=N`

Existing markers preserved: `shell.interact.drag.*`, `shell.frame.rim.drag.start`.
