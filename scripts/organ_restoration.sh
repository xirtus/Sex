#!/bin/bash
# SexOS SASOS - Phase 18.20: Organ Restoration
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Locating sex_pdx and sex_rt for the Host Tool..."
# We need the real paths to link sexbuild to the rest of the workspace
PDX_PATH=$(find . -name "Cargo.toml" -exec grep -l 'name = "sex[-_]pdx"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')

if [ -z "$PDX_PATH" ]; then
    echo " -> Warning: sex_pdx not found. Attempting to skip local link..."
fi

echo "2. Reconstructing the sexbuild Manifest..."

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
$( [ -n "$PDX_PATH" ] && echo "sex_pdx = { path = \"../../$PDX_PATH\" }" )
EOF

echo "3. Fixing the Never-Type Fallback (Line 339)..."
# Correcting the toml parsing line to satisfy the nightly compiler's type inference
sed -i.bak 's/Ok(toml::from_str(&content)?)/Ok(toml::from_str::<Recipe>(\&content)?)/' sex-packages/sexbuild/src/main.rs

echo "4. Firing ISOLATED HOST SYNTHESIS..."

# Move the microkernel config aside to ensure a clean host environment
[ -f ".cargo/config.toml" ] && mv .cargo/config.toml .cargo/config.toml.bak

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build -p sexbuild --release" || { 
        echo "ERR: Host tool build failed."; 
        [ -f ".cargo/config.toml.bak" ] && mv .cargo/config.toml.bak .cargo/config.toml;
        exit 1; 
    }

echo "5. Restoring Microkernel Synthesis Config..."
[ -f ".cargo/config.toml.bak" ] && mv .cargo/config.toml.bak .cargo/config.toml

echo "6. Final Microkernel Synthesis..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_organ_build.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."
if grep -q "Finished release" final_organ_build.log; then
    echo "=== PHASE 18.20: ORGAN RESTORATION SUCCESSFUL ==="
    echo "Host Tool & Kernel ELFs are synchronized."
else
    echo "BLOCKER DETECTED. Analyzing logs..."
    tail -n 20 final_organ_build.log
fi
