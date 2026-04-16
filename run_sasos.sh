#!/bin/bash
# Sex SASOS Microkernel - QEMU Boot Launcher
cd /Users/xirtus/sites/microkernel || { echo "FATAL: Sex root not found"; exit 1; }

ISO_FILE="sexos-v1.0.0.iso"

# Check if ISO is in root or dist/
if [ -f "dist/${ISO_FILE}" ]; then
    TARGET="dist/${ISO_FILE}"
elif [ -f "${ISO_FILE}" ]; then
    TARGET="${ISO_FILE}"
else
    echo "ERROR: ${ISO_FILE} not found. Run make release first."
    exit 1
fi

QEMU_CMD="qemu-system-x86_64 \
  -machine q35 \
  -cpu max,pku=on \
  -smp 4 \
  -m 2G \
  -serial stdio \
  -display none \
  -cdrom ${TARGET}"

echo "=== Sex SASOS QEMU Production Boot ==="
echo "Launching with full PKU + SAS invariants..."
echo "Command: ${QEMU_CMD}"
echo "Output will appear below (serial console)."
echo "Press Ctrl+A then X to exit."
echo "---------------------------------------"

eval "${QEMU_CMD}"
