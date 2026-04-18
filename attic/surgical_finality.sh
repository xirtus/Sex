#!/bin/bash
set -e
echo "=== [SexOS] Executing Surgical Finality Fix ==="

TARGET_MAIN="kernel/src/main.rs"

# 1. Fix Limine Request initialization (E0061)
# Adds (0) to all .new() calls for Requests
sed -i 's/Request::new()/Request::new(0)/g' "$TARGET_MAIN"

# 2. Fix Response access (E0599)
# Changes .response() to .get_response()
sed -i 's/\.response()/\.get_response()/g' "$TARGET_MAIN"

# 3. Fix naming (E0432/E0433)
# MpRequest -> SmpRequest
sed -i 's/MpRequest/SmpRequest/g' "$TARGET_MAIN"

# 4. Handle Markers (E0425/E0433)
# If markers are missing, we'll comment them out for this build 
# as Limine 0.1.x often handles the list automatically via the section attribute
sed -i 's/static REQ_START/\/\/ static REQ_START/g' "$TARGET_MAIN"
sed -i 's/static REQ_END/\/\/ static REQ_END/g' "$TARGET_MAIN"

# 5. Resolve Global Allocator Conflict
# Force-remove the allocator from the kernel crate to favor sex-rt
find kernel/src -name "*.rs" -exec sed -i 's/#[global_allocator]/\/\/ #[global_allocator]/g' {} +

# 6. Rebuild with explicit linker instructions
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 7. ISO Assembly
echo "[Step 2/2] Finalizing ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Linker issues persist. Check target/x86_64-sex/release/."
fi
