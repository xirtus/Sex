#!/bin/bash
set -euo pipefail

echo "🔭 Scanning all subdirectories for SexOS ELFs..."

# 1. Search for ANY file starting with 'sex' in the target directory
FOUND_BINS=$(find target -type f -name "sex*" ! -name "*.d" ! -name "*.rlib" 2>/dev/null)

if [ -z "$FOUND_BINS" ]; then
    echo "❌ [EMPTY TARGET] No binaries found. Re-triggering Docker build with host-sync enforcement..."
    
    docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
        rustup default nightly && \
        cargo build --target x86_64-sex.json \
        -Z build-std=core,alloc \
        -p sex-kernel --release"
    
    # Re-scan after build
    FOUND_BINS=$(find target -type f -name "sex*" ! -name "*.d" ! -name "*.rlib" 2>/dev/null)
fi

if [ -n "$FOUND_BINS" ]; then
    # Take the first one found
    KERNEL_PATH=$(echo "$FOUND_BINS" | head -n 1)
    echo "💎 Artifact Located: $KERNEL_PATH"
    
    mkdir -p iso_root/boot/limine
    cp "$KERNEL_PATH" iso_root/boot/sex-kernel
    echo "✅ Staged for ISO."
    
    # 2. Final ISO Synthesis
    if command -v xorriso >/dev/null 2>&1; then
        xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o sexos-sasos.iso
        
        echo "=== PHASE 18.39: RECOVERY SUCCESSFUL ==="
        qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
    else
        echo "⚠️ xorriso missing."
    fi
else
    echo "💀 FATAL: Build completed but target/ is still empty. Check Docker mount permissions."
    ls -la .
fi
