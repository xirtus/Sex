#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In Phase 19, this will be replaced by a PDX call to the kernel 
    // to map the shared heap region.
    loop {}
}

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
