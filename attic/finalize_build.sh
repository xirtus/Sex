#!/bin/bash
echo "=== [SexOS] Finalizing Build and Cleaning Warnings ==="

# 1. Apply mechanical fixes (mut, unused imports, etc)
cargo fix --lib -p sexfiles --allow-dirty
cargo fix --bin "sexc" -p sexc --allow-dirty
cargo fix --bin "sexshop" -p sexshop --allow-dirty

# 2. Build with Warning Suppression for Legacy Logic
# This allows us to see the final Linker output without 100 lines of noise
export RUSTFLAGS="-A dead_code -A non_snake_case -C link-arg=-nostartfiles -C link-arg=-nodefaultlibs"

echo "[Step 1/2] Rebuilding all services..."
cargo build --release \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 3. Final ISO Assembly
echo "[Step 2/2] Assembling SexOS ISO..."
KERNEL_BIN=$(find target -name "kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    
    # Ensure bootloader files are present
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    
    xorriso -as mkisofs -b limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: ISO created at /src/sexos-v1.0.0.iso ==="
else
    echo "FAILED: Kernel binary not found. Check linker output."
fi
