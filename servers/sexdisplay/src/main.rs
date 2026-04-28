#![no_std]
#![no_main]

const W: usize = 1280;
const H: usize = 800;

fn bg(y: usize) -> u32 {
    if y < 200      { 0x007B4FA0 }
    else if y < 350 { 0x006B3FA0 }
    else if y < 500 { 0x005B2F90 }
    else if y < 650 { 0x004B1F80 }
    else            { 0x003B0F70 }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let fb = 0xffff8000fd000000 as *mut u32;
    for y in 0..H {
        for x in 0..W {
            let c: u32;
            if y < 50 {
                if      x >= 10   && x < 90   && y >= 10 && y < 40 { c = 0x0000FF00; }
                else if x >= 1040 && x < 1100 && y >= 10 && y < 40 { c = 0x00FF0000; }
                else if x >= 1110 && x < 1170 && y >= 10 && y < 40 { c = 0x000000FF; }
                else if x >= 1180 && x < 1240 && y >= 10 && y < 40 { c = 0x00000000; }
                else                                                { c = 0x00FFFFFF; }
            } else if y < 52 {
                c = 0x00333333;
            } else {
                c = bg(y);
            }
            unsafe { core::ptr::write_volatile(fb.add(y * W + x), c); }
        }
    }
    loop { core::hint::spin_loop(); }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
