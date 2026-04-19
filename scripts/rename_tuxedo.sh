#!/bin/bash
# SexOS SASOS - Rename DDE Broker to 'tuxedo' (Comprehensive Sweep)
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Renaming directory..."
if [ -d "servers/tuxedo" ]; then
    mv servers/tuxedo servers/tuxedo
    echo " -> servers/tuxedo renamed to servers/tuxedo"
else
    echo " -> servers/tuxedo not found. Already renamed?"
fi

echo "2. Updating internal Cargo.toml..."
if [ -f "servers/tuxedo/Cargo.toml" ]; then
    sed -i.bak 's/name = "tuxedo"/name = "tuxedo"/' servers/tuxedo/Cargo.toml
    echo " -> Updated name in servers/tuxedo/Cargo.toml"
fi

echo "3. Sweeping workspace and all other crates for dependencies..."
find . -name "Cargo.toml" -type f | while read -r toml_file; do
    if grep -q "tuxedo" "$toml_file"; then
        # Replace path-based dependency
        sed -i.bak 's/tuxedo = { path = "\([^"]*\)tuxedo"/tuxedo = { path = "\1tuxedo"/' "$toml_file"
        # Replace workspace members
        sed -i.bak 's/"servers\/tuxedo"/"servers\/tuxedo"/' "$toml_file"
        echo " -> Patched dependencies in $toml_file"
    fi
done

echo "4. Sweeping source code for 'extern crate' or 'use' statements..."
find servers kernel sex-packages -name "*.rs" -type f 2>/dev/null | while read -r rs_file; do
    if grep -q "tuxedo" "$rs_file"; then
        sed -i.bak 's/tuxedo::/tuxedo::/g' "$rs_file"
        sed -i.bak 's/extern crate tuxedo;/extern crate tuxedo;/g' "$rs_file"
        echo " -> Patched Rust imports in $rs_file"
    fi
done

echo "5. Sweeping Documentation, Scripts, and Makefiles..."
find . -type f \( -name "*.md" -o -name "*.sh" -o -name "Makefile" -o -name "*.txt" \) | while read -r doc_file; do
    if grep -q "tuxedo" "$doc_file"; then
        # Replace standalone word references (case sensitive to preserve history if needed, but aggressive here)
        sed -i.bak 's/\btuxedo\b/tuxedo/g' "$doc_file"
        echo " -> Patched text references in $doc_file"
    fi
done

echo "6. Firing isolated build for tuxedo to find the true errors..."
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "rustup default nightly && cargo build --package tuxedo --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release" > tuxedo_build.log 2>&1 || true

echo "--> Renaming complete. Here are the actual tuxedo errors blocking the build:"
grep -B 2 -A 10 "error\[" tuxedo_build.log | head -n 40
