#!/bin/bash
set -euo pipefail

echo "==> 1. Patching sexfiles (Fixing E0463: can't find crate for std)"
# Ensure sexfiles is pure no_std
sed -i '1i #![no_std]' servers/sexfiles/src/lib.rs 2>/dev/null || true
sed -i 's/use std/use core/g' servers/sexfiles/src/lib.rs 2>/dev/null || true

echo "==> 2. Patching APIC (Fixing E0793: Reference to packed field)"
# Fix: Copy packed fields to stack instead of referencing them
# Pattern: Accessing fields in Limine/ACPI packed structs
python3 -c "
import sys
f = 'kernel/src/apic.rs'
with open(f, 'r') as file: content = file.read()
# Replace direct references with local copies for common APIC/ACPI packed patterns
content = content.replace('&sdt.signature', '{ let s = sdt.signature; s }')
with open(f, 'w') as file: file.write(content)
"

echo "==> 3. Patching Memory Closures (Fixing E0521: Lifetime escapes)"
# Force closures to take ownership to satisfy Higher-Half mapping lifetimes
sed -i 's/map_range(|/map_range(move |/g' kernel/src/memory.rs 2>/dev/null || true
sed -i 's/map(|/map(move |/g' kernel/src/memory.rs 2>/dev/null || true

echo "==> 4. Executing FINAL VALIDATED BUILD"
# Using -Zjson-target-spec (no space) as required by nightly 1.97.0
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    export RUSTFLAGS='-C target-feature=+soft-float'
    rustup default nightly
    cargo build \
        --target x86_64-sex.json \
        -Zbuild-std=core,alloc \
        -Zjson-target-spec \
        --release
" | tee build_final.log

if grep -q "Finished release" build_final.log; then
    echo "SUCCESS: SASOS KERNEL AND SERVERS ALIGNED."
    echo "PROCEED TO: make run-sasos"
else
    echo "BUILD FAILED. Check build_final.log for remaining E0521/E0793 instances."
fi
