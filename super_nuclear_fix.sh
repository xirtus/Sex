#!/bin/bash
echo "--- [1/5] Resetting Workspace & Locks ---"
rm -f Cargo.lock
export PATH="$HOME/.cargo/bin:$PATH"

echo "--- [2/5] Standardizing Dependencies (No Duplicates) ---"
# We use a more specific Perl regex that replaces the entire line for known problem crates.
# This ensures we don't get 'duplicate key' errors.
find . -name "Cargo.toml" -exec perl -i -pe 's/^serde\s*=.*/serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }/g' {} +
find . -name "Cargo.toml" -exec perl -i -pe 's/^bitflags\s*=.*/bitflags = { version = "2.6", default-features = false }/g' {} +

echo "--- [3/5] Creating Cargo Config Overrides ---"
mkdir -p .cargo
cat << 'TOC' > .cargo/config.toml
[build]
target = "x86_64-unknown-none"

[unstable]
build-std = ["core", "alloc"]
build-std-features = ["compiler-builtins-mem"]

[target.x86_64-unknown-none]
runner = "qemu-system-x86_64 -machine q35 -cpu Skylake-Client,+pku,+smep,+smap -m 2G -serial stdio -display none -device intel-iommu -drive format=raw,file="
TOC

echo "--- [4/5] Building with Nightly ---"
# We now use 'cargo build' directly because .cargo/config.toml handles the flags
cargo build --release

echo "--- [5/5] Final Binary Check ---"
KERNEL_BIN=$(find target/x86_64-unknown-none/release -maxdepth 1 -type f ! -name ".*" ! -name "*.d" ! -name "*.json" ! -name "*.rlib" | head -n 1)

if [ -z "$KERNEL_BIN" ]; then
    echo "ERROR: Build failed. A transitive dependency is still pulling in 'std'."
    echo "Run: 'cargo tree -e features | grep \"std\"' to find the traitor."
    exit 1
fi

echo "--- Launching QEMU ---"
qemu-system-x86_64 \
    -machine q35 \
    -cpu Skylake-Client,+pku,+smep,+smap \
    -m 2G \
    -drive format=raw,file="$KERNEL_BIN" \
    -serial stdio \
    -display none \
    -device intel-iommu
