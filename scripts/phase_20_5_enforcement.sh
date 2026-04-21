#!/bin/bash
# 🛠️ SASOS Phase 20.5: Hardware Enforcement (JSON Spec Fix)
set -euo pipefail

echo "🚀 Phase 20.5: Transitioning to Hardware Enforcement."

# 1. The PKU Guard (kernel/src/memory/pku.rs)
cat > kernel/src/memory/pku.rs << 'PKU_EOF'
#![no_std]
use core::arch::asm;
use crate::{FB_REQUEST, HHDM_REQUEST};

static mut HHDM_OFFSET: u64 = 0;

fn init_hhdm_offset() {
    unsafe {
        if let Some(resp) = HHDM_REQUEST.response() {
            HHDM_OFFSET = resp.offset();
        }
    }
}

fn get_cr3() -> u64 {
    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3) };
    cr3 & 0xFFFF_FFFF_FFFF_F000
}

pub unsafe fn tag_virtual_address(va: u64, pkey: u8) {
    init_hhdm_offset();
    let pml4_phys = get_cr3();
    let pml4_virt = (pml4_phys + HHDM_OFFSET) as *const u64;

    let pml4_idx = (va >> 39) & 0x1FF;
    let pdpt_idx = (va >> 30) & 0x1FF;
    let pd_idx   = (va >> 21) & 0x1FF;
    let pt_idx   = (va >> 12) & 0x1FF;

    let pdpt_entry = *pml4_virt.add(pml4_idx as usize);
    let pdpt_virt = ((pdpt_entry & 0xFFFF_FFFF_FFFF_F000) + HHDM_OFFSET) as *const u64;

    let pd_entry = *pdpt_virt.add(pdpt_idx as usize);
    let pd_virt = ((pd_entry & 0xFFFF_FFFF_FFFF_F000) + HHDM_OFFSET) as *const u64;

    let pt_entry = *pd_virt.add(pd_idx as usize);
    let pt_virt = ((pt_entry & 0xFFFF_FFFF_FFFF_F000) + HHDM_OFFSET) as *mut u64;

    let mut pte = *pt_virt.add(pt_idx as usize);
    pte &= !(0xF << 59); // Clear PKEY bits
    pte |= ((pkey as u64 & 0xF) << 59); // Set new PKEY
    *pt_virt.add(pt_idx as usize) = pte;

    asm!("invlpg [{}]", in(reg) va, options(nostack, preserves_flags));
}

pub unsafe fn wrpkru(value: u32) {
    asm!("xor ecx, ecx", "xor edx, edx", "wrpkru", in("eax") value);
}

pub fn init_pku_isolation() {
    init_hhdm_offset();
    if let Some(fb_res) = FB_REQUEST.response() {
        if let Some(fb) = fb_res.framebuffers().first() {
            let fb_virt = fb.address() as u64;
            unsafe { tag_virtual_address(fb_virt, 1); }
            crate::serial_println!("[SexOS] Framebuffer PTE tagged with PKEY 1");
        }
    }
    // Revoke kernel write access to PKEY 1
    unsafe { wrpkru(0b1100); } // AD and WD for PKEY 1
    crate::serial_println!("[SexOS] Kernel write access revoked via WRPKRU.");
}
PKU_EOF

# 2. Update lib.rs to call the isolation sequence
sed -i '' 's/unsafe { \*ptr.offset(i) = 0x191970; }/unsafe { *ptr.offset(i) = 0x191970; }\n    memory::pku::init_pku_isolation();/' kernel/src/lib.rs || \
sed -i 's/unsafe { \*ptr.offset(i) = 0x191970; }/unsafe { *ptr.offset(i) = 0x191970; }\n    memory::pku::init_pku_isolation();/' kernel/src/lib.rs

# 3. Final Synthesis (Corrected Build Command)
echo "🔨 Synthesis via macOS Cross-Compile Protocol..."
export RUSTFLAGS="-C linker=lld"
export RUSTC_BOOTSTRAP=1

rustup run nightly cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z json-target-spec \
    -p sex-kernel \
    --release

echo "✅ Phase 20.5: HARDWARE ENFORCEMENT LIVE."
echo "Next Step: Run ./scripts/final_payload.sh and boot with +pku"
