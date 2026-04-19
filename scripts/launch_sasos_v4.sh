#!/bin/bash
set -e

# 1. Setup paths
ISO_ROOT="iso_root"
KERNEL_SRC="target/x86_64-sex/release/sex-kernel"
# This MUST match the 'KERNEL_PATH' or 'path' in your limine.cfg
KERNEL_DEST_NAME="sexos-kernel"
ISO_NAME="sexos-v1.0.0.iso"

echo "--- 1. Validating Kernel Artifact ---"
if [ ! -f "$KERNEL_SRC" ]; then
    echo "ERROR: Kernel binary not found at $KERNEL_SRC."
    exit 1
fi

mkdir -p $ISO_ROOT

# 2. Ensure Limine binaries are present locally
if [ ! -f "limine-bios-cd.bin" ]; then
    echo "→ Fetching Limine v7 binaries..."
    curl -Lo limine.zip https://github.com/limine-bootloader/limine/archive/refs/heads/v7.x-binary.zip
    unzip -q limine.zip
    mv limine-7.x-binary/* . 2>/dev/null || mv limine-v7.x-binary/* . 2>/dev/null || true
    rm -rf limine.zip limine-7.x-binary limine-v7.x-binary
fi

echo "--- 2. Aligning Naming for limine.cfg ---"
# We copy 'sex-kernel' and rename it to 'sexos-kernel' inside the ISO root
cp "$KERNEL_SRC" "$ISO_ROOT/$KERNEL_DEST_NAME"
cp limine.cfg "$ISO_ROOT/"
cp limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin "$ISO_ROOT/"

echo "--- 3. Synthesizing Bootable ISO ---"
xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        "$ISO_ROOT" -o "$ISO_NAME"

echo "--- 4. Launching SASOS (Path Error Resolved) ---"
qemu-system-x86_64 -cdrom "$ISO_NAME" \
                   -serial stdio \
                   -m 512M \
                   -vga std \
                   -cpu max,+pku \
                   -device intel-hda -device hda-duplex \
                   -net none
