#!/bin/bash
# SexOS SASOS - Phase 18.21: Naming Convention Alignment
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Harmonizing sex-pdx Naming..."

# Dynamically locate the sex-pdx path again
PDX_PATH=$(find . -name "Cargo.toml" -exec grep -l 'name = "sex[-_]pdx"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')

if [ -z "$PDX_PATH" ]; then
    echo " !! ERROR: sex-pdx not found. Build cannot continue."
    exit 1
fi

# Extract the literal package name (likely sex-pdx)
ACTUAL_PDX_NAME=$(grep 'name =' "$PDX_PATH/Cargo.toml" | sed 's/name = "//;s/"//')

echo " -> Found Package: $ACTUAL_PDX_NAME at $PDX_PATH"

echo "2. Reconstructing the sexbuild Manifest with Package Alias..."

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
reqwest = { version = "0.11", features = ["blocking"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
# Bridge: Use underscore identifier but point to hyphenated package
sex_pdx = { package = "$ACTUAL_PDX_NAME", path = "../../$PDX_PATH" }
EOF

echo "3. Firing ISOLATED HOST SYNTHESIS (Stage 1)..."

# Move the microkernel config aside to ensure a clean host environment
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

# Now build the kernel and the egui-hello server
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_alignment_build.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_alignment_build.log; then
    echo "=== PHASE 18.21: NAMING ALIGNMENT SUCCESSFUL ==="
    echo "Host Tool: target/release/sexbuild (READY)"
    echo "Kernel: target/x86_64-sex/release/sex-kernel (READY)"
    echo "Architecture is now consistent."
else
    echo "BLOCKER DETECTED. Final log analysis:"
    tail -n 20 final_alignment_build.log
fi
