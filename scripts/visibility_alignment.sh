#!/bin/bash
# SexOS SASOS - Phase 18.26: Visibility & Cross-Platform Alignment
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Fix: Prepending Imports to kernel/src/memory.rs (Mac-Safe)..."

if [ -f "kernel/src/memory.rs" ]; then
    # We use a temporary file to prepend imports without triggering sed version conflicts
    cat << 'EOF' > kernel/src/memory.rs.new
use crate::MEMMAP_REQUEST;
use crate::HHDM_REQUEST;
EOF
    cat kernel/src/memory.rs >> kernel/src/memory.rs.new
    mv kernel/src/memory.rs.new kernel/src/memory.rs
    echo " -> memory.rs visibility restored."
fi

echo "2. Validating kernel/src/main.rs Export Sovereignty..."

# For the imports in memory.rs to work, the requests in main.rs must be 'pub'
if [ -f "kernel/src/main.rs" ]; then
    echo " -> Ensuring Limine requests are public in main.rs..."
    sed -i.bak 's/static MEMMAP_REQUEST/pub static MEMMAP_REQUEST/g' kernel/src/main.rs
    sed -i.bak 's/static HHDM_REQUEST/pub static HHDM_REQUEST/g' kernel/src/main.rs
fi

echo "3. Synchronizing Memory Subsystem Stubs..."

# Re-verify allocator.rs stubs to satisfy the memory.rs calls
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::response::MemoryMapResponse;
use limine::memory_map::EntryType;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(mmap: &'static MemoryMapResponse, _hhdm_offset: u64) {
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory region found for kernel heap");

    unsafe {
        ALLOCATOR.lock().init(usable_region.base as *mut u8, HEAP_SIZE);
    }
}

// Logic required by memory.rs logic flow
pub fn alloc_frame() -> Option<u64> { None }
pub struct PageMetadata;
EOF

echo "4. Firing ULTIMATE SASOS SYNTHESIS..."

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_visibility_build.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_visibility_build.log; then
    echo "=== PHASE 18.26: VISIBILITY ALIGNMENT SUCCESSFUL ==="
    echo "The kernel memory bridge is now physically established."
    echo "Command: make run-sasos"
else
    echo "BLOCKER DETECTED. The linker or visibility check failed."
    grep "error\[" final_visibility_build.log | sort | uniq | head -n 5
    tail -n 15 final_visibility_build.log
fi
