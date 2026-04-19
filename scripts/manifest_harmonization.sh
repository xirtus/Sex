#!/bin/bash
# SexOS SASOS - Phase 18.16: Manifest Harmonization
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

MANIFEST="sex-packages/sexbuild/Cargo.toml"
echo "1. Harmonizing the sexbuild Manifest..."

if [ -f "$MANIFEST" ]; then
    echo " -> De-duplicating $MANIFEST..."
    # Create a clean version of the manifest by removing the specific 'serde' and 'toml' lines
    # then re-injecting them once to ensure a clean state.
    grep -v "serde =" "$MANIFEST" | grep -v "toml =" > "$MANIFEST.tmp"
    
    # Re-inject the necessary dependencies under [dependencies]
    # We use the specific version and features required for the SASOS build toolchain.
    sed -i.bak '/\[dependencies\]/a \
serde = { version = "1.0.228", default-features = false, features = ["derive", "alloc"] }\
toml = "0.8"' "$MANIFEST.tmp"
    
    mv "$MANIFEST.tmp" "$MANIFEST"
    rm -f "$MANIFEST.bak"
else
    echo " !! ERROR: Manifest not found at $MANIFEST"
    exit 1
fi

echo "2. Purging Synthesis Artifacts..."
rm -rf target/

echo "3. Firing ULTIMATE SASOS SYNTHESIS..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    --release" > final_harmonization_build.log 2>&1 || true

echo "--> Synthesis complete. Checking logs..."

if grep -q "Finished release" final_harmonization_build.log; then
    echo "=== PHASE 18.16: MANIFEST HARMONIZATION SUCCESSFUL ==="
    echo "The duplicate keys are gone. The workspace is valid."
    echo "The Fleet is Ready: target/x86_64-sex/release/ contains all artifacts."
else
    echo "BLOCKER DETECTED. Remaining issues in final_harmonization_build.log:"
    grep "error\[" final_harmonization_build.log | sort | uniq | head -n 5
    tail -n 20 final_harmonization_build.log
fi
