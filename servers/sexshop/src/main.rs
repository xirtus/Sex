#![no_std]
#![no_main]

extern crate alloc;
extern crate spin;
extern crate sex_rt;

mod pdx;
mod trampoline;
mod storage;
mod cache;
mod transactions;

use crate::trampoline::trampoline_main;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // sexshop: Phase 20 Pure-Rust PDX Server
    // Replacement for legacy sex-store
    trampoline_main();
    
    loop {
        libsys::sched::park_on_ring();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { libsys::sched::park_on_ring(); }
}
