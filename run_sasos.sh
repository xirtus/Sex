#!/bin/bash
# Sex SASOS Microkernel - QEMU Boot Launcher (Phase 28)
# Supports strict x86_64 cross-build and persistent sexshop storage.

ISO_FILE="sexos-v28.0.0.iso"
SHOP_DIR="./sexshop"

mkdir -p "${SHOP_DIR}"

# QEMU Flags: Force x17r1 i7 hardware profile
QEMU_FLAGS="-machine q35 -cpu skylake,+pku -smp 4 -m 2G -serial stdio -display none"

# Phase 27 HAL + PCI Debug Devices
QEMU_FLAGS="${QEMU_FLAGS} -device intel-hda -device hda-duplex"
QEMU_FLAGS="${QEMU_FLAGS} -drive file=/dev/null,format=raw,if=none,id=nvm1 -device nvme,serial=1234,drive=nvm1"

# Persistent Volume for sexshop (Slot 4)
QEMU_FLAGS="${QEMU_FLAGS} -drive file=fat:rw:${SHOP_DIR},format=raw,if=none,id=shop -device virtio-blk-pci,drive=shop"

# KVM Acceleration if available
if [ -e /dev/kvm ]; then
    QEMU_FLAGS="${QEMU_FLAGS} -accel kvm"
fi

# Check for ISO
if [ -f "dist/${ISO_FILE}" ]; then
    TARGET="dist/${ISO_FILE}"
elif [ -f "${ISO_FILE}" ]; then
    TARGET="${ISO_FILE}"
else
    echo "ERROR: ${ISO_FILE} not found. Run make release first."
    exit 1
fi

QEMU_CMD="qemu-system-x86_64 ${QEMU_FLAGS} -cdrom ${TARGET}"

echo "=== Sex SASOS QEMU Phase 28 Boot ==="
echo "Launch x86_64 cross-built SASOS with persistent sexshop..."
echo "Command: ${QEMU_CMD}"
echo "---------------------------------------"

eval "${QEMU_CMD}"
