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
mod cache;

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
