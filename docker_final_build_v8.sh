#!/bin/bash
set -e
echo "=== [SexOS] Final Assembly (Fixing Kernel Path) ==="

# 1. Prepare ISO Directory with the /boot folder!
mkdir -p build_dir/boot

# 2. Compile Kernel
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 3. Copy Kernel to the exact path Limine is looking for
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/boot/sexos-kernel

# 4. Copy config and Limine binaries
cp limine.cfg build_dir/
echo "Copying Limine v7 binaries..."
cp limine_bin/limine-bios-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/

# 5. Create ISO
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

# 6. Seal BIOS install
./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: The Kernel is now in the right place! ==="
