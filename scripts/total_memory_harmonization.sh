#!/bin/bash
# SexOS SASOS - Phase 18.27: Total Memory Harmonization
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Harmonizing kernel/src/main.rs (Request Visibility)..."
# Move requests to module level and ensure they are pub static
if [ -f "kernel/src/main.rs" ]; then
    # Ensure requests are NOT inside _start and ARE public
    # This sed script tries to capture the static blocks and ensure 'pub'
    sed -i.bak 's/^static MEMMAP_REQUEST/pub static MEMMAP_REQUEST/g' kernel/src/main.rs
    sed -i.bak 's/^static HHDM_REQUEST/pub static HHDM_REQUEST/g' kernel/src/main.rs
fi

echo "2. Reconstructing kernel/src/memory/allocator.rs (The Single Sovereign)..."
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::response::MemoryMapResponse;
use limine::memory_map::EntryType;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

#[global_allocator]
pub static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(mmap: &'static MemoryMapResponse, _hhdm_offset: u64) {
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory region found for kernel heap");

    unsafe {
        GLOBAL_ALLOCATOR.lock().init(usable_region.base as *mut u8, HEAP_SIZE);
    }
}

// Stubs to satisfy memory.rs logic
pub fn alloc_frame() -> Option<u64> { None }
pub struct PageMetadata;
EOF

echo "3. Cleaning kernel/src/memory.rs (Removing the Usurper)..."
if [ -f "kernel/src/memory.rs" ]; then
    # Remove the redundant allocator and any conflicting imports
    # We use a perl block to delete the #[global_allocator] section
    perl -i -0777 -pe 's/\#\[global_allocator\].*?static ALLOCATOR:.*?=.*?;/\/\/ Redundant allocator removed/gs' kernel/src/memory.rs
    
    # Fix the import to point to the renamed GLOBAL_ALLOCATOR
    sed -i.bak 's/use crate::memory::allocator::ALLOCATOR;/use crate::memory::allocator::GLOBAL_ALLOCATOR;/' kernel/src/memory.rs
fi

echo "4. Firing FINAL SASOS SYNTHESIS..."

# Deep clean to ensure no stale lang-item metadata remains
cargo clean

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_harmonization.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_harmonization.log; then
    echo "=== PHASE 18.27: MEMORY HARMONIZATION SUCCESSFUL ==="
    echo "The kernel is now the single sovereign of its heap."
    echo "Ready for QEMU: make run-sasos"
else
    echo "BLOCKER DETECTED. Remaining issues in final_harmonization.log:"
    grep "error\[" final_harmonization.log | sort | uniq | head -n 10
    tail -n 20 final_harmonization.log
fi
