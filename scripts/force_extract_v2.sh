#!/bin/bash
set -euo pipefail

echo "⚡ Force-Extracting Kernel (Unstable Flags Authorized)..."

# 1. Execute build with explicit -Z flags for JSON target specs
docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    cargo build \
        --target x86_64-sex.json \
        -Z json-target-spec \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -p sex-kernel \
        --release && \
    cat target/x86_64-sex/release/sex_kernel" > ./sex-kernel.elf

# 2. Integrity Check
if [ -s "./sex-kernel.elf" ]; then
    echo "💎 Extraction Success: $(du -h ./sex-kernel.elf)"
    
    # 3. Final ISO Assembly
    mkdir -p iso_root/boot/limine
    cp ./sex-kernel.elf iso_root/boot/sex-kernel
    
    # Ensure configuration matches the extracted name
    cat << 'CFG' > iso_root/boot/limine.cfg
TIMEOUT=0
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
CFG

    if command -v xorriso >/dev/null 2>&1; then
        echo "💿 Minting Final ISO..."
        xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o sexos-sasos.iso
        
        echo "=== PHASE 18.41: SASOS BOOT READY ==="
        qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
    else
        echo "⚠️ xorriso missing on host. Install via brew."
    fi
else
    echo "❌ Extraction failed. Check target/x86_64-sex/release/ for naming drift."
    exit 1
fi
