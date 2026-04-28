#![no_std]
#![no_main]

// Zero-syscall visual probe for PD1 (sexdisplay)
// Goal: prove execution reaches PD1 and can write to framebuffer without crashing.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let fb_ptr = 0xffff8000fd000000 as *mut u32;
    let width = 1280usize;
    let height = 800usize;

    for y in 0..height {
        let color = if y < 50 {
            0x00FFFFFFu32 // White topbar
        } else {
            0x00FF00FFu32 // Purple background
        };
        for x in 0..width {
            unsafe {
                core::ptr::write_volatile(fb_ptr.add(y * width + x), color);
            }
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
