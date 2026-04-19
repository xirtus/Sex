#!/bin/bash
# SexOS SASOS - Phase 18.32: Final Stabilization
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Harmonizing the Module Hierarchy (kernel/src/memory.rs)..."

# Declare the allocator sub-module so it exists in the crate's namespace
cat << 'EOF' > kernel/src/memory.rs
use crate::MEMMAP_REQUEST;

// Establishing the hierarchy
pub mod allocator;

pub fn init() {
    // Calling into our recognized sub-module
    allocator::init_heap(&MEMMAP_REQUEST);
}

pub fn alloc_frame() -> Option<u64> { None }
EOF

echo "2. Correcting Limine API Naming (kernel/src/memory/allocator.rs)..."

# Switching MemoryMapRequest to MemmapRequest
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::request::MemmapRequest;
use limine::memory_map::EntryType;

pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

#[global_allocator]
pub static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(mmap_request: &MemmapRequest) {
    let response = mmap_request.get_response().expect("Limine Memmap Request Failed");
    
    let usable_region = response.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory region found for kernel heap");

    unsafe {
        GLOBAL_ALLOCATOR.lock().init(usable_region.base as *mut u8, HEAP_SIZE);
    }
}
EOF

echo "3. Cleaning Init Logic (kernel/src/init.rs)..."

# Remove the redundant #![no_std] attribute
cat << 'EOF' > kernel/src/init.rs
use crate::memory;

pub fn kernel_init() {
    memory::init();
}
EOF

echo "4. Firing THE FINAL SYNTHESIS..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_stabilization.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_stabilization.log; then
    echo "=== PHASE 18.32: STABILIZATION SUCCESSFUL ==="
    echo "The kernel is officially 100% synthesized."
    echo "Next Command: ./scripts/artifact_validation_and_iso.sh"
else
    echo "BLOCKER DETECTED. Final log analysis:"
    grep "error\[" final_stabilization.log | sort | uniq
    tail -n 15 final_stabilization.log
fi
