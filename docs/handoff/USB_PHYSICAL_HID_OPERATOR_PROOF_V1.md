# USB_PHYSICAL_HID_OPERATOR_PROOF_V1

status: operator-required

date: 2026-05-20

## Why this exists
QMP `input-send-event` lane is already audited and does not produce `sexusb.hid.report.nonzero` in this environment, even though:
- QMP socket/handshake/events are PASS
- SexOS boot markers and `sexusb.route.sexinput.ready` are PASS
- No `#PF/#GP/panic/fault.kill` seen

Route-audit conclusion: QMP accepted events are not proving delivery into the USB HID report path consumed by SexOS.

## Exact operator proof commands
Mouse lane:
```bash
./scripts/usb_physical_hid_operator_probe.sh mouse
```

Tablet lane:
```bash
./scripts/usb_physical_hid_operator_probe.sh tablet
```

If ISO must be rebuilt first:
```bash
./scripts/usb_physical_hid_operator_probe.sh mouse --build
```

Raw harness equivalent (reference):
```bash
SEXOS_QEMU_DISPLAY=gtk SEXUSB_QEMU_DEVICE=mouse ./scripts/qemu_harness.sh --timeout 45 --display gtk
```

## Operator steps (interactive)
1. Start probe command above.
2. Wait for QEMU GTK window to become interactive.
3. Move mouse and click repeatedly inside the VM window for at least 10 seconds.
4. Let probe finish or timeout.
5. Check result line and marker grep output.

Log path:
- `/tmp/usb_physical_hid_operator_probe.log`

## Expected markers
- Positive path marker:
  - `sexusb.hid.report.nonzero`
- Supporting markers:
  - `sexusb.route.sexinput.ready`
  - optionally `sexusb.hid.report.idle`
- Negative marker:
  - `sexusb.hid.report.timeout`
- Fault markers (must be absent):
  - `#PF`
  - `#GP`
  - `panic`
  - `fault.kill`

## PASS / SKIP / FAIL rules
- PASS:
  - `sexusb.hid.report.nonzero` present
  - no fault markers
  - script exit code `0`
- SKIP:
  - no `sexusb.hid.report.nonzero`
  - no fault markers
  - script exit code `2`
- FAIL:
  - build failure, harness failure, or any fault marker present
  - script exit code `1`

## After PASS
1. Freeze proof evidence:
   - keep `/tmp/usb_physical_hid_operator_probe.log`
   - capture `rg` excerpt with `sexusb.hid.report.nonzero` line(s)
2. Then and only then proceed to next pointer/input producer phase.
3. Do not use QMP synthetic lane as HID proof substitute.
