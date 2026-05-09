#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use sex_pdx::serial_println;
use silkbar_model::{SilkBar, SilkBarUpdate, UpdateKind, apply_update, DEFAULT_SILK_BAR,
                    DEFAULT_THEME, ChipKind, ModuleSlot, validate_silkbar_contract, SILKBAR_ABI_VERSION,
                    APPEARANCE_TOKEN_FOCUS_SURFACE, APPEARANCE_TOKEN_FRAME_RIM,
                    APPEARANCE_TOKEN_FRAME_TOP_BAR, APPEARANCE_TOKEN_ACTIVE_TAB,
                    APPEARANCE_TOKEN_INACTIVE_TAB, APPEARANCE_TOKEN_CLOSE_LIGHT,
                    APPEARANCE_TOKEN_MINIMIZE_LIGHT, APPEARANCE_TOKEN_ZOOM_LIGHT};

const FALLBACK_PTR: u64 = 0xffff8000fd000000;
const FALLBACK_W: u32 = 1280;
const FALLBACK_H: u32 = 800;
const HIGH_HALF_BASE: u64 = 0xffff_8000_0000_0000;
const MAX_FB_W: usize = 8192;
const MAX_FB_H: usize = 4320;
const DRAIN_MAX: usize = 8;
const BAR_BG_W_CAP: usize = 2560;
const BAR_BG_H: usize = 50;

// Runtime FB config — starts as fallback, updated by OP_PRIMARY_FB
static mut FB_PTR: u64 = FALLBACK_PTR;
static mut FB_W: u32 = FALLBACK_W;
static mut FB_H: u32 = FALLBACK_H;
static mut BAR_BG_BUF: [u32; BAR_BG_H * BAR_BG_W_CAP] = [0u32; BAR_BG_H * BAR_BG_W_CAP];
static mut BAR_BLUR_BUF: [u32; BAR_BG_H * BAR_BG_W_CAP] = [0u32; BAR_BG_H * BAR_BG_W_CAP];

// ── Surface Registry (V1: safe inline ABI, no backing buffers) ──────────────

/// Max fill rects per surface. rect_index in 0xEF selects which slot to write.
const MAX_RECTS: usize = 8;

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
    // Per-surface tab info (V1: set via 0xFD from shell; used for colored tab blocks)
    tab_count: u8,
    active_tab: u8,
    // Per-surface chrome flags (V1: bit 0 = top bar enabled)
    chrome_flags: u8,
    // Per-surface fill rects (up to MAX_RECTS; fill_count tracks highest set index + 1)
    fill_count: u8,
    fill_sx: [i32; MAX_RECTS],
    fill_sy: [i32; MAX_RECTS],
    fill_sw: [u32; MAX_RECTS],
    fill_sh: [u32; MAX_RECTS],
    fill_color: [u32; MAX_RECTS],
    text_len: u8,
    text_color: u32,
    text_buf: [u8; 128],
}

const MAX_SURFACES: usize = 16;
const SURFACE_EMPTY: Surface = Surface {
    surface_id: 0, owner_pd: 0, x: 0, y: 0, w: 0, h: 0, color: 0, active: false,
    tab_count: 0, active_tab: 0, chrome_flags: 0,
    fill_count: 0,
    fill_sx: [0i32; MAX_RECTS], fill_sy: [0i32; MAX_RECTS],
    fill_sw: [0u32; MAX_RECTS], fill_sh: [0u32; MAX_RECTS],
    fill_color: [0u32; MAX_RECTS],
    text_len: 0, text_color: 0x00FFFFFF, text_buf: [0u8; 128],
};
static mut SURFACES: [Surface; MAX_SURFACES] = [SURFACE_EMPTY; MAX_SURFACES];
static mut FOCUSED_SURFACE_ID: u64 = 0;
const FOCUS_SURFACE_COLOR: u32 = 0x007AAFA4;

// ── Frame Chrome Rim (focused surface, matches shell FRAME_RIM_PX) ──
const FRAME_RIM_PX: usize = 4;
const FRAME_RIM_COLOR: u32 = 0x00B8F2E8;

// ── Frame Light Colors (top-left rim band, matches shell FRAME_LIGHTS_HIT_TARGET_V1) ──
const FRAME_LIGHT_SIZE_PX: usize = 4;
const FRAME_LIGHT_GAP_PX: usize = 2;
const FRAME_LIGHT_CLOSE_COLOR: u32 = 0x00FF4444;
const FRAME_LIGHT_MINIMIZE_COLOR: u32 = 0x00FFCC44;
const FRAME_LIGHT_ZOOM_COLOR: u32 = 0x0044FF44;

// ── Tab Strip Constants (matches shell FRAME_TAB_LIGHT_EXCLUSION_PX and TAB_*) ──
const TAB_STRIP_LIGHT_EXCLUSION_PX: usize = 20;
const TAB_ACTIVE_COLOR: u32 = FOCUS_SURFACE_COLOR; // 0x00A8E0FF (bright cyan)
const TAB_INACTIVE_COLOR: u32 = 0x006080B0; // dimmed cyan

// ── Top Bar Chrome Mode (default mode, matches shell FRAME_TOP_BAR_*) ──
/// Surface.chrome_flags bit: top bar enabled (24px top band replaces 4px top rim).
const SURFACE_CHROME_TOP_BAR: u8 = 1 << 0;
/// Height of the top bar chrome band in default mode.
const FRAME_TOP_BAR_HEIGHT_PX: usize = 28;
/// Width and height of each frame light in default mode.
const FRAME_TOP_BAR_LIGHT_SIZE_PX: usize = 10;
/// Gap between adjacent frame lights in default mode.
const FRAME_TOP_BAR_LIGHT_GAP_PX: usize = 5;
/// X-width of the Frame Lights exclusion zone in default mode.
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: usize = 50;
/// Top bar background color: dark navy-blue header, distinct from content teal.
const FRAME_TOP_BAR_COLOR: u32 = 0x00182730;
/// Toolbar divider color (bottom edge of top bar).
const FRAME_TOP_BAR_DIVIDER_COLOR: u32 = 0x00B8F2E8;
/// Light vertical zone: top (y offset within top bar), centered in 28px.
const FRAME_TOP_BAR_LIGHT_TOP: usize = 9;
/// Light vertical zone: bottom (exclusive).
const FRAME_TOP_BAR_LIGHT_BOTTOM: usize = 19; // 9 + 10

// ── Appearance Tokens (V1: flat ARGB, no alpha blending, blur forced zero) ────

#[derive(Clone, Copy)]
#[repr(C)]
struct RenderTokensV1 {
    focus_surface_color: u32,
    frame_rim_color: u32,
    frame_top_bar_color: u32,
    active_tab_color: u32,
    inactive_tab_color: u32,
    close_light_color: u32,
    minimize_light_color: u32,
    zoom_light_color: u32,
    appearance_flags: u8,
    effect_levels: u8,
}

const DEFAULT_RENDER_TOKENS: RenderTokensV1 = RenderTokensV1 {
    focus_surface_color:  FOCUS_SURFACE_COLOR,
    frame_rim_color:      FRAME_RIM_COLOR,
    frame_top_bar_color:  FRAME_TOP_BAR_COLOR,
    active_tab_color:     TAB_ACTIVE_COLOR,
    inactive_tab_color:   TAB_INACTIVE_COLOR,
    close_light_color:    FRAME_LIGHT_CLOSE_COLOR,
    minimize_light_color: FRAME_LIGHT_MINIMIZE_COLOR,
    zoom_light_color:     FRAME_LIGHT_ZOOM_COLOR,
    appearance_flags:     0,
    effect_levels:        0,
};

static mut DISPLAY_TOKENS: RenderTokensV1 = DEFAULT_RENDER_TOKENS;

// Token reception state machine. Call 1 buffers; Call 2 commits.
static mut TOKEN_BUF_CALL1_RECEIVED: bool = false;
static mut TOKEN_BUF_ARG0: u64 = 0;
static mut TOKEN_BUF_ARG1: u64 = 0;
static mut TOKEN_BUF_ARG2: u64 = 0;

/// Force alpha byte to 0xFF. Renderer ignores alpha (raw framebuffer write),
/// but this guards against any future alpha-blending path receiving a zero.
#[inline]
fn clamp_color_token(c: u32) -> u32 {
    c | 0xFF000000
}

// Shell-owned OS cursor surface. Always rendered last (above all app surfaces).
const CURSOR_SURFACE_ID: u64 = 0x90;
const LAUNCHER_PANEL_SURFACE_ID: u64 = 0x92;

/// Rate-limited rejection counter for unauthorized surface ops.
/// Logs at most every 64 rejections to prevent IPC/log storms.
static REJECT_COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static CURSOR_Z_TOP_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static DISPLAY_WM_PD: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static RENDER_FRAME_COUNTER: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);
static ROW_BUFFER_PROOF_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static BLUR_PROOF_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static ANIM_PROOF_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static R6_GLASS_PROOF_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
/// Clock source for redraw marker: 0=silkbar, 1=fallback
static mut CLOCK_REDRAW_SOURCE: u8 = 0;

/// Clamp a surface rectangle against framebuffer dimensions.
/// Returns `(x, y, w, h)` guaranteed to be within FB bounds and below the bar.
/// The `y` coordinate is clamped to at least `BAR_H` to prevent covering the top strip.
fn clamp_surface(surf: &Surface, fb_w: usize, fb_h: usize) -> (usize, usize, usize, usize) {
    // BAR_H covers model PANEL_Y+PANEL_H (10+38=48) plus 2px safety margin.
    // Must remain >= PANEL_Y + PANEL_H from silkbar-model.
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
                // ── Text rendering: glyph foreground overrides fill-rect color ──
                if let Some(tc) = surface_text_fg_at(x, y, surf) { c = tc; }
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
                    // Rim + Frame Light + Top Bar check (focused surface only).
                    let lx = x - sx;  // local x within clamped surface
                    let ly = y - sy;  // local y within clamped surface

                    // Edge detection with saturating_sub to prevent underflow
                    // on tiny surfaces (rim fills entire surface if sw/sh < RIM_PX).
                    let rim_right = sw.saturating_sub(FRAME_RIM_PX);
                    let rim_bottom = sh.saturating_sub(FRAME_RIM_PX);
                    let top_bar_active = (surf.chrome_flags & SURFACE_CHROME_TOP_BAR) != 0;

                    // ── TOP BAR ZONE (default chrome mode) ──
                    if top_bar_active && ly < FRAME_TOP_BAR_HEIGHT_PX {
                        // Default to top bar background color.
                        c = DISPLAY_TOKENS.frame_top_bar_color;
                        // Divider: bright line at bottom edge of toolbar.
                        if ly == FRAME_TOP_BAR_HEIGHT_PX - 1 {
                            c = FRAME_TOP_BAR_DIVIDER_COLOR;
                        }
                        // Toolbar title text: render surface title in top bar.
                        // Uses same 5x7 font as content text, positioned after lights.
                        if let Some(fg) = toolbar_title_fg_at(lx, ly, surf.surface_id) {
                            c = fg;
                        }
                        // Tab strip override: full 16px height, after light exclusion,
                        // before right rim. Uses same tab block geometry as minimal mode
                        // but with wider exclusion zone for larger lights.
                        if surf.tab_count > 0
                            && lx >= FRAME_TOP_BAR_LIGHT_EXCLUSION_PX
                            && lx < rim_right
                        {
                            let available = rim_right - FRAME_TOP_BAR_LIGHT_EXCLUSION_PX;
                            let slot_w = available / surf.tab_count as usize;
                            if slot_w > 0 {
                                let tab_idx = (lx - FRAME_TOP_BAR_LIGHT_EXCLUSION_PX) / slot_w;
                                if tab_idx == surf.active_tab as usize {
                                    c = DISPLAY_TOKENS.active_tab_color;
                                } else {
                                    c = DISPLAY_TOKENS.inactive_tab_color;
                                }
                            }
                        }
                        // Light override: only within the lights vertical band (y=4..12).
                        // Lights are 8x8px with 4px gaps, matching shell top bar model.
                        if ly >= FRAME_TOP_BAR_LIGHT_TOP && ly < FRAME_TOP_BAR_LIGHT_BOTTOM {
                            let l1_end = FRAME_TOP_BAR_LIGHT_GAP_PX + FRAME_TOP_BAR_LIGHT_SIZE_PX;
                            if lx >= FRAME_TOP_BAR_LIGHT_GAP_PX && lx < l1_end {
                                c = DISPLAY_TOKENS.close_light_color;
                            } else {
                                let l2_start = l1_end + FRAME_TOP_BAR_LIGHT_GAP_PX;
                                let l2_end = l2_start + FRAME_TOP_BAR_LIGHT_SIZE_PX;
                                if lx >= l2_start && lx < l2_end {
                                    c = DISPLAY_TOKENS.minimize_light_color;
                                } else {
                                    let l3_start = l2_end + FRAME_TOP_BAR_LIGHT_GAP_PX;
                                    let l3_end = l3_start + FRAME_TOP_BAR_LIGHT_SIZE_PX;
                                    if lx >= l3_start && lx < l3_end {
                                        c = DISPLAY_TOKENS.zoom_light_color;
                                    }
                                }
                            }
                        }
                    }
                    // ── RIM BAND (minimal mode top edge, or left/right/bottom edges always) ──
                    else if ly < FRAME_RIM_PX || lx < FRAME_RIM_PX
                        || lx >= rim_right || ly >= rim_bottom
                    {
                        // In the rim band. Check for Frame Lights in top-left corner.
                        // This path is reached in minimal mode (top_bar_active=false) or
                        // for left/right/bottom edges when top bar is active.
                        if ly < FRAME_RIM_PX && !top_bar_active {
                            // Minimal mode top rim. Lights are within top 4px rim band.
                            // CLOSE: gap from left edge
                            if lx >= FRAME_LIGHT_GAP_PX
                                && lx < FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX
                            {
                                c = DISPLAY_TOKENS.close_light_color;
                            }
                            // MINIMIZE: gap + size + gap
                            else if lx >= FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX
                                && lx < FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX
                            {
                                c = DISPLAY_TOKENS.minimize_light_color;
                            }
                            // ZOOM: gap + 2*(size + gap)
                            else if lx >= FRAME_LIGHT_GAP_PX + 2 * (FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX)
                                && lx < FRAME_LIGHT_GAP_PX + 2 * (FRAME_LIGHT_SIZE_PX + FRAME_LIGHT_GAP_PX) + FRAME_LIGHT_SIZE_PX
                            {
                                c = DISPLAY_TOKENS.zoom_light_color;
                            }
                            // Tab strip: equal-width colored blocks after light exclusion
                            // zone and before right rim. Active tab is bright, inactive dim.
                            else if surf.tab_count > 0
                                && lx >= TAB_STRIP_LIGHT_EXCLUSION_PX
                                && lx < rim_right
                            {
                                let available = rim_right - TAB_STRIP_LIGHT_EXCLUSION_PX;
                                let slot_w = available / surf.tab_count as usize;
                                if slot_w > 0 {
                                    let tab_idx = (lx - TAB_STRIP_LIGHT_EXCLUSION_PX) / slot_w;
                                    if tab_idx == surf.active_tab as usize {
                                        c = DISPLAY_TOKENS.active_tab_color;
                                    } else {
                                        c = DISPLAY_TOKENS.inactive_tab_color;
                                    }
                                } else {
                                    c = DISPLAY_TOKENS.frame_rim_color;
                                }
                            } else {
                                c = DISPLAY_TOKENS.frame_rim_color;
                            }
                        } else {
                            // Left, right, or bottom rim edge (or top edge in minimal mode
                            // already handled above — this catches left/right/bottom always).
                            c = DISPLAY_TOKENS.frame_rim_color;
                        }
                    } else {
                        // ── SURFACE CONTENT AREA ──
                        c = fill_rect_color(surf, x, y, DISPLAY_TOKENS.focus_surface_color);
                        // ── Text rendering: glyph foreground overrides focus color ──
                        if let Some(tc) = surface_text_fg_at(x, y, surf) { c = tc; }
                    }
                    break;
                }
            }
        }
    }
    c
}

/// For each active fill rect in the surface, if the global pixel (x,y) falls
/// within the rect, update the running color (painter's order: last match wins).
/// Used in both passes of composite_pixel to prevent logic drift.
fn fill_rect_color(surf: &Surface, x: usize, y: usize, base_color: u32) -> u32 {
    if surf.fill_count == 0 { return base_color; }
    let lx = (x as i32) - surf.x;
    let ly = (y as i32) - surf.y;
    let mut c = base_color;
    for i in 0..surf.fill_count as usize {
        if surf.fill_sw[i] == 0 || surf.fill_sh[i] == 0 { continue; }
        if lx >= surf.fill_sx[i] && lx < surf.fill_sx[i] + surf.fill_sw[i] as i32
            && ly >= surf.fill_sy[i] && ly < surf.fill_sy[i] + surf.fill_sh[i] as i32
        {
            c = surf.fill_color[i];
        }
    }
    c
}

/// Fixed-point linear interpolation between two XRGB colors.
/// t=0 returns a, t=255 returns b.  Alpha byte forced to 0xFF.
#[inline]
fn lerp_color_xrgb(a: u32, b: u32, t: u8) -> u32 {
    if t == 0 { return a | 0xFF000000; }
    if t == 255 { return b | 0xFF000000; }
    let ar = (a & 0xFF) as u32;
    let ag = ((a >> 8) & 0xFF) as u32;
    let ab = ((a >> 16) & 0xFF) as u32;
    let br = (b & 0xFF) as u32;
    let bg = ((b >> 8) & 0xFF) as u32;
    let bb = ((b >> 16) & 0xFF) as u32;
    let ti = t as u32;
    let r = ar + ((br.wrapping_sub(ar)).wrapping_mul(ti) / 255);
    let g = ag + ((bg.wrapping_sub(ag)).wrapping_mul(ti) / 255);
    let b_ = ab + ((bb.wrapping_sub(ab)).wrapping_mul(ti) / 255);
    0xFF000000 | (b_ << 16) | (g << 8) | r
}

/// Smooth vertical gradient between two XRGB colors.
/// y=0 returns top, y=h-1 returns bottom.
#[inline]
fn vertical_gradient_xrgb(y: usize, h: usize, top: u32, bottom: u32) -> u32 {
    if h <= 1 { return top; }
    let t = ((y.min(h - 1) as u32).wrapping_mul(255)) / ((h - 1) as u32);
    lerp_color_xrgb(top, bottom, t as u8)
}

/// Desktop background: smooth gradient from bg_top (at y=0) to bg_bottom (at bottom).
/// Replaces the old 4-band hard gradient.  Endpoint colors preserved from Theme.
fn bg(y: usize, h: usize) -> u32 {
    vertical_gradient_xrgb(y, h, DEFAULT_THEME.bg_top, DEFAULT_THEME.bg_bottom)
}

fn fill_bar_bg_buffer(fb_w: u32, fb_h: u32) -> bool {
    let fill_w = fb_w as usize;
    if fill_w > BAR_BG_W_CAP {
        return false;
    }
    let fill_h = BAR_BG_H.min(fb_h as usize);
    for y in 0..fill_h {
        let c = bg(y, fb_h as usize);
        for x in 0..fill_w {
            unsafe { BAR_BG_BUF[y * BAR_BG_W_CAP + x] = c; }
        }
    }
    true
}

fn blur_bar_bg_buffer_radius1(fb_w: u32, fb_h: u32) -> bool {
    let width = fb_w as usize;
    if width > BAR_BG_W_CAP {
        return false;
    }
    let height = BAR_BG_H.min(fb_h as usize);
    for y in 0..height {
        for x in 0..width {
            let mut acc_r: u32 = 0;
            let mut acc_g: u32 = 0;
            let mut acc_b: u32 = 0;
            let y0 = y.saturating_sub(1);
            let y1 = (y + 1).min(height.saturating_sub(1));
            let x0 = x.saturating_sub(1);
            let x1 = (x + 1).min(width.saturating_sub(1));
            for sy in y0..=y1 {
                for sx in x0..=x1 {
                    let c = unsafe { BAR_BG_BUF[sy * BAR_BG_W_CAP + sx] };
                    acc_r += c & 0xFF;
                    acc_g += (c >> 8) & 0xFF;
                    acc_b += (c >> 16) & 0xFF;
                }
            }
            let count = ((y1 - y0 + 1) * (x1 - x0 + 1)) as u32;
            let r = (acc_r / count) & 0xFF;
            let g = (acc_g / count) & 0xFF;
            let b = (acc_b / count) & 0xFF;
            unsafe { BAR_BLUR_BUF[y * BAR_BG_W_CAP + x] = 0xFF000000 | (b << 16) | (g << 8) | r; }
        }
    }
    true
}

fn sample_bar_blur_bg_xrgb(x: u32, y: u32, fb_w: u32, fb_h: u32) -> u32 {
    if fb_w as usize > BAR_BG_W_CAP || y as usize >= BAR_BG_H || x >= fb_w {
        return bg(y as usize, fb_h as usize);
    }
    unsafe { BAR_BLUR_BUF[y as usize * BAR_BG_W_CAP + x as usize] }
}

#[inline]
fn triangle_wave_u8(frame: u64, period: u64) -> u8 {
    if period == 0 {
        return 0;
    }
    let pos = frame % period;
    let half = period / 2;
    if half == 0 {
        return 0;
    }
    if pos < half {
        ((pos * 255) / half) as u8
    } else {
        let denom = period - half;
        if denom == 0 {
            0
        } else {
            (((period - pos) * 255) / denom) as u8
        }
    }
}

#[inline]
fn pulse_alpha(base: u8, amp: u8, frame: u64, period: u64) -> u8 {
    if period == 0 {
        return base;
    }
    let wave = triangle_wave_u8(frame, period) as i32; // 0..255
    let centered = wave - 127; // ~[-127,128]
    let delta = (centered * amp as i32) / 127;
    let out = base as i32 + delta;
    out.clamp(0, 255) as u8
}

/// Alpha-blend an XRGB foreground over an XRGB background.
/// alpha=0 → bg, alpha=255 → fg.  Result is opaque XRGB.
/// Uses integer arithmetic only; no framebuffer read.
#[inline]
fn alpha_blend_xrgb_over_xrgb(fg: u32, bg: u32, alpha: u8) -> u32 {
    if alpha == 0 { return bg; }
    if alpha == 255 { return fg; }
    let a = alpha as u32;
    let inv = 255 - a;
    let fr = (fg & 0xFF) as u32;
    let fg_ = ((fg >> 8) & 0xFF) as u32;
    let fb = ((fg >> 16) & 0xFF) as u32;
    let br = (bg & 0xFF) as u32;
    let bg_ = ((bg >> 8) & 0xFF) as u32;
    let bb = ((bg >> 16) & 0xFF) as u32;
    let r = (fr * a + br * inv) / 255;
    let g = (fg_ * a + bg_ * inv) / 255;
    let b = (fb * a + bb * inv) / 255;
    0xFF000000 | (b << 16) | (g << 8) | r
}

/// Alpha-blend a glass fg over the desktop background gradient.
/// Uses FB_H for height; alpha controls translucency.
#[inline]
fn glass_over_bg(fg: u32, x: usize, y: usize, alpha: u8) -> u32 {
    let fb_w = unsafe { FB_W };
    let fb_h = unsafe { FB_H };
    if fb_h == 0 { return fg; }
    let bg = sample_bar_blur_bg_xrgb(x as u32, y as u32, fb_w, fb_h);
    alpha_blend_xrgb_over_xrgb(fg, bg, alpha)
}

/// Linear edge glow alpha: distance 0 → max_alpha, distance ≥ spread → 0.
/// Integer-only, no floats.
#[inline]
fn glow_edge_alpha(dist: u32, spread: u32, max_alpha: u8) -> u8 {
    if spread == 0 || dist >= spread { return 0; }
    (max_alpha as u32 * (spread - dist) / spread) as u8
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
            if ws.active {
                // Active workspace gets a subtle glow tint over bg.
                const WORKSPACE_GLOW_BASE: u8 = 48;
                const WORKSPACE_GLOW_AMP: u8 = 32;
                const WORKSPACE_GLOW_PERIOD: u64 = 96;
                let frame = RENDER_FRAME_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
                let glow_alpha = pulse_alpha(
                    WORKSPACE_GLOW_BASE,
                    WORKSPACE_GLOW_AMP,
                    frame,
                    WORKSPACE_GLOW_PERIOD,
                );
                let active_glow = alpha_blend_xrgb_over_xrgb(
                    0x00FFFFFF, DEFAULT_THEME.active, glow_alpha);
                return Some(glass_over_bg(active_glow, x, y, 224));
            }
            if ws.urgent { return Some(DEFAULT_THEME.urgent); }
            return Some(DEFAULT_THEME.muted);
        }
    }
    None
}

fn chip_color(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const R6_CHIP_ALPHA: u8 = 212;
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
                ChipKind::Net     => return Some(glass_over_bg(DEFAULT_THEME.chip_fill, x, y, R6_CHIP_ALPHA)),
                ChipKind::Wifi    => return Some(0x0038D6C8),
                ChipKind::Battery => return Some(0x00FFB84D),
                ChipKind::Clock   => return Some(DEFAULT_THEME.chip_border),
            }
        }
    }
    None
}

fn bar_color(x: usize, y: usize, bar: &SilkBar) -> u32 {
    const R6_BAR_ALPHA: u8 = 196;
    const R6_EDGE_ALPHA: u8 = 72;
    const BAR_TOP_HIGHLIGHT: u32 = 0x00A8D8FF;
    const BAR_BOTTOM_DARK_EDGE: u32 = 0x000B1220;
    const BAR_CRYSTAL_TINT: u32 = 0x001C2A54;
    // Selected-window option indicators (display-only, no action behavior).
    // Rendered as small colored dots between launcher and workspace indicators.
    // Each dot is 3x3 px at a fixed position, colored by option bit.
    if bar.selected_options_mask != 0 {
        const OPTIONS_X: usize = 135; // after launcher (x=100, w=20)
        const OPTIONS_Y: usize = 19;  // same row as workspace/chip indicators
        const DOT_W: usize = 3;
        const DOT_H: usize = 3;
        const DOT_GAP: usize = 5;     // spacing between dots
        if y >= OPTIONS_Y && y < OPTIONS_Y + DOT_H {
            for bit in 0..4 {
                let dot_x = OPTIONS_X + bit * DOT_GAP;
                if x >= dot_x && x < dot_x + DOT_W {
                    if (bar.selected_options_mask >> bit) & 1 != 0 {
                        return match bit {
                            0 => 0x00FF4444, // CLOSE   → red
                            1 => 0x0044FF44, // ZOOM    → green
                            2 => 0x00FFCC44, // MINIMIZE→ yellow
                            3 => 0x0044CCFF, // MOVE    → cyan
                            _ => DEFAULT_THEME.panel_fill,
                        };
                    } else {
                        return DEFAULT_THEME.panel_fill; // unset bit: no dot
                    }
                }
            }
            // In options row but between dots → panel background
            return DEFAULT_THEME.panel_fill;
        }
    }
    // Workspace indicators
    if let Some(c) = workspace_color(x, y, bar) { return c; }
    // Status chip backgrounds
    if let Some(c) = chip_color(x, y, bar) { return c; }
    // Bell indicator (privacy-safe presence dot between Battery and Clock)
    let (bx, by, bw, bh) = module_rect(bar, ModuleSlot::Bell);
    if in_rect(x, y, bx, by, bw, bh) {
        let s = &bar.bell_state;
        if s.flags & 1 == 0 {
            // Bell unavailable (no SLOT_BELL grant or server not running) → dim dot
            return DEFAULT_THEME.muted;
        }
        if s.total_visible == 0 {
            // No events → dim/inactive dot
            return DEFAULT_THEME.muted;
        }
        if s.redacted_count > 0 {
            // Events with privacy-redacted content → warm tint
            return 0x00FFAA44; // amber
        }
        // Active events → bright dot
        return 0x00FFD700; // gold
    }
    // Launcher button with rounded-illusion border (model position)
    let (lx, ly, lw, lh) = module_rect(bar, ModuleSlot::Launcher);
    if in_rect(x, y, lx, ly, lw, lh) {
        let x2 = lx + 2;
        let y2 = ly + 2;
        let xw = lx + lw - 2;
        let yh = ly + lh - 2;
        if x < x2 || x >= xw || y < y2 || y >= yh {
            return glass_over_bg(DEFAULT_THEME.panel_glow, x, y, R6_EDGE_ALPHA);
        }
        let launcher_body = glass_over_bg(0x005CAEE8, x, y, 200);
        return alpha_blend_xrgb_over_xrgb(0x0082D6FF, launcher_body, 64);
    }
    // Bar body: glass translucency over desktop background,
    // with subtle bottom-edge glow falloff.
    const BAR_BOTTOM: usize = 50;
    const GLOW_SPREAD: usize = 4;
    if y == 0 {
        let body = glass_over_bg(DEFAULT_THEME.panel_fill, x, y, R6_BAR_ALPHA);
        return alpha_blend_xrgb_over_xrgb(BAR_TOP_HIGHLIGHT, body, 64);
    }
    if y == BAR_BOTTOM - 1 {
        let body = glass_over_bg(DEFAULT_THEME.panel_fill, x, y, R6_BAR_ALPHA);
        let darkened = alpha_blend_xrgb_over_xrgb(BAR_BOTTOM_DARK_EDGE, body, 88);
        return alpha_blend_xrgb_over_xrgb(DEFAULT_THEME.panel_glow, darkened, 48);
    }
    if y + GLOW_SPREAD >= BAR_BOTTOM && y < BAR_BOTTOM {
        let dist = (BAR_BOTTOM - 1 - y) as u32;
        let glow_a = glow_edge_alpha(dist, GLOW_SPREAD as u32, R6_EDGE_ALPHA);
        if glow_a > 0 {
            let base = glass_over_bg(DEFAULT_THEME.panel_fill, x, y, R6_BAR_ALPHA);
            let body = alpha_blend_xrgb_over_xrgb(BAR_CRYSTAL_TINT, base, 36);
            return alpha_blend_xrgb_over_xrgb(DEFAULT_THEME.panel_glow, body, glow_a);
        }
    }
    let base = glass_over_bg(DEFAULT_THEME.panel_fill, x, y, R6_BAR_ALPHA);
    alpha_blend_xrgb_over_xrgb(BAR_CRYSTAL_TINT, base, 32)
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

// 5×7 ASCII font: glyphs for chars 0x20 (space) through 0x5A (Z), 59 glyphs
// Each glyph: 7 bytes, MSB = leftmost pixel, 5 bits wide, 3 bits padding
// Used by surface_text_fg_at() for text rendering on surfaces
const FONT_ASCII_5X7: [[u8; 7]; 59] = [
    [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
    [0b01010, 0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000],
    [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
    [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100],
    [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
    [0b01100, 0b10010, 0b10010, 0b01100, 0b10101, 0b10010, 0b01101],
    [0b01100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
    [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
    [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
    [0b00000, 0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000],
    [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
    [0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000],
    [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
    [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
    [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
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
    [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
    [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000],
    [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
    [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
    [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
    [0b01110, 0b10001, 0b10011, 0b10101, 0b10011, 0b10000, 0b01110],
    [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
    [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
    [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
    [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
    [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b00111, 0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b01110],
    [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
    [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
    [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
    [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
    [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110],
    [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
    [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
    [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
    [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]
];

/// Returns `Some(text_color)` if pixel (lx, ly) is a toolbar title glyph pixel
/// for the given surface_id, or `None` if not in the toolbar title text area.
/// Uses FONT_ASCII_5X7 at a fixed position within the top bar.
fn toolbar_title_fg_at(lx: usize, ly: usize, surface_id: u64) -> Option<u32> {
    const GLYPH_W: usize = 5;
    const GLYPH_H: usize = 7;
    // Title text position: after lights exclusion, centered vertically in 32px bar.
    let title_x = FRAME_TOP_BAR_LIGHT_EXCLUSION_PX + 12;
    let title_y: usize = 10; // top padding for 5x7 glyph in 28px bar

    if (lx as isize) < (title_x as isize) || ly < title_y || ly >= title_y + GLYPH_H {
        return None;
    }

    let col = (lx - title_x) / (GLYPH_W + 1);
    let px = (lx - title_x) % (GLYPH_W + 1);
    let py = ly - title_y;
    if px >= GLYPH_W { return None; }

    // Hardcoded titles by surface_id
    let title: &[u8] = match surface_id {
        200 => b"Linen",
        201 => b"Quil",
        0x99 => b"Spindle",
        100 => b"App",
        _ => return None,
    };
    if col >= title.len() { return None; }
    let ch = title[col];
    if ch < 0x20 || ch > 0x5A { return None; }
    let glyph_idx = (ch - 0x20) as usize;
    if glyph_idx >= 59 { return None; }
    let row_bits = FONT_ASCII_5X7[glyph_idx][py];
    let pixel_on = (row_bits >> (4 - px)) & 1;
    if pixel_on != 0 {
        Some(0x00E8FFFFu32) // bright cyan-white
    } else {
        None
    }
}

/// Returns `Some(fg_color)` if pixel (x, y) is a text glyph foreground pixel
/// within the given surface, or `None` if background/not in the text area.
/// Uses FONT_ASCII_5X7 (5×7 glyphs) with 1px spacing between characters.
fn surface_text_fg_at(x: usize, y: usize, surf: &Surface) -> Option<u32> {
    if surf.text_len == 0 { return None; }
    const GLYPH_W: usize = 5;
    const GLYPH_H: usize = 7;
    const CHAR_SPACING: usize = 1; // 1px gap between chars
    const LINE_H: usize = GLYPH_H + 2; // line height with spacing
    const CHARS_PER_LINE: usize = 20; // max chars per line before wrap

    // Text area inset: 8px from left, 8px from top bar / rim
    let tx = (surf.x as usize).saturating_add(8);
    let ty = (surf.y as usize).saturating_add(24); // below title/chrome

    if x < tx || y < ty { return None; }

    let col = (x - tx) / (GLYPH_W + CHAR_SPACING);
    let row = (y - ty) / LINE_H;
    let px = (x - tx) % (GLYPH_W + CHAR_SPACING);
    let py = (y - ty) % LINE_H;

    if px >= GLYPH_W || py >= GLYPH_H { return None; }

    // Calculate character index in text buffer
    let char_idx = row * CHARS_PER_LINE + col;
    if char_idx >= surf.text_len as usize { return None; }

    let c = surf.text_buf[char_idx];
    // Map ASCII to font glyph index: 0x20=0, 0x21=1, ..., 0x5A=58
    if c < 0x20 || c > 0x5A { return None; }
    // Map lowercase to uppercase for V1
    let glyph_idx = if c >= 0x61 && c <= 0x7A {
        (c - 0x61 + 33) as usize  // 'a' maps to index of 'A' (33)
    } else {
        (c - 0x20) as usize
    };
    if glyph_idx >= 59 { return None; }

    let glyph_row = FONT_ASCII_5X7[glyph_idx][py];
    let bit = (glyph_row >> (GLYPH_W - 1 - px)) & 1;
    if bit != 0 {
        Some(surf.text_color)
    } else {
        None
    }
}

/// Returns `Some(fg)` if pixel (x, y) is a clock-digit foreground pixel,
/// or `None` if it is background/not in the clock area.
/// This is called inline during rendering to avoid a separate overlay pass
/// that would create a tear window between bar-fill and clock-overlay.
fn clock_fg_at(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    const CLOCK_FG: u32 = DEFAULT_THEME.text;
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

/// Returns `Some(fg)` if pixel (x, y) is a Bell count badge foreground pixel,
/// or `None` if it is background / not in the badge area.
/// Uses the same 5×7 FONT as the clock digits.
/// Badge is positioned at the right side of the Bell module slot.
fn bell_badge_at(x: usize, y: usize, bar: &SilkBar) -> Option<u32> {
    let n = bar.bell_state.total_visible;
    if n == 0 || bar.bell_state.flags & 1 == 0 {
        return None; // no badge when empty or Bell unavailable
    }
    // Clamp to 99 (two digits max)
    let n = n.min(99);
    let tens = n / 10;
    let ones = n % 10;

    let (bx, by, _, bh) = module_rect(bar, ModuleSlot::Bell);
    let badge_y = by + (bh / 2) - 3; // vertically centered, FONT is 7px tall
    if y < badge_y || y >= badge_y + 7 {
        return None;
    }
    let row = y - badge_y;

    // Two-digit badge: tens at bx+9, ones at bx+14 (within 18px slot)
    let left = if tens > 0 { bx + 9 } else { bx + 11 }; // single digit offset if <10
    let right = bx + 14;

    for (di, digit_x) in [(0, left), (1, right)].iter() {
        let digit = if *di == 0 { tens } else { ones };
        if *di == 0 && tens == 0 {
            continue; // skip leading zero
        }
        if x < *digit_x || x >= *digit_x + 5 {
            continue;
        }
        let col = x - *digit_x;
        if (FONT[digit as usize][row] >> (4 - col)) & 1 != 0 {
            return Some(DEFAULT_THEME.text); // use same color as clock
        }
        return None;
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
    let frame = RENDER_FRAME_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    // Budgeted marker: full-FB render invoked (proof of render entry + boundedness).
    unsafe {
        static mut FULL_RENDER_BUDGET: u32 = 8;
        let b = &mut FULL_RENDER_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[sexdisplay.render.full] fb_w={} fb_h={}", w, h);
        }
    }
    let total_pixels = pixels;
    let row_buffer_filled = fill_bar_bg_buffer(w as u32, h as u32);
    let blur_filled = if row_buffer_filled {
        blur_bar_bg_buffer_radius1(w as u32, h as u32)
    } else {
        false
    };
    if !ROW_BUFFER_PROOF_LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
        serial_println!(
            "[sexdisplay.render.row_buffer] cap_w={} fb_w={} filled={}",
            BAR_BG_W_CAP,
            w,
            if row_buffer_filled { 1 } else { 0 }
        );
    }
    if !BLUR_PROOF_LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
        serial_println!(
            "[sexdisplay.render.blur] radius=1 fb_w={} filled={}",
            w,
            if blur_filled { 1 } else { 0 }
        );
    }
    if !ANIM_PROOF_LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
        let _ = frame;
        serial_println!("[sexdisplay.render.anim] period=96 base=48 amp=32");
    }

    let focused_id = unsafe { FOCUSED_SURFACE_ID };
    // Top strip boundary: BAR_H = 50. Keep in sync with BAR_H constant.
    // Rows 0..49 = SilkBar panel background, 50 = glow edge.
    for y in 0..h {
        for x in 0..w {
            let c: u32 = if y < 50 {
                // SilkBar/top strip always on top (layer 3)
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else if let Some(fg) = bell_badge_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else if y == 50 {
                DEFAULT_THEME.panel_glow // low-contrast bar edge
            } else {
                composite_pixel(x, y, w, h, bg(y, h), focused_id)
            };
            let idx = y * w + x;
            if idx < total_pixels {
                unsafe { core::ptr::write_volatile(fb.add(idx), c); }
            }
        }
    }
    draw_cursor_z_top(fb, w, h, total_pixels);
    draw_launcher_panel(fb, w, h, total_pixels);
}

/// Redraw the top strip (rows 0..50) — bar background, workspace indicators, chips, and clock digits.
/// Called for all SilkBar updates (clock, workspace active/urgent, chip visibility/kind).
/// This is the only function that touches y<50 pixels after initial render.
fn redraw_top_strip(fb: *mut u32, w: usize, h: usize, bar: &SilkBar) {
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
    // Budgeted marker: top-strip redraw invoked (proof of bar render + boundedness).
    unsafe {
        static mut TOP_STRIP_REDRAW_BUDGET: u32 = 16;
        let b = &mut TOP_STRIP_REDRAW_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[sexdisplay.render.top_strip] fb_w={} fb_h={}", w, h);
        }
        // Clock redraw proof: what time is drawn on screen.
        // Source tracked via module-level CLOCK_REDRAW_SOURCE set by apply path.
        unsafe {
            static mut CLOCK_REDRAW_BUDGET: u32 = 64;
            let b = &mut CLOCK_REDRAW_BUDGET;
            if *b > 0 {
                *b -= 1;
                let src = if CLOCK_REDRAW_SOURCE == 1 { "fallback" } else { "silkbar" };
                serial_println!("[sexdisplay.clock.redraw] h={} m={} s={} source={}",
                    bar.clock_hh, bar.clock_mm, bar.clock_ss, src);
            }
        }
    }
    // Top strip boundary: rows 0..50. Keep in sync with BAR_H constant.
    // Row 50 = glow edge; rows 0..49 = SilkBar panel fill + modules.
    for y in 0..51 {
        for x in 0..w {
            let c: u32 = if y < 50 {
                if let Some(fg) = clock_fg_at(x, y, bar) {
                    fg
                } else if let Some(fg) = bell_badge_at(x, y, bar) {
                    fg
                } else {
                    bar_color(x, y, bar)
                }
            } else {
                DEFAULT_THEME.panel_glow
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
    // Budgeted marker: surface-area redraw invoked (proof of compositor render + boundedness).
    unsafe {
        static mut SURFACE_AREA_REDRAW_BUDGET: u32 = 16;
        let b = &mut SURFACE_AREA_REDRAW_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[sexdisplay.render.surface_area] fb_w={} fb_h={}", w, h);
        }
    }
    let focused_id = unsafe { FOCUSED_SURFACE_ID };
    for y in 50..h {
        for x in 0..w {
            let c: u32 = if y == 50 {
                DEFAULT_THEME.panel_glow // low-contrast bar edge
            } else {
                composite_pixel(x, y, w, h, bg(y, h), focused_id)
            };
            let idx = y * w + x;
            if idx < total_pixels {
                unsafe { core::ptr::write_volatile(fb.add(idx), c); }
            }
        }
    }
    draw_cursor_z_top(fb, w, h, total_pixels);
    draw_launcher_panel(fb, w, h, total_pixels);
}

// Classic NW arrow, 8 wide × 16 tall.
// Each u8 row: bit 7 = leftmost pixel. Set bit = draw CURSOR_ARROW_COLOR; clear = transparent.
const CURSOR_ARROW_W: usize = 8;
const CURSOR_ARROW_H: usize = 16;
const CURSOR_ARROW_COLOR: u32 = 0x00FFFFFF; // white
const CURSOR_ARROW_BITMAP: [u8; 16] = [
    0b10000000, //  *
    0b11000000, //  **
    0b11100000, //  ***
    0b11110000, //  ****
    0b11111000, //  *****
    0b11111100, //  ******
    0b11111110, //  *******
    0b11111100, //  ******   (start contracting)
    0b11011000, //  ** **    (waist split)
    0b10001100, //  *   **
    0b00001100, //      **
    0b00000110, //       **
    0b00000110, //       **
    0b00000011, //        **
    0b00000011, //        **
    0b00000001, //         *
];

/// Read back rows 0..50 (SilkBar top strip) and emit a deterministic FNV-1a hash.
/// Fires once after the first live render (after OP_PRIMARY_FB handoff).
/// Proves top-strip pixels are non-zero/rendered; never touches write path.
unsafe fn top_strip_render_proof(fb: *const u32, w: usize, h: usize) {
    let fb_addr = fb as u64;
    if fb_addr < HIGH_HALF_BASE { return; }
    if w == 0 || w > MAX_FB_W || h < 50 { return; }
    let strip_rows: usize = 50;
    let strip_pixels = match strip_rows.checked_mul(w) {
        Some(v) => v,
        None => return,
    };
    serial_println!("[silk.render_proof.top_strip.start]");
    let mut h_val: u64 = 0xcbf29ce484222325;
    let mut any_nonzero = false;
    for i in 0..strip_pixels {
        let px = core::ptr::read_volatile(fb.add(i));
        if px != 0 { any_nonzero = true; }
        h_val ^= px as u64;
        h_val = h_val.wrapping_mul(0x100000001b3);
    }
    // Print hash atomically: format into stack buffer, single pdx_call(0,69)
    // avoids scheduler interleave between format chunks.
    {
        let prefix = b"[silk.render_proof.top_strip.hash] value=0x";
        let digits = b"0123456789abcdef";
        let mut buf = [0u8; 64];
        let plen = prefix.len();
        buf[..plen].copy_from_slice(prefix);
        for i in 0..16usize {
            buf[plen + i] = digits[((h_val >> (60 - i * 4)) & 0xF) as usize];
        }
        buf[plen + 16] = b'\n';
        let _ = sex_pdx::pdx_call(0, 69, buf.as_ptr() as u64, (plen + 17) as u64, 0);
    }
    if any_nonzero {
        serial_println!("[silk.render_proof.top_strip.ok]");
    } else {
        serial_println!("[silk.render_proof.top_strip.fail] reason=all_zero");
    }
}

/// Pass 3: draw cursor surface (CURSOR_SURFACE_ID) unconditionally on top of all other surfaces.
/// Renders an arrow bitmap instead of a solid rect; transparent pixels are not written.
/// Called at the end of both render() and redraw_surface_area().
fn draw_cursor_z_top(fb: *mut u32, w: usize, h: usize, total_pixels: usize) {
    unsafe {
        for surf in SURFACES.iter() {
            if !surf.active || surf.surface_id != CURSOR_SURFACE_ID { continue; }
            let ox = surf.x.max(0) as usize; // origin x (not clamped — arrow clips naturally)
            let oy = surf.y.max(0) as usize; // origin y
            if !CURSOR_Z_TOP_LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
                serial_println!("[sexdisplay.cursor_surface.z_top.ok] id={:#x}", CURSOR_SURFACE_ID);
                serial_println!("[sexdisplay.cursor_shape.arrow.ok] id={:#x}", CURSOR_SURFACE_ID);
            }
            // Budgeted marker: cursor actually drawn to framebuffer.
            unsafe {
                static mut DISPLAY_CURSOR_DRAW_BUDGET: u32 = 16;
                let rem = &mut DISPLAY_CURSOR_DRAW_BUDGET;
                if *rem > 0 {
                    *rem -= 1;
                    serial_println!("[sexdisplay.cursor.draw] n=0 x={} y={}", ox as i32, oy as i32);
                }
            }
            for row in 0..CURSOR_ARROW_H {
                let py = oy + row;
                if py >= h { break; }
                let bits = CURSOR_ARROW_BITMAP[row];
                for col in 0..CURSOR_ARROW_W {
                    if bits & (0x80u8 >> col) == 0 { continue; }
                    let px = ox + col;
                    if px >= w { continue; }
                    let idx = py * w + px;
                    if idx < total_pixels {
                        core::ptr::write_volatile(fb.add(idx), CURSOR_ARROW_COLOR);
                    }
                }
            }
            break;
        }
    }
}

/// Pass 4: draw launcher panel surface (LAUNCHER_PANEL_SURFACE_ID) on top of all
/// app surfaces but under the cursor. Renders a solid rect with a thin border.
/// Only active when the launcher surface is active (toggled by shell).
/// Does not affect the top-strip render proof (y<50 area).
fn draw_launcher_panel(fb: *mut u32, w: usize, h: usize, total_pixels: usize) {
    unsafe {
        for surf in SURFACES.iter() {
            if !surf.active || surf.surface_id != LAUNCHER_PANEL_SURFACE_ID { continue; }
            let (sx, sy, sw, sh) = clamp_surface(surf, w, h);
            if sw == 0 || sh == 0 { break; }
            const LAUNCHER_PANEL_COLOR: u32 = 0x00203058;
            const LAUNCHER_BORDER_COLOR: u32 = 0x00405880;
            // Budgeted marker: launcher panel drawn to framebuffer.
            unsafe {
                static mut LAUNCHER_PANEL_DRAW_BUDGET: u32 = 8;
                let b = &mut LAUNCHER_PANEL_DRAW_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[sexdisplay.launcher_panel.draw] id={:#x} x={} y={} w={} h={}",
                        LAUNCHER_PANEL_SURFACE_ID, sx, sy, sw, sh);
                }
            }
            for row in 0..sh {
                let py = sy + row;
                if py >= h { break; }
                for col in 0..sw {
                    let px = sx + col;
                    if px >= w { continue; }
                    let idx = py * w + px;
                    if idx >= total_pixels { continue; }
                    // 1px border on all sides
                    let color = if row == 0 || row == sh - 1 || col == 0 || col == sw - 1 {
                        LAUNCHER_BORDER_COLOR
                    } else {
                        LAUNCHER_PANEL_COLOR
                    };
                    core::ptr::write_volatile(fb.add(idx), color);
                }
            }
            break;
        }
    }
}

fn handle_primary_fb(ptr: u64, packed: u64) -> bool {
    if ptr == 0 {
        return false;
    }
    // Reject non-canonical/low addresses that would fault on dereference.
    // Keep existing known-good fallback FB_PTR if kernel sends bogus address.
    if ptr < HIGH_HALF_BASE {
        return false;
    }
    let w = (packed as u32) as usize;
    let h = ((packed >> 32) as u32) as usize;
    if w == 0 || h == 0 || w > MAX_FB_W || h > MAX_FB_H {
        return false;
    }
    if w.checked_mul(h).is_none() {
        return false;
    }
    unsafe {
        FB_PTR = ptr;
        FB_W = w as u32;
        FB_H = h as u32;
    }
    true
}

fn handle_silkbar_update(bar: &mut SilkBar, arg0: u64, arg1: u64, arg2: u64) -> (bool, u32) {
    let kind = arg0 as u32;
    let update = SilkBarUpdate {
        kind,
        index: (arg1 >> 32) as u8,
        a: arg1 as u32,
        b: arg2 as u32,
    };
    let ok = apply_update(bar, update);
    // Budgeted marker for selected-window options update.
    if ok && kind == UpdateKind::SetSelectedOptions as u32 {
        unsafe {
            static mut SELECTED_OPTIONS_UPDATE_BUDGET: u32 = 8;
            let b = &mut SELECTED_OPTIONS_UPDATE_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[sexdisplay.selected.options.update] mask={:#x}", bar.selected_options_mask);
            }
        }
    }
    // Budgeted marker for Bell presence update.
    if ok && kind == UpdateKind::SetBellPresence as u32 {
        unsafe {
            static mut BELL_RENDER_BUDGET: u32 = 8;
            let b = &mut BELL_RENDER_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[sexdisplay.bell.render] total={} redacted={} flags={:#x}",
                    bar.bell_state.total_visible, bar.bell_state.redacted_count, bar.bell_state.flags);
            }
        }
    }
    (ok, kind)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexdisplay.init.start]");
    serial_println!("[silk.contract.validate.start]");
    let contract_err = validate_silkbar_contract();
    if contract_err != 0 {
        serial_println!("[silk.contract.validate.fail] reason={}", contract_err);
        loop { core::hint::spin_loop(); }
    } else {
        serial_println!("[silk.contract.validate.ok] version={}", SILKBAR_ABI_VERSION);
    }

    // Local SilkBar model — initialized from DEFAULT_SILK_BAR, mutated by OP_SILKBAR_UPDATE
    let mut bar = DEFAULT_SILK_BAR;
    let mut last_clock_tick = sex_pdx::get_ticks();     // raw ticks for fallback
    let mut last_clock_second = last_clock_tick / 62;    // sec_now for liveness gate
    let mut clock_from_silkbar = false;
    let mut last_silkbar_clock_ticks: u64 = 0;            // raw ticks of last silkbar update
    let mut fb_live = false;
    let mut render_proof_done = false;
    let mut loop_iter: u64 = 0;

    // 1. Render immediately with fallback — visible before any IPC
    unsafe { render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
    serial_println!("[sexdisplay.ready]");

    // 2. Listen for runtime FB handoff and SilkBar updates
    loop {
        // ── Per-cycle flags ──
        let mut needs_surface_redraw = false;
        let mut needs_top_strip_redraw = false;
        let mut did_primary_fb_render = false;
        let mut drained: usize = 0;
        if !fb_live {
            unsafe {
                static mut FB_LIVE_WAIT_BUDGET: u32 = 4;
                if FB_LIVE_WAIT_BUDGET > 0 {
                    FB_LIVE_WAIT_BUDGET -= 1;
                    serial_println!("[sexdisplay.fb.live.wait] iter={}", loop_iter);
                }
            }
        }

        // ── Clock tick ──
        // Use raw ticks to detect time advancement. Dividing by 62 assumes
        // a calibrated LAPIC timer; under QEMU TCG ticks may be far slower,
        // causing sec_now to stay 0 and the clock to freeze.
        let raw_ticks = sex_pdx::get_ticks();
        let sec_now = raw_ticks / 62; // still computed for silkbar liveness gate

        // Liveness-gate: if SilkBar clock stalls, resume local fallback.
        // Uses raw_ticks so a slow timer doesn't falsely trigger staleness.
        if clock_from_silkbar && raw_ticks.saturating_sub(last_silkbar_clock_ticks) > (62 * 5) {
            clock_from_silkbar = false;
            // Budgeted: first 4 fallback-resume events.
            unsafe {
                static mut CLOCK_FALLBACK_RESUME_BUDGET: u32 = 4;
                let remaining = &mut CLOCK_FALLBACK_RESUME_BUDGET;
                if *remaining > 0 {
                    *remaining -= 1;
                    sex_pdx::serial_println!("[sexdisplay.clock.fallback.resume] reason=silkbar_stale");
                }
            }
        }

        // Fallback: advance one second each time raw_ticks increments.
        // Works even when ticks < 62 (i.e. sec_now stays 0 for many cycles).
        if !clock_from_silkbar && raw_ticks > last_clock_tick {
            last_clock_tick = raw_ticks;
            last_clock_second = sec_now; // still track for silkbar liveness
            bar.clock_ss = bar.clock_ss.wrapping_add(1);
            if bar.clock_ss >= 60 { bar.clock_ss = 0; bar.clock_mm = bar.clock_mm.wrapping_add(1); }
            if bar.clock_mm >= 60 { bar.clock_mm = 0; bar.clock_hh = bar.clock_hh.wrapping_add(1); }
            if bar.clock_hh >= 24 { bar.clock_hh = 0; }
            if fb_live {
                needs_top_strip_redraw = true;
                unsafe { CLOCK_REDRAW_SOURCE = 1; } // fallback source
            }
        }

        // ── Clock state marker: budgeted, per unique raw_ticks ──
        unsafe {
            static mut CLOCK_STATE_BUDGET: u32 = 64;
            static mut CLOCK_STATE_LAST_TICK: u64 = u64::MAX;
            if CLOCK_STATE_BUDGET > 0 && raw_ticks != CLOCK_STATE_LAST_TICK {
                CLOCK_STATE_LAST_TICK = raw_ticks;
                CLOCK_STATE_BUDGET -= 1;
                serial_println!("[sexdisplay.clock.fallback.tick] ticks={} sec={} h={} m={} s={} from_silkbar={}",
                    raw_ticks, sec_now, bar.clock_hh, bar.clock_mm, bar.clock_ss,
                    clock_from_silkbar as u8);
            }
        }

        // ── Bounded drain: process up to DRAIN_MAX messages ──
        for _drain_i in 0..DRAIN_MAX {
            let Some(msg) = sex_pdx::pdx_try_listen_raw(0) else {
                break;
            };
            drained += 1;

            match msg.type_id {
                silkbar_model::OP_SILKBAR_UPDATE => {
                    // For SetClock: capture old bar values and decode incoming
                    // BEFORE apply_update mutates bar. This gives accurate
                    // old vs incoming comparison in markers.
                    let update_kind = msg.arg0 as u32;
                    let (old_hh, old_mm, old_ss, in_hh, in_mm, in_ss) =
                        if update_kind == UpdateKind::SetClock as u32 {
                            let ih = (msg.arg1 as u32).min(23);
                            let im = ((msg.arg2 >> 8) as u32).min(59);
                            let is_ = (msg.arg2 as u8 as u32).min(59);
                            (bar.clock_hh as u32, bar.clock_mm as u32, bar.clock_ss as u32,
                             ih, im, is_)
                        } else {
                            (0, 0, 0, 0, 0, 0)
                        };

                    let (applied, kind) = handle_silkbar_update(&mut bar, msg.arg0, msg.arg1, msg.arg2);
                    if applied {
                        if kind == UpdateKind::SetClock as u32 {
                            let changed = in_hh != old_hh || in_mm != old_mm || in_ss != old_ss;
                            // Always trust silkbar for clock ownership — no modulo-based
                            // stale gate. get_ticks() may be frozen under QEMU TCG.
                            clock_from_silkbar = true;
                            last_silkbar_clock_ticks = raw_ticks;
                            unsafe { CLOCK_REDRAW_SOURCE = 0; } // silkbar source
                            unsafe {
                                static mut CLOCK_RECV_BUDGET: u32 = 64;
                                let b = &mut CLOCK_RECV_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!(
                                        "[sexdisplay.clock.recv] in_h={} in_m={} in_s={} old_h={} old_m={} old_s={} sec_now={} changed={} from_silkbar=1",
                                        in_hh, in_mm, in_ss,
                                        old_hh, old_mm, old_ss,
                                        sec_now, changed as u8);
                                }
                            }
                            unsafe {
                                static mut CLOCK_APPLY_BUDGET: u32 = 64;
                                let b = &mut CLOCK_APPLY_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!(
                                        "[sexdisplay.clock.apply] source=silkbar h={} m={} s={} accepted=1 reason={}",
                                        bar.clock_hh, bar.clock_mm, bar.clock_ss,
                                        if changed { "changed" } else { "same" });
                                }
                            }
                        }
                        if fb_live {
                            needs_top_strip_redraw = true;
                        }
                    } else {
                        serial_println!("[silkde.m2.assert.bad] apply_update=false kind={}", kind);
                    }
                }
                0x11 => { // OP_PRIMARY_FB
                    if handle_primary_fb(msg.arg0, msg.arg1) {
                        fb_live = true;
                        serial_println!("[pdx.identity.accept] display owner_pd validation active");
                        // Coalesce startup repaints into one clean first frame.
                        unsafe {
                            render(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar);
                            serial_println!("[sexdisplay.render.live.ok] fb_w={} fb_h={}", FB_W, FB_H);
                        }
                        did_primary_fb_render = true;
                        if !render_proof_done {
                            render_proof_done = true;
                            unsafe { top_strip_render_proof(FB_PTR as *const u32, FB_W as usize, FB_H as usize); }
                        }
                    }
                }
                0 => continue,
                0xE4 => {
                    // OP_WINDOW_CREATE safe inline ABI: arg0=x, arg1=y, arg2=(h<<32)|w
                    // Store only, no immediate placeholder paint.
                    // Immediate blue paint here causes startup flicker.
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
                                    tab_count: 0, active_tab: 0, chrome_flags: 0,
                                    fill_count: 0,
                                    fill_sx: [0i32; MAX_RECTS], fill_sy: [0i32; MAX_RECTS],
                                    fill_sw: [0u32; MAX_RECTS], fill_sh: [0u32; MAX_RECTS],
                                    fill_color: [0u32; MAX_RECTS],
                                    text_len: 0, text_color: 0x00FFFFFF, text_buf: [0u8; 128],
                                };
                                break;
                            }
                        }
                    }
                    continue;
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
                                        tab_count: 0, active_tab: 0, chrome_flags: 0,
                                        fill_count: 0,
                                        fill_sx: [0i32; MAX_RECTS], fill_sy: [0i32; MAX_RECTS],
                                        fill_sw: [0u32; MAX_RECTS], fill_sh: [0u32; MAX_RECTS],
                                        fill_color: [0u32; MAX_RECTS],
                                        text_len: 0, text_color: 0x00FFFFFF, text_buf: [0u8; 128],
                                    };
                                    handled = true;
                                    break;
                                }
                            }
                        }
                        // Composite full below-bar area to respect registry z-order
                        if handled && fb_live {
                            needs_surface_redraw = true;
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
                        let wm_pd = DISPLAY_WM_PD.load(core::sync::atomic::Ordering::Acquire);
                        let mut found = false;
                        for slot in SURFACES.iter_mut() {
                            if slot.active && slot.surface_id == target_id {
                                if slot.owner_pd != msg.caller_pd && !(wm_pd != 0 && msg.caller_pd == wm_pd) {
                                    let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                    if n & 0x3F == 0 {
                                        serial_println!("[sexdisplay.auth.deny] op=0xeb caller={} sid={} owner={} wm={}",
                                            msg.caller_pd, target_id, slot.owner_pd, wm_pd);
                                    }
                                    continue;
                                }
                                slot.x = new_x;
                                slot.y = new_y;
                                found = true;
                                break;
                            }
                        }
                        if found && fb_live {
                            if target_id == CURSOR_SURFACE_ID {
                                // Budgeted marker: display received cursor surface update from shell.
                                unsafe {
                                    static mut DISPLAY_CURSOR_UPDATE_BUDGET: u32 = 16;
                                    let rem = &mut DISPLAY_CURSOR_UPDATE_BUDGET;
                                    if *rem > 0 {
                                        *rem -= 1;
                                        serial_println!("[sexdisplay.cursor.surface.update] n=0 x={} y={}", new_x, new_y);
                                    }
                                }
                            }
                            needs_surface_redraw = true;
                        }
                    }
                }
                0xED => {
                    // OP_SET_FOCUS: arg0=surface_id (0 clears focus). Owner or registered WM only.
                    if !fb_live { continue; }
                    let sid = msg.arg0;
                    if sid != 0 {
                        let wm_pd = DISPLAY_WM_PD.load(core::sync::atomic::Ordering::Acquire);
                        let authorized = unsafe {
                            SURFACES.iter().any(|s| s.active && s.surface_id == sid
                                && (s.owner_pd == msg.caller_pd || (wm_pd != 0 && msg.caller_pd == wm_pd)))
                        };
                        if !authorized {
                            let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                            if n & 0x3F == 0 {
                                serial_println!("[sexdisplay.auth.deny] op=0xed caller={} sid={} wm={}",
                                    msg.caller_pd, sid, wm_pd);
                            }
                            continue;
                        }
                    }
                    unsafe {
                        FOCUSED_SURFACE_ID = sid;
                        needs_surface_redraw = true;
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
                        if found && fb_live {
                            if FOCUSED_SURFACE_ID == target_id {
                                FOCUSED_SURFACE_ID = 0;
                            }
                            needs_surface_redraw = true;
                        }
                    }
                }
                0xF5 => {
                    // OP_REGISTER_WM: first caller wins; idempotent for same caller.
                    let caller = msg.caller_pd;
                    if caller == 0 { continue; }
                    match DISPLAY_WM_PD.compare_exchange(
                        0, caller,
                        core::sync::atomic::Ordering::AcqRel,
                        core::sync::atomic::Ordering::Acquire,
                    ) {
                        Ok(_) => { serial_println!("[sexdisplay.auth.wm.register] caller={} ok=1", caller); }
                        Err(existing) if existing != caller => {
                            serial_println!("[sexdisplay.auth.wm.deny] caller={} registered={}", caller, existing);
                        }
                        _ => {}
                    }
                }
                0xEF => {
                    // OP_SURFACE_FILL_RECT: arg0=surface_id, arg1=(sy<<32)|sx,
                    // arg2=(rect_index<<56)|(color_rgb<<32)|(sh<<16)|sw.
                    // rect_index in bits 56-59 (top nibble of color's alpha byte).
                    // Existing callers use 0x00RRGGBB so bits 56-63 are zero → rect_index=0.
                    let surface_id = msg.arg0;
                    if surface_id == 0 { continue; }
                    if !fb_live { continue; }

                    let sx = (msg.arg1 & 0xFFFF_FFFF) as i32;
                    let sy = ((msg.arg1 >> 32) & 0xFFFF_FFFF) as i32;
                    let mut sw = (msg.arg2 & 0xFFFF) as u32;
                    let mut sh = ((msg.arg2 >> 16) & 0xFFFF) as u32;
                    let color = ((msg.arg2 >> 32) & 0x00FF_FFFF) as u32;
                    let rect_index = ((msg.arg2 >> 56) & 0xF) as usize;
                    if sw == 0 || sh == 0 { continue; }

                    unsafe {
                        let mut updated = false;
                        for slot in SURFACES.iter_mut() {
                            if !(slot.active && slot.surface_id == surface_id) {
                                continue;
                            }
                            if slot.owner_pd != msg.caller_pd {
                                let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                if n & 0x3F == 0 {
                                    serial_println!("AUTH: 0xEF fill rejected sid={} caller={} owner={}",
                                        surface_id, msg.caller_pd, slot.owner_pd);
                                }
                                break;
                            }
                            if rect_index >= MAX_RECTS {
                                serial_println!("[display.fill_rect.reject.index] sid={} index={}",
                                    surface_id, rect_index);
                                break;
                            }

                            sw = sw.min(slot.w);
                            sh = sh.min(slot.h);
                            if sw == 0 || sh == 0 { break; }

                            let max_sx = slot.w.saturating_sub(sw) as i32;
                            let max_sy = slot.h.saturating_sub(sh) as i32;
                            let fill_sx = sx.clamp(0, max_sx);
                            let fill_sy = sy.clamp(0, max_sy);

                            slot.fill_sx[rect_index] = fill_sx;
                            slot.fill_sy[rect_index] = fill_sy;
                            slot.fill_sw[rect_index] = sw;
                            slot.fill_sh[rect_index] = sh;
                            slot.fill_color[rect_index] = color;
                            if rect_index + 1 > slot.fill_count as usize {
                                slot.fill_count = (rect_index + 1) as u8;
                            }
                            serial_println!("[display.fill_rect.set] sid={} index={}", surface_id, rect_index);
                            updated = true;
                            break;
                        }

                        if updated {
                            needs_surface_redraw = true;
                        }
                    }
                }
                // ── Text rendering opcodes (V1) ───────────────────────────────
                0xFA => {
                    // OP_TEXT_CLEAR: arg0 = surface_id
                    let sid = msg.arg0;
                    if sid == 0 { continue; }
                    unsafe {
                        for slot in SURFACES.iter_mut() {
                            if slot.active && slot.surface_id == sid {
                                if slot.owner_pd != msg.caller_pd {
                                    let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                    if n & 0x3F == 0 {
                                        serial_println!("AUTH: OP_TEXT_CLEAR rejected sid={} caller={} owner={}",
                                            sid, msg.caller_pd, slot.owner_pd);
                                    }
                                    break;
                                }
                                slot.text_len = 0;
                                slot.text_buf = [0u8; 128];
                                break;
                            }
                        }
                    }
                }
                0xFB => {
                    // OP_TEXT_DRAW: arg0=surface_id, arg1=8 ASCII bytes packed LE,
                    //              arg2 = byte_offset (bits 0-7) | char_count (bits 8-11) | text_color (bits 32-63)
                    let sid = msg.arg0;
                    if sid == 0 { continue; }
                    let packed = msg.arg1;
                    let byte_offset = (msg.arg2 & 0xFF) as usize;
                    let char_count = ((msg.arg2 >> 8) & 0xF) as usize;
                    let text_color = ((msg.arg2 >> 32) as u32) | 0xFF000000; // force opaque alpha
                    let max_buf = 128usize;
                    unsafe {
                        for slot in SURFACES.iter_mut() {
                            if slot.active && slot.surface_id == sid {
                                if slot.owner_pd != msg.caller_pd {
                                    let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                    if n & 0x3F == 0 {
                                        serial_println!("AUTH: OP_TEXT_DRAW rejected sid={} caller={} owner={}",
                                            sid, msg.caller_pd, slot.owner_pd);
                                    }
                                    break;
                                }
                                slot.text_color = text_color;
                                // Unpack 8 bytes from arg1 (little-endian)
                                let mut i = 0usize;
                                while i < char_count && i < 8 && byte_offset + i < max_buf {
                                    let byte = ((packed >> (i * 8)) & 0xFF) as u8;
                                    slot.text_buf[byte_offset + i] = byte;
                                    i += 1;
                                }
                                let new_len = byte_offset + i;
                                if new_len > slot.text_len as usize {
                                    slot.text_len = new_len.min(max_buf) as u8;
                                }
                                // Diagnostic: confirm text was written to surface
                                if slot.text_len > 0 && slot.text_len <= 32 {
                                    serial_println!("[sexdisplay.text.draw] sid={} len={} color={:#x}",
                                        sid, slot.text_len, slot.text_color);
                                }
                                break;
                            }
                        }
                    }
                }
                0xFC => {
                    // OP_APPEARANCE_TOKENS: two-call state machine.
                    // Call 1 (TOKEN_BUF_CALL1_RECEIVED=false): buffer 6 packed colors in arg0/arg1/arg2.
                    // Call 2 (TOKEN_BUF_CALL1_RECEIVED=true): commit 8 colors + flags to DISPLAY_TOKENS.
                    // Sequenced by receiver state only — no arg2 tagging (color data in Call 1 arg2).
                    unsafe {
                        if !TOKEN_BUF_CALL1_RECEIVED {
                            TOKEN_BUF_ARG0 = msg.arg0;
                            TOKEN_BUF_ARG1 = msg.arg1;
                            TOKEN_BUF_ARG2 = msg.arg2;
                            TOKEN_BUF_CALL1_RECEIVED = true;
                            static mut APPEARANCE_TOKENS_SEQ0_BUDGET: u32 = 4;
                            if APPEARANCE_TOKENS_SEQ0_BUDGET > 0 {
                                APPEARANCE_TOKENS_SEQ0_BUDGET -= 1;
                                serial_println!("[sexdisplay.appearance.tokens] seq=0 buffered");
                            }
                        } else {
                            // Unpack token preset by APPEARANCE_TOKEN_* index from silkbar-model.
                            // Call 1 packed: [0|1] in arg0, [2|3] in arg1, [4|5] in arg2.
                            // Call 2 packed: [6|7] in arg0, flags in arg1.
                            let focus_surface_color  = clamp_color_token(TOKEN_BUF_ARG0 as u32);
                            let frame_rim_color      = clamp_color_token((TOKEN_BUF_ARG0 >> 32) as u32);
                            let frame_top_bar_color  = clamp_color_token(TOKEN_BUF_ARG1 as u32);
                            let active_tab_color     = clamp_color_token((TOKEN_BUF_ARG1 >> 32) as u32);
                            let inactive_tab_color   = clamp_color_token(TOKEN_BUF_ARG2 as u32);
                            let close_light_color    = clamp_color_token((TOKEN_BUF_ARG2 >> 32) as u32);
                            let minimize_light_color = clamp_color_token(msg.arg0 as u32);
                            let zoom_light_color     = clamp_color_token((msg.arg0 >> 32) as u32);
                            let appearance_flags     = (msg.arg1 & 0xFF) as u8;
                            let effect_levels        = ((msg.arg1 >> 8) & 0xFF) as u8 & !0x0F; // blur=0 in V1
                            DISPLAY_TOKENS = RenderTokensV1 {
                                focus_surface_color,
                                frame_rim_color,
                                frame_top_bar_color,
                                active_tab_color,
                                inactive_tab_color,
                                close_light_color,
                                minimize_light_color,
                                zoom_light_color,
                                appearance_flags,
                                effect_levels,
                            };
                            TOKEN_BUF_CALL1_RECEIVED = false;
                            static mut APPEARANCE_TOKENS_APPLIED: u32 = 0;
                            APPEARANCE_TOKENS_APPLIED += 1;
                            let applied = APPEARANCE_TOKENS_APPLIED;
                            static mut APPEARANCE_TOKENS_SEQ1_BUDGET: u32 = 4;
                            if APPEARANCE_TOKENS_SEQ1_BUDGET > 0 {
                                APPEARANCE_TOKENS_SEQ1_BUDGET -= 1;
                                serial_println!("[sexdisplay.appearance.tokens] seq=1 applied={}", applied);
                            }
                            if fb_live {
                                needs_surface_redraw = true;
                            }
                        }
                    }
                }
                0xFD => {
                    // OP_SURFACE_TAB_INFO: arg0=surface_id, arg1=tab_count,
                    //   arg2 low 8 bits=active_tab, arg2 bit 8=chrome_flags
                    let surface_id = msg.arg0;
                    if surface_id == 0 { continue; }
                    let tab_count = (msg.arg1 as u8).min(8);
                    let raw_arg2 = msg.arg2;
                    let active_tab_raw = raw_arg2 as u8;
                    let chrome_flags_raw = ((raw_arg2 >> 8) & 0xff) as u8;
                    let active_tab = if tab_count > 0 {
                        active_tab_raw.min(tab_count.saturating_sub(1))
                    } else { 0 };
                    unsafe {
                        let wm_pd = DISPLAY_WM_PD.load(core::sync::atomic::Ordering::Acquire);
                        let mut updated = false;
                        for slot in SURFACES.iter_mut() {
                            if slot.active && slot.surface_id == surface_id {
                                if slot.owner_pd != msg.caller_pd && !(wm_pd != 0 && msg.caller_pd == wm_pd) {
                                    let n = REJECT_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                                    if n & 0x3F == 0 {
                                        serial_println!("[sexdisplay.auth.deny] op=0xfd caller={} sid={} owner={} wm={}",
                                            msg.caller_pd, surface_id, slot.owner_pd, wm_pd);
                                    }
                                    break;
                                }
                                slot.tab_count = tab_count;
                                slot.active_tab = active_tab;
                                slot.chrome_flags = chrome_flags_raw;
                                updated = true;
                                break;
                            }
                        }
                        if updated {
                            static mut SURFACE_TAB_INFO_BUDGET: u32 = 8;
                            let b = &mut SURFACE_TAB_INFO_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[sexdisplay.surface.tab.info] surface={} tabs={} active={} chrome={}",
                                    surface_id, tab_count, active_tab, chrome_flags_raw);
                            }
                            if fb_live {
                                needs_surface_redraw = true;
                            }
                        }
                    }
                }
                _ => {
                    serial_println!("[pdx.opcode.unknown] display type_id={:#x} caller={}", msg.type_id, msg.caller_pd);
                    // Ignore unrelated messages and continue draining.
                    continue;
                }
            }
        }

        // ── Budgeted marker: first 32 cycles ──
        unsafe {
            static mut RENDER_BUDGET_CYCLE: u32 = 32;
            if RENDER_BUDGET_CYCLE > 0 {
                RENDER_BUDGET_CYCLE -= 1;
                serial_println!("[sexdisplay.render.budget] drained={} surface={} strip={} primary={}",
                    drained,
                    needs_surface_redraw as u8,
                    needs_top_strip_redraw as u8,
                    did_primary_fb_render as u8);
            }
        }

        // ── Post-drain redraws ──
        if !did_primary_fb_render {
            if needs_surface_redraw {
                unsafe { redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize); }
            }
            if needs_top_strip_redraw {
                unsafe { redraw_top_strip(FB_PTR as *mut u32, FB_W as usize, FB_H as usize, &bar); }
            }
        }

        // ── Yield once regardless of whether messages remain queued ──
        loop_iter = loop_iter.wrapping_add(1);
        sex_pdx::sys_yield();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
