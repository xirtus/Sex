#!/bin/bash
set -euo pipefail

echo "🔍 Artifact Recovery initiated..."

# 1. Define possible paths for the kernel
# Rust often converts hyphens to underscores in binary names
POSSIBLE_KERNELS=(
    "target/x86_64-sex/release/sex-kernel"
    "target/x86_64-sex/release/sex_kernel"
    "target/x86_64-unknown-none/release/sex-kernel"
    "target/x86_64-unknown-none/release/sex_kernel"
)

FOUND_KERNEL=""
for K in "${POSSIBLE_KERNELS[@]}"; do
    if [ -f "$K" ]; then
        FOUND_KERNEL="$K"
        break
    fi
done

if [ -z "$FOUND_KERNEL" ]; then
    echo "❌ CRITICAL: Kernel binary not found."
    echo "Dumping target directory for inspection:"
    find target -name "sex*" || echo "No files starting with 'sex' found."
    exit 1
fi

echo "✅ Found Kernel: $FOUND_KERNEL"
mkdir -p iso_root/boot/limine
cp "$FOUND_KERNEL" iso_root/boot/sex-kernel

# 2. Ensure Limine configuration is mapped to the final binary name
cat << 'EOF' > iso_root/boot/limine.cfg
TIMEOUT=0
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
EOF

# 3. ISO Synthesis
if command -v xorriso >/dev/null 2>&1; then
    echo "💿 Minting sexos-sasos.iso..."
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "=== PHASE 18.38: SUCCESSFUL ASSEMBLY ==="
    echo "🚀 Launching QEMU (M1/M2 TCG Mode)..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "⚠️ xorriso missing. Run: brew install xorriso"
fi
