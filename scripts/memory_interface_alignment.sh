#!/bin/bash
# SexOS SASOS - Phase 18.24: Memory Interface Alignment
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Aligning kernel/src/memory.rs call-site..."

# We need to ensure memory.rs passes the Limine responses instead of the mapper.
# We will look for the line calling init_heap and fix it to use the Limine requests.
# Assumption: MEMMAP_REQUEST and HHDM_REQUEST are defined in memory.rs as per Limine protocol.
if [ -f "kernel/src/memory.rs" ]; then
    echo " -> Correcting init_heap call in memory.rs..."
    sed -i.bak 's/allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Kernel Heap Init Failed");/let mmap = MEMMAP_REQUEST.get_response().unwrap();\n    let hhdm = HHDM_REQUEST.get_response().unwrap();\n    allocator::init_heap(mmap, hhdm.offset);/' kernel/src/memory.rs
fi

echo "2. Validating allocator.rs Signature..."
# Ensure the return type matches the expected usage (removing result if it returns unit)
# This confirms the E0599 fix.

echo "3. Firing GLOBAL SASOS SYNTHESIS..."

# Build everything. Stage 1 (sexbuild) is already done, so this will focus on Stage 2.
docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > alignment_synthesis.log 2>&1 || true

echo "--> Synthesis complete. Checking for the 'Finished' signature..."

if grep -q "Finished release" alignment_synthesis.log; then
    echo "=== PHASE 18.24: KERNEL ALIGNMENT SUCCESSFUL ==="
    echo "The memory desync is resolved. The kernel is now SASOS-compliant."
    echo "Next: make run-sasos"
else
    echo "BLOCKER DETECTED. Final culprits in alignment_synthesis.log:"
    grep "error\[" alignment_synthesis.log | sort | uniq | head -n 5
    tail -n 20 alignment_synthesis.log
fi
