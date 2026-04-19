#!/bin/bash
set -euo pipefail

echo "🩹 Patching source with Compiler-Suggested Identifiers..."

# 1. Update lib.rs with MemmapRequest (the 'e' and 'ory' are gone)
cat << 'EOF' > kernel/src/lib.rs
#![no_std]
#![feature(alloc_error_handler)]

pub mod memory;
pub mod init;

use limine::request::{MemmapRequest, HhdmRequest};

#[used]
#[link_section = ".limine_reqs"]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section = ".limine_reqs"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}
EOF

# 2. Update allocator.rs with .response() and Usable
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
// The compiler says memory_map is missing; we'll pull EntryType from where it usually lives in the new API
use limine::response::MemoryMapEntryType;
use crate::{MEMMAP_REQUEST, HHDM_REQUEST};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    // API FIX: .get_response() -> .response()
    let mmap = MEMMAP_REQUEST.response().expect("MMAP Fail");
    let hhdm = HHDM_REQUEST.response().expect("HHDM Fail");
    
    let usable_region = mmap.entries().iter()
        .find(|e| e.typ == MemoryMapEntryType::USABLE && e.length >= (HEAP_SIZE as u64))
        .expect("No usable memory region");

    let virt_addr = usable_region.base + hhdm.offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
}
EOF

echo "⚡ Re-triggering Build & Extraction..."

docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    cargo build \
        --target x86_64-sex.json \
        -Z json-target-spec \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -p sex-kernel \
        --release > /dev/stderr 2>&1 && \
    find target/x86_64-sex/release/ -maxdepth 1 -type f ! -name '*.d' ! -name '*.rlib' -name 'sex*' -exec cat {} + " > ./sex-kernel.elf

if [ -s "./sex-kernel.elf" ]; then
    echo "💎 Artifact Extracted. Minting ISO..."
    mkdir -p iso_root/boot/limine
    cp ./sex-kernel.elf iso_root/boot/sex-kernel
    
    # Final check on xorriso before launch
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "🚀 Booting Final Precision Build..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "❌ Build still failing. Check stderr logs."
fi
