use core::arch::asm;

pub struct Pkru;

impl Pkru {
    /// Reads the current PKRU register value.
    #[inline]
    pub fn read() -> u32 {
        let pkru: u32;
        unsafe {
            asm!(
                "rdpkru",
                out("eax") pkru,
                in("ecx") 0,
                out("edx") _,
            );
        }
        pkru
    }

    /// Writes a new value to the PKRU register.
    #[inline]
    pub fn write(val: u32) {
        unsafe {
            asm!(
                "wrpkru",
                in("eax") val,
                in("ecx") 0,
                in("edx") 0,
            );
        }
    }
}

/// Helpers for safe domain entry/exit.
pub fn safe_pku_enter(key: u8, disable_access: bool, disable_write: bool) {
    let mut current = Pkru::read();
    let shift = key * 2;
    current &= !(0b11 << shift);
    if disable_access { current |= 0b01 << shift; }
    if disable_write { current |= 0b10 << shift; }
    Pkru::write(current);
}

pub fn safe_pku_exit(key: u8) {
    safe_pku_enter(key, true, true);
}
