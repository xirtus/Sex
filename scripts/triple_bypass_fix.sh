#!/bin/bash
# SexOS SASOS - Phase 18.18: The Triple Bypass
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Neutralizing Target Lockdown for Host Tools..."

# Build sexbuild while explicitly forcing the HOST triple to bypass .cargo/config.toml
# We use x86_64-unknown-linux-gnu because we are inside the Docker builder
echo " -> Stage 1: Building sexbuild for the Host (Linux)..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build -p sexbuild --release --target x86_64-unknown-linux-gnu" || { echo "ERR: Host tool build failed."; exit 1; }

echo "2. Purging Cross-Compilation Corruption..."
# Remove the broken target folder to clear the 5000+ serde errors
rm -rf target/x86_64-sex/

echo "3. Building the SASOS Microkernel Ecosystem..."

# Now build the actual system ELFs using the custom JSON target
# We explicitly list the packages to prevent 'sexbuild' from being pulled in again
echo " -> Stage 2: Building Microkernel & Servers (SexOS Target)..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    -p sexdisplay \
    -p tuxedo \
    --release" > triple_bypass.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" triple_bypass.log; then
    echo "=== PHASE 18.18: TRIPLE BYPASS SUCCESSFUL ==="
    echo "1. Host Tool: target/x86_64-unknown-linux-gnu/release/sexbuild"
    echo "2. Kernel: target/x86_64-sex/release/sex-kernel"
    echo "3. Userland: target/x86_64-sex/release/egui-hello"
    echo "The System is fully synthesized."
else
    echo "BLOCKER DETECTED. Analyzing remaining errors..."
    grep "error\[" triple_bypass.log | sort | uniq | head -n 10
    tail -n 20 triple_bypass.log
fi
