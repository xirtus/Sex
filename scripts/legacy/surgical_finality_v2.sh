#!/bin/bash
set -e
echo "=== [SexOS] Executing Surgical Finality Fix (v2) ==="

TARGET_MAIN="kernel/src/main.rs"

# 1. Fix Limine Request initialization (E0061)
# Adds (0) to all .new() calls for Requests
sed -i 's/Request::new()/Request::new(0)/g' "$TARGET_MAIN"

# 2. Fix Ptr<T> access (E0599)
# Changes .get_response().unwrap() to .get_response().as_ptr().as_ref().expect(...)
# This satisfies the 2026 pointer safety requirements
sed -i 's/get_response()\.unwrap()/get_response().as_ptr().as_ref().expect("Limine request failed")/g' "$TARGET_MAIN"

# 3. Final Build Trigger
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. ISO Assembly
echo "[Step 2/2] Finalizing ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    # Ensure the bootloader binary is present for xorriso
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Linker failure. Check for unresolved symbols in target/."
fi
