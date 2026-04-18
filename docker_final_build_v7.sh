#!/bin/bash
set -e
echo "=== [SexOS] Final Assembly with Limine v7 Binaries ==="

# 1. Prepare ISO Directory
mkdir -p build_dir

# 2. Compile Kernel
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 3. Copy Kernel and Config
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/kernel
cp limine.cfg build_dir/

# 4. Sync the correct Limine v7 Files from your limine_bin folder
echo "Copying identified Limine v7 binaries..."
cp limine_bin/limine-bios-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/

# 5. Create ISO
# Using 'limine-bios-cd.bin' as the boot image
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

# 6. Seal BIOS install using the Linux executable (limine, NOT limine.exe)
./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
