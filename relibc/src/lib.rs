#![no_std]
#![feature(alloc_error_handler)]
#![feature(c_variadic)]

pub mod platform;

#[no_mangle]
pub unsafe extern "C" fn printf(_fmt: *const u8, _: ...) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn _start() {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    loop {}
}
