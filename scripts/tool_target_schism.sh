#!/bin/bash
# SexOS SASOS - Phase 18.17: The Tool/Target Schism
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Restoring sexbuild to Host-Side Sovereignty..."

# Ensure sexbuild is a standard 'std' crate. It's a tool, not a kernel server.
MANIFEST="sex-packages/sexbuild/Cargo.toml"
if [ -f "$MANIFEST" ]; then
    echo " -> Reconfiguring $MANIFEST for Host Synthesis..."
    cat << EOF > "$MANIFEST"
[package]
name = "sexbuild"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
EOF
fi

echo "2. Purging Corrupted Cross-Compilation Cache..."
# This is mandatory to clear the 5,000+ error artifacts
rm -rf target/
cargo clean

echo "3. Firing DUAL-STAGE SYNTHESIS..."

# STAGE 1: Build the Host Tools (Linux x86_64)
echo " -> Stage 1: Building Host Utilities (sexbuild)..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build -p sexbuild --release" || { echo "ERR: Host tool build failed."; exit 1; }

# STAGE 2: Build the System (SexOS x86_64)
# We only pass the specific system packages to the cross-compiler
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
    --release" > final_schism_build.log 2>&1 || true

echo "--> Synthesis complete. Checking logs..."

if grep -q "Finished release" final_schism_build.log; then
    echo "=== PHASE 18.17: SUCCESSFUL DUAL-STAGE SYNTHESIS ==="
    echo "1. Host Tool: target/release/sexbuild (READY)"
    echo "2. Kernel: target/x86_64-sex/release/sex-kernel (READY)"
    echo "3. Userland: target/x86_64-sex/release/egui-hello (READY)"
    echo "The Fleet is Armed. Next: make run-sasos"
else
    echo "BLOCKER REMAINS. Check final_schism_build.log for errors."
    grep "error\[" final_schism_build.log | sort | uniq | head -n 5
    tail -n 20 final_schism_build.log
fi
