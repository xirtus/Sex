#!/bin/bash
# SexOS SASOS - Phase 18.14: Underscore Reconciliation
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Re-scanning for the Sovereign Runtime..."
# Locate the runtime crate again
REAL_RT_PATH=$(find . -maxdepth 2 -name "Cargo.toml" -exec grep -l 'name = "sex[-_]rt"' {} \+ | head -n 1 | xargs dirname | sed 's|^\./||')

if [ -z "$REAL_RT_PATH" ]; then
    echo " !! ERROR: sex-rt not found. Check repository root."
    exit 1
fi

# Extract the literal name from Cargo.toml
RT_CRATE_NAME=$(grep 'name =' "$REAL_RT_PATH/Cargo.toml" | sed 's/name = "//;s/"//')
# Convert hyphens to underscores for valid Rust 'use' syntax
RT_IDENTIFIER=$(echo "$RT_CRATE_NAME" | sed 's/-/_/g')

echo " -> Found Crate: $RT_CRATE_NAME"
echo " -> Mapped Identifier: $RT_IDENTIFIER"

echo "2. Hard-Fixing egui-hello Manifest and Source..."

# Ensure the path is correct (../../sex-rt if it's at the root)
cat << EOF > crates/egui-hello/Cargo.toml
[package]
name = "egui-hello"
version = "0.1.0"
edition = "2021"

[dependencies]
$RT_CRATE_NAME = { path = "../../$REAL_RT_PATH", default-features = false }
EOF

# Redefine source with the corrected underscore identifier
cat << EOF > crates/egui-hello/src/main.rs
#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use $RT_IDENTIFIER; // Corrected syntax: underscores only

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _greeting = String::from("Hello from the Orbital Userland!");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

echo "3. Firing FINAL SASOS SYNTHESIS..."

# We must clear the cache to ensure the new manifest is picked up
cargo clean

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > final_reconciliation.log 2>&1 || true

echo "--> Synthesis complete. Checking logs..."
if grep -q "Finished release" final_reconciliation.log; then
    echo "=== PHASE 18.14: SUCCESSFUL SYNTHESIS ==="
    echo "The naming mismatch is resolved. The binary is ready."
else
    echo "BLOCKER REMAINS. Check final_reconciliation.log for the new error."
    tail -n 20 final_reconciliation.log
fi
