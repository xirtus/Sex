#!/usr/bin/env bash

# Exit immediately if any command fails
set -e

echo "[*] INITIATING PHASE 20 BUILD AUTOMATION"

# 1. Force the correct Rust environment (Bypass Homebrew)
export PATH="$HOME/.cargo/bin:$PATH"

# Ensure we are locked to nightly for OS dev
rustup override set nightly > /dev/null 2>&1

echo ">>> Compiling sexdisplay userland ELF..."

# 2. Compile the payload with custom OS target flags
cargo build \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --manifest-path servers/sexdisplay/Cargo.toml \
    --target x86_64-sex.json \
    --release

echo ">>> Staging payload into ISO directory..."

# 3. Ensure destination exists and copy the binary
mkdir -p iso_root/servers
cp target/x86_64-sex/release/sexdisplay iso_root/servers/

echo "[*] BUILD SUCCESS. Payload staged at iso_root/servers/sexdisplay"
echo "[!] Ready for Limine boot."
