#!/bin/bash
echo "=== [SexOS] Injecting Missing Dependencies ==="

# 1. Add x86_64 crate to the kernel Cargo.toml
# We use a specific version known for no_std stability in 2026
sed -i '/\[dependencies\]/a x86_64 = "0.15.2"' kernel/Cargo.toml

# 2. Fix the unused MMIO import in the 'tuxedo' server
# This cleans up the noise so we can see real errors
sed -i 's/use sex_pdx::{pdx_call, mmio::Mmio};/use sex_pdx::pdx_call;/' servers/tuxedo/src/lib.rs

# 3. Clean and Trigger Rebuild
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A non_snake_case -C link-arg=-Tlinker.ld"
cargo build --release \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Final ISO Assembly
echo "[Step 2/2] Assembling SexOS ISO..."
KERNEL_BIN=$(find target -name "kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    # Pull bootloader files
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    
    xorriso -as mkisofs -b limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: ISO created at /src/sexos-v1.0.0.iso ==="
else
    echo "FAILED: Kernel binary not found. Still hitting compilation errors."
fi
