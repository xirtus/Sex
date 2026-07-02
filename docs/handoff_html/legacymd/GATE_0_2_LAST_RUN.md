# GATE_0_2_LAST_RUN

- date: 2026-05-06T03:36:16+02:00
- git commit: 492f5bd
- qmp_sock: /home/xirtus_arch/Documents/microkernel/.gate_0_2/qmp.sock
- log_path: /home/xirtus_arch/Documents/microkernel/.gate_0_2/sexos-input.log
- qmp_environment_failure: yes
- qemu lane: qemu-system-x86_64 -M q35 -m 512M -cdrom sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:/home/xirtus_arch/Documents/microkernel/.gate_0_2/sexos-input.log -qmp unix:/home/xirtus_arch/Documents/microkernel/.gate_0_2/qmp.sock,server=on,wait=off -no-reboot -no-shutdown

## Gate Results

- BUILD_GATE: PASS
- BOOT_GATE: FAIL
- POINTER_LIVE_GATE: FAIL
- KEYBOARD_LIVE_GATE: FAIL
- INPUT_OWNERSHIP_GATE: PASS
- FAULT_REGRESSION_GATE: FAIL
- SCOPE_GATE: WARN
- FINAL_SCORE: RED_0_2

## Marker Counts

- [ps2.irq1.entry]: 0
- [ps2.port60.read]: 0
- [ps2.input_ring.enqueue]: 0
- [sexinput.ps2.scancode]: 0
- [sexinput.keyboard.send]: 0
- [silk-shell.keyboard.recv]: 0

- [sexinput.pointer.recv]: 0
- [sexinput.pointer.send]: 0
- [silk-shell.pointer.recv]: 0
- [silk-shell.cursor.update]: 0
- [sexdisplay.cursor.draw]: 0

## First Missing Marker

- pointer chain: [sexinput.pointer.recv]
- keyboard chain: [ps2.irq1.entry]
- overall: [sexinput.pointer.recv]

## Remaining Risks

- GUI backend availability on host affects interactive proof reliability.
- QMP injection may not perfectly emulate real human timing/capture.
- Dirty tree scope warnings are advisory and non-blocking.
- If FAIL_QMP_ENVIRONMENT is set, treat as host policy/runtime issue, not SexOS regression.
