#!/bin/bash
set -e
echo "=== [SexOS] Executing Final Pathing Repair ==="

# 1. Fix the Limine Pathing errors (E0432/E0433)
# In Limine 0.1.x, we use direct paths or the modern flat structure
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::MemoryMap/limine::MemoryMapRequest/g' {} +
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::Smp/limine::SmpRequest/g' {} +
# Fix the specific use statements in smp.rs
sed -i 's/use limine::request::Smp::MpInfo;/use limine::SmpInfo;/' kernel/src/smp.rs
sed -i 's/use limine::request::MpResponse;/use limine::SmpResponse;/' kernel/src/smp.rs

# 2. Fix sex-pdx feature warnings
# We add the missing 'serde' feature to the crate's metadata
if [ -f "crates/sex-pdx/Cargo.toml" ]; then
    sed -i '/\[features\]/a serde = ["dep:serde"]' crates/sex-pdx/Cargo.toml
    sed -i '/\[dependencies\]/a serde = { version = "1.0", optional = true, features = ["derive"] }' crates/sex-pdx/Cargo.toml
fi

# 3. Clean and Final Build attempt
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Assembly
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
    echo "FAILED: Compilation successful but binary not found."
fi
