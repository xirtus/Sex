#!/bin/bash
set -euo pipefail

echo "==> STARTING KERNEL REPAIR (E0521 & E0793)..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    python3 -c \"
import os

def patch_file(path, search, replace):
    if not os.path.exists(path): return
    with open(path, 'r') as f: content = f.read()
    if search in content:
        print(f'Patching {path}...')
        with open(path, 'w') as f: f.write(content.replace(search, replace))

# 1. Fix E0793: Reference to packed field in APIC
# Common in ACPI/Limine structs. Change reference to copy.
patch_file('kernel/src/apic.rs', '&sdt.signature', 'sdt.signature')
patch_file('kernel/src/apic.rs', '&header.signature', 'header.signature')

# 2. Fix E0521: Lifetime escapes in closures
# Injecting 'move' to closures in memory management
patch_file('kernel/src/memory.rs', 'map_range(|', 'map_range(move |')
patch_file('kernel/src/memory.rs', 'map(|', 'map(move |')
patch_file('kernel/src/memory/allocator.rs', 'lock(|', 'lock(move |')
\"

    echo '==> TRIGGERING VERIFICATION BUILD...'
    rustup default nightly
    cargo build \
        --target x86_64-sex.json \
        -Zbuild-std=core,alloc \
        -Zjson-target-spec \
        --release
" | tee build_kernel_fix.log

if grep -q "Finished release" build_kernel_fix.log; then
    echo "SUCCESS: KERNEL COMPILED CLEANLY."
    echo "PROCEED TO: make run-sasos"
else
    echo "FAILED: Remaining errors in build_kernel_fix.log."
fi
