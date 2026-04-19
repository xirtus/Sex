#!/bin/bash
set -euo pipefail

echo "⚡ Force-Extracting Kernel (Absolute Glob Mode)..."

# 1. Build and then use find inside the container to stream the binary
docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    cargo build \
        --target x86_64-sex.json \
        -Z json-target-spec \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -p sex-kernel \
        --release > /dev/stderr 2>&1 && \
    find target/x86_64-sex/release/ -maxdepth 1 -type f ! -name '*.d' ! -name '*.rlib' -name 'sex*' -exec cat {} + " > ./sex-kernel.elf

# 2. Validation
if [ -s "./sex-kernel.elf" ]; then
    echo "💎 Artifact Extracted: $(du -h ./sex-kernel.elf)"
    
    # 3. Setup ISO Root
    mkdir -p iso_root/boot/limine
    cp ./sex-kernel.elf iso_root/boot/sex-kernel
    
    # 4. Limine Config
    cat << 'CFG' > iso_root/boot/limine.cfg
TIMEOUT=0
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
CFG

    # 5. Synthesis
    if command -v xorriso >/dev/null 2>&1; then
        echo "💿 Minting Final ISO..."
        xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o sexos-sasos.iso
        
        echo "=== PHASE 18.42: SASOS RECOVERY COMPLETE ==="
        echo "🚀 Booting..."
        qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
    else
        echo "⚠️ xorriso missing. Run 'brew install xorriso'."
    fi
else
    echo "❌ FATAL: Container did not produce an ELF. Check Cargo.toml for [bin] names."
    # List the target dir from inside the container to debug
    docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 ls -R target/x86_64-sex/release/
    exit 1
fi
