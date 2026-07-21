# GATE_0_2_LAST_RUN

- date: 2026-07-21T11:01:35+02:00
- git commit: f905780a
- qmp_sock: /tmp/sexos_gate_0_2/qmp.sock
- log_path: /home/xirtus_arch/Projects/Sex/logs/qemu-latest.log
- qmp_environment_failure: no
- qemu lane: qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:/home/xirtus_arch/Projects/Sex/logs/qemu-latest.log -qmp unix:/tmp/sexos_gate_0_2/qmp.sock,server=on,wait=off -no-reboot -no-shutdown

## Gate Results

- BUILD_GATE: PASS
- BOOT_GATE: PASS
- POINTER_LIVE_GATE: FAIL
- KEYBOARD_LIVE_GATE: FAIL
- INPUT_OWNERSHIP_GATE: PASS
- FAULT_REGRESSION_GATE: PASS
- SCOPE_GATE: WARN
- FINAL_SCORE: RED_0_2

## Marker Counts

- [ps2.irq1.entry]: 12
- [ps2.port60.read]: 10
- [ps2.input_ring.enqueue]: 10
- [sexinput.ps2.scancode]: 10
- [sexinput.keyboard.send]: 0
- [silk-shell.keyboard.recv]: 0

- [sexinput.pointer.recv]: 16
- [sexinput.pointer.send]: 2048
- [silk-shell.pointer.recv]: 2068
- [silk-shell.cursor.update]: 16
- [sexdisplay.cursor.draw]: 16

## First Missing Marker

- pointer chain: none
- keyboard chain: [sexinput.keyboard.send]
- overall: [sexinput.keyboard.send]

## Remaining Risks

- GUI backend availability on host affects interactive proof reliability.
- QMP injection may not perfectly emulate real human timing/capture.
- Dirty tree scope warnings are advisory and non-blocking.
- If FAIL_QMP_ENVIRONMENT is set, treat as host policy/runtime issue, not SexOS regression.
