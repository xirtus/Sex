#![no_std]
#![no_main]

use sex_orbclient::Window;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut win = Window::new(100, 100, 400, 300, "SexOS Egui").expect("Orbital window failed");
    
    loop {
        let pixels = win.data_mut();
        // Fill Red background
        for p in pixels.iter_mut() { *p = 0xFFFF0000; }
        
        // Sync to compositor
        win.sync();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
