#[no_mangle]
pub extern "C" fn sys_park() {
    #[cfg(target_arch = "x86_64")] #[cfg(target_arch = "x86_64")] unsafe {
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("syscall", in("rax") 24);
    }
}

pub fn park_on_ring() {
    // Standard FLSCHED park pattern: park until unparked by interrupt or message
    sys_park();
}
