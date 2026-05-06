#![no_std]
#![no_main]

extern crate alloc;
extern crate spin;
extern crate sex_rt;

mod vfs;
mod messages;
mod pdx;
mod trampoline;
mod backends;
mod proof;

use crate::trampoline::trampoline_main;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexfiles: RamFS Contract Lock V1
    // Bounded RAM-backed filesystem with no POSIX semantics.
    sex_rt::heap_init();
    trampoline_main();

    // Fallback: trampoline_main should never return
    loop {
        unsafe {
            core::arch::asm!("syscall", in("rax") 24, lateout("rcx") _, lateout("r11") _);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("syscall", in("rax") 24, lateout("rcx") _, lateout("r11") _);
        }
    }
}
