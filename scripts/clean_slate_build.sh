#!/bin/bash
set -euo pipefail

echo "🧹 Nuking redundant Docker containers..."
docker rm -f $(docker ps -aq) || true

echo "🩹 Applying Final Crate Path Fix..."

# The compiler can't find limine::memory_map. 
# In recent versions, EntryType is often moved to limine::request or limine::response.
# We will use the most common stable path: limine::response::MemoryMapEntryType
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
use limine::response::MemoryMapEntryType;
use crate::{MEMMAP_REQUEST, HHDM_REQUEST};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    let mmap = MEMMAP_REQUEST.response().expect("MMAP Fail");
    let hhdm = HHDM_REQUEST.response().expect("HHDM Fail");
    
    // Using EntryType::Usable (PascalCase) and .type_ (underscored)
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == MemoryMapEntryType::USABLE && e.length >= (HEAP_SIZE as u64))
        .expect("No usable memory region");

    let virt_addr = usable_region.base + hhdm.offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
}
EOF

echo "⚡ Launching Sovereign Build..."

# We use ONE container with explicit cleanup (--rm)
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
    
    # Synthesis
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "🚀 BOOTING..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "❌ Build failed. If it's another import error, the crate version is likely 0.x.x (Legacy)."
fi
