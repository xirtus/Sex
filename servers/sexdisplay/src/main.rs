#![no_std]
#![no_main]

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;

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

fn render(fb: *mut u32, w: usize, h: usize) {
    for y in 0..h {
        for x in 0..w {
            let c: u32 = if y < 50 {
                bar_color(x, y)
            } else if y == 50 {
                0x002D1A3A // thin shadow line
            } else {
                bg(y)
            };
            unsafe { core::ptr::write_volatile(fb.add(y * w + x), c); }
        }
    }
}

fn handle_primary_fb(ptr: u64, packed: u64) {
    if ptr == 0 {
        return;
    }
    let w = packed as u32;
    let h = (packed >> 32) as u32;
    if w == 0 || h == 0 {
        return;
    }
    unsafe {
        FB_PTR = ptr;
        FB_W = w;
        FB_H = h;
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Render immediately with fallback — visible before any IPC
    unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize); }

    // 2. Listen for runtime FB handoff and SilkBar updates
    loop {
        let msg = sex_pdx::pdx_listen_raw(0);
        match msg.type_id {
            0x11 => { // OP_PRIMARY_FB
                handle_primary_fb(msg.arg0, msg.arg1);
                // Re-render with runtime values
                unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize); }
            }
            0xF2 => { // OP_SILKBAR_UPDATE — acknowledged
                // silkbar clock/tray updates: render later when protocol settled
            }
            _ => {
                // Unknown message — ignore
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
