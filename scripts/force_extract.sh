#!/bin/bash
set -euo pipefail

echo "⚡ Force-Extracting Kernel via Container Stream..."

# 1. Spin up the builder, compile, and stream the binary out to stdout
# This bypasses the volume mount entirely for the final artifact
docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -p sex-kernel --release && \
    cat target/x86_64-sex/release/sex_kernel" > ./sex-kernel.elf

# 2. Check if the stream succeeded
if [ -s "./sex-kernel.elf" ]; then
    echo "💎 Artifact Extracted Successfully: $(du -h ./sex-kernel.elf)"
    chmod +x ./sex-kernel.elf
    
    mkdir -p iso_root/boot/limine
    cp ./sex-kernel.elf iso_root/boot/sex-kernel
    
    # 3. Final ISO Minting
    if command -v xorriso >/dev/null 2>&1; then
        echo "💿 Minting sexos-sasos.iso..."
        xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o sexos-sasos.iso
        
        echo "=== PHASE 18.40: EXTRACTION SUCCESSFUL ==="
        echo "🚀 Booting SASOS..."
        qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
    else
        echo "⚠️ xorriso missing on host."
    fi
else
    echo "💀 Extraction failed. The binary was not found inside the container."
    exit 1
fi
