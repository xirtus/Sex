#[no_mangle]
pub extern "C" fn sys_park() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("syscall", 
            in("rax") 24,
            lateout("rcx") _, lateout("r11") _,
        );
    }
}

pub fn park_on_ring() {
    // Standard FLSCHED park pattern: park until unparked by interrupt or message
    sys_park();
}
