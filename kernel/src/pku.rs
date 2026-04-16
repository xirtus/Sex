use x86_64::registers::control::{Cr4, Cr4Flags};
use crate::serial_println;

/// Checks if the CPU supports Memory Protection Keys (PKU).
pub fn is_pku_supported() -> bool {
    use raw_cpuid::CpuId;
    let cpuid = CpuId::new();
    
    // PKU is in Leaf 7, Subleaf 0, EBX bit 3
    if let Some(features) = cpuid.get_extended_feature_info() {
        features.has_pku()
    } else {
        false
    }
}

/// Enables Memory Protection Keys (PKU) in the CR4 register.
/// 
/// # SAFETY
/// This function is unsafe because it modifies a critical control register.
/// PKU support must be verified with `is_pku_supported()` before calling this.
pub unsafe fn enable_pku() {
    let mut flags = Cr4::read();
    flags.insert(Cr4Flags::PROTECTION_KEY_USER);
    Cr4::write(flags);
    serial_println!("PKU: Protection Keys enabled in CR4.");
}

/// A wrapper around the PKRU register.
pub struct Pkru;

impl Pkru {
    /// Reads the current value of the PKRU register.
    pub fn read() -> u32 {
        let pkru: u32;
        unsafe {
            core::arch::asm!(
                "rdpkru",
                out("eax") pkru,
                out("edx") _, // edx is ignored
                in("ecx") 0,  // ecx must be 0
            );
        }
        pkru
    }

    /// Writes a new value to the PKRU register.
    /// 
    /// # SAFETY
    /// Modifying PKRU directly affects memory access permissions for the current thread.
    pub unsafe fn write(value: u32) {
        core::arch::asm!(
            "wrpkru",
            in("eax") value,
            in("edx") 0, // edx must be 0
            in("ecx") 0, // ecx must be 0
        );
    }

    /// Sets the permissions for a specific key.
    /// `access_disabled`: if true, prevents all access.
    /// `write_disabled`: if true, prevents write access.
    pub fn set_permissions(key: u8, access_disabled: bool, write_disabled: bool) {
        assert!(key < 16, "PKU key must be between 0 and 15");
        let mut pkru = Self::read();
        let shift = key * 2;
        
        // Clear existing bits for this key
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
    pkru
}
