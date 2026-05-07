#!/usr/bin/env bash
# qemuX-kbd - Keyboard-only test (no tablet, no mouse)
QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-v1.0.0.iso"
QMP_SOCK="/tmp/sexos-qmp.sock"

if [ ! -f "$QEMU_BIN" ]; then
    echo "Error: Patched QEMU binary not found at $QEMU_BIN"
    exit 1
fi
if [ ! -f "$ISO" ]; then
    echo "Warning: $ISO not found. Running native build..."
    ./scripts/entrypoint_build.sh
fi

ARGS=(
    -M q35,i8042=off
    -m 512M
    -cpu max,+pku
    -cdrom "$ISO"
    -device nec-usb-xhci,id=xhci
    -device usb-kbd,bus=xhci.0
    -serial stdio
    -display sdl
    -qmp unix:"$QMP_SOCK",server,nowait
    -no-reboot
)

echo "🚀 Launching Keyboard-Only QEMU (USB-KBD Poll Cadence Test)..."
echo "Monitor: $QMP_SOCK"
"$QEMU_BIN" "${ARGS[@]}" "$@"
