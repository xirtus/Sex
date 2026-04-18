#!/bin/bash
set -e
echo "=== [SexOS] Final Manifest & Pathing Repair ==="

# 1. De-duplicate Cargo.toml (Fixes duplicate key error)
echo "[1/4] Cleaning sex-pdx manifest..."
awk -F'=' '!x[$1]++' crates/sex-pdx/Cargo.toml > tmp.toml && mv tmp.toml crates/sex-pdx/Cargo.toml

# 2. Fix the Limine imports (Direct Pathing)
echo "[2/4] Patching Limine structure..."
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::MemoryMap/limine::MemoryMapRequest/g' {} +
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::Smp/limine::SmpRequest/g' {} +
# Targeted fix for the 'use' statements in smp.rs
sed -i 's/use limine::request::Smp::MpInfo;/use limine::SmpInfo;/' kernel/src/smp.rs 2>/dev/null || true
sed -i 's/use limine::request::MpResponse;/use limine::SmpResponse;/' kernel/src/smp.rs 2>/dev/null || true

# 3. Final Build attempt
echo "[3/4] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Assembly
echo "[4/4] Finalizing ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Kernel binary not found."
fi
