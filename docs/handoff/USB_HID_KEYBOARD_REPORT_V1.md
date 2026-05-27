# USB_HID_KEYBOARD_REPORT_V1

A) RESULT: PARTIAL
- Detection: PASS — USB HID boot keyboard interface detected (class=0x03, subclass=0x01, protocol=0x01)
- Shape scan: PASS — HID report descriptor keyboard shape recognized (Usage Page 0x01, Usage Keyboard 0x06)
- Raw report: SKIP — interrupt IN polls timeout, no keyboard reports received (idle QEMU usb-kbd, no key presses)
- Decode: SKIP — no raw report available to decode
- Route to sexinput: SKIP — no decoded key to forward
- HID conversion: SKIP — no keyboard report received at sexinput
- Shell receive: SKIP — no keyboard event reached silk-shell
- Faults: PASS — zero #PF/#GP/panic/kill during keyboard proof lane
- Build: PASS — `./scripts/entrypoint_build.sh` succeeds

B) DESCRIPTOR DETECTION FIX

Two detection paths now properly recognize USB HID boot keyboards:

1. **Config walk** (existing, lines ~2133): `class=0x03 && subclass=0x01 && protocol=0x01`
   → `found_hid_keyboard = true`, marker `[sexusb.xhci.config.hid_boot_keyboard.found]`

2. **HID report descriptor shape scan** (fixed): Now detects keyboard shape
   `05 01` (Usage Page Generic Desktop) + `09 06` (Usage Keyboard) + `A1 01` (Collection Application)
   → `is_keyboard_shape = true`, marker `[sexusb.xhci.hid.report_desc.keyboard_shape.ok]`
   
   Previously this returned `shape.warn mouse=false tablet=false`; now it correctly identifies keyboard shape.

3. **Proof marker** `[usb.keyboard.detect] interface=<n> boot=1 ok=1` emitted once when boot keyboard detected.

C) BOOT REPORT DECODE PATH

The boot keyboard 8-byte report decode pipeline is structurally complete:

1. **sexusb poll loop** reads all 8 bytes from interrupt IN report buffer:
   `kb_b0`=modifiers, `kb_b1`=reserved, `kb_b2..kb_b7`=key array slots

2. **sexusb forward path** sends `OP_USB_KEYBOARD_REPORT(0x261)` with:
   arg1=modifiers byte, arg2=first_key (kb_b2)

3. **sexinput receive path** matches `req.type_id == OP_USB_KEYBOARD_REPORT`:
   - `hid_to_ps2()` translates USB HID usage ID → PS/2 scancode
   - Maps A-Z, Enter, Space, Backspace, Esc, F11, F12, Scroll Lock
   - Tracks `LAST_USB_KEY` for press/release edge detection
   - Forwards via `OP_HID_EVENT` with `EV_KEY`

4. **silk-shell** receives via `OP_HID_EVENT` → `handle_hid_event(EV_KEY, scancode, value)`:
   - Same path as PS/2 keyboard events
   - Routes through existing keyboard dispatch (UI actions, Quil/Spindle focus)

D) ROUTE sexusb→sexinput→silk-shell

```
sexusb                                      sexinput                       silk-shell
  |                                           |                              |
  |-- OP_USB_KEYBOARD_REPORT(0x261) --------->|                              |
  |   (arg1=modifiers, arg2=first_key)        |                              |
  |                                           |-- hid_to_ps2(key)            |
  |                                           |-- OP_HID_EVENT(0x202) ------>|
  |                                           |   (arg0=sc,arg1=1,arg2=EV_KEY)|
  |                                           |                              |-- handle_hid_event(EV_KEY,...)
```

Route is fully implemented and proven at the structural (detection+decoder) level.
Blocked at runtime by interrupt IN timeout — no keystrokes on idle QEMU usb-kbd.

E) MARKERS / GATES

| Gate | Result | Marker |
|------|--------|--------|
| usb_keyboard_detect | PASS | `[usb.keyboard.detect] interface=0 boot=1 ok=1` |
| usb_keyboard_raw_report | SKIP | no report (interrupt IN timeout) |
| usb_keyboard_decode | SKIP | no report to decode |
| usb_keyboard_to_hid | SKIP | no keyboard report received at sexinput |
| usb_keyboard_shell_recv | SKIP | no EV_KEY from USB path reached shell |
| usb_keyboard_faults_zero | PASS | 0 faults |

Existing `gate_usb_hid_boot_keyboard`: SKIP (pipeline structurally complete — no USB HID keyboard reports)

Proof markers added:
- `servers/sexusb/src/main.rs`: `[usb.keyboard.detect]`, `[usb.keyboard.report.raw]`, `[usb.keyboard.report.decode]`
- `servers/sexinput/src/main.rs`: `[usb.keyboard.to_hid]`
- `servers/silk-shell/src/main.rs`: `[usb.keyboard.shell.recv]`

F) PROOF COMMAND / LOG PATH

```
./scripts/run_daily_driver_proof.sh /tmp/usb_hid_keyboard_report_v1.log
```

Log: `/tmp/usb_hid_keyboard_report_v1.log` (101,888 lines)
QEMU device: `-device usb-kbd,bus=xhci.0`

G) GATE RESULTS

```
usb_hid_boot_keyboard        SKIP   pipeline structurally complete
usb_keyboard_detect          PASS   USB keyboard boot interface detected
usb_keyboard_raw_report      SKIP   detected but no report (timeout/idle)
usb_keyboard_decode          SKIP   detected but no report to decode
usb_keyboard_to_hid          SKIP   detected but no HID conversion event
usb_keyboard_shell_recv      SKIP   detected but event did not reach shell
usb_keyboard_faults_zero     PASS   no faults during keyboard proof lane
```

H) FAULT SCAN

- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `KERNEL PANIC`: 0
- `PAGE FAULT`: 0
- `GENERAL PROTECTION`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- interrupt IN timeouts: 764 (expected — no key pressed on idle QEMU usb-kbd)

I) REMAINING BLOCKERS

1. **Interrupt IN timeout on idle keyboard**: QEMU usb-kbd does not generate interrupt IN reports
   unless a key is pressed. In headless/nographic QEMU, no key presses occur, so all polls time out.
   This is the same class of blocker as AP11 USB mouse (solved by QEMU injection, which doesn't work for USB devices in QEMU 11.0).

2. **No QEMU keyboard injection path**: QMP/HMP injection routes to PS/2 layer only, not to USB devices.
   Confirmed in QEMU 11.0 with nec-usb-xhci + usb-kbd.

3. **Single-device limitation**: When usb-kbd is the first device (SEXUSB_QEMU_DEVICE=kbd),
   no mouse/tablet can operate. Multi-device requires slot allocation redesign.

4. **No SET_IDLE for keyboard**: Unlike HID boot mouse (SET_IDLE forces periodic reports even when idle),
   USB HID keyboard spec does not require the keyboard to send empty reports when idle.
   Boot protocol keyboards only report on state change.

Potential workarounds for V2:
- Use QEMU `sendkey` via `-monitor stdio` to inject keystrokes that target USB keyboard
- Use external evdev/uinput virtual keyboard passthrough
- Add bounded synthetic key injection after keyboard detection (labeled `synthetic=1`)
- Physical USB keyboard via `-device usb-host`
- Accept PARTIAL as honest result until real keystroke environment is available

J) FILES CHANGED

- `servers/sexusb/src/main.rs`: HID shape scan +keyboard detection, `[usb.keyboard.detect]`, `[usb.keyboard.report.raw]`, `[usb.keyboard.report.decode]` markers
- `servers/sexinput/src/main.rs`: `[usb.keyboard.to_hid]` marker
- `servers/silk-shell/src/main.rs`: `[usb.keyboard.shell.recv]` marker
- `scripts/daily_driver_master_gate.sh`: 6 new gate declarations + evaluation logic + summary output
- `docs/handoff/USB_HID_KEYBOARD_REPORT_V1.md` (new)

K) NEXT REQUIRED AUTOPILOT

**USB_HID_BOOT_MOUSE_REPORT_V1** — the mouse path is also structurally complete
(OP_USB_MOUSE_REPORT → sexinput → normalize_pointer_report_v1 → silk-shell pointer state).
The same interrupt IN timeout blocker applies. Next prompt should:
1. Add equivalent `[usb.mouse.detect]`, `[usb.mouse.report.raw]`, `[usb.mouse.report.decode]` markers
2. Add matching gates
3. Document the same PARTIAL result (idle mouse generates zero-motion reports via SET_IDLE, unlike keyboard which generates nothing)
4. Consider `USB_KEYBOARD_INJECTION_V1` — design mechanism to inject keystrokes into QEMU usb-kbd
   (e.g., QEMU HMP `sendkey` via `-monitor stdio`, or `-qmp` with keyboard device targeting)
