#!/bin/bash
set -e

# 1. Setup paths
ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
ISO_NAME="sexos-v1.0.0.iso"

echo "--- 1. Validating Environment ---"
if [ ! -f "$KERNEL_BIN" ]; then
    echo "ERROR: Kernel binary not found at $KERNEL_BIN. Run the build script first."
    exit 1
fi

mkdir -p $ISO_ROOT

# 2. Ensure Limine binaries exist
# If they are missing, we download the binary distribution for the protocol version
if [ ! -f "limine-bios-cd.bin" ] && [ ! -f "limine/limine-bios-cd.bin" ]; then
    echo "→ Limine binaries missing. Fetching v7.x binaries..."
    curl -Lo limine.zip https://github.com/limine-bootloader/limine/archive/refs/tags/v7.x-binary.zip
    unzip -q limine.zip
    mv limine-7.x-binary/* .
    rm -rf limine.zip limine-7.x-binary
fi

echo "--- 2. Populating ISO Root ---"
cp "$KERNEL_BIN" "$ISO_ROOT/"
cp limine.cfg "$ISO_ROOT/"
# Standard Limine deployment files
cp limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin "$ISO_ROOT/" 2>/dev/null || \
cp limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin "$ISO_ROOT/"

echo "--- 3. Generating Bootable ISO (xorriso) ---"
# We reference the files relative to the ISO root
xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        "$ISO_ROOT" -o "$ISO_NAME"

echo "--- 4. Launching SASOS in QEMU ---"
qemu-system-x86_64 -cdrom "$ISO_NAME" \
                   -serial stdio \
                   -m 512M \
                   -vga std \
                   -cpu max,+pku \
                   -device intel-hda -device hda-duplex \
                   -net none
