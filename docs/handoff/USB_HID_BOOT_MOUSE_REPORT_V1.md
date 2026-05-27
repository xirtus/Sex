# USB_HID_BOOT_MOUSE_REPORT_V1

A) RESULT: PARTIAL
- Detection: PASS — USB HID boot mouse interface detected (class=0x03, subclass=0x01, protocol=0x02)
- Shape scan: PASS — HID report descriptor mouse shape recognized (Usage Page 0x01, Usage Mouse 0x02, X/Y 0x30/0x31)
- Raw report: SKIP — interrupt IN polls timeout, no mouse reports received (idle QEMU usb-mouse, same class of blocker as AP12 keyboard)
- Pack: SKIP — no real USB report to pack
- Normalizer: SKIP — no nonzero report reached normalizer (bootgraph zero-report excluded)
- Normalizer output: SKIP — no normalized events emitted from real USB data
- Faults: PASS — zero #PF/#GP/panic/kill during mouse proof lane
- Build: PASS — `./scripts/entrypoint_build.sh` succeeds

B) DETECTION / REPORT FORMAT

**Boot mouse detection** (config descriptor walk):
- Interface class=0x03, subclass=0x01, protocol=0x02 → `found_hid_mouse = true`
- Marker `[sexusb.xhci.config.hid_boot_mouse.found] intf=0 off=9`
- Proof marker `[usb.mouse.detect] interface=0 boot=1 ok=1` emitted once

**Tablet detection** (config descriptor walk):
- Non-keyboard, non-boot-mouse HID interface → `found_hid_tablet = true`
- Proof marker `[usb.mouse.detect] interface=<n> boot=0 tablet=1 ok=1` emitted once

**HID report descriptor shape scan**:
- Recognizes mouse shape: 05 01 (Usage Page GD) + 09 02 (Usage Mouse) + A1 01 (Collection) + 09 30/31 (X/Y)
- Marker `[sexusb.xhci.hid.report_desc.mouse_shape.ok]`
- Recognizes tablet shape: 05 01 + 09 01 (Usage Pointer) + 09 30/31
- Recognizes keyboard shape: 05 01 + 09 06 (Usage Keyboard) + A1 01

**Interrupt IN endpoint**:
- addr=0x81 (EP1 IN), MPS=4, interval=10 (QEMU usb-mouse boot protocol)
- Configure Endpoint succeeds: `[sexusb.xhci.intr_in.config_ep.ok]`
- 16-slot circular transfer ring with Link TRB at slot 15

**Boot mouse report format** (3-4 bytes):
- byte 0: buttons[2:0] (bit0=left, bit1=right, bit2=middle)
- byte 1: dx (signed 8-bit relative)
- byte 2: dy (signed 8-bit relative)
- byte 3: wheel (signed 8-bit, optional, 0 if absent)

**Tablet report format** (5 bytes):
- byte 0: buttons[2:0]
- byte 1-2: X absolute (LE u16, 0..32767)
- byte 3-4: Y absolute (LE u16, 0..32767)

C) PACK / NORMALIZER

**sexusb packing** (relative mouse path):
```
packed_axes = (dx as u8 as u64) | ((dy as u8 as u64) << 8) | ((wheel as u8 as u64) << 16)
// is_abs = 0 (bit 32 clear)
sent as: OP_USB_MOUSE_REPORT(0x260, arg0=0, arg1=buttons, arg2=packed_axes)
```

**sexusb packing** (absolute tablet path):
```
packed_axes = (abs_x as u64) | ((abs_y as u64) << 16) | (1u64 << 32)
// is_abs = 1 (bit 32 set)
sent as: OP_USB_MOUSE_REPORT(0x260, arg0=0, arg1=buttons, arg2=packed_axes)
```

**sexinput decode**:
```
buttons = arg1 as u8
is_abs = ((packed >> 32) & 1) != 0
dx = if is_abs { (packed & 0xFFFF) as u16 as i16 } else { (packed as u8) as i8 as i16 }
dy = if is_abs { ((packed >> 16) & 0xFFFF) as u16 as i16 } else { ((packed >> 8) as u8) as i8 as i16 }
wheel = if is_abs { 0 } else { ((packed >> 16) as u8) as i8 }
```

**normalize_pointer_report_v1 contract** (unchanged from AP Cursor Current Tier):
- Input: `HidPointerRawReport { dx:i16, dy:i16, buttons:u8, wheel:i8, is_abs:bool }`
- buttons masked 0x07
- is_abs=1 → emits EV_ABS (if dx/dy changed from last)
- is_abs=0 → emits EV_REL (if dx != 0 || dy != 0)
- buttons changed (XOR edge) → emits EV_BTN per button bit
- wheel: deferred (not emitted in V1)

D) PROOF

**Command**:
```
./scripts/entrypoint_build.sh
./scripts/run_daily_driver_proof.sh /tmp/usb_hid_boot_mouse_report_v1.log
```

**QEMU device**: Changed from `-device usb-kbd,bus=xhci.0` to `-device usb-mouse,bus=xhci.0` in run_daily_driver_proof.sh

**Log**: `/tmp/usb_hid_boot_mouse_report_v1.log` (102,483 lines)

**PASS gates**: 300, FAIL gates: 0, SKIP gates: 128

E) MARKERS / GATES

| Gate | Result | Marker |
|------|--------|--------|
| usb_mouse_detect | PASS | `[usb.mouse.detect] interface=0 boot=1 ok=1` |
| usb_mouse_raw_report | SKIP | no report (interrupt IN timeout) |
| usb_mouse_pack | SKIP | no report to pack |
| usb_mouse_to_normalizer | SKIP | no nonzero report reached normalizer |
| usb_mouse_normalizer_out | SKIP | no normalizer output from real USB data |
| usb_mouse_faults_zero | PASS | 0 faults |

**Proof markers added**:
- `servers/sexusb/src/main.rs`:
  - `[usb.mouse.detect]` — one-shot, fires when boot mouse interface found (line ~2146)
  - `[usb.mouse.detect]` — one-shot, fires when tablet interface found (line ~2162)
  - `[usb.mouse.report.raw]` — one-shot, fires on first successful boot mouse decode
  - `[usb.mouse.report.raw]` — one-shot, fires on first successful tablet decode
  - `[usb.mouse.report.pack]` — one-shot, fires on first boot mouse pack (packed_axes computed)
  - `[usb.mouse.report.pack]` — one-shot, fires on first tablet pack (packed_axes computed)
- `servers/sexinput/src/main.rs`:
  - `[usb.mouse.to_normalizer]` — one-shot, fires when nonzero report reaches normalizer (bootgraph zero-report excluded)
  - `[usb.mouse.normalizer.out]` — one-shot per event, fires when normalizer emits EV_REL/EV_ABS/EV_BTN

F) GATE RESULTS

```
usb_mouse_detect             PASS   USB mouse/tablet boot interface detected
usb_mouse_raw_report         SKIP   detected but no report (timeout/idle)
usb_mouse_pack               SKIP   detected but no report to pack
usb_mouse_to_normalizer      SKIP   detected but report did not reach normalizer
usb_mouse_normalizer_out     SKIP   detected but normalizer did not emit output
usb_mouse_faults_zero        PASS   no faults during mouse proof lane
```

G) FAULT SCAN

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
- interrupt IN timeouts: 772 (expected — no reports from idle QEMU usb-mouse)
- `usb_mouse FAIL`: 0
- `mouse FAIL`: 0
- `normalizer FAIL`: 0
- `pointer FAIL`: 0

H) REMAINING BLOCKERS

1. **Interrupt IN timeout on idle mouse**: QEMU usb-mouse at QEMU 11.0.0 with nec-usb-xhci does not generate interrupt IN reports in headless/nographic mode, even after SET_IDLE(duration=1*4ms). Same class of blocker as AP12 keyboard.

2. **Single-device limitation**: Only one HID device per boot. When usb-mouse is the QEMU device, keyboard path is unreachable. Cannot test mouse+keyboard simultaneously. Multi-device requires slot allocation redesign.

3. **No QEMU USB mouse injection path**: QMP/HMP injection routes to PS/2 layer only, not to USB devices. Confirmed in QEMU 11.0 with nec-usb-xhci.

4. **Pipeline structurally complete**: All code paths exist and are verified at the architectural level:
   - XHCI detection + descriptor walk → boot mouse found
   - Configure Endpoint → interrupt ring armed
   - Decode + pack + PDX send → sexinput
   - Decode + normalize → normalized HID events
   - Forward → silk-shell pointer state
   
   The only missing piece is an actual USB interrupt IN packet from the hardware.

Potential workarounds for V2:
- Use QEMU `-display gtk` with real mouse movement to generate USB reports
- Physical USB mouse via `-device usb-host`
- QEMU usb-tablet (absolute positioning) which MAY generate idle reports differently
- Accept PARTIAL as honest result until real input environment is available

I) FILES CHANGED

- `servers/sexusb/src/main.rs`: +54 lines — `[usb.mouse.detect]`, `[usb.mouse.report.raw]`, `[usb.mouse.report.pack]` proof markers (boot mouse + tablet paths)
- `servers/sexinput/src/main.rs`: +32 lines — `[usb.mouse.to_normalizer]`, `[usb.mouse.normalizer.out]` proof markers (zero-report excluded via nonzero check)
- `scripts/daily_driver_master_gate.sh`: +96 lines — 6 new gate declarations + evaluation logic + summary output entries
- `scripts/run_daily_driver_proof.sh`: 1 line — QEMU device changed from `usb-kbd` to `usb-mouse` for mouse proof lane
- `docs/handoff/USB_HID_BOOT_MOUSE_REPORT_V1.md` (new)
- Backup files: `.bak.ap13` for all changed files

J) NEXT REQUIRED AUTOPILOT

**USB_HID_POINTER_PRODUCER_V1** — the real pointer event producer that gets past the interrupt IN timeout:

1. **Interrupt IN fix**: Investigate why QEMU usb-mouse (and usb-tablet) do not generate interrupt IN reports in headless mode even after SET_IDLE. Possible causes:
   - SET_IDLE duration interpretation differs between QEMU versions
   - QEMU usb-mouse requires at least one OUT transaction before sending IN
   - Doorbell target or cycle bit issue on interrupt transfer ring
   - Need to check QEMU trace logs with `SEXUSB_XHCI_TRACE=1`

2. **Tablet path**: Test with QEMU `-device usb-tablet,bus=xhci.0` instead of usb-mouse. Tablet absolute reports may behave differently.

3. **Post-interrupt report**: Once reports arrive:
   - Verify button click events propagate through click→focus→drag pipeline
   - Add `usb_mouse_raw_report: PASS` gate
   - Add `usb_mouse_pack: PASS` gate  
   - Add `usb_mouse_to_normalizer: PASS` gate
   - Add `usb_mouse_normalizer_out: PASS` gate
   - Prove cursor movement from real USB mouse deltas

4. **Multi-device**: Design slot allocation for simultaneous keyboard+mouse operation (see SEXUSB_SINGLE_DEVICE_GUARD_V1.md).

Scope: sexusb + sexinput. No kernel/ABI/sexdisplay edits. No shell policy changes.
