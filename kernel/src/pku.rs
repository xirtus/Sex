use x86_64::registers::control::{Cr4, Cr4Flags};
use crate::serial_println;

/// Checks if the CPU supports Memory Protection Keys (PKU).
pub fn is_pku_supported() -> bool {
    use raw_cpuid::CpuId;
    let cpuid = CpuId::new();
    if let Some(features) = cpuid.get_extended_feature_info() {
        features.has_pku()
    } else {
        false
    }
}

/// Enables Memory Protection Keys (PKU) in the CR4 register.
pub unsafe fn enable_pku() {
    let mut flags = Cr4::read();
    flags.insert(Cr4Flags::PROTECTION_KEY_USER);
    Cr4::write(flags);
    serial_println!("PKU: Protection Keys enabled in CR4.");
}

#[inline(always)]
pub unsafe fn rdpkru() -> u32 {
    let pkru: u32;
    core::arch::asm!(
        "xor ecx, ecx",
        "rdpkru",
        out("eax") pkru,
        out("edx") _,
        out("ecx") _,
        options(nomem, nostack, preserves_flags)
    );
    pkru
}

#[inline(always)]
pub unsafe fn wrpkru(mask: u32) {
    core::arch::asm!(
        "xor ecx, ecx",
        "xor edx, edx",
        "wrpkru",
        in("eax") mask,
        out("ecx") _,
        out("edx") _,
        options(nomem, nostack)
    );
}

/// A wrapper around the PKRU register.
pub struct Pkru;

impl Pkru {
    pub fn read() -> u32 { unsafe { rdpkru() } }
    pub unsafe fn write(value: u32) { wrpkru(value); }
    pub fn set_permissions(key: u8, access_disabled: bool, write_disabled: bool) {
        assert!(key < 16, "PKU key must be between 0 and 15");
        let mut pkru = Self::read();
        let shift = key * 2;
        pkru &= !(0b11 << shift);
        let mut bits = 0;
        if access_disabled { bits |= 0b01; }
        if write_disabled { bits |= 0b10; }
        pkru |= bits << shift;
        unsafe { Self::write(pkru); }
    }
}

pub fn init_pd_pkru(key: u8) -> u32 {
    let mut pkru = 0xFFFF_FFFF;
    let shift = key * 2;
    pkru &= !(0b11 << shift);
    let pkey15_shift = 15 * 2;
    pkru &= !(0b11 << pkey15_shift);
    pkru
}

pub fn multicast_revoke_key(_key: u8) {
    unsafe { crate::apic::send_ipi(0, 0x40, 0b11 << 18); }
    crate::hal::tlb_flush_local();
}

pub unsafe fn tag_virtual_address(va: u64, pkey: u8) {
    use x86_64::registers::control::Cr3;
    let hhdm_offset = crate::HHDM_REQUEST.response().unwrap().offset;
    let (pml4_phys, _) = Cr3::read();
    let pml4_virt = (pml4_phys.start_address().as_u64() + hhdm_offset) as *const u64;

    let pml4_idx = (va >> 39) & 0x1FF;
    let pdpt_idx = (va >> 30) & 0x1FF;
    let pd_idx   = (va >> 21) & 0x1FF;
    let pt_idx   = (va >> 12) & 0x1FF;

    let pdpt_entry = *pml4_virt.add(pml4_idx as usize);
    let pdpt_virt = ((pdpt_entry & 0xFFFF_FFFF_FFFF_F000) + hhdm_offset) as *const u64;

    let pd_entry = *pdpt_virt.add(pdpt_idx as usize);
    let pd_virt = ((pd_entry & 0xFFFF_FFFF_FFFF_F000) + hhdm_offset) as *const u64;

    let pt_entry = *pd_virt.add(pd_idx as usize);
    let pt_virt = ((pt_entry & 0xFFFF_FFFF_FFFF_F000) + hhdm_offset) as *mut u64;

    let mut pte = *pt_virt.add(pt_idx as usize);
    pte &= !(0xF << 59); 
    pte |= (pkey as u64 & 0xF) << 59; 
    *pt_virt.add(pt_idx as usize) = pte;

    core::arch::asm!("invlpg [{}]", in(reg) va, options(nostack, preserves_flags));
}

pub fn rdseed_u64() -> Option<u64> {
    let mut val: u64;
    let success: u8;
    unsafe {
        core::arch::asm!(
            "rdseed {0}",
            "setc {1}",
            out(reg) val,
            out(reg_byte) success,
        );
    }
    if success != 0 { Some(val) } else { None }
}
