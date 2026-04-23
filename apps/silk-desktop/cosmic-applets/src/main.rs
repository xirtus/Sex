#![no_std]
#![no_main]

extern crate alloc;
use sex_pdx::{serial_println, pdx_call, PDX_SEX_WINDOW_CREATE, SexWindowCreateParams};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("cosmic-applets: silk-shell desktop surfaces initialized.");

    // Initialize surface using the exact struct we defined in sex-pdx
    let params = SexWindowCreateParams {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
        title: b"Silk Desktop Applet\0",
    };

    // Dispatch to sexdisplay (Compositor is bound to PDX Slot 5) 
    unsafe {
        pdx_call(5, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0, 0);
    }

    // Enter lock-free parking loop
    loop {
        core::hint::spin_loop();
    }
}
