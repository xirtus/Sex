#!/bin/bash
export PATH="$HOME/.cargo/bin:$PATH"

echo "--- 1. Scrubbing Environment ---"
rm -f Cargo.lock
# We don't cargo clean every time (too slow), but we do it if the build failed
if [ ! -d "target/x86_64-unknown-none" ]; then cargo clean; fi

echo "--- 2. Building Microkernel (Intel x86_64) ---"
# -Z build-std=core,alloc is mandatory for x86_64-unknown-none
cargo build \
    -Z build-std=core,alloc \
    --target x86_64-unknown-none \
    --release

# 3. Find the Binary
KERNEL_BIN=$(find target/x86_64-unknown-none/release -maxdepth 1 -type f ! -name ".*" ! -name "*.d" ! -name "*.json" ! -name "*.rlib" | head -n 1)

if [ -z "$KERNEL_BIN" ]; then
    echo "ERROR: Build failed. You likely have a crate that REQUIRES 'std' and cannot be used in a microkernel."
    exit 1
fi

echo "--- 3. Launching QEMU (i7 Emulation) ---"
# -machine q35 + intel-iommu + pku = The Intel i7 "Isolation" Suite
qemu-system-x86_64 \
    -machine q35 \
    -cpu Skylake-Client,+pku,+smep,+smap \
    -m 2G \
    -drive format=raw,file="$KERNEL_BIN" \
    -serial stdio \
    -display none \
    -device intel-iommu
