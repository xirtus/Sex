#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

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
    sex_rt::heap_init();
    trampoline_main();
    
    // Safety: trampoline_main is an infinite loop
    loop {
        unsafe {
            core::arch::asm!("syscall", in("rax") 24, lateout("rcx") _, lateout("r11") _); // sys_park
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("syscall", in("rax") 24, lateout("rcx") _, lateout("r11") _); // sys_park
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}
