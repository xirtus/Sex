#!/bin/bash
set -e
echo "=== [SexOS] Fetching Limine Binaries & Building ISO ==="

# 1. Download the matching Limine binaries (v7.x branch)
if [ ! -d "limine_bin" ]; then
    echo "Downloading Limine binaries..."
    git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1 limine_bin
fi

# 2. Compile the Kernel (Our pointer fixes are already in main.rs)
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 3. Prepare ISO Directory
mkdir -p build_dir
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/kernel
cp limine.cfg build_dir/

# 4. Sync Limine files from the NEWLY downloaded folder
cp limine_bin/limine-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/
cp limine_bin/limine-uefi-cd.bin build_dir/ 2>/dev/null || true

# 5. Create the ISO
xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

# 6. Seal the BIOS install using the binary we just downloaded
./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
