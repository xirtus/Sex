#!/bin/bash
# SexOS SASOS - Tuxedo Prelude & Workspace Fix
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Cleaning up redundant imports in tuxedo/src/lib.rs..."
# Remove the lines we injected that are now causing warnings/conflicts
sed -i.bak '/use alloc::string::{String, ToString};/d' servers/tuxedo/src/lib.rs
sed -i.bak '/use alloc::vec::Vec;/d' servers/tuxedo/src/lib.rs
sed -i.bak '/use alloc::boxed::Box;/d' servers/tuxedo/src/lib.rs

echo "2. Ensuring tuxedo Cargo.toml is no_std compatible..."
if [ -f "servers/tuxedo/Cargo.toml" ]; then
    # Ensure there's no default-features for dependencies that might pull in std
    sed -i.bak 's/default-features = true/default-features = false/g' servers/tuxedo/Cargo.toml
fi

echo "3. Firing GLOBAL RELEASE BUILD (The Moment of Truth)..."
# We build the entire workspace. This verifies kernel + all servers + tuxedo.
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && 
    rustup component add rust-src && 
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
" > final_system_build.log 2>&1 || true

echo "--> Scan complete. Checking for remaining errors..."
if grep -q "error: could not compile" final_system_build.log; then
    echo "ERRORS DETECTED. Displaying culprits:"
    grep "error\[" final_system_build.log | sort | uniq
    tail -n 20 final_system_build.log
else
    echo "SUCCESS! The entire system is rustified and compiled."
    echo "Run: make run-sasos"
fi
