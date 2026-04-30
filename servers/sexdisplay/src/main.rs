#![no_std]
#![no_main]

use silkbar_model::{SilkBar, SilkBarUpdate, apply_update, DEFAULT_SILK_BAR,
                    WS_X0, WS_X1, WS_X2, WS_X3, WS_X4, WS_Y, WS_H,
                    WS_INACTIVE_W,
                    CHIP_X0, CHIP_X1, CHIP_X2, CHIP_X3, CHIP_Y, CHIP_H, CHIP_W, CLOCK_W,
                    LAUNCHER_X, LAUNCHER_Y, LAUNCHER_W, LAUNCHER_H,
                    ChipKind};

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;
const HIGH_HALF_BASE: u64 = 0xffff_8000_0000_0000;
const MAX_FB_W: usize = 8192;
const MAX_FB_H: usize = 4320;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;

fn bg(y: usize) -> u32 {
    if      y < 200 { 0x00081024 }  // deep navy
    else if y < 350 { 0x00142038 }  // deep blue
    else if y < 500 { 0x00182030 }  // blue-violet
    else if y < 650 { 0x00201830 }  // warm purple
    else            { 0x00201830 }  // warm purple
}

#[inline]
fn in_rect(x: usize, y: usize, rx: usize, ry: usize, rw: usize, rh: usize) -> bool {
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

fn workspace_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const WS_TABS: [(usize, usize); 5] = [
        (WS_X0, 0), (WS_X1, 1), (WS_X2, 2), (WS_X3, 3), (WS_X4, 4),
    ];
    for &(wx, idx) in &WS_TABS {
        if y >= WS_Y && y < WS_Y + WS_H && x >= wx && x < wx + WS_INACTIVE_W {
            let ws = &bar.workspaces[idx];
            if ws.active { return Some(0x00A8A0FF); }
            if ws.urgent { return Some(0x00FF7070); }
            return Some(0x00303860);
        }
    }
    None
}

fn chip_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    // Chips 0-2 use standard width; chip 3 (Clock) spans full clock width
    const CHIP_POS: [(usize, usize, usize); 4] = [
        (CHIP_X0, CHIP_W, 0),
        (CHIP_X1, CHIP_W, 1),
        (CHIP_X2, CHIP_W, 2),
        (CHIP_X3, CLOCK_W, 3),
    ];
    for &(cx, cw, idx) in &CHIP_POS {
        if y >= CHIP_Y && y < CHIP_Y + CHIP_H && x >= cx && x < cx + cw {
            let chip = &bar.chips[idx];
            if !chip.visible { return Some(0x00142038); }
            match chip.kind {
                ChipKind::Net     => return Some(0x006098FF),
                ChipKind::Wifi    => return Some(0x004ACC88),
                ChipKind::Battery => return Some(0x00F0A830),
                ChipKind::Clock   => return Some(0x00507090),
            }
        }
    }
    None
}

fn bar_color(x: usize, y: usize, bar: &SilkBar) -> u32 {
    // Workspace indicators
    if let Some(c) = workspace_color(x, y, bar) { return c; }
    // Status chip backgrounds
    if let Some(c) = chip_color(x, y, bar) { return c; }
    // Launcher button with rounded-illusion border (model position)
    if in_rect(x, y, LAUNCHER_X, LAUNCHER_Y, LAUNCHER_W, LAUNCHER_H) {
        let x2 = LAUNCHER_X + 2;
        let y2 = LAUNCHER_Y + 2;
        let xw = LAUNCHER_X + LAUNCHER_W - 2;
        let yh = LAUNCHER_Y + LAUNCHER_H - 2;
        if x < x2 || x >= xw || y < y2 || y >= yh {
            return 0x004088AA; // subtle teal edge
        }
        return 0x0070CCFF; // cyan launcher dot
    }
    0x00182040 // deep blue-violet glass bar default
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
fn clock_fg_at(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const CLOCK_FG: u32 = 0x00C8D8FF;
    const CX: usize = CHIP_X3;    // model clock-area start
    const CY: usize = CHIP_Y + 1; // slight inset into chip area

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
            0 => (bar.clock_hh / 10) as usize,
            1 => (bar.clock_hh % 10) as usize,
            2 => (bar.clock_mm / 10) as usize,
            3 => (bar.clock_mm % 10) as usize,
            4 => (bar.clock_ss / 10) as usize,
            5 => (bar.clock_ss % 10) as usize,
            _ => return None,
        };
        if (FONT[digit][row] >> (4 - col)) & 1 != 0 {
            return Some(CLOCK_FG);
        }
        return None; // digit background pixel
    }
    None
}

fn render(fb: *mut u32, w: usize, h: usize, bar: &SilkBar) {
    let fb_addr = fb as u64;
    if fb_addr < HIGH_HALF_BASE {
        return;
    }
    if w == 0 || h == 0 || w > MAX_FB_W || h > MAX_FB_H {
        return;
    }
    let pixels = match w.checked_mul(h) {
        Some(v) => v,
        None => return,
    };
    let bytes = match pixels.checked_mul(4) {
        Some(v) => v as u64,
        None => return,
    };
    let end_addr = match fb_addr.checked_add(bytes) {
        Some(v) => v,
        None => return,
    };
    // Guard full framebuffer range before first write.
    if end_addr < HIGH_HALF_BASE {
        return;
    }

    for y in 0..h {
        for x in 0..w {
            let c: u32 = if y < 50 {
                // Check clock pixel inline — no separate overlay pass
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else if y == 50 {
                0x000A1020 // deep glass edge shadow
            } else {
                bg(y)
            };
            let idx = y * w + x;
            unsafe { core::ptr::write_volatile(fb.add(idx), c); }
        }
    }
}

fn redraw_clock_only(fb: *mut u32, w: usize, h: usize, bar: &SilkBar) {
    let fb_addr = fb as u64;
    if fb_addr < HIGH_HALF_BASE {
        return;
    }
    if w == 0 || h == 0 || w > MAX_FB_W || h > MAX_FB_H {
        return;
    }
    if h < 51 {
        return;
    }
    for y in 0..51 {
        for x in 0..w {
            let c: u32 = if y < 50 {
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else {
                0x000A1020
            };
            let idx = y * w + x;
            unsafe { core::ptr::write_volatile(fb.add(idx), c); }
        }
    }
}

fn handle_primary_fb(ptr: u64, packed: u64) {
    if ptr == 0 {
        return;
    }
    // Reject non-canonical/low addresses that would fault on dereference.
    // Keep existing known-good fallback FB_PTR if kernel sends bogus address.
    if ptr < HIGH_HALF_BASE {
        return;
    }
    let w = (packed as u32) as usize;
    let h = ((packed >> 32) as u32) as usize;
    if w == 0 || h == 0 || w > MAX_FB_W || h > MAX_FB_H {
        return;
    }
    if w.checked_mul(h).is_none() {
        return;
    }
    unsafe {
        FB_PTR = ptr;
        FB_W = w as u32;
        FB_H = h as u32;
    }
}

fn handle_silkbar_update(bar: &mut SilkBar, arg0: u64, arg1: u64, arg2: u64) {
    // arg0 = UpdateKind, arg1 = (index << 32) | a, arg2 = b
    let update = SilkBarUpdate {
        kind: arg0 as u32,
        index: (arg1 >> 32) as u8,
        a: arg1 as u32,
        b: arg2 as u32,
    };
    apply_update(bar, update);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Local SilkBar model — initialized from DEFAULT_SILK_BAR, mutated by OP_SILKBAR_UPDATE
    let mut bar = DEFAULT_SILK_BAR;

    // 1. Render immediately with fallback — visible before any IPC
    unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }

    // 2. Listen for runtime FB handoff and SilkBar updates
    loop {
        let msg = sex_pdx::pdx_listen_raw(0);
        match msg.type_id {
            silkbar_model::OP_SILKBAR_UPDATE => {
                handle_silkbar_update(&mut bar, msg.arg0, msg.arg1, msg.arg2);
                unsafe { redraw_clock_only(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
            }
            0x11 => { // OP_PRIMARY_FB
                handle_primary_fb(msg.arg0, msg.arg1);
                unsafe { redraw_clock_only(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
            }
            0 => {
                // pdx_listen_raw already yields internally on empty.
                continue;
            }
            _ => {
                // Ignore unrelated messages and continue draining.
                continue;
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
