#![no_std]
#![no_main]

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;

use silkbar_model::{SilkBar, DEFAULT_THEME};

fn bg(y: usize) -> u32 {
    if      y < 200 { DEFAULT_THEME.bg_top }
    else if y < 350 { DEFAULT_THEME.panel_glow }
    else if y < 500 { DEFAULT_THEME.panel_fill }
    else if y < 650 { DEFAULT_THEME.chip_border }
    else            { DEFAULT_THEME.bg_bottom }
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
    if in_rect(x, y, 1040, 12, 56, 26) { return DEFAULT_THEME.urgent; } // red
    if in_rect(x, y, 1116, 12, 56, 26) { return DEFAULT_THEME.active; } // blue
    if in_rect(x, y, 1192, 12, 56, 26) { return DEFAULT_THEME.muted; }  // black

    DEFAULT_THEME.text // off-white bar default
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

fn render_digit(fb: *mut u32, x: usize, y: usize, digit: usize, fg: u32, stride: usize) {
    let glyph = FONT[digit];
    for row in 0..7 {
        let bits = glyph[row];
        for col in 0..5 {
            if (bits >> (4 - col)) & 1 != 0 {
                unsafe { core::ptr::write_volatile(fb.add((y + row) * stride + (x + col)), fg); }
            }
        }
    }
}

fn render_clock(fb: *mut u32, stride: usize, bar: &SilkBar) {
    let hh = bar.clock_hh;
    let mm = bar.clock_mm;
    let fg = DEFAULT_THEME.text;
    let x = 1192;
    let y = 16;
    // Hour digits
    render_digit(fb, x,      y, (hh / 10) as usize, fg, stride);
    render_digit(fb, x + 7,  y, (hh % 10) as usize, fg, stride);
    // Colon
    unsafe {
        core::ptr::write_volatile(fb.add((y + 1) * stride + (x + 14)), fg);
        core::ptr::write_volatile(fb.add((y + 5) * stride + (x + 14)), fg);
    }
    // Minute digits
    render_digit(fb, x + 17, y, (mm / 10) as usize, fg, stride);
    render_digit(fb, x + 24, y, (mm % 10) as usize, fg, stride);
}

fn render(fb: *mut u32, w: usize, h: usize, bar: &SilkBar) {
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
    // Overlay clock digits on the bar
    render_clock(fb, w, bar);
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

fn handle_silkbar_update(bar: &mut SilkBar, arg0: u64, arg1: u64, arg2: u64) {
    // Wire: arg0=kind, arg1=(index<<32)|a, arg2=b
    let kind = arg0 as u32;
    if kind == silkbar_model::UpdateKind::SetClock as u32 {
        // SetClock: a=hour, b=minute
        bar.clock_hh = (arg1 as u32).min(23) as u8;
        bar.clock_mm = (arg2 as u32).min(59) as u8;
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // SilkBar model state — initialized from DEFAULT_SILK_BAR (clock=10:42)
    let mut bar = silkbar_model::DEFAULT_SILK_BAR;

    // 1. Render immediately with fallback — visible before any IPC
    unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }

    // 2. Listen for runtime FB handoff and SilkBar updates
    loop {
        let msg = sex_pdx::pdx_listen_raw(0);
        match msg.type_id {
            0x11 => { // OP_PRIMARY_FB
                handle_primary_fb(msg.arg0, msg.arg1);
                unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
            }
            silkbar_model::OP_SILKBAR_UPDATE => {
                handle_silkbar_update(&mut bar, msg.arg0, msg.arg1, msg.arg2);
                unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
            }
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
