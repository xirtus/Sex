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
