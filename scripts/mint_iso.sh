#!/bin/bash
# SASOS Phase 18.36: Zsh-Escaped ISO Synthesis
set -euo pipefail

echo "💿 Finalizing ISO Assembly..."

# Ensure directory structure exists
mkdir -p iso_root/boot/limine

# Copy the stabilized kernel
if [ -f "target/x86_64-sex/release/sex-kernel" ]; then
    cp target/x86_64-sex/release/sex-kernel iso_root/boot/sex-kernel
    echo "✅ Kernel copied to iso_root."
else
    echo "❌ Error: Kernel binary not found at target/x86_64-sex/release/sex-kernel"
    exit 1
fi

# Verify Limine configuration exists
if [ ! -f "iso_root/boot/limine.cfg" ]; then
    echo "📝 Creating missing limine.cfg..."
    cat << 'CFG' > iso_root/boot/limine.cfg
TIMEOUT=3
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
CFG
fi

# Synthesis via xorriso
if command -v xorriso >/dev/null 2>&1; then
    echo "🔨 Running xorriso synthesis..."
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "=== PHASE 18.35: HHDM SHIELD SUCCESSFUL ==="
    echo "🚀 Launching QEMU..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "⚠️ xorriso not found. Install via: brew install xorriso"
fi
