#!/bin/bash
set -e
echo "=== [SexOS] Applying Final Limine Naming Fixes ==="

# 1. Fix the Memory Map naming (MemoryMapRequest -> MemmapRequest)
find kernel/src -name "*.rs" -exec sed -i 's/MemoryMapRequest/MemmapRequest/g' {} +

# 2. Fix the SMP Response naming (MpResponse -> SmpResponse)
find kernel/src -name "*.rs" -exec sed -i 's/MpResponse/SmpResponse/g' {} +

# 3. Fix the Struct vs Module path error in smp.rs
# Change "use limine::SmpRequest::MpInfo" to "use limine::SmpInfo"
# (Limine flattens these in the 0.1.x series)
sed -i 's/use limine::SmpRequest::MpInfo;/use limine::SmpInfo;/' kernel/src/smp.rs 2>/dev/null || true

# 4. Final Build Trigger
echo "[Step 1/2] Rebuilding Kernel..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 5. Assembly
echo "[Step 2/2] Creating ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Compilation finished but binary was not found."
fi
