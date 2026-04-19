#!/bin/bash
set -euo pipefail

echo "🧹 Cleaning Docker environment..."
docker rm -f $(docker ps -aq) || true

echo "🩹 Applying Universal Identifier Fix..."

# We are going to use the most common path for the modern crate.
# If 'memory_map' wasn't found, it's often because it needs to be accessed 
# via limine::MemoryMapEntryType or similar.
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
// Modern Limine crate structure:
use limine::memory_map::EntryType;
use crate::{MEMMAP_REQUEST, HHDM_REQUEST};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    let mmap = MEMMAP_REQUEST.response().expect("MMAP Fail");
    let hhdm = HHDM_REQUEST.response().expect("HHDM Fail");
    
    // Using EntryType::Usable and .type_
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= (HEAP_SIZE as u64))
        .expect("No usable memory region");

    let virt_addr = usable_region.base + hhdm.offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
}
EOF

echo "⚡ Starting Build..."

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
    
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "🚀 BOOTING..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "❌ Build failed. Dumping crate structure for analysis..."
    docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 \
        cargo pkgid -p limine
fi
