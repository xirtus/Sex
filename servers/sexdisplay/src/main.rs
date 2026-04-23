#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_call, pdx_reply, serial_println, DisplayInfo, OP_SET_BG, OP_WINDOW_COMMIT, OP_WINDOW_CREATE};
use core::hint::spin_loop;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { spin_loop(); }
}

const OP_PAINT_BG: u64 = 0x100;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[RESTORE] sexdisplay: Starting Phase 5 Compositor...");

    let mut fb_info = DisplayInfo::default();
    unsafe {
        // Query Slot 0 for FB info
        if pdx_call(0, 0x03, &mut fb_info as *mut DisplayInfo as u64, 0, 0) != 0 {
            serial_println!("[ERROR] sexdisplay: Failed to get DisplayInfo");
            loop { spin_loop(); }
        }
    }

    let width = fb_info.width as usize;
    let height = fb_info.height as usize;
    let pitch = fb_info.pitch as usize;
    let stride = pitch / 4;
    let fb_ptr = fb_info.virt_addr as *mut u32;

    serial_println!("[DEBUG] sexdisplay: FB at {:#x}, {}x{}, pitch {}, stride {}", 
        fb_info.virt_addr, width, height, pitch, stride);

    // Core write_pixel function
    let write_pixel = |x: usize, y: usize, color: u32| {
        unsafe {
            fb_ptr.add(y * stride + x).write_volatile(color);
        }
    };

    let sovereign_buffer = 0x4000_0000 as *const u32;

    loop {
        let ev = unsafe { pdx_listen(5) };
        if ev.num == 0 {
            spin_loop();
            continue;
        }

        match ev.num {
            OP_WINDOW_CREATE => {
                serial_println!("[DEBUG] sexdisplay: OP_WINDOW_CREATE from PD {}", ev.caller_pd);
                unsafe {
                    pdx_reply(ev.caller_pd, 0x4000_0000);
                }
            }
            OP_PAINT_BG => {
                serial_println!("[DEBUG] sexdisplay: OP_PAINT_BG -> Cosmic Dark Grey");
                for y in 0..height {
                    for x in 0..width {
                        write_pixel(x, y, 0xFF1E1E2E);
                    }
                }
                unsafe { pdx_reply(ev.caller_pd, 1); }
            }
            OP_SET_BG => {
                let color = ev.arg0 as u32;
                serial_println!("[DEBUG] sexdisplay: SET_BG to {:#x}", color);
                for y in 0..height {
                    for x in 0..width {
                        write_pixel(x, y, color);
                    }
                }
                unsafe { pdx_reply(ev.caller_pd, 1); }
            }
            OP_WINDOW_COMMIT => {
                serial_println!("[DEBUG] sexdisplay: Blitting from Sovereign Buffer...");
                for y in 0..32 {
                    for x in 0..width {
                        if y < height {
                            unsafe {
                                let color = sovereign_buffer.add(y * stride + x).read_volatile();
                                write_pixel(x, y, color);
                            }
                        }
                    }
                }
                unsafe { pdx_reply(ev.caller_pd, 1); }
            }
            _ => {
                serial_println!("[DEBUG] sexdisplay: Unknown opcode {:#x}", ev.num);
            }
        }
    }
}
