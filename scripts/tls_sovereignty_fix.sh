#!/bin/bash
# SexOS SASOS - Phase 18.22: TLS Sovereignty (OpenSSL Bypass)
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Locating Workspace Components..."
PDX_PATH=$(find . -name "Cargo.toml" -exec grep -l 'name = "sex[-_]pdx"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')
ACTUAL_PDX_NAME=$(grep 'name =' "$PDX_PATH/Cargo.toml" | sed 's/name = "//;s/"//')

echo "2. Reconstructing sexbuild Manifest (The Rustls Pivot)..."

cat << EOF > sex-packages/sexbuild/Cargo.toml
[package]
name = "sexbuild"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
petgraph = "0.6"
sha2 = "0.10"
hex = "0.4"
walkdir = "2.4"
# Pivot: Disable default features (OpenSSL) and enable rustls-tls
reqwest = { version = "0.11", default-features = false, features = ["blocking", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
sex_pdx = { package = "$ACTUAL_PDX_NAME", path = "../../$PDX_PATH" }
EOF

echo "3. Firing ISOLATED HOST SYNTHESIS (Stage 1)..."

# Move config aside to prevent target contamination
[ -f ".cargo/config.toml" ] && mv .cargo/config.toml .cargo/config.toml.bak

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build -p sexbuild --release" || { 
        echo "ERR: Host tool build failed."; 
        [ -f ".cargo/config.toml.bak" ] && mv .cargo/config.toml.bak .cargo/config.toml;
        exit 1; 
    }

echo "4. Restoring Microkernel Synthesis Config..."
[ -f ".cargo/config.toml.bak" ] && mv .cargo/config.toml.bak .cargo/config.toml

echo "5. Final System Synthesis (Stage 2)..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_tls_build.log 2>&1 || true

echo "--> Synthesis complete. Analyzing results..."

if grep -q "Finished release" final_tls_build.log; then
    echo "=== PHASE 18.22: TLS SOVEREIGNTY SUCCESSFUL ==="
    echo "1. Host Tool: target/release/sexbuild (READY - No OpenSSL needed)"
    echo "2. Kernel: target/x86_64-sex/release/sex-kernel (READY)"
    echo "The build system is now entirely Rust-native."
else
    echo "BLOCKER DETECTED. Final log analysis:"
    tail -n 20 final_tls_build.log
fi
