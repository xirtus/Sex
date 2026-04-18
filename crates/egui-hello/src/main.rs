#![no_std]
#![no_main]

use sex_orbclient::Window;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut win = Window::new(100, 100, 400, 300, "SexOS Egui").expect("Orbital window failed");
    let mut bg_color = 0xFFFF0000; // Red
    
    loop {
        // Poll events
        while let Some(ev) = win.events() {
            match ev {
                sex_pdx::OrbitalEvent::Key { code, pressed } => {
                    if pressed {
                        // Change color on key press
                        bg_color = 0xFF00FF00; // Green
                    } else {
                        bg_color = 0xFFFF0000; // Red
                    }
                }
                sex_pdx::OrbitalEvent::Mouse { x, y } => {
                    // Blue flash on movement (simple feedback)
                    if x % 10 == 0 {
                        bg_color = 0xFF0000FF; // Blue
                    }
                }
                sex_pdx::OrbitalEvent::Quit => {
                    // Shutdown or just spin
                }
                _ => {}
            }
        }

        let pixels = win.data_mut();
        // Fill background
        for p in pixels.iter_mut() { *p = bg_color; }
        
        // Sync to compositor
        win.sync();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
