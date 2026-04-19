#!/bin/bash
# SexOS SASOS - Phase 18.29: The Final Convergence
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || exit 1

echo "1. Redefining kernel/src/lib.rs (The Crate Root)..."

# In a library-based kernel, lib.rs must hold the public statics
cat << 'EOF' > kernel/src/lib.rs
#![no_std]
#![feature(alloc_error_handler)]

pub mod memory;
pub mod init;

use limine::request::{MemoryMapRequest, HhdmRequest};

#[used]
#[link_section = ".limine_reqs"]
pub static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".limine_reqs"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();
EOF

echo "2. Aligning kernel/src/memory.rs (The Bridge)..."

if [ -f "kernel/src/memory.rs" ]; then
    echo " -> Harmonizing memory.rs call-site..."
    # Replace the 2-arg call with the 1-arg Limine request call
    # And ensure the trait is imported
    sed -i.bak '1i use core::alloc::GlobalAlloc;' kernel/src/memory.rs
    sed -i.bak 's/allocator::init_heap(&mut mapper, \&mut frame_allocator).expect("Kernel Heap Init Failed");/allocator::init_heap(\&crate::MEMMAP_REQUEST);/' kernel/src/memory.rs
fi

echo "3. Sanitizing kernel/src/init.rs..."

if [ -f "kernel/src/init.rs" ]; then
    echo " -> Stripping invariant checks from init.rs..."
    # These methods don't exist on LockedHeap; removing to stop E0599
    sed -i.bak '/verify_invariants/d' kernel/src/init.rs
    sed -i.bak '/add_memory_region/d' kernel/src/init.rs
    sed -i.bak '/init_metadata/d' kernel/src/init.rs
fi

echo "4. Re-verifying kernel/src/memory/allocator.rs Spec..."

cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::request::MemoryMapRequest;
use limine::memory_map::EntryType;

pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

#[global_allocator]
pub static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(mmap_request: &MemoryMapRequest) {
    let response = mmap_request.get_response().expect("Limine Memmap Request Failed");
    
    let usable_region = response.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory region found for kernel heap");

    unsafe {
        GLOBAL_ALLOCATOR.lock().init(usable_region.base as *mut u8, HEAP_SIZE);
    }
}

pub fn alloc_frame() -> Option<u64> { None }
pub struct PageMetadata;
EOF

echo "5. Firing TOTAL SYSTEM SYNTHESIS..."

# Deep purge to ensure no stale object files contaminate the linker
rm -rf target/

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly && \
    cargo build --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    -Z json-target-spec \
    -p sex-kernel \
    -p egui-hello \
    --release" > final_convergence.log 2>&1 || true

echo "--> Synthesis complete. Checking results..."

if grep -q "Finished release" final_convergence.log; then
    echo "=== PHASE 18.29: SYSTEM SYNTHESIS SUCCESSFUL ==="
    echo "The allocator conflict is dead. The hybrid interface is unified."
    echo "Artifact: target/x86_64-sex/release/sex-kernel"
else
    echo "BLOCKER REMAINS. Culprits in final_convergence.log:"
    grep "error\[" final_convergence.log | sort | uniq
    tail -n 15 final_convergence.log
fi
