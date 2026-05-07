#!/usr/bin/env bash
# qemuX - Wrapper for Patched QEMU (XHCI/HID Fixes)
# Usage: ./qemuX.sh [additional arguments]

QEMU_BIN="/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64"
ISO="/home/xirtus_arch/Documents/microkernel/sexos-v1.0.0.iso"
QMP_SOCK="/tmp/sexos-qmp.sock"

if [ ! -f "$QEMU_BIN" ]; then
    echo "Error: Patched QEMU binary not found at $QEMU_BIN"
    echo "Run: ninja -C tools/qemu/build"
    exit 1
fi

if [ ! -f "$ISO" ]; then
    echo "Warning: $ISO not found. Running native build..."
    ./scripts/entrypoint_build.sh
fi

# Default SexOS Optimized Arguments
ARGS=(
    -M q35,i8042=off            # Disable PS/2 for USB-only testing
    -m 512M                     # SASOS standard memory
    -cpu max,+pku               # Enable MPK/PKRU
    -cdrom "$ISO"               # Latest patched guest drivers
    -device nec-usb-xhci,id=xhci
    -device usb-kbd,bus=xhci.0
    -device usb-tablet,bus=xhci.0
    -serial stdio               # Vital for driver logs
    -display sdl                # Host GUI for mouse interaction
    -qmp unix:"$QMP_SOCK",server,nowait
    -no-reboot
)

echo "🚀 Launching Patched QEMU (XHCI+HID Fixes)..."
echo "Monitor: $QMP_SOCK"

"$QEMU_BIN" "${ARGS[@]}" "$@"
