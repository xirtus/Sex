# CURSOR_INPUT_INTERACTIVE_QEMU_DEFAULTS_V1

## A) PASS/FAIL
- PASS (build clean, 156 gates PASS, 0 FAIL, 0 faults)

## B) Exact Root Causes Addressed

### 1. QEMU tablet device not bound to SDL display by default
`dev.sh` defaulted to `SEXUSB_QEMU_DEVICE=mouse`, which produces a boot-HID-only
USB mouse with no SDL binding. QEMU routes SDL window mouse events to PS/2 i8042
by default, leaving the USB tablet endpoint idle (zero Transfer Events).

**Fix**: Changed default to `tablet-display-sdl`. This passes `display=sdl` to
`usb-tablet,bus=xhci.0`, wiring SDL mouse events directly to USB HID. Transfer
Events now arrive in the interrupt-IN ring and sexusb delivers them to sexinput.

Override: `SEXUSB_QEMU_DEVICE=mouse` to revert to boot-HID-only probe mode.

### 2. SEXOS_KEYBOARD_CURSOR=1 triggered cursor autodemo, fighting physical input
`CURSOR_AUTODEMO_ENABLED` in both sexinput and silk-shell was:
```rust
option_env!("SEXOS_CURSOR_AUTODEMO").is_some()
    || option_env!("SEXOS_KEYBOARD_CURSOR").is_some()
```
Setting `SEXOS_KEYBOARD_CURSOR=1` for physical keyboard cursor input also activated
the autonomous cursor demo loop (OP_AUTODEMO_TICK every 30 ticks, ticks 30–750).
The demo moves cursor right→down→left→up independently, overwriting positions set
by physical key input.

**Fix**: Decoupled. `CURSOR_AUTODEMO_ENABLED` now only depends on
`SEXOS_CURSOR_AUTODEMO`. Keyboard cursor and autodemo are independent flags.

### 3. Keyboard cursor debug unreachable from live EV_KEY dispatch
The `KEYBOARD_CURSOR_DEBUG_ENABLED` cursor move and keytrace code lived only in
`handle_hid_event()`. The main event loop's `OP_HID_EVENT` handler dispatches
EV_KEY events inline — it never calls `handle_hid_event` for normal input. So
`[keyboard.cursor.debug.keytrace]` and `[keyboard.cursor.debug.move]` never fired
during live PS/2 keyboard input.

**Fix**: Added keyboard cursor debug block to the main event loop EV_KEY dispatch
path (before `scancode_to_action` / focus routing). Block fires keytrace for every
key-down and moves cursor for WASD/arrow keys when `KEYBOARD_CURSOR_DEBUG_ENABLED`.
The handle_hid_event path is preserved for linen_sync_reply and pre-linen-drain
coverage.

## C) Files Changed

- `dev.sh`
- `servers/sexinput/src/main.rs`
- `servers/silk-shell/src/main.rs`
- `docs/handoff/CURSOR_INPUT_INTERACTIVE_QEMU_DEFAULTS_V1.md`

## D) Minimal Diff Summary

### dev.sh
- Default `SEXUSB_QEMU_DEVICE` changed: `mouse` → `tablet-display-sdl`
- Help text updated to reflect new default
- Comment updated to explain SDL binding benefit

### servers/sexinput/src/main.rs
- `CURSOR_AUTODEMO_ENABLED`: removed `|| option_env!("SEXOS_KEYBOARD_CURSOR").is_some()`
- Added boot marker block (fires only when `KEYBOARD_CURSOR_ENABLED=1`):
  ```
  [cursor.input.autodemo.decoupled] keyboard_cursor=1 autodemo=0 ok=1
  ```

### servers/silk-shell/src/main.rs
- `CURSOR_AUTODEMO_ENABLED`: removed `|| option_env!("SEXOS_KEYBOARD_CURSOR").is_some()`
- Added keyboard cursor debug block inside `if event_class == EV_KEY && value == 1`
  at main event loop, before `scancode_to_action`:
  - Emits `[keyboard.cursor.debug.keytrace]` for every key-down
  - Emits `[keyboard.cursor.debug.begin]`, `[keyboard.cursor.debug.move]`,
    `[keyboard.cursor.debug.done]` on WASD/arrow cursor movement
  - Sets `mutated = true` for cursor moves
  - Does NOT consume the key; normal focus routing continues after

## E) Proof Markers

### Observed in headless harness (logs/qemu-latest.log):
```
[cursor.autodemo.gate] enabled=0
[usb.hid.boot_mouse.pass] ok=1
[usb.hid.intr.event] slot=1 dci=3 code=13 actual=6 ok=1
[sexusb.hid.tablet.report] i=0 buttons=0x0 x=0 y=0
[usb.hid.pointer.pass] ok=1
```

### Expected markers (interactive SDL run with SEXOS_KEYBOARD_CURSOR=1):
```
[cursor.input.autodemo.decoupled] keyboard_cursor=1 autodemo=0 ok=1
[keyboard.cursor.debug.keytrace] key=0x11 pressed=1 ok=1       ← W key
[keyboard.cursor.debug.begin] ok=1
[keyboard.cursor.debug.move] key=0x11 dx=0 dy=-32 old_x=640 old_y=360 new_x=640 new_y=328 ok=1
[keyboard.cursor.debug.done] ok=1
```

### Expected markers (interactive SDL run, tablet-display-sdl, mouse moved):
```
[usb.hid.intr.event] slot=1 dci=3 code=1 actual=... ok=1
[sexusb.hid.tablet.report] buttons=0 x=<nonzero> y=<nonzero>
[usb.hid.pointer.emit] op=OP_HID_EVENT ...
[silk-shell.pointer.recv] class=EV_ABS ...
[cursor.motion.bounds] source=abs ... ok=1
```

## F) Commands Run and Results

```bash
# Backup
cp dev.sh dev.sh.bak.cursor_defaults_v1
cp servers/sexinput/src/main.rs servers/sexinput/src/main.rs.bak.cursor_defaults_v1
cp servers/silk-shell/src/main.rs servers/silk-shell/src/main.rs.bak.cursor_defaults_v1

# Build
./scripts/entrypoint_build.sh
# Result: PASS — ISO generated, limine installed, verification complete.

# Harness
./scripts/qemu_harness.sh --timeout 30 --markers
# Result: exit 124 (timeout, not crash). Log at logs/qemu-latest.log.

# Gate scan
./scripts/daily_driver_master_gate.sh logs/qemu-latest.log
# Result: PASS — 156 gates PASS, 0 FAIL, 0 faults.
```

Fault scan clean: no `#PF`, `#GP`, `panic`, `fault.kill` in log.

## G) Remaining Deferred Work / STOP FIRST Notes

- **PS/2 mouse IRQ12**: not implemented; SexOS PS/2 mouse lane still absent.
  STOP FIRST if implementing — requires kernel interrupt handler edit.
- **TOUCHPAD_ABS_CONTACT_V1**: not started.
- **TRACKPAD_GESTURES_V1**: not started.
- **USB_HID_POINTER_PRODUCER live movement/click stimulus**: headless harness
  delivers idle tablet reports (x=0, y=0). Live cursor movement proof requires
  interactive SDL run with mouse physically moved in SDL window.
  Next prompt: `LIVE_CURSOR_TABLET_SDL_PROOF_V1` — run dev.sh (now defaults to
  tablet-display-sdl), move mouse in SDL window, verify `cursor.motion.bounds ok=1`.
- **KEYBOARD_CURSOR_DEBUG click path in main loop**: Space/Enter click synthesis
  (EV_BTN path) was NOT added to the main loop cursor debug block — only in
  handle_hid_event. Click requires `ABS_SEEN_VALID || POINTER_USB_STATE_INIT`.
  Add to main loop if click-via-keyboard proof is needed.

## H) This Phase Does Not

- Implement gesture policy, click-focus policy changes, or drag policy changes
- Change display rendering or sexdisplay
- Edit kernel, sex-pdx, or ABI
- Implement PS/2 mouse IRQ12
- Rewrite input stack or delete synthetic proof infrastructure

## I) Handoff Path

`docs/handoff/CURSOR_INPUT_INTERACTIVE_QEMU_DEFAULTS_V1.md`

## J) Interactive Test Command (after this phase)

```bash
# SDL cursor (mouse moves cursor):
./dev.sh

# SDL keyboard cursor (WASD moves cursor, no SEXOS_PROOFS_DISABLED needed):
SEXOS_KEYBOARD_CURSOR=1 ./scripts/entrypoint_build.sh && ./dev.sh
```
