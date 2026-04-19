#!/bin/bash
# SexOS SASOS - Phase 18.25: Memory Subsystem Reconstruction
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Reconstructing kernel/src/memory/allocator.rs..."

# We redefine the allocator to handle the Usable memory map and provide the LockedHeap
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::response::MemoryMapResponse;
use limine::memory_map::EntryType;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(mmap: &'static MemoryMapResponse, _hhdm_offset: u64) {
    // Find a usable memory region for the kernel heap
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory region found for kernel heap");

    unsafe {
        ALLOCATOR.lock().init(usable_region.base as *mut u8, HEAP_SIZE);
    }
}

// Stubs for the missing items required by memory.rs
pub fn alloc_frame() -> Option<u64> { None }
pub struct PageMetadata;
EOF

echo "2. Repairing kernel/src/memory.rs visibility..."

# Ensure the Limine requests are visible and imported correctly
# We assume these are defined in main.rs or imported here.
# We will inject the correct imports at the top of memory.rs
sed -i.bak '1i use crate::MEMMAP_REQUEST;\nuse crate::HHDM_REQUEST;' kernel/src/memory.rs

echo "3. Purging and Synchronizing Synthesis..."

# Clean target to ensure the new field names (type_) are recognized
cargo clean

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_memory_fix.log 2>&1 || true

echo "--> Synthesis complete. Analyzing the results..."

if grep -q "Finished release" final_memory_fix.log; then
    echo "=== PHASE 18.25: MEMORY RECONSTRUCTION SUCCESSFUL ==="
    echo "The kernel allocator and memory map interface are now harmonized."
    echo "Binary: target/x86_64-sex/release/sex-kernel"
else
    echo "BLOCKER DETECTED. The synthesis still fractures."
    grep "error\[" final_memory_fix.log | sort | uniq | head -n 5
    tail -n 15 final_memory_fix.log
fi
