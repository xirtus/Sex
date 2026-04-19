#!/bin/bash
# SexOS SASOS - Phase 18.11: Runtime Sovereignty
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Harmonizing egui-hello with the sex_rt runtime..."

# Remove the conflicting global_allocator block from egui-hello
if [ -f "crates/egui-hello/src/main.rs" ]; then
    echo " -> Stripping redundant allocator from egui-hello..."
    # This perl command deletes the block from #[global_allocator] to the end of the static declaration
    perl -0777 -i -pe 's/\#\[global_allocator\].*?static ALLOCATOR:.*?=.*?;/ \/\/ Inheriting GlobalAlloc from sex_rt/gs' crates/egui-hello/src/main.rs
    
    # Remove the unresolved import
    sed -i.bak '/use linked_list_allocator::LockedHeap;/d' crates/egui-hello/src/main.rs
fi

echo "2. Synchronizing Cargo manifests..."

# Ensure egui-hello points to sex_rt if it doesn't already
if [ -f "crates/egui-hello/Cargo.toml" ]; then
    if ! grep -q "sex_rt" "crates/egui-hello/Cargo.toml"; then
        echo " -> Linking egui-hello to sex-runtime..."
        echo 'sex_rt = { path = "../../crates/sex_rt" }' >> crates/egui-hello/Cargo.toml
    fi
fi

echo "3. Firing ULTIMATE SASOS SYNTHESIS..."

# We must use the JSON target spec and the compiler-builtins-mem flags discovered in Phase 18.7
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    rustup component add rust-src && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > final_synthesis.log 2>&1 || true

echo "--> Synthesis complete. Checking for the 'Success' signature..."

if grep -q "Finished release" final_synthesis.log; then
    echo "=== PHASE 18.11: SYSTEM SYNTHESIS SUCCESSFUL ==="
    echo "All binaries are now valid SASOS ELFs."
    echo "The hardware handoff is ready: make run-sasos"
else
    echo "BLOCKER DETECTED. Culprits found in final_synthesis.log:"
    grep "error\[" final_synthesis.log | sort | uniq
    tail -n 15 final_synthesis.log
fi
