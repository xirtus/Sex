#!/bin/bash
# SexOS SASOS - Phase 18.15: Dependency Restoration
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Repairing the sexbuild Manifest..."

MANIFEST="sex-packages/sexbuild/Cargo.toml"
if [ -f "$MANIFEST" ]; then
    # Check if toml is already there; if not, add it
    if ! grep -q 'toml =' "$MANIFEST"; then
        echo " -> Injecting 'toml' dependency into $MANIFEST"
        # Adding toml and serde (usually required for toml parsing in Rust)
        sed -i.bak '/\[dependencies\]/a \
toml = "0.8" \
serde = { version = "1.0", features = ["derive"] }' "$MANIFEST"
    fi
else
    echo " !! ERROR: Manifest not found at $MANIFEST"
    exit 1
fi

echo "2. Purging Synthesis Artifacts..."
# Clean target to ensure no stale 'unresolved import' metadata persists
cargo clean

echo "3. Firing ULTIMATE SASOS SYNTHESIS..."

# We execute the synthesis using the known-good toolchain and target spec
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > final_dependency_fix.log 2>&1 || true

echo "--> Synthesis complete. Analyzing the log..."

if grep -q "Finished release" final_dependency_fix.log; then
    echo "=== PHASE 18.15: DEPENDENCY RESTORATION SUCCESSFUL ==="
    echo "All server ELFs and build tools are now synchronized."
    echo "Next: verify the artifacts in target/x86_64-sex/release/"
else
    echo "BLOCKER DETECTED. Remaining issues in final_dependency_fix.log:"
    grep "error\[" final_dependency_fix.log | sort | uniq | head -n 5
    tail -n 20 final_dependency_fix.log
fi
