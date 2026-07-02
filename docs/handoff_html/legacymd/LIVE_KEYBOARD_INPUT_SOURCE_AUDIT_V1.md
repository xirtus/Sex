# LIVE_KEYBOARD_INPUT_SOURCE_AUDIT_V1

## A) RESULT

Root cause identified. No code change required.
Enter keytrace is synthetic, not physical keyboard.
Physical keyboard not reaching guest (SDL grab inactive).

---

## B) WHY ENTER ARRIVES — AND WASD DO NOT

### Source of `[keyboard.cursor.debug.keytrace] key=0x1c`

NOT physical keyboard. Origin: **`KEYBOARD_EDGE_PROOF_V1`** in
`servers/sexinput/src/main.rs:957-970`.

```rust
// 6a. KEYBOARD_EDGE_PROOF_V1: one-shot EV_KEY down+up for Enter (scancode 0x1C).
//     Proves sexinput→silk-shell EV_KEY path without SEXOS_KEYBOARD_PROOF env var.
//     Gated by !SYNTHETIC_INPUT_PROOFS_DISABLED (same gate as other default proofs).
if !SYNTHETIC_INPUT_PROOFS_DISABLED {
    match ev_key_edge_stage {
        0 if tick == 3 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 0x1Cu64, 1, EV_KEY);
            // → shell handle_hid_event → keytrace → cursor debug click
            ev_key_edge_stage = 1;
        }
        1 if tick == 4 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 0x1Cu64, 0, EV_KEY);
            ev_key_edge_stage = 2;
        }
        _ => {}
    }
}
```

Fires at boot tick 3, unconditionally. No `SEXOS_KEYBOARD_PROOF` required.
Sends `EV_KEY(code=0x1C, value=1)` to silk-shell via `OP_HID_EVENT`.
Shell receives it → `handle_hid_event(EV_KEY, 0x1C, 1)` → keytrace fires.

### What happens with Enter in shell

Shell `handle_hid_event` (silk-shell/main.rs:9140-9149):
```rust
0x39 | 0x1C => {
    let pointer_ready = ABS_SEEN_VALID || POINTER_USB_STATE_INIT;
    if pointer_ready {
        handle_hid_event(EV_BTN, 1, 1);  // click down
        handle_hid_event(EV_BTN, 1, 0);  // click up
        serial_println!("[keyboard.cursor.debug.click] key=0x1c button=1 ok=1");
    }
```

`ABS_SEEN_VALID` is set by sexinput's synthetic EV_ABS at tick 6.
At tick 3 (Enter proof), ABS not yet seen → click fires `ok=0 reason=pointer_not_ready`.
If user presses Enter physically after tick 6: EV_ABS already set → click `ok=1`.

### Why WASD produce no keytrace

No synthetic proof sends `EV_KEY` for 0x11/0x1E/0x1F/0x20.
Physical keyboard (PS/2) not delivering scancodes to sexinput.
See §C.

---

## C) WHERE PHYSICAL W/A/S/D GO

### QEMU keyboard routing with sdl-grab

Running:
```
SEXOS_KEYBOARD_CURSOR=1 SEXOS_QEMU_DISPLAY=sdl-grab SEXUSB_QEMU_DEVICE=tablet ./dev.sh
```

Generates:
```
-M q35                              ← i8042 PS/2 controller ON (default)
-device usb-tablet,bus=xhci.0
-display sdl,grab-mod=lctrl-lalt   ← requires explicit grab
```

**SDL `grab-mod=lctrl-lalt` behavior:**
- SDL window starts in **ungrabbed** state
- Keyboard (and mouse) route to **host** until user presses LCtrl+LAlt inside SDL window
- Without grab activation: all keystrokes (`w a s d Enter Space`) → host terminal
- With grab active: keystrokes → QEMU PS/2 i8042 → kernel IRQ1 → `INPUT_RING` → sexinput

### PS/2 keyboard pipeline (when grab IS active)

```
SDL key event
→ QEMU PS/2 scancode (set 1: W=0x11 A=0x1E S=0x1F D=0x20)
→ kernel keyboard_interrupt_handler (interrupts.rs:762)
→ INPUT_RING.enqueue(scancode)
→ sexinput pdx_try_listen_raw(SLOT_INPUT) → type_id=0x201
→ pdx_call(SLOT_SHELL, OP_HID_EVENT, code, value, EV_KEY)   ← line 686 (unconditional)
→ if KEYBOARD_CURSOR_ENABLED:
    pdx_call(SLOT_SHELL, OP_HID_EVENT, dx, dy, EV_REL)       ← line 709 (WASD only)
```

Sexinput log markers (appear only if grab is active and key pressed):
```
[sexinput.ps2.scancode] raw=0x11          ← W pressed
[keyboard_cursor.key] code=0x11 dx=0 dy=-8
[keyboard_cursor.emit.rel] dx=0 dy=-8
```

### WASD destination without grab

W/A/S/D keystrokes go to fish shell (terminal running dev.sh). They produce
fish shell completions / history navigation / output. Never reach QEMU guest.

---

## D) SMALLEST FIX

### Fix A — Drop grab-mod (recommended, zero code change)

```bash
SEXOS_KEYBOARD_CURSOR=1 SEXOS_QEMU_DISPLAY=sdl SEXUSB_QEMU_DEVICE=tablet ./dev.sh
```

`-display sdl` (no `grab-mod`): keyboard goes to guest when SDL window has focus.
Click SDL window once to focus → type W/A/S/D → scancodes reach PS/2 → sexinput → shell.

Expected new markers:
```
[sexinput.ps2.scancode] raw=0x11
[keyboard_cursor.key] code=0x11 dx=0 dy=-8
[keyboard_cursor.emit.rel] dx=0 dy=-8
[keyboard.cursor.debug.keytrace] key=0x11 pressed=1 ok=1
[keyboard.cursor.debug.move] key=0x11 dx=0 dy=-32 old_x=640 old_y=360 new_x=640 new_y=328 ok=1
```

### Fix B — Activate grab first (same binary, sdl-grab mode)

With `SEXOS_QEMU_DISPLAY=sdl-grab`:
1. SDL window opens
2. Click inside it
3. Press **LCtrl+LAlt** inside SDL window to activate grab
4. Type W/A/S/D

Same markers as Fix A appear after grab.

### Fix C — Add WASD to synthetic proof lane (tiny code change)

Extend `KEYBOARD_EDGE_PROOF_V1` in `sexinput/main.rs` to also send WASD
EV_KEY events after tick 4. This proves keyboard cursor path without physical
keyboard or SDL grab. Useful for CI/headless boots.

Candidate ticks: 5=W down, 6=W up, 7=A down, 8=A up, etc.
Each: `pdx_call(SLOT_SHELL, OP_HID_EVENT, 0x11u64, 1, EV_KEY);`

Not patched here — audit only.

---

## E) FILES CHANGED

None. Audit only.

---

## F) NEXT PROMPT NAME

`LIVE_KEYBOARD_WASD_GRAB_PROOF_V1`

**Mission:** Run with `SEXOS_QEMU_DISPLAY=sdl` (not sdl-grab), click SDL window,
press W/A/S/D, verify `[sexinput.ps2.scancode]`, `[keyboard_cursor.key]`, and
`[keyboard.cursor.debug.move]` appear. Gate: cursor moves from (640,360) by WASD
input. Confirm keytrace fires for 0x11/0x1E/0x1F/0x20, not just 0x1C.
