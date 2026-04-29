#![no_std]
#![no_main]

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;

struct ClockState { hh: u8, mm: u8, ss: u8 }

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

// 5×7 bitmap glyphs for digits 0-9 (MSB = leftmost pixel)
const FONT: [[u8; 7]; 10] = [
    [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
    [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
    [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
    [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
    [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
    [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
    [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
];

/// Returns `Some(fg)` if pixel (x, y) is a clock-digit foreground pixel,
/// or `None` if it is background/not in the clock area.
/// This is called inline during rendering to avoid a separate overlay pass
/// that would create a tear window between bar-fill and clock-overlay.
fn clock_fg_at(x: usize, y: usize, clock: &ClockState) -> Option<u32> {
    const CLOCK_FG: u32 = 0x00F2F2F2;
    const CX: usize = 1192;
    const CY: usize = 16;

    // Quick bounding-box reject
    if y < CY || y >= CY + 7 {
        return None;
    }
    if x < CX || x > CX + 45 {
        return None;
    }

    // Colon 1 at offset 14, Colon 2 at offset 31
    if x == CX + 14 || x == CX + 31 {
        if y == CY + 1 || y == CY + 5 {
            return Some(CLOCK_FG);
        }
        return None;
    }

    // Digit offsets: 0, 7, 17, 24, 34, 41
    const DIGITS: [usize; 6] = [0, 7, 17, 24, 34, 41];
    for (di, &dx) in DIGITS.iter().enumerate() {
        if x < CX + dx || x >= CX + dx + 5 {
            continue;
        }
        let col = x - (CX + dx);
        let row = y - CY;
        let digit: usize = match di {
            0 => (clock.hh / 10) as usize,
            1 => (clock.hh % 10) as usize,
            2 => (clock.mm / 10) as usize,
            3 => (clock.mm % 10) as usize,
            4 => (clock.ss / 10) as usize,
            5 => (clock.ss % 10) as usize,
            _ => return None,
        };
        if (FONT[digit][row] >> (4 - col)) & 1 != 0 {
            return Some(CLOCK_FG);
        }
        return None; // digit background pixel
    }
    None
}

fn render(fb: *mut u32, w: usize, h: usize, clock: &ClockState) {
    for y in 0..h {
        for x in 0..w {
            let c: u32 = if y < 50 {
                // Check clock pixel inline — no separate overlay pass
                if let Some(fg) = clock_fg_at(x, y, clock) {
                    fg
                } else {
                    bar_color(x, y)
                }
            } else if y == 50 {
                0x002D1A3A // thin shadow line
            } else {
                bg(y)
            };
            // black_box prevents LLVM from vectorizing past the fb boundary
            let idx = y * w + x;
            unsafe { core::ptr::write_volatile(fb.add(core::hint::black_box(idx)), c); }
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

fn handle_silkbar_update(clock: &mut ClockState, arg0: u64, arg1: u64, arg2: u64) {
    // Wire: arg0=kind, arg1=(index<<32)|a, arg2=b
    let kind = arg0 as u32;
    if kind == silkbar_model::UpdateKind::SetClock as u32 {
        // SetClock: a=hour, b packed = (mm << 8) | ss
        let hh = (arg1 as u32).min(23) as u8;
        let mm = ((arg2 >> 8) as u32).min(59) as u8;
        let ss = (arg2 as u8).min(59) as u8;
        *clock = ClockState { hh, mm, ss };
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Local clock state — initialized 10:42, mutated by OP_SILKBAR_UPDATE
    let mut clock = ClockState { hh: 10, mm: 42, ss: 0 };

    // 1. Render immediately with fallback — visible before any IPC
    unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &clock); }

    // 2. Listen for runtime FB handoff and SilkBar updates
    loop {
        let msg = sex_pdx::pdx_listen_raw(0);
        match msg.type_id {
            0x11 => { // OP_PRIMARY_FB
                handle_primary_fb(msg.arg0, msg.arg1);
                unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &clock); }
            }
            silkbar_model::OP_SILKBAR_UPDATE => {
                handle_silkbar_update(&mut clock, msg.arg0, msg.arg1, msg.arg2);
                unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &clock); }
            }
            0 => {
                // Empty (pdx_listen_raw should never return this)
                sex_pdx::sys_yield();
            }
            _ => {
                // Unknown type_id — yield to avoid busy-spin
                sex_pdx::sys_yield();
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
