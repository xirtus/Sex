#!/usr/bin/env bash
# qemuX-kbd.sh — Manual keyboard capture test wrapper.
#
# Differences from qemuX.sh:
#   - Serial goes to file (not stdio) so stdin does not compete with SDL keyboard.
#   - SDL grab-mod enabled: press Ctrl+Alt+G to lock keyboard to QEMU window.
#   - No QMP socket (not needed for manual test).
#
# Usage:
#   ./qemuX-kbd.sh
#   (click QEMU window, or press Ctrl+Alt+G to grab keyboard)
#
# Scan log after exit:
#   rg "usb_kbd|spindle|AccessClose|kbd.raw|kbd.recv|panic|#PF|#GP" \
#     /tmp/spindle_manual_verify.log | head -260

QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-v1.0.0.iso"
LOG="/tmp/spindle_manual_verify.log"

if [ ! -f "$QEMU_BIN" ]; then
    echo "Error: Patched QEMU binary not found at $QEMU_BIN"
    echo "Run: ninja -C tools/qemu/build"
    exit 1
fi

if [ ! -f "$ISO" ]; then
    echo "Warning: $ISO not found. Run ./scripts/entrypoint_build.sh first."
    exit 1
fi

echo "Serial log: $LOG"
echo "Press Ctrl+Alt+G in QEMU window to grab keyboard."
echo "Press Ctrl+Alt+G again to release."

"$QEMU_BIN" \
    -M q35,i8042=off \
    -m 512M \
    -cpu max,+pku \
    -cdrom "$ISO" \
    -device nec-usb-xhci,id=xhci \
    -device usb-kbd,bus=xhci.0 \
    -device usb-tablet,bus=xhci.0 \
    -serial file:"$LOG" \
    -display sdl,grab-mod=lctrl-lalt \
    -no-reboot \
    "$@"
