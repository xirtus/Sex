# KEYBOARD_SAFE_CLOSE_PROOF_V1

**Date**: 2026-05-14
**Status**: PASS — F11 / AccessClose safely proven in 1 attempt

## Summary

F11 keyboard close (scancode 0x57 → SurfaceAction::AccessClose) was skipped in the
broad keyboard GUI proof as "safe_close_not_proven" because no safe disposable
target existed. This proof creates a disposable test surface (102/TEST3), focuses
it, dispatches F11 through the real `handle_hid_event` EV_KEY path, and verifies
the close destroys only the disposable target while Quil (201) and Linen (200)
remain intact.

## Proof Design

### Disposable Target
- **Surface 102 (TEST3)**: shell-owned test surface, focusable, closeable
- Created on sexdisplay via existing 0xEC opcode (no new ABI)
- No frame association → close doesn't affect tiling layout
- Already lifecycle-registered at boot via `lifecycle_init_all()`

### Proof Sequence (SEXOS_KEYBOARD_SAFE_CLOSE_PROOF=1)

| Stage | Action                  | Mechanism                        |
|-------|-------------------------|----------------------------------|
| 0     | CreateTarget            | 0xEC → sexdisplay, set alive     |
| 1     | FocusTarget             | `try_set_focus(102)`             |
| 2     | DispatchF11             | `handle_hid_event(EV_KEY, 0x57, 1)` |
| 3     | Verify                  | Check closed_102, quil_alive, linen_alive  |

### Key Markers Emitted

```
[shell.kbd.close.proof] stage=0 action=CreateTarget surface=102 alive=1
[shell.kbd.close.target] frame=0 sid=102 disposable=1 focused=1
[shell.kbd.ui.consume] scancode=87 action=AccessClose down=1 consumed=1 path=handle_hid_event_drain
[shell.window.action] action=Close frame=0 sid=102 ok=1 reason=ok
[shell.kbd.close.proof] stage=3 action=Verify closed_102=1 quil_alive=1 linen_alive=1 faults=0
[shell.frame.close.proof.done] ok=1 frame=0 sid=102 reason=safe_close_proven
```

## Close Path Exercised

`handle_hid_event(EV_KEY, 0x57, 1)` → `scancode_to_action(0x57)` → `AccessClose`
→ `access_handle_keyboard_action(AccessClose)` → `close_surface_from_frame_light(102)`
→ lifecycle: Visible → Closing → Tombstoned → Destroyed
→ `pdx_call(OP_SURFACE_DEACTIVATE, 102)` → sexdisplay deactivates slot
→ `SURFACE_102_ALIVE = false`
→ focus fallback to Quil (201)
→ `tile_active_scene_frames()`

## Verification

- closed_102 = 1 (SURFACE_102_ALIVE = false after close)
- quil_alive = 1 (surface 201 still alive, not tombstoned)
- linen_alive = 1 (surface 200 still alive, not tombstoned)
- faults = 0 (no #PF, #GP, KERNEL PANIC in serial log)
- Focus correctly fell back to Quil (201) after close

## Files Changed

- `servers/silk-shell/src/main.rs` — added proof gate, proof function, event-loop wiring

Not touched:
- sexdisplay, sexusb, sexinput, kernel, ABI

## Build

```
SEXOS_KEYBOARD_SAFE_CLOSE_PROOF=1 ./scripts/entrypoint_build.sh   # PASS
./scripts/entrypoint_build.sh                                       # PASS
```

## Autopilot Result

- Attempts: 1
- Close target: surface 102 (TEST3) — safe, disposable, non-framed
- F11 dispatch: through real `handle_hid_event` path
- Close proven: yes
- Core surfaces preserved: yes (Quil 201, Linen 200)
- Faults: 0
- No STOP FIRST needed
