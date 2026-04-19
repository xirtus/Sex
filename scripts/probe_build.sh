#!/bin/bash
set -euo pipefail

echo "🧹 Clearing Docker artifacts and zombie containers..."
docker rm -f $(docker ps -aq) || true

echo "🔬 Adjusting Limine imports to Root Level..."

# Attempting the flattened namespace: limine::memory_map -> limine::MemoryMapEntryType 
# or limine::EntryType. We will try the most common modern flattened path.
cat << 'EOF' > kernel/src/memory/allocator.rs
use linked_list_allocator::LockedHeap;
// Probing root-level or nested type
use limine::MemmapEntryType as EntryType; 
use crate::{MEMMAP_REQUEST, HHDM_REQUEST};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    let mmap = MEMMAP_REQUEST.response().expect("MMAP Fail");
    let hhdm = HHDM_REQUEST.response().expect("HHDM Fail");
    
    let usable_region = mmap.entries().iter()
        .find(|e| e.type_ == EntryType::Usable && e.length >= (HEAP_SIZE as u64))
        .expect("No usable memory region");

    let virt_addr = usable_region.base + hhdm.offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
}
EOF

echo "⚡ Executing Probe Build..."

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
    echo "💎 SUCCESS: Artifact Extracted."
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
    echo "❌ Build failed. Probing crate metadata..."
    # This command will help us find the exact name of the enum
    docker run --rm --platform linux/amd64 -v "$(pwd)":/src -w /src sexos-builder:v28 \
        bash -c "cargo doc -p limine --no-deps && find target/doc/limine -name 'enum.*.html'"
fi
