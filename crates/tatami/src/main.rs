//! tatami — Silk Desktop Environment unified settings panel.
//!
//! Sidebar + content layout; modular sections via enum dispatch.
//! Ported from cosmic-settings architecture; surface = SexCompositor PDX frames.
//!
//! Sections: Display · Network · Sound · Input · Capabilities · About

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use sex_pdx::{
    pdx_call, pdx_listen, pdx_reply,
    PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY, PDX_MAP_MEMORY,
    PDX_WINDOW_COMMIT_FRAME, PDX_GET_DISPLAY_INFO,
    SexWindowCreateParams, Rect,
};
use sex_graphics::{WindowBuffer, font};

// ─── Bump allocator ───────────────────────────────────────────────────────────
const HEAP_START: usize = 0x5000_0000;
const HEAP_END:   usize = HEAP_START + 128 * 1024 * 1024;
static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);

struct Bump;
unsafe impl core::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: core::alloc::Layout) -> *mut u8 {
        let mut c = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let a = (c + l.align() - 1) & !(l.align() - 1);
            let n = a + l.size();
            if n > HEAP_END { return core::ptr::null_mut(); }
            match HEAP_TOP.compare_exchange_weak(c, n, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => return a as *mut u8,
                Err(x) => c = x,
            }
        }
    }
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {}
}
#[global_allocator]
static ALLOC: Bump = Bump;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

// ─── Colours ─────────────────────────────────────────────────────────────────
const BG:       u32 = 0xFF1E1E2E;
const MANTLE:   u32 = 0xFF181825;
const SURFACE0: u32 = 0xFF313244;
const SURFACE1: u32 = 0xFF45475A;
const TEXT:     u32 = 0xFFCDD6F4;
const SUBTEXT:  u32 = 0xFFA6ADC8;
const ACCENT:   u32 = 0xFF89B4FA;
const GREEN:    u32 = 0xFFA6E3A1;
const YELLOW:   u32 = 0xFFF9E2AF;

const WIN_W:      u32 = 900;
const WIN_H:      u32 = 640;
const SIDEBAR_W:  u32 = 200;
const CONTENT_X:  u32 = SIDEBAR_W + 1;
const CONTENT_W:  u32 = WIN_W - CONTENT_X;
const TITLE_H:    u32 = 48;
const ROW_H:      u32 = 40;

const SEXDISPLAY_PD: u32 = 1;

// ─── Section IDs ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Display,
    Network,
    Sound,
    Input,
    Capabilities,
    About,
}

impl Section {
    const ALL: &'static [Section] = &[
        Section::Display,
        Section::Network,
        Section::Sound,
        Section::Input,
        Section::Capabilities,
        Section::About,
    ];

    fn label(self) -> &'static [u8] {
        match self {
            Section::Display      => b"Display",
            Section::Network      => b"Network",
            Section::Sound        => b"Sound",
            Section::Input        => b"Input",
            Section::Capabilities => b"Capabilities",
            Section::About        => b"About",
        }
    }
}

// ─── Settings state ───────────────────────────────────────────────────────────

struct DisplaySettings {
    brightness: u8,   // 0–100
    resolution: u8,   // index into known resolutions
    scale: u8,        // 100, 125, 150, 200 (as index)
}

struct SoundSettings {
    volume:  u8,
    muted:   bool,
}

struct InputSettings {
    kb_repeat_delay: u16,
    mouse_speed: u8,
}

struct Settings {
    display: DisplaySettings,
    sound: SoundSettings,
    input: InputSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            display: DisplaySettings { brightness: 80, resolution: 0, scale: 0 },
            sound:   SoundSettings   { volume: 75, muted: false },
            input:   InputSettings   { kb_repeat_delay: 300, mouse_speed: 50 },
        }
    }
}

// ─── Tatami app ───────────────────────────────────────────────────────────────

struct Tatami {
    buf:       WindowBuffer,
    pfn_base:  u64,
    window_id: u64,
    section:   Section,
    settings:  Settings,
    dirty:     bool,
}

impl Tatami {
    fn new() -> Option<Self> {
        let sz   = (WIN_W * WIN_H * 4) as u64;
        let pfn  = unsafe { pdx_call(0, PDX_ALLOCATE_MEMORY, sz, 0) };
        if pfn == u64::MAX { return None; }
        let virt = unsafe { pdx_call(0, PDX_MAP_MEMORY, pfn, sz) };
        if virt == u64::MAX { return None; }

        let params = SexWindowCreateParams { x: 100, y: 80, width: WIN_W, height: WIN_H, pfn_base: pfn };
        let wid = unsafe {
            pdx_call(SEXDISPLAY_PD, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0)
        };
        if wid == 0 { return None; }

        Some(Tatami {
            buf: unsafe { WindowBuffer::new(virt, WIN_W, WIN_H, WIN_W) },
            pfn_base: pfn,
            window_id: wid,
            section: Section::Display,
            settings: Settings::default(),
            dirty: true,
        })
    }

    fn draw(&mut self) {
        if !self.dirty { return; }
        self.dirty = false;

        unsafe { self.buf.clear(BG); }

        self.draw_titlebar();
        self.draw_sidebar();
        self.draw_separator();
        self.draw_content();
        self.commit();
    }

    fn draw_titlebar(&mut self) {
        unsafe { self.buf.draw_rect(Rect { x: 0, y: 0, w: WIN_W, h: TITLE_H }, MANTLE); }
        font::draw_str(&mut self.buf, 16, 16, b"Settings", TEXT, None);
    }

    fn draw_sidebar(&mut self) {
        unsafe { self.buf.draw_rect(Rect { x: 0, y: TITLE_H as i32, w: SIDEBAR_W, h: WIN_H - TITLE_H }, MANTLE); }

        let mut sy = TITLE_H + 8;
        for &sec in Section::ALL {
            let active = sec == self.section;
            if active {
                unsafe {
                    self.buf.draw_rect(Rect { x: 4, y: sy as i32, w: SIDEBAR_W - 8, h: ROW_H - 4 }, SURFACE1);
                    // accent left bar
                    self.buf.draw_rect(Rect { x: 4, y: sy as i32, w: 3, h: ROW_H - 4 }, ACCENT);
                }
            }
            let col = if active { TEXT } else { SUBTEXT };
            font::draw_str(&mut self.buf, 20, sy + 12, sec.label(), col, None);
            sy += ROW_H;
        }
    }

    fn draw_separator(&mut self) {
        unsafe {
            self.buf.draw_rect(Rect { x: SIDEBAR_W as i32, y: TITLE_H as i32, w: 1, h: WIN_H - TITLE_H }, SURFACE0);
        }
    }

    fn draw_content(&mut self) {
        let cx = CONTENT_X;
        let mut cy = TITLE_H + 16;

        // Section heading
        font::draw_str(&mut self.buf, cx + 16, cy, self.section.label(), TEXT, None);
        cy += 32;
        unsafe {
            self.buf.draw_rect(Rect { x: cx as i32, y: cy as i32, w: CONTENT_W - 24, h: 1 }, SURFACE0);
        }
        cy += 12;

        match self.section {
            Section::Display      => self.draw_display(cx + 16, cy),
            Section::Network      => self.draw_network(cx + 16, cy),
            Section::Sound        => self.draw_sound(cx + 16, cy),
            Section::Input        => self.draw_input(cx + 16, cy),
            Section::Capabilities => self.draw_capabilities(cx + 16, cy),
            Section::About        => self.draw_about(cx + 16, cy),
        }
    }

    fn draw_slider(&mut self, x: u32, y: u32, label: &[u8], value: u8) {
        font::draw_str(&mut self.buf, x, y, label, SUBTEXT, None);
        let track_w = CONTENT_W - 160;
        let track_x = x + 120;
        unsafe {
            self.buf.draw_rect(Rect { x: track_x as i32, y: (y + 4) as i32, w: track_w, h: 6 }, SURFACE0);
        }
        let fill = (track_w * value as u32) / 100;
        if fill > 0 {
            unsafe {
                self.buf.draw_rect(Rect { x: track_x as i32, y: (y + 4) as i32, w: fill, h: 6 }, ACCENT);
            }
        }
        // knob
        unsafe {
            self.buf.draw_rect(
                Rect { x: (track_x + fill) as i32 - 4, y: y as i32, w: 8, h: 14 },
                TEXT,
            );
        }
    }

    fn draw_toggle(&mut self, x: u32, y: u32, label: &[u8], on: bool) {
        font::draw_str(&mut self.buf, x, y, label, SUBTEXT, None);
        let tx = x + 200;
        let bg = if on { ACCENT } else { SURFACE0 };
        unsafe {
            self.buf.draw_rect(Rect { x: tx as i32, y: y as i32, w: 36, h: 16 }, bg);
            let kx = if on { tx + 22 } else { tx + 2 };
            self.buf.draw_rect(Rect { x: kx as i32, y: (y + 2) as i32, w: 12, h: 12 }, TEXT);
        }
    }

    fn draw_display(&mut self, x: u32, mut y: u32) {
        self.draw_slider(x, y, b"Brightness", self.settings.display.brightness);
        y += 40;
        font::draw_str(&mut self.buf, x, y, b"Resolution", SUBTEXT, None);
        let resolutions: [&[u8]; 4] = [b"1920x1080", b"2560x1440", b"3840x2160", b"1280x720"];
        for (i, &res) in resolutions.iter().enumerate() {
            let ry = y + 20 + i as u32 * 28;
            let col = if i == self.settings.display.resolution as usize { ACCENT } else { SURFACE0 };
            unsafe { self.buf.draw_rect(Rect { x: x as i32 + 120, y: ry as i32, w: 14, h: 14 }, col); }
            font::draw_str(&mut self.buf, x + 140, ry, res, TEXT, None);
        }
        y += 140;
        font::draw_str(&mut self.buf, x, y, b"Scale", SUBTEXT, None);
        let scales: [&[u8]; 4] = [b"100%", b"125%", b"150%", b"200%"];
        for (i, &sc) in scales.iter().enumerate() {
            let sx = x + 120 + i as u32 * 72;
            let bg = if i == self.settings.display.scale as usize { ACCENT } else { SURFACE0 };
            unsafe { self.buf.draw_rect(Rect { x: sx as i32, y: y as i32, w: 64, h: 24 }, bg); }
            let fc = if i == self.settings.display.scale as usize { BG } else { TEXT };
            font::draw_str(&mut self.buf, sx + 12, y + 8, sc, fc, None);
        }
    }

    fn draw_network(&mut self, x: u32, y: u32) {
        // Embed silknet status view inline (simplified)
        font::draw_str(&mut self.buf, x, y, b"Open Network Settings...", ACCENT, None);
        font::draw_str(&mut self.buf, x, y + 24, b"(launches silknet window)", SUBTEXT, None);
    }

    fn draw_sound(&mut self, x: u32, mut y: u32) {
        self.draw_slider(x, y, b"Volume", self.settings.sound.volume);
        y += 40;
        self.draw_toggle(x, y, b"Mute", self.settings.sound.muted);
    }

    fn draw_input(&mut self, x: u32, mut y: u32) {
        self.draw_slider(x, y, b"Mouse Speed", self.settings.input.mouse_speed);
        y += 40;
        font::draw_str(&mut self.buf, x, y, b"KB Repeat Delay", SUBTEXT, None);
        // numeric display
        let d = self.settings.input.kb_repeat_delay;
        let mut nbuf = [0u8; 6];
        let mut n = d as u32;
        let mut nlen = 0usize;
        if n == 0 { nbuf[0] = b'0'; nlen = 1; } else {
            let mut tmp = [0u8; 6];
            while n > 0 { tmp[nlen] = b'0' + (n % 10) as u8; nlen += 1; n /= 10; }
            for i in 0..nlen { nbuf[i] = tmp[nlen - 1 - i]; }
        }
        font::draw_str(&mut self.buf, x + 200, y, &nbuf[..nlen], TEXT, None);
        font::draw_str(&mut self.buf, x + 200 + nlen as u32 * 8 + 4, y, b"ms", SUBTEXT, None);
    }

    fn draw_capabilities(&mut self, x: u32, mut y: u32) {
        font::draw_str(&mut self.buf, x, y, b"Active PDX Capabilities", SUBTEXT, None);
        y += 24;
        // Query active caps from kernel (placeholder list)
        let caps: &[&[u8]] = &[
            b"Display: sexdisplay (PD 1)",
            b"Input:   sexinput   (PD 3)",
            b"Network: sexnet     (PD 5)",
            b"Storage: sexdrive   (PD 6)",
            b"Audio:   sexaudio   (PD 7)",
        ];
        for &cap in caps {
            unsafe { self.buf.draw_rect(Rect { x: x as i32, y: y as i32, w: 8, h: 8 }, GREEN); }
            font::draw_str(&mut self.buf, x + 16, y, cap, TEXT, None);
            y += 22;
        }
        y += 12;
        font::draw_str(&mut self.buf, x, y, b"Microkernel Diagnostics", SUBTEXT, None);
        y += 22;
        font::draw_str(&mut self.buf, x, y, b"PDX calls/sec: --", YELLOW, None);
        font::draw_str(&mut self.buf, x + 200, y, b"IPC errors: 0", GREEN, None);
    }

    fn draw_about(&mut self, x: u32, mut y: u32) {
        font::draw_str(&mut self.buf, x, y, b"SexOS Microkernel", TEXT, None);
        y += 20;
        font::draw_str(&mut self.buf, x, y, b"Silk Desktop Environment", SUBTEXT, None);
        y += 36;
        let items: &[(&[u8], &[u8])] = &[
            (b"Kernel:",    b"Sex microkernel (SASOS/PDX)"),
            (b"Compositor:",b"SexCompositor (zero-copy PDX)"),
            (b"Shell:",     b"Silk Shell"),
            (b"Edition:",   b"2024"),
        ];
        for &(k, v) in items {
            font::draw_str(&mut self.buf, x, y, k, SUBTEXT, None);
            font::draw_str(&mut self.buf, x + 120, y, v, TEXT, None);
            y += 22;
        }
    }

    fn commit(&self) {
        unsafe { pdx_call(SEXDISPLAY_PD, PDX_WINDOW_COMMIT_FRAME, self.window_id, self.pfn_base); }
    }

    fn handle_click(&mut self, px: i32, py: i32) {
        // Sidebar navigation
        if px < SIDEBAR_W as i32 {
            let idx = (py - TITLE_H as i32 - 8) / ROW_H as i32;
            if idx >= 0 && (idx as usize) < Section::ALL.len() {
                self.section = Section::ALL[idx as usize];
                self.dirty = true;
            }
            return;
        }

        // Content-area interactions
        let cx = CONTENT_X as i32 + 16;
        let mut cy = TITLE_H as i32 + 60;

        match self.section {
            Section::Display => {
                // Brightness slider
                let track_x = cx + 120;
                let track_w = (CONTENT_W - 160) as i32;
                if py >= cy && py < cy + 20 && px >= track_x && px < track_x + track_w {
                    self.settings.display.brightness = ((px - track_x) * 100 / track_w) as u8;
                    self.dirty = true;
                }
                cy += 40;
                // Resolution buttons
                for i in 0..4usize {
                    let ry = cy + 20 + i as i32 * 28;
                    if py >= ry && py < ry + 20 && px >= cx + 120 && px < cx + 300 {
                        self.settings.display.resolution = i as u8;
                        self.dirty = true;
                    }
                }
                cy += 140;
                // Scale buttons
                for i in 0..4usize {
                    let sx = cx + 120 + i as i32 * 72;
                    if py >= cy && py < cy + 24 && px >= sx && px < sx + 64 {
                        self.settings.display.scale = i as u8;
                        self.dirty = true;
                    }
                }
            }
            Section::Sound => {
                let track_x = cx + 120;
                let track_w = (CONTENT_W - 160) as i32;
                if py >= cy && py < cy + 20 && px >= track_x && px < track_x + track_w {
                    self.settings.sound.volume = ((px - track_x) * 100 / track_w) as u8;
                    self.dirty = true;
                }
                cy += 40;
                if py >= cy && py < cy + 20 && px >= cx + 200 && px < cx + 240 {
                    self.settings.sound.muted = !self.settings.sound.muted;
                    self.dirty = true;
                }
            }
            Section::Network => {
                // Spawn silknet window on click
                if py >= cy && py < cy + 20 {
                    unsafe { pdx_call(0, sex_pdx::PDX_GET_TIME, 0, 0); } // placeholder spawn
                }
            }
            _ => {}
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut app = loop {
        if let Some(a) = Tatami::new() { break a; }
    };

    loop {
        app.draw();
        let req = pdx_listen(1);
        // HID click event
        if req.num == 0x10 {
            let ev_type = (req.arg0 >> 16) as u16;
            let value   = req.arg1 as i32;
            if ev_type == 1 && value == 1 {
                let px = (req.arg0 & 0xFFFF) as i32;
                let py = ((req.arg0 >> 32) & 0xFFFF) as i32;
                app.handle_click(px, py);
            }
        }
    }
}
