#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use sex_rt; // Corrected syntax: underscores only

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _greeting = String::from("Hello from the Orbital Userland!");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
