#![no_std]
#![no_main]

mod vfs;
mod messages;
mod pdx;
mod trampoline;

use crate::trampoline::trampoline_main;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexfiles: Advanced PDX Zero-Copy VFS Server
    // Phase 19: Handover Trampoline Architecture
    trampoline_main();
    
    // Safety: trampoline_main is an infinite loop
    loop {
        libsys::sched::park_on_ring();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { libsys::sched::park_on_ring(); }
}
