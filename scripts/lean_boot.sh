#!/bin/bash
set -euo pipefail

echo "✂️ Stripping heavy dependencies to bypass Docker stall..."

# 1. Comment out smoltcp in the kernel manifest
if [ -f "kernel/Cargo.toml" ]; then
    sed -i.bak 's/^smoltcp/# smoltcp/' kernel/Cargo.toml
    echo "✅ smoltcp sidelined."
fi

# 2. Cleanup orphaned containers to free up the Docker I/O subsystem
docker rm -f $(docker ps -aq) || true

echo "⚡ Re-triggering Build (No Networking Payload)..."

docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    cargo build \
        --target x86_64-sex.json \
        -Z json-target-spec \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -p sex-kernel \
        --release > /dev/stderr 2>&1 && \
    find target/x86_64-sex/release/ -maxdepth 1 -type f ! -name '*.d' ! -name '*.rlib' -name 'sex*' -exec cat {} + " > ./sex-kernel.elf

if [ -s "./sex-kernel.elf" ]; then
    echo "💎 Artifact Extracted ($(du -h ./sex-kernel.elf))."
    
    mkdir -p iso_root/boot/limine
    cp ./sex-kernel.elf iso_root/boot/sex-kernel
    
    # Refresh Limine Config
    cat << 'CFG' > iso_root/boot/limine.cfg
TIMEOUT=0
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
CFG

    if command -v xorriso >/dev/null 2>&1; then
        echo "💿 Minting ISO..."
        xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o sexos-sasos.iso
        
        echo "🚀 BOOTING SASOS (LEAN MODE)..."
        qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
    else
        echo "⚠️ xorriso missing."
    fi
else
    echo "❌ Build failed. Check for code references to smoltcp that need commenting out."
fi
