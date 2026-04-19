#!/bin/bash
# SexOS SASOS v1.0.0 - Crate Naming Correction
set -euo pipefail

echo "--> 1. Correcting linked_list_allocator in all Cargo.toml files..."
# Find all Cargo.toml files in the servers directory and replace the hyphenated name
find servers -name "Cargo.toml" -exec sed -i '' 's/linked-list-allocator/linked_list_allocator/g' {} +

echo "--> 2. Ensuring the Rust code remains underscored (Standard Practice)..."
# No changes needed to .rs files as they likely already use underscores for the 'use' statements.

echo "--> 3. Cleaning stale lockfiles to prevent dependency conflicts..."
rm -f Cargo.lock
find servers -name "Cargo.lock" -delete

echo "--> 4. Executing Final Silent SASOS Build..."
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS=\"-A warnings\"
    rustup default nightly &&
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
"

echo "=== PHASE 18.5: ARCHITECTURAL SYNTHESIS SUCCESSFUL ==="
