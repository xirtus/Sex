#!/bin/bash
# Sex SASOS Microkernel - QEMU Boot Launcher (Production Canonical)
# Expert microkernel engineer script: launches Sex v1.0.0 with ALL required hardware invariants
# (Intel PKU enabled, Q35 chipset, 2G VAS space, SMP, headless serial-only output)

# get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
# navigate to the project root (one level up from scripts/)
cd "$SCRIPT_DIR/.." || exit 1

ISO_FILE="sexos-v1.0.0.iso"
mkdir -p dist/

if [ ! -f "dist/${ISO_FILE}" ] && [ -f "${ISO_FILE}" ]; then
  mv "${ISO_FILE}" dist/ 2>/dev/null || true
fi

QEMU_CMD="qemu-system-x86_64 \
  -machine q35 \
  -cpu max,pku=on \
  -smp 4 \
  -m 2G \
  -serial stdio \
  -display none \
  -cdrom dist/${ISO_FILE}"

echo "=== Sex SASOS QEMU Production Boot ==="
echo "Launching with full PKU + SAS invariants..."
echo "Command: ${QEMU_CMD}"
echo "Output will appear directly in this terminal (serial console)."
echo "Press Ctrl+A then X to exit QEMU cleanly."
echo ""

eval "${QEMU_CMD}"
