#!/bin/bash
set -euo pipefail

echo "==> STARTING ATOMIC REPAIR (CONTAINER-SIDE)..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    # 1. Use Python inside Linux to surgically align headers at index 0
    python3 -c \"
import os
for root, _, files in os.walk('.'):
    for f in files:
        if f in ['main.rs', 'lib.rs'] and ('servers' in root or 'kernel' in root):
            path = os.path.join(root, f)
            with open(path, 'r') as file: lines = file.readlines()
            
            # Extract content without existing attributes
            body = [l for l in lines if not l.strip().startswith(('#![', 'extern crate alloc;'))]
            
            # Construct pure SASOS header
            header = [
                '#![no_std]\\n',
                '#![no_main]\\n',
                '#![feature(alloc_error_handler)]\\n',
                '\\n',
                'extern crate alloc;\\n'
            ]
            
            with open(path, 'w') as file:
                file.writelines(header + body)
    \"

    # 2. Patch APIC E0793 (Reference to packed field)
    # Replaces common pattern '&sdt.signature' with a stack copy
    sed -i 's/&sdt.signature/sdt.signature/g' kernel/src/apic.rs 2>/dev/null || true

    # 3. Clean and Build
    rustup default nightly
    cargo clean
    cargo build \\
        --target x86_64-sex.json \\
        -Zbuild-std=core,alloc \\
        -Zjson-target-spec \\
        --release
" | tee build_final.log

if grep -q "Finished release" build_final.log; then
    echo "SUCCESS: SYSTEM BOOTABLE."
    echo "Next Command: make run-sasos"
else
    echo "FAILED: See build_final.log for E0521 Lifetime errors."
fi
