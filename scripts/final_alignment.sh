#!/bin/bash
set -euo pipefail

echo "🧹 Clearing Docker environment..."
docker rm -f $(docker ps -aq) || true

echo "🩹 Reconciling Library and Binary Entry Points..."

# 1. Update lib.rs (The Core Library)
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
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}
EOF

# 2. Update main.rs (The Entry Point Trampoline)
# We use the library name (sex_kernel) to bridge the module gap
cat << 'EOF' > kernel/src/main.rs
#![no_std]
#![no_main]

use sex_kernel::init;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Jump into the library initialization
    init::kernel_init();

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

echo "⚡ Executing Sovereign Build..."

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
    
    # Generate fresh Limine config
    cat << 'CFG' > iso_root/boot/limine.cfg
TIMEOUT=0
:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/sex-kernel
CFG

    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-sasos.iso
    
    echo "🚀 BOOTING..."
    qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku
else
    echo "❌ Build failed. Check the pathing in main.rs vs lib.rs."
fi
