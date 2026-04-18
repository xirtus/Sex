#!/bin/bash

# 1. CLEAN ENVIRONMENT
echo "--- [1/6] Cleaning Rustup Overrides ---"
export PATH="$HOME/.cargo/bin:$PATH"
rustup override unset
rustup default nightly

# 2. FORCE PROJECT LOCKS
echo "--- [2/6] Creating Infinite Nightly Lock ---"
cat << 'TOC' > rust-toolchain.toml
[toolchain]
channel = "nightly"
components = ["rust-src", "llvm-tools-preview"]
targets = ["x86_64-unknown-none"]
TOC

# 3. FIX WORKSPACE RESOLVER
echo "--- [3/6] Forcing Workspace Resolver 2 ---"
perl -i -pe 's/resolver = "1"/resolver = "2"/g' Cargo.toml
if ! grep -q "resolver =" Cargo.toml; then
    perl -i -pe 's/\[workspace\]/\[workspace\]\nresolver = "2"/g' Cargo.toml
fi

# 4. SURGICAL DEPENDENCY PATCHING (Idempotent)
echo "--- [4/6] Cleaning and Standardizing Cargo.toml files ---"
# Step A: Remove any existing default-features keys to prevent duplicates
find . -name "Cargo.toml" -exec perl -i -pe 's/default-features\s*=\s*(true|false),?\s*//g' {} +

# Step B: Normalize serde entries to include default-features = false cleanly
find . -name "Cargo.toml" -exec perl -i -pe 's/serde\s*=\s*\{/serde = { default-features = false, /g' {} +

# Step C: Handle simple string versions like serde = "1.0"
find . -name "Cargo.toml" -exec perl -i -pe 's/serde\s*=\s*"([^"]+)"/serde = { version = "$1", default-features = false, features = ["derive"] }/g' {} +

# Step D: Ensure bitflags is also standard
find . -name "Cargo.toml" -exec perl -i -pe 's/bitflags\s*=\s*"([^"]+)"/bitflags = { version = "$1", default-features = false }/g' {} +

# 5. THE BUILD
echo "--- [5/6] Running Build (Intel x86_64) ---"
rm -f Cargo.lock
cargo build -Z build-std=core,alloc --target x86_64-unknown-none --release

# 6. QEMU LAUNCHER
echo "--- [6/6] Launching QEMU ---"
KERNEL_BIN=$(find target/x86_64-unknown-none/release -maxdepth 1 -type f ! -name ".*" ! -name "*.d" ! -name "*.json" ! -name "*.rlib" | head -n 1)

if [ -z "$KERNEL_BIN" ]; then
    echo "CRITICAL ERROR: Build failed. Check for duplicate keys in Cargo.toml manually if errors persist."
    exit 1
fi

qemu-system-x86_64 \
    -machine q35 \
    -cpu Skylake-Client,+pku,+smep,+smap \
    -m 2G \
    -drive format=raw,file="$KERNEL_BIN" \
    -serial stdio \
    -display none \
    -device intel-iommu
