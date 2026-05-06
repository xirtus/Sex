# FOCUS_WINDOW_ALIVE_V1

Status: PASS

## Proven

SexOS now has a primitive but real interactive desktop loop:

1. keyboard input reaches silk-shell
2. silk-shell routes key events to the focused surface owner
3. Quil receives routed HID events
4. Quil reacts by drawing through SexDisplay
5. visible state changes appear on screen

## Implementation summary

### Track C1: Focus State
- silk-shell default focus is set to `SURFACE_ID_QUIL` / `201`
- Quil surface participates in existing z-order / click-to-focus path

### Track C2: Key Routing
- silk-shell EV_KEY handler routes keyboard events to Quil when:
  - `FOCUSED_SURFACE_ID == SURFACE_ID_QUIL`
- route uses:
  - `pdx_call(SLOT_QUIL, OP_HID_EVENT, scancode, value, EV_KEY)`

### Track C3: Visible Change
- kernel init grants `SLOT_DISPLAY` to Quil PD
- Quil listens for `OP_HID_EVENT` via `pdx_listen_raw()`
- on key press, Quil calls SexDisplay fill-rect opcode `0xEF`
- Quil toggles visible color between:
  - Magenta `0xFF00FF`
  - Cyan `0x00FFFF`

## Proof markers

Observed probe log:

```text
[silk-shell.key.route] owner=quil sid=201 scancode=0x1e
[quil.key.recv] scancode=0x1e val=1
[silk-shell.focus.visual_update] color=0xff00ff

[silk-shell.key.route] owner=quil sid=201 scancode=0x30
[quil.key.recv] scancode=0x30 val=1
[silk-shell.focus.visual_update] color=0xffff
```

## Commit

`input: implement primitive focus, keyboard routing, and visual update for Quil`

## Next phase

Move from primitive focus proof to minimal window semantics:

1. explicit surface registry
2. click hit-test selects focused surface
3. focus ring / visual border
4. keyboard routed to focused surface only
5. unfocused surfaces do not receive keys
6. deterministic z-order update
7. minimal two-surface demo