#![no_std]
#![no_main]

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;

use silkbar_model::{SilkBar, ChipKind, Module, DEFAULT_THEME};

const BAR_H: usize = 50;

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

fn launcher_color(x: usize, y: usize, _bar: &SilkBar) -> Option<u32> {
    if !in_rect(x, y, silkbar_model::LAUNCHER_X, silkbar_model::LAUNCHER_Y, silkbar_model::LAUNCHER_W, silkbar_model::LAUNCHER_H) {
        return None;
    }
    // 2px border inset to fake rounded corners
    let lx = silkbar_model::LAUNCHER_X;
    let ly = silkbar_model::LAUNCHER_Y;
    let lw = silkbar_model::LAUNCHER_W;
    let lh = silkbar_model::LAUNCHER_H;
    if x < lx + 2 || x >= lx + lw - 2 || y < ly + 2 || y >= ly + lh - 2 {
        Some(DEFAULT_THEME.launcher_border)
    } else {
        Some(DEFAULT_THEME.launcher_fill)
    }
}

fn chip_kind_color(kind: ChipKind) -> u32 {
    match kind {
        ChipKind::Net     => DEFAULT_THEME.urgent,
        ChipKind::Wifi    => DEFAULT_THEME.active,
        ChipKind::Battery => DEFAULT_THEME.muted,
        ChipKind::Clock   => DEFAULT_THEME.text,
    }
}

fn workspace_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    // Workspace 0
    let w = if bar.workspaces[0].active { silkbar_model::WS_ACTIVE_W } else { silkbar_model::WS_INACTIVE_W };
    if in_rect(x, y, silkbar_model::WS_X0, silkbar_model::WS_Y, w, silkbar_model::WS_H) {
        return Some(if bar.workspaces[0].urgent { DEFAULT_THEME.urgent } else if bar.workspaces[0].active { DEFAULT_THEME.active } else { DEFAULT_THEME.muted });
    }
    // Workspace 1
    let w = if bar.workspaces[1].active { silkbar_model::WS_ACTIVE_W } else { silkbar_model::WS_INACTIVE_W };
    if in_rect(x, y, silkbar_model::WS_X1, silkbar_model::WS_Y, w, silkbar_model::WS_H) {
        return Some(if bar.workspaces[1].urgent { DEFAULT_THEME.urgent } else if bar.workspaces[1].active { DEFAULT_THEME.active } else { DEFAULT_THEME.muted });
    }
    // Workspace 2
    let w = if bar.workspaces[2].active { silkbar_model::WS_ACTIVE_W } else { silkbar_model::WS_INACTIVE_W };
    if in_rect(x, y, silkbar_model::WS_X2, silkbar_model::WS_Y, w, silkbar_model::WS_H) {
        return Some(if bar.workspaces[2].urgent { DEFAULT_THEME.urgent } else if bar.workspaces[2].active { DEFAULT_THEME.active } else { DEFAULT_THEME.muted });
    }
    // Workspace 3
    let w = if bar.workspaces[3].active { silkbar_model::WS_ACTIVE_W } else { silkbar_model::WS_INACTIVE_W };
    if in_rect(x, y, silkbar_model::WS_X3, silkbar_model::WS_Y, w, silkbar_model::WS_H) {
        return Some(if bar.workspaces[3].urgent { DEFAULT_THEME.urgent } else if bar.workspaces[3].active { DEFAULT_THEME.active } else { DEFAULT_THEME.muted });
    }
    // Workspace 4
    let w = if bar.workspaces[4].active { silkbar_model::WS_ACTIVE_W } else { silkbar_model::WS_INACTIVE_W };
    if in_rect(x, y, silkbar_model::WS_X4, silkbar_model::WS_Y, w, silkbar_model::WS_H) {
        return Some(if bar.workspaces[4].urgent { DEFAULT_THEME.urgent } else if bar.workspaces[4].active { DEFAULT_THEME.active } else { DEFAULT_THEME.muted });
    }
    None
}

fn chip_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    // Chip 0
    if bar.chips[0].visible && in_rect(x, y, silkbar_model::CHIP_X0, silkbar_model::CHIP_Y, silkbar_model::CHIP_W, silkbar_model::CHIP_H) {
        return Some(chip_kind_color(bar.chips[0].kind));
    }
    // Chip 1
    if bar.chips[1].visible && in_rect(x, y, silkbar_model::CHIP_X1, silkbar_model::CHIP_Y, silkbar_model::CHIP_W, silkbar_model::CHIP_H) {
        return Some(chip_kind_color(bar.chips[1].kind));
    }
    // Chip 2
    if bar.chips[2].visible && in_rect(x, y, silkbar_model::CHIP_X2, silkbar_model::CHIP_Y, silkbar_model::CHIP_W, silkbar_model::CHIP_H) {
        return Some(chip_kind_color(bar.chips[2].kind));
    }
    None
}

fn module_color(bar: &SilkBar, module: Module, x: usize, y: usize) -> Option<u32> {
    match module {
        Module::Launcher => launcher_color(x, y, bar),
        Module::Workspaces(_) => workspace_color(x, y, bar),
        Module::StatusChip(_) => chip_color(x, y, bar),
        Module::Clock => None,
    }
}

fn bar_color(x: usize, y: usize, bar: &SilkBar) -> u32 {
    for lb in &bar.layout {
        if in_rect(x, y, lb.x, lb.y, lb.w, lb.h) {
            if let Some(c) = module_color(bar, lb.module, x, y) {
                return c;
            }
        }
    }
    DEFAULT_THEME.text
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
    let x = silkbar_model::CLOCK_X;
    let y = silkbar_model::CLOCK_Y;
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
            let c: u32 = if y < BAR_H {
                bar_color(x, y, bar)
            } else if y == BAR_H {
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
