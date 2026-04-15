#[no_mangle]
pub extern "C" fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

pub fn park_on_ring() {
    // Standard FLSCHED park pattern: park until unparked by interrupt or message
    sys_park();
}
