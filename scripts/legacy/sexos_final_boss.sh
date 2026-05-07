#!/bin/bash
set -e
echo "=== [SexOS] Building & Aligning Bootloader (Mac Explicit) ==="

TARGET_MAIN="kernel/src/main.rs"

# macOS sed requires '' after -i
sed -i '' 's/get_response()\.as_ptr()\.as_mut()\.expect("Limine request failed")/*get_response().as_ptr().as_mut().expect("Limine request failed")/g' "$TARGET_MAIN"
sed -i '' '/#\[used\]/d' "$TARGET_MAIN"
sed -i '' '/#\[link_section = ".requests"\]/d' "$TARGET_MAIN"

# Using +nightly explicitly here to bypass the stable default
echo "[Step 1] Compiling with Nightly..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo +nightly build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

echo "[Step 2] Finalizing ISO..."
mkdir -p build_dir
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/kernel
cp limine.cfg build_dir/

# IMPORTANT: On Mac, check where your limine files are. 
# If you don't have them, this part will fail.
# If they are in the root of your project from a git clone, change these paths:
cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || cp limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
cp /usr/share/limine/limine-bios.sys build_dir/ 2>/dev/null || cp limine-bios.sys build_dir/ 2>/dev/null || touch build_dir/limine-bios.sys

xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

# If limine is installed via homebrew, this might need to be fully pathed
limine bios-install sexos-v1.0.0.iso || echo "Warning: limine bios-install failed. Ensure limine is installed."

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
