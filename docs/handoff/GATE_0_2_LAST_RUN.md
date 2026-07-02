# GATE_0_2_LAST_RUN

- date: 2026-07-02T17:40:20+02:00
- git commit: 167bf934
- qmp_sock: /tmp/qmp_lane_cdc_v3/qmp.sock
- log_path: /home/xirtus_arch/Projects/Sex/logs/qemu-latest.log
- qmp_environment_failure: no
- qemu lane: qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:/home/xirtus_arch/Projects/Sex/logs/qemu-latest.log -qmp unix:/tmp/qmp_lane_cdc_v3/qmp.sock,server=on,wait=off -no-reboot -no-shutdown

## Gate Results

- BUILD_GATE: PASS
- BOOT_GATE: PASS
- POINTER_LIVE_GATE: FAIL
- KEYBOARD_LIVE_GATE: FAIL
- INPUT_OWNERSHIP_GATE: PASS
- FAULT_REGRESSION_GATE: FAIL
- SCOPE_GATE: WARN
- FINAL_SCORE: RED_0_2

## Marker Counts

- [ps2.irq1.entry]: 4
- [ps2.port60.read]: 2
- [ps2.input_ring.enqueue]: 2
- [sexinput.ps2.scancode]: 2
- [sexinput.keyboard.send]: 0
- [silk-shell.keyboard.recv]: 0

- [sexinput.pointer.recv]: 16
- [sexinput.pointer.send]: 202
- [silk-shell.pointer.recv]: 0
- [silk-shell.cursor.update]: 0
- [sexdisplay.cursor.draw]: 16

## First Missing Marker

- pointer chain: [silk-shell.pointer.recv]
- keyboard chain: [sexinput.keyboard.send]
- overall: [silk-shell.pointer.recv]

## Remaining Risks

- GUI backend availability on host affects interactive proof reliability.
- QMP injection may not perfectly emulate real human timing/capture.
- Dirty tree scope warnings are advisory and non-blocking.
- If FAIL_QMP_ENVIRONMENT is set, treat as host policy/runtime issue, not SexOS regression.
