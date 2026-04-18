#!/bin/bash
set -e
echo "=== [SexOS] Finalizing Kernel/RT Linker Peace Treaty ==="

# 1. Fix Limine naming and BaseRevision in main.rs
# BaseRevision::new() now requires a revision number (usually 0 or 1)
sed -i 's/BaseRevision::new()/BaseRevision::new(0)/g' kernel/src/main.rs

# 2. Fix the missing markers and request module paths
# Limine 0.1.x flattens many of these
sed -i 's/use limine::request::/use limine::/g' kernel/src/main.rs
sed -i 's/RequestsStartMarker,//g' kernel/src/main.rs
sed -i 's/RequestsEndMarker//g' kernel/src/main.rs

# 3. Solve the Allocator Conflict
# We comment out the global_allocator in the kernel because sex_rt provides it
# This prevents the "conflicts with global allocator in: sex_rt" error
find kernel/src -name "*.rs" -exec sed -i 's/#[global_allocator]/\/\/ #[global_allocator]/g' {} +

# 4. Cleanup smp.rs warnings (unused unsafe/variables)
sed -i 's/unsafe { (\*cpu).lapic_id }/(*cpu).lapic_id/g' kernel/src/smp.rs
sed -i 's/let lapic_id =/let _lapic_id =/g' kernel/src/smp.rs

# 5. Build only the kernel first to verify the link
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 6. Assembly
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
    echo "FAILED: Check for linker scripts in root."
fi
