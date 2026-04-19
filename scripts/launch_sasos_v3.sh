#!/bin/bash
set -e

# 1. Setup paths
ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
ISO_NAME="sexos-v1.0.0.iso"

echo "--- 1. Validating Kernel Artifact ---"
if [ ! -f "$KERNEL_BIN" ]; then
    echo "ERROR: Kernel binary not found. Run the build script first."
    exit 1
fi

mkdir -p $ISO_ROOT

# 2. Force-fetching valid Limine v7 binaries (Binary Branch)
if [ ! -f "limine-bios-cd.bin" ]; then
    echo "→ Fetching Limine v7.x-binary branch..."
    rm -f limine.zip
    # Using the archive link for the specific binary branch
    curl -Lo limine.zip https://github.com/limine-bootloader/limine/archive/refs/heads/v7.x-binary.zip
    unzip -q limine.zip
    # Files land in limine-7.x-binary/ or limine-v7.x-binary/
    mv limine-7.x-binary/* . 2>/dev/null || mv limine-v7.x-binary/* . 2>/dev/null || true
    rm -rf limine.zip limine-7.x-binary limine-v7.x-binary
fi

echo "--- 2. Building ISO Structure ---"
cp "$KERNEL_BIN" "$ISO_ROOT/"
cp limine.cfg "$ISO_ROOT/"
cp limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin "$ISO_ROOT/"

echo "--- 3. Synthesizing Bootable ISO (xorriso) ---"
xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        "$ISO_ROOT" -o "$ISO_NAME"

echo "--- 4. Launching SASOS in QEMU (Intel PKU Emulation) ---"
# -cpu max,+pku is required for the SASOS Protection Domain architecture
qemu-system-x86_64 -cdrom "$ISO_NAME" \
                   -serial stdio \
                   -m 512M \
                   -vga std \
                   -cpu max,+pku \
                   -device intel-hda -device hda-duplex \
                   -net none
