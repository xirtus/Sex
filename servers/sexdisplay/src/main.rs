#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use sex_pdx::serial_println;
use silkbar_model::{SilkBar, SilkBarUpdate, apply_update, DEFAULT_SILK_BAR,
                    ChipKind, ModuleSlot, validate_contract};

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

// ── Surface Registry (V1: safe inline ABI, no backing buffers) ──────────────

/// A compositor surface. Rendered as a solid-color filled rect below the bar.
/// No backing buffer, no alpha, no z-ordering (insertion order only).
/// Ownership invariant: owner_pd is set on first create and never changes
/// while active. Only the owning PD may mutate or destroy the surface.
/// Focus (z-order/color) is compositor state — open to all callers.
struct Surface {
    surface_id: u64,
    owner_pd: u32,       // PD that created this surface; 0 = unbound
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: u32,
    active: bool,
    // Per-surface fill rect (V1: single rect, last 0xEF wins)
    fill_sx: i32,
    fill_sy: i32,
    fill_sw: u32,
    fill_sh: u32,
    fill_color: u32,
    fill_active: bool,
}

const MAX_SURFACES: usize = 16;
const SURFACE_EMPTY: Surface = Surface {
    surface_id: 0, owner_pd: 0, x: 0, y: 0, w: 0, h: 0, color: 0, active: false,
    fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0, fill_color: 0, fill_active: false,
};
static mut SURFACES: [Surface; MAX_SURFACES] = [SURFACE_EMPTY; MAX_SURFACES];
static mut FOCUSED_SURFACE_ID: u64 = 0;
const FOCUS_SURFACE_COLOR: u32 = 0x00A8E0FF;

/// Rate-limited rejection counter for unauthorized surface ops.
/// Logs at most every 64 rejections to prevent IPC/log storms.
static REJECT_COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Clamp a surface rectangle against framebuffer dimensions.
/// Returns `(x, y, w, h)` guaranteed to be within FB bounds and below the bar.
/// The `y` coordinate is clamped to at least `BAR_H` to prevent covering the top strip.
fn clamp_surface(surf: &Surface, fb_w: usize, fb_h: usize) -> (usize, usize, usize, usize) {
    const BAR_H: usize = 50;
    let x = (surf.x.max(0) as usize).min(fb_w.saturating_sub(1));
    let y = (surf.y.max(BAR_H as i32) as usize).min(fb_h.saturating_sub(1));
    let max_w = fb_w.saturating_sub(x);
    let max_h = fb_h.saturating_sub(y);
    let w = (surf.w as usize).min(max_w);
    let h = (surf.h as usize).min(max_h);
    (x, y, w, h)
}

/// Composite a single pixel: non-focused surfaces in slot order, then focused on top.
/// Shared by render() and redraw_surface_area() to prevent drift.
fn composite_pixel(x: usize, y: usize, w: usize, h: usize, bg: u32, focused_id: u64) -> u32 {
    let mut c = bg;
    unsafe {
        // Pass 1: non-focused surfaces (slot order, break on first hit)
        for surf in SURFACES.iter() {
            if !surf.active || surf.surface_id == focused_id { continue; }
            let (sx, sy, sw, sh) = clamp_surface(surf, w, h);
            if sw == 0 || sh == 0 { continue; }
            if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
                c = fill_rect_color(surf, x, y, surf.color);
                break;
            }
        }
        // Pass 2: focused surface (always drawn on top)
        if focused_id != 0 {
            for surf in SURFACES.iter() {
                if !surf.active || surf.surface_id != focused_id { continue; }
                let (sx, sy, sw, sh) = clamp_surface(surf, w, h);
                if sw == 0 || sh == 0 { continue; }
                if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
                    c = fill_rect_color(surf, x, y, FOCUS_SURFACE_COLOR);
                    break;
                }
            }
        }
    }
    c
}

/// If the global pixel (x,y) falls within the surface's active fill rect,
/// return the fill color; otherwise return `base_color`.
/// Used in both passes of composite_pixel to prevent logic drift.
fn fill_rect_color(surf: &Surface, x: usize, y: usize, base_color: u32) -> u32 {
    if !surf.fill_active { return base_color; }
    let lx = (x as i32) - surf.x;
    let ly = (y as i32) - surf.y;
    if lx >= surf.fill_sx && lx < surf.fill_sx + surf.fill_sw as i32
        && ly >= surf.fill_sy && ly < surf.fill_sy + surf.fill_sh as i32
    {
        surf.fill_color
    } else {
        base_color
    }
}

fn bg(y: usize) -> u32 {
    if      y < 200 { 0x00081424 }  // deep navy
    else if y < 350 { 0x00102038 }  // deep blue
    else if y < 500 { 0x00182850 }  // violet blue
    else if y < 650 { 0x00281848 }  // warm purple
    else            { 0x00281848 }  // warm purple
}

#[inline]
fn in_rect(x: usize, y: usize, rx: usize, ry: usize, rw: usize, rh: usize) -> bool {
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

#[inline]
fn module_rect(bar: &SilkBar, slot: ModuleSlot) -> (usize, usize, usize, usize) {
    let lb = &bar.layout[slot as usize];
    (lb.x, lb.y, lb.w, lb.h)
}

fn workspace_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const WS_SLOTS: [ModuleSlot; 5] = [
        ModuleSlot::Workspace0,
        ModuleSlot::Workspace1,
        ModuleSlot::Workspace2,
        ModuleSlot::Workspace3,
        ModuleSlot::Workspace4,
    ];
    for (idx, slot) in WS_SLOTS.iter().enumerate() {
        let (wx, wy, ww, wh) = module_rect(bar, *slot);
        if in_rect(x, y, wx, wy, ww, wh) {
            let ws = &bar.workspaces[idx];
            if ws.active { return Some(0x00A8A0FF); }
            if ws.urgent { return Some(0x00FF6666); }
            return Some(0x00304068);
        }
    }
    None
}

fn chip_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const CHIP_SLOTS: [ModuleSlot; 4] = [
        ModuleSlot::Chip0,
        ModuleSlot::Chip1,
        ModuleSlot::Chip2,
        ModuleSlot::Clock,
    ];
    for (idx, slot) in CHIP_SLOTS.iter().enumerate() {
        let (cx, cy, cw, ch) = module_rect(bar, *slot);
        if in_rect(x, y, cx, cy, cw, ch) {
            let chip = &bar.chips[idx];
            if !chip.visible { return Some(0x00102038); }
            match chip.kind {
                ChipKind::Net     => return Some(0x004C8DFF),
                ChipKind::Wifi    => return Some(0x0038D6C8),
                ChipKind::Battery => return Some(0x00FFB84D),
                ChipKind::Clock   => return Some(0x006F86A8),
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
    let (lx, ly, lw, lh) = module_rect(bar, ModuleSlot::Launcher);
    if in_rect(x, y, lx, ly, lw, lh) {
        let x2 = lx + 2;
        let y2 = ly + 2;
        let xw = lx + lw - 2;
        let yh = ly + lh - 2;
        if x < x2 || x >= xw || y < y2 || y >= yh {
            return 0x00385078; // low-contrast glass edge
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
    let (cx, cy, _, _) = module_rect(bar, ModuleSlot::Clock);
    let cx = cx;
    let cy = cy + 1; // slight inset into chip area

    // Quick bounding-box reject
    if y < cy || y >= cy + 7 {
        return None;
    }
    if x < cx || x > cx + 45 {
        return None;
    }

    // Colon 1 at offset 14, Colon 2 at offset 31
    if x == cx + 14 || x == cx + 31 {
        if y == cy + 1 || y == cy + 5 {
            return Some(CLOCK_FG);
        }
        return None;
    }

    // Digit offsets: 0, 7, 17, 24, 34, 41
    const DIGITS: [usize; 6] = [0, 7, 17, 24, 34, 41];
    for (di, &dx) in DIGITS.iter().enumerate() {
        if x < cx + dx || x >= cx + dx + 5 {
            continue;
        }
        let col = x - (cx + dx);
        let row = y - cy;
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
    let total_pixels = pixels;

    let focused_id = unsafe { FOCUSED_SURFACE_ID };
    for y in 0..h {
        for x in 0..w {
            let c: u32 = if y < 50 {
                // SilkBar/top strip always on top (layer 3)
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else if y == 50 {
                0x00385078 // low-contrast bar edge
            } else {
                composite_pixel(x, y, w, h, bg(y), focused_id)
            };
            let idx = y * w + x;
            if idx < total_pixels {
                unsafe { core::ptr::write_volatile(fb.add(idx), c); }
            }
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
    let total_pixels = match w.checked_mul(h) {
        Some(v) => v,
        None => return,
    };
    for y in 0..51 {
        for x in 0..w {
            let c: u32 = if y < 50 {
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else {
                0x00385078
            };
            let idx = y * w + x;
            if idx < total_pixels {
                unsafe { core::ptr::write_volatile(fb.add(idx), c); }
            }
        }
    }
}

/// Redraw all pixels from y=50 to bottom (background gradient + all active surfaces).
/// Never touches y<50 (SilkBar/clock/chips). Not a full render() call.
fn redraw_surface_area(fb: *mut u32, w: usize, h: usize) {
    let fb_addr = fb as u64;
    if fb_addr < HIGH_HALF_BASE { return; }
    if w == 0 || h == 0 || w > MAX_FB_W || h > MAX_FB_H { return; }
    if h < 51 { return; }
    let total_pixels = match w.checked_mul(h) {
        Some(v) => v,
        None => return,
    };
    let focused_id = unsafe { FOCUSED_SURFACE_ID };
    for y in 50..h {
        for x in 0..w {
            let c: u32 = if y == 50 {
                0x00385078 // low-contrast bar edge
            } else {
                composite_pixel(x, y, w, h, bg(y), focused_id)
            };
            let idx = y * w + x;
            if idx < total_pixels {
                unsafe { core::ptr::write_volatile(fb.add(idx), c); }
            }
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
    if !validate_contract() {
        loop { core::hint::spin_loop(); }
    }

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
            0xE4 => {
                // OP_WINDOW_CREATE safe inline ABI: arg0=x, arg1=y, arg2=(h<<32)|w
                // V1: store only — no dynamic redraw. Visible on next boot render.
                let x = msg.arg0 as i32;
                let y = msg.arg1 as i32;
                let w = (msg.arg2 as u32).min(MAX_FB_W as u32);
                let h = ((msg.arg2 >> 32) as u32).min(MAX_FB_H as u32);
                if w == 0 || h == 0 { continue; }
                unsafe {
                    for slot in SURFACES.iter_mut() {
                        if !slot.active {
                            *slot = Surface {
                                surface_id: 0,
                                owner_pd: 0,
                                x, y, w, h,
                                color: 0x00303860,
                                active: true,
                                fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0,
                                fill_color: 0, fill_active: false,
                            };
                            break;
                        }
                    }
                }
                // Inline rect present — immediately show the client surface on screen.
                // Bounded double-loop: y >= BAR_H (50), x/w/h clamped to FB dimensions.
                unsafe {
                    let fb_w = FB_W as usize;
                    let fb_h = FB_H as usize;
                    let temp = Surface { surface_id: 0, owner_pd: 0, x, y, w, h, color: 0x00303860, active: true,
                        fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0, fill_color: 0, fill_active: false };
                    let (sx, sy, sw, sh) = clamp_surface(&temp, fb_w, fb_h);
                    if sw > 0 && sh > 0 && FB_PTR >= HIGH_HALF_BASE {
                        let fb = FB_PTR as *mut u32;
                        for py in sy..sy+sh {
                            let row = py * fb_w;
                            for px in sx..sx+sw {
                                fb.add(row + px).write_volatile(0x00303860);
                            }
                        }
                    }
                }
            }
            0xEC => {
                // OP_SURFACE_CREATE_ID: arg0=surface_id(non-zero), arg1=(y<<32)|x, arg2=(h<<32)|w
                let surface_id = msg.arg0;
                if surface_id == 0 { continue; }
                let x = msg.arg1 as i32;
                let y = (msg.arg1 >> 32) as i32;
                let w = (msg.arg2 as u32).min(MAX_FB_W as u32);
                let h = ((msg.arg2 >> 32) as u32).min(MAX_FB_H as u32);
                if w == 0 || h == 0 { continue; }
                let color = if surface_id & 1 == 0 { 0x00303860u32 } else { 0x00704890u32 };
                unsafe {
                    // Upsert: update existing surface or allocate new slot
                    // Ownership invariant: only the owning PD may upsert an active surface.
                    let mut handled = false;
                    for slot in SURFACES.iter_mut() {
                        if slot.active && slot.surface_id == surface_id {
                            if slot.owner_pd != msg.caller_pd {
                                let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                if n & 0x3F == 0 {
                                    serial_println!("AUTH: 0xEC upsert rejected sid={} caller={} owner={}",
                                        surface_id, msg.caller_pd, slot.owner_pd);
                                }
                                continue;
                            }
                            slot.x = x; slot.y = y; slot.w = w; slot.h = h;
                            if slot.color != color { slot.color = color; }
                            handled = true;
                            break;
                        }
                    }
                    // Create: bind owner_pd on first allocation of an inactive slot.
                    // After destroy (active=false) the same or a different PD may
                    // reclaim the slot, becoming the new owner.
                    if !handled {
                        for slot in SURFACES.iter_mut() {
                            if !slot.active {
                                *slot = Surface {
                                    surface_id, owner_pd: msg.caller_pd, x, y, w, h,
                                    color,
                                    active: true,
                                    fill_sx: 0, fill_sy: 0, fill_sw: 0, fill_sh: 0,
                                    fill_color: 0, fill_active: false,
                                };
                                handled = true;
                                break;
                            }
                        }
                    }
                    // Composite full below-bar area to respect registry z-order
                    if handled {
                        redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
                    }
                }
            }
            0xDE => {
                // OP_WINDOW_CREATE (legacy pointer protocol) — arg0 is cross-PD pointer.
                // Must NOT dereference. Unsupported until kernel-mediated copy exists.
                continue;
            }
            0xEB => {
                // OP_SURFACE_UPDATE: arg0=surface_id, arg1=x, arg2=y
                let target_id = msg.arg0;
                if target_id == 0 { continue; }
                let new_x = msg.arg1 as i32;
                let new_y = msg.arg2 as i32;
                unsafe {
                    let mut found = false;
                    for slot in SURFACES.iter_mut() {
                        if slot.active && slot.surface_id == target_id {
                            if slot.owner_pd != msg.caller_pd {
                                let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                if n & 0x3F == 0 {
                                    serial_println!("AUTH: 0xEB move rejected sid={} caller={} owner={}",
                                        target_id, msg.caller_pd, slot.owner_pd);
                                }
                                continue;
                            }
                            slot.x = new_x;
                            slot.y = new_y;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
                    }
                }
            }
            0xED => {
                // OP_SET_FOCUS: arg0=surface_id (0 clears focus). Unknown id safe.
                unsafe {
                    FOCUSED_SURFACE_ID = msg.arg0;
                    redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
                }
            }
            0xEE => {
                // OP_SURFACE_DESTROY: arg0=surface_id. Hide/deactivate surface.
                let target_id = msg.arg0;
                if target_id == 0 { continue; }
                unsafe {
                    let mut found = false;
                    for slot in SURFACES.iter_mut() {
                        if slot.active && slot.surface_id == target_id {
                            if slot.owner_pd != msg.caller_pd {
                                let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                if n & 0x3F == 0 {
                                    serial_println!("AUTH: 0xEE destroy rejected sid={} caller={} owner={}",
                                        target_id, msg.caller_pd, slot.owner_pd);
                                }
                                continue;
                            }
                            slot.active = false;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        if FOCUSED_SURFACE_ID == target_id {
                            FOCUSED_SURFACE_ID = 0;
                        }
                        redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
                    }
                }
            }
            0xEF => {
                // OP_SURFACE_FILL_RECT: arg0=surface_id, arg1=(sy<<32)|sx,
                // arg2=(color<<32)|(sh<<16)|sw. Fill a surface-local rect.
                let surface_id = msg.arg0;
                if surface_id == 0 { continue; }
                let sx = (msg.arg1 & 0xFFFFFFFF) as i32;
                let sy = ((msg.arg1 >> 32) & 0xFFFFFFFF) as i32;
                let mut sw = (msg.arg2 & 0xFFFF) as u32;
                let mut sh = ((msg.arg2 >> 16) & 0xFFFF) as u32;
                let color = (msg.arg2 >> 32) as u32;
                if sw == 0 || sh == 0 { continue; }
                unsafe {
                    for slot in SURFACES.iter_mut() {
                        if slot.active && slot.surface_id == surface_id {
                            if slot.owner_pd != msg.caller_pd {
                                let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                if n & 0x3F == 0 {
                                    serial_println!("AUTH: 0xEF fill rejected sid={} caller={} owner={}",
                                        surface_id, msg.caller_pd, slot.owner_pd);
                                }
                                break;
                            }
                            // 1. Clamp local rect to surface bounds
                            sw = sw.min(slot.w);
                            sh = sh.min(slot.h);
                            if sw == 0 || sh == 0 { break; }
                            let fill_sx = sx.clamp(0, (slot.w as i32).saturating_sub(sw as i32));
                            let fill_sy = sy.clamp(0, (slot.h as i32).saturating_sub(sh as i32));
                            // 2. Translate to global, clamp to FB, enforce bar_height
                            let gx = slot.x + fill_sx;
                            let gy = slot.y + fill_sy;
                            let gx2 = gx.max(0);
                            let gy2 = gy.max(50); // bar_height
                            let gw = sw as i32 - (gx2 - gx);
                            let gh = sh as i32 - (gy2 - gy);
                            // 3. Abort if globally invisible
                            if gw <= 0 || gh <= 0 { break; }
                            // 4. Store (surface-local coords)
                            slot.fill_sx = fill_sx;
                            slot.fill_sy = fill_sy;
                            slot.fill_sw = sw;
                            slot.fill_sh = sh;
                            slot.fill_color = color;
                            slot.fill_active = true;
                            serial_println!("[sexdisplay] Fill rect surface_id={} local=({},{},{},{}) color={:#x}",
                                surface_id, fill_sx, fill_sy, sw, sh, color);
                            break;
                        }
                    }
                }
                unsafe {
                    redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
                }
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
