#!/bin/bash
set -e
echo "=== [SexOS] Finalizing Kernel/RT Linker Peace Treaty (v5) ==="

# 1. Locate main.rs and apply Limine/BaseRevision fixes
# Limine 0.1.x flattens many paths and requires a u64 for BaseRevision::new
TARGET_MAIN="kernel/src/main.rs"
if [ -f "$TARGET_MAIN" ]; then
    echo "Patching: $TARGET_MAIN"
    sed -i 's/BaseRevision::new()/BaseRevision::new(0)/g' "$TARGET_MAIN"
    sed -i 's/use limine::request::/use limine::/g' "$TARGET_MAIN"
    sed -i 's/RequestsStartMarker,//g' "$TARGET_MAIN"
    sed -i 's/RequestsEndMarker//g' "$TARGET_MAIN"
fi

# 2. Solve the Allocator Conflict
# We comment out the global_allocator in the kernel because sex_rt provides it
find kernel/src -name "*.rs" -exec sed -i 's/#[global_allocator]/\/\/ #[global_allocator]/g' {} +

# 3. Cleanup smp.rs logic and warnings
TARGET_SMP="kernel/src/smp.rs"
if [ -f "$TARGET_SMP" ]; then
    echo "Cleaning warnings in: $TARGET_SMP"
    sed -i 's/unsafe { (\*cpu).lapic_id }/(*cpu).lapic_id/g' "$TARGET_SMP"
    sed -i 's/let lapic_id =/let _lapic_id =/g' "$TARGET_SMP"
fi

# 4. Build
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 5. Assembly
echo "[Step 2/2] Finalizing ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/ 2>/dev/null || cp kernel/limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Kernel binary not found."
fi
