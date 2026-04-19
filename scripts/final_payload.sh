#!/bin/bash
set -euo pipefail

echo "🎯 Binary Synthesis Complete. Finalizing Hardware Payload..."

# 1. Ensure the directory structure is perfect
mkdir -p iso_root/boot/limine

# 2. LOCATE LIMINE BINARIES
# We search the project for the required bootloader files
LIMINE_FILES=("limine-bios-cd.bin" "limine-uefi-cd.bin" "limine-bios.sys")

for FILE in "${LIMINE_FILES[@]}"; do
    FOUND=$(find . -name "$FILE" | head -n 1)
    if [ -n "$FOUND" ]; then
        echo "✅ Found $FILE at $FOUND"
        cp "$FOUND" iso_root/boot/limine/
    else
        echo "❌ MISSING CRITICAL FILE: $FILE"
        echo "Searching for any limine files to help debug..."
        find . -name "limine*"
        exit 1
    fi
done

# 3. Final Minting with Absolute Precision
echo "💿 Minting sexos-sasos.iso..."
xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot boot/limine/limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o sexos-sasos.iso

echo "=== PHASE 18.52: PAYLOAD MINTED SUCCESSFULLY ==="
echo "🚀 BOOTING SASOS..."
qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
