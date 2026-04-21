#!/bin/bash
# 🛠️ SASOS Phase 20.5: Module Tree & Enforcement Fix
set -euo pipefail

echo "─── Step 1: Declaring Memory Submodules ───"
# This is the missing link that caused the 'could not find pku' error
cat > kernel/src/memory/mod.rs << 'MEM_MOD_EOF'
pub mod allocator;
pub mod pku;
MEM_MOD_EOF

echo "─── Step 2: Cleaning and Reconstructing lib.rs ───"
# We reconstruct lib.rs from scratch to ensure no duplicate sed-injected calls
cat > kernel/src/lib.rs << 'LIB_EOF'
#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod memory;

use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest};

#[used]
pub static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();
#[used]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();
#[used]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[allow(static_mut_refs)]
pub fn kernel_init() {
    // 1. Initialize Telemetry
    unsafe { arch::x86_64::serial::COM1.init(); }
    serial_println!("[SexOS] Substrate Phase 19 Hardened.");

    // 2. Claim Framebuffer and clear to Midnight Blue
    if let Some(fb_res) = FB_REQUEST.response() {
        if let Some(fb) = fb_res.framebuffers().first() {
            let ptr = fb.address() as *mut u32;
            serial_println!("[SexOS] Claiming FB: {}x{} at Midnight Blue", fb.width, fb.height);
            
            let size = (fb.width * fb.height) as isize;
            for i in 0..size {
                unsafe { *ptr.offset(i) = 0x191970; }
            }

            // 3. Trigger Hardware Enforcement (PKU Lockdown)
            serial_println!("[SexOS] Deploying Page Table Walker...");
            memory::pku::init_pku_isolation();
        }
    }
}
LIB_EOF

echo "─── Step 3: Atomic Synthesis ───"
export RUSTFLAGS="-C linker=lld"
export RUSTC_BOOTSTRAP=1

rustup run nightly cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z json-target-spec \
    -p sex-kernel \
    --release

echo "✅ PHASE 20.5 SYNTHESIS COMPLETE."
echo "Launch QEMU: qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku"
