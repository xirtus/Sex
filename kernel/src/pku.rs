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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PkruValue {
    /// God Mode: All PKEYs accessible (Access and Write).
    GodMode = 0x0000_0000,
    /// Secure Default: All PKEYs blocked except PKEY 0 (Kernel).
    SecureDefault = 0xFFFF_FFFC,
}

impl PkruValue {
    /// Computes the standard PKRU mask for a Protection Domain.
    /// Opens PKEY 0 (Kernel), PKEY 15 (IPC Shared), and the target pkey.
    pub fn for_domain(pkey: u8) -> u32 {
        let mut mask: u32 = 0xFFFF_FFFF;
        // Open PKEY 0 (Default/Kernel)
        mask &= !0b11;
        // Open PKEY 15 (Shared IPC)
        mask &= !(0b11 << 30);
        // Open target PKEY
        if pkey < 16 {
            mask &= !(0b11 << (pkey * 2));
        }
        mask
    }
}

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
    PkruValue::for_domain(key)
}

/// Manually walks the page tables to find the PKEY assigned to a virtual address.
pub fn get_pkey(va: u64) -> Option<u8> {
    use x86_64::registers::control::Cr3;
    let hhdm_offset = crate::HHDM_REQUEST.response()?.offset;
    let (pml4_phys, _) = Cr3::read();
    let pml4_virt = (pml4_phys.start_address().as_u64() + hhdm_offset) as *const u64;

    let pml4_idx = (va >> 39) & 0x1FF;
    let pdpt_idx = (va >> 30) & 0x1FF;
    let pd_idx   = (va >> 21) & 0x1FF;
    let pt_idx   = (va >> 12) & 0x1FF;

    unsafe {
        let pml4e = *pml4_virt.add(pml4_idx as usize);
        if pml4e & 1 == 0 { return None; }
        let pdpt_virt = ((pml4e & 0x000F_FFFF_FFFF_F000) + hhdm_offset) as *const u64;

        let pdpte = *pdpt_virt.add(pdpt_idx as usize);
        if pdpte & 1 == 0 { return None; }
        // Handle 1GB huge pages
        if pdpte & 0x80 != 0 {
            return Some(((pdpte >> 59) & 0xF) as u8);
        }
        let pd_virt = ((pdpte & 0x000F_FFFF_FFFF_F000) + hhdm_offset) as *const u64;

        let pde = *pd_virt.add(pd_idx as usize);
        if pde & 1 == 0 { return None; }
        // Handle 2MB huge pages
        if pde & 0x80 != 0 {
            return Some(((pde >> 59) & 0xF) as u8);
        }
        let pt_virt = ((pde & 0x000F_FFFF_FFFF_F000) + hhdm_offset) as *const u64;

        let pte = *pt_virt.add(pt_idx as usize);
        if pte & 1 == 0 { return None; }
        
        Some(((pte >> 59) & 0xF) as u8)
    }
}

/// The PKU Warden: reports and analyzes hardware security violations.
pub fn pku_warden(fault_addr: u64, rip: u64, error_code: u64) {
    let pkey = get_pkey(fault_addr).unwrap_or(0);
    let pkru = unsafe { rdpkru() };
    
    serial_println!("------------------------------------------------------------");
    serial_println!("🔥 HARDWARE SECURITY VIOLATION: PKU LOCK ENGAGED 🔥");
    serial_println!("FAULT ADDR: {:#x}", fault_addr);
    serial_println!("FAULT RIP:  {:#x}", rip);
    serial_println!("PKEY COLOR: {}", pkey);
    
    let ad = (pkru >> (pkey * 2)) & 1 != 0;
    let wd = (pkru >> (pkey * 2 + 1)) & 1 != 0;
    serial_println!("PKRU STATE: AD={} WD={} (PKRU: {:#010x})", ad, wd, pkru);

    let is_instr_fetch = (error_code & 0x10) != 0;
    let is_write = (error_code & 0x02) != 0;

    if is_instr_fetch {
        serial_println!("VIOLATION: Illegal Domain Execute (Bit 4 set)");
        serial_println!("CAUSE: Attempted to execute code in PKEY {} without permission.", pkey);
        serial_println!("       Ensure pdx_call was used for domain transition.");
    } else if is_write {
        serial_println!("VIOLATION: ReadOnly Capability Violation (Bit 1 set)");
        serial_println!("CAUSE: Attempted to write to PKEY {} which is ReadOnly in current context.", pkey);
    } else {
        serial_println!("VIOLATION: Access Denied (Read/Data)");
    }
    serial_println!("------------------------------------------------------------");
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
