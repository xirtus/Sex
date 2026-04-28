#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let fb = 0xffff8000fd000000 as *mut u32;
    let w = 1280usize;
    let h = 800usize;

    for y in 0..h {
        for x in 0..w {
            let c: u32;
            if y < 50 {
                if x >= 10 && x < 90 && y >= 10 && y < 40 {
                    c = 0x0000FF00; // green launcher icon
                } else if x >= 1040 && x < 1100 && y >= 10 && y < 40 {
                    c = 0x00FF0000; // red status icon
                } else if x >= 1110 && x < 1170 && y >= 10 && y < 40 {
                    c = 0x000000FF; // blue status icon
                } else if x >= 1180 && x < 1240 && y >= 10 && y < 40 {
                    c = 0x00000000; // black status icon
                } else {
                    c = 0x00FFFFFF; // white bar
                }
            } else if y < 52 {
                c = 0x00333333; // border line
            } else {
                c = 0x00FF00FF; // purple background
            }
            unsafe { core::ptr::write_volatile(fb.add(y * w + x), c); }
        }
    }

    loop { core::hint::spin_loop(); }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
