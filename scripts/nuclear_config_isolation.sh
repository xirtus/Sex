#!/bin/bash
# SexOS SASOS - Phase 18.19: Nuclear Config Isolation
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Isolate Host Synthesis from Microkernel Config..."

# Move the config aside so Cargo doesn't see 'build-std' during the host build
if [ -f ".cargo/config.toml" ]; then
    echo " -> Temporarily deactivating .cargo/config.toml"
    mv .cargo/config.toml .cargo/config.toml.workspace
fi

echo "2. Building Host-Side Build Tool (sexbuild)..."

# Build sexbuild as a standard 'std' binary. 
# We purge the target folder first to clear the 5000+ error artifacts.
rm -rf target/release/deps/libcore-*
rm -rf target/x86_64-unknown-linux-gnu/

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build -p sexbuild --release" || { 
        echo "ERR: Host tool build failed."; 
        mv .cargo/config.toml.workspace .cargo/config.toml 2>/dev/null || true; 
        exit 1; 
    }

echo "3. Restoring Workspace Sovereignty..."

if [ -f ".cargo/config.toml.workspace" ]; then
    mv .cargo/config.toml.workspace .cargo/config.toml
    echo " -> .cargo/config.toml reactivated."
fi

echo "4. Synthesizing the SASOS Microkernel Ecosystem..."

# Now build the microkernel and servers using the custom JSON target.
# Since sexbuild is already built, Cargo will skip it.
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
    --release" > final_isolated_build.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_isolated_build.log; then
    echo "=== PHASE 18.19: NUCLEAR ISOLATION SUCCESSFUL ==="
    echo "1. Host Tool: target/release/sexbuild (READY)"
    echo "2. Kernel: target/x86_64-sex/release/sex-kernel (READY)"
    echo "The Fleet is fully synchronized."
else
    echo "BLOCKER DETECTED. The collision persists."
    grep "error\[" final_isolated_build.log | sort | uniq | head -n 5
    tail -n 20 final_isolated_build.log
fi
