#!/bin/bash
set -e
echo "=== [SexOS] Fetching Limine & Building ISO ==="

# 1. Install git if missing
if ! command -v git &> /dev/null; then
    echo "git not found, attempting to install..."
    apt-get update && apt-get install -y git
fi

# 2. Download Limine binaries
if [ ! -d "limine_bin" ]; then
    git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1 limine_bin
fi

# 3. Compile the Kernel
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 4. ISO Assembly
mkdir -p build_dir
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/kernel
cp limine.cfg build_dir/

cp limine_bin/limine-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/

xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
