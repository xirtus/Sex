#![no_std]
#![no_main]

use sex_pdx::{pdx_call, pdx_listen, serial_println, SLOT_DISPLAY, OP_SET_BG, OP_WINDOW_COMMIT, OP_WINDOW_CREATE, DisplayInfo, get_keystroke, sched_yield};
use core::ptr::write_volatile;
use core::hint::spin_loop;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { spin_loop(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    for _ in 0..2_000_000 { spin_loop(); }

    serial_println!("[RESTORE] silk-shell: Starting Phase 7 UI Rendering...");

    unsafe {
        let mut fb_info = DisplayInfo::default();
        pdx_call(0, 0x03, &mut fb_info as *mut DisplayInfo as u64, 0, 0);
        let screen_w = if fb_info.width > 0 { fb_info.width as u64 } else { 1920 };

        serial_println!("[DEBUG] silk-shell: OP_SET_BG -> Dark Grey");
        pdx_call(SLOT_DISPLAY, OP_SET_BG, 0xFFA9A9A9, 0, 0);
        loop {
            let ev = pdx_listen(SLOT_DISPLAY);
            if ev.arg0 == 1 || ev.num == 1 {
                break;
            }
            spin_loop();
        }

        serial_println!("[DEBUG] silk-shell: Allocating Silkbar (0, 0, {}, 32)", screen_w);
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, 0, 0, (screen_w << 16) | 32);

        let mut canvas_ptr: u64 = 0;
        loop {
            let ev = pdx_listen(SLOT_DISPLAY);
            if ev.arg0 == 0x4000_0000 {
                canvas_ptr = ev.arg0;
                break;
            }
            spin_loop();
        }
        
        let silkbar_canvas = canvas_ptr as *mut u32;
        let silkbar_pixels = (screen_w * 32) as usize;
        for i in 0..silkbar_pixels {
            write_volatile(silkbar_canvas.add(i), 0xFF000000); // Pure Black
        }
        pdx_call(SLOT_DISPLAY, OP_WINDOW_COMMIT, 0, 0, 0);

        serial_println!("[DEBUG] silk-shell: Allocating Mondrian Window (100, 100, 400, 300)");
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, 100, 100, (400 << 16) | 300);

        canvas_ptr = 0;
        loop {
            let ev = pdx_listen(SLOT_DISPLAY);
            if ev.arg0 == 0x4000_0000 {
                canvas_ptr = ev.arg0;
                break;
            }
            spin_loop();
        }

        let mondrian_canvas = canvas_ptr as *mut u32;
        let mondrian_pixels = 400 * 300;
        for i in 0..mondrian_pixels {
            write_volatile(mondrian_canvas.add(i), 0xFFFF69B4); // Hot Pink
        }
        pdx_call(SLOT_DISPLAY, OP_WINDOW_COMMIT, 0, 0, 0);

        serial_println!("[DEBUG] silk-shell: UI Rendering Complete.");
    }

    loop {
        if let Some(key) = get_keystroke() {
            serial_println!("Keystroke: {}", key as char);
        } else {
            let ev = unsafe { pdx_listen(SLOT_DISPLAY) };
            if ev.num == 0 {
                sched_yield();
            }
        }
    }
}