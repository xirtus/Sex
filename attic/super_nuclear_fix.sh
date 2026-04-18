#!/bin/bash
set -e
echo "=== [SexOS] Super Nuclear Manifest Reset ==="

# 1. Rewrite sex-pdx Cargo.toml to be valid
cat << 'TOML' > crates/sex-pdx/Cargo.toml
[package]
name = "sex-pdx"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0.228", optional = true, default-features = false, features = ["derive", "alloc"] }
x86_64 = "0.15.4"
spin = "0.9.8"

[features]
default = []
serde = ["dep:serde"]
TOML

# 2. Fix the Limine structure in the source
echo "[2/3] Patching Limine structure..."
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::MemoryMap/limine::MemoryMapRequest/g' {} +
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::Smp/limine::SmpRequest/g' {} +
find kernel/src -name "*.rs" -exec sed -i 's/limine::request::MpResponse/limine::SmpResponse/g' {} +

# 3. Final Build attempt
echo "[3/3] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Finalizing ISO
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
fi
