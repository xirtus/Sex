#![no_std]
#![no_main]

const W: usize = 1280;
const H: usize = 800;

fn bg(y: usize) -> u32 {
    if      y < 200 { 0x007B4FA0 }
    else if y < 350 { 0x006B3FA0 }
    else if y < 500 { 0x005B2F90 }
    else if y < 650 { 0x004B1F80 }
    else            { 0x003B0F70 }
}

#[inline]
fn in_rect(x: usize, y: usize, rx: usize, ry: usize, rw: usize, rh: usize) -> bool {
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

fn bar_color(x: usize, y: usize) -> u32 {
    // Launcher button with rounded-illusion border
    if in_rect(x, y, 10, 10, 80, 30) {
        // Border pixels (2px inset) — darker green to fake radius
        if x < 12 || x >= 88 || y < 12 || y >= 38 {
            return 0x0000AA00; // dark green edge
        }
        return 0x0000FF00; // bright green center
    }

    // Status indicators — cleaner spacing
    if in_rect(x, y, 1040, 12, 56, 26) { return 0x00FF0000; } // red
    if in_rect(x, y, 1116, 12, 56, 26) { return 0x000000FF; } // blue
    if in_rect(x, y, 1192, 12, 56, 26) { return 0x00000000; } // black

    0x00F2F2F2 // off-white bar default
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let fb = 0xffff8000fd000000 as *mut u32;
    for y in 0..H {
        for x in 0..W {
            let c: u32 = if y < 50 {
                bar_color(x, y)
            } else if y == 50 {
                0x002D1A3A // thin shadow line
            } else {
                bg(y)
            };
            unsafe { core::ptr::write_volatile(fb.add(y * W + x), c); }
        }
    }
    loop { core::hint::spin_loop(); }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
