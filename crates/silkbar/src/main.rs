//! silkbar — Silk Desktop Environment panel daemon.
//!
//! Anchored full-width bar at screen bottom. Zones (left→right):
//!   [launcher] [task list ···] [tray applets] [clock]
//!
//! Inspired by cosmic-panel architecture; surface is silk-client +
//! SexCompositor zero-copy frames, not iced/winit.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use sex_graphics::{WindowBuffer, font};
use sex_pdx::{
    pdx_call, pdx_listen, pdx_reply, PdxRequest,
    PDX_SEX_WINDOW_CREATE, PDX_MOVE_WINDOW, PDX_ALLOCATE_MEMORY, PDX_MAP_MEMORY,
    PDX_WINDOW_COMMIT_FRAME, PDX_GET_DISPLAY_INFO, PDX_GET_TIME,
    PDX_SILKBAR_REGISTER, PDX_SILKBAR_UNREGISTER,
    PDX_SILKBAR_NOTIFY, PDX_SILKBAR_WINDOW_OPEN,
    PDX_SILKBAR_WINDOW_CLOSE, PDX_SILKBAR_WINDOW_FOCUS,
    SexWindowCreateParams, SilkbarRegisterParams, SilkbarNotifyParams,
};
use sex_pdx::Rect;

// ─── Bump allocator (256 MiB arena at 0x4000_0000) ───────────────────────────
const HEAP_START: usize = 0x4000_0000;
const HEAP_END: usize = HEAP_START + 256 * 1024 * 1024;
static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);

struct BumpAlloc;
unsafe impl core::alloc::GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut cur = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let aligned = (cur + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();
            if next > HEAP_END {
                return core::ptr::null_mut();
            }
            match HEAP_TOP.compare_exchange_weak(cur, next, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => return aligned as *mut u8,
                Err(x) => cur = x,
            }
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAlloc = BumpAlloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ─── Colours (0xAARRGGBB) ────────────────────────────────────────────────────
const BG:            u32 = 0xFF1E1E2E; // Catppuccin Mocha Base
const SURFACE0:      u32 = 0xFF313244;
const SURFACE1:      u32 = 0xFF45475A;
const TEXT:          u32 = 0xFFCDD6F4;
const SUBTEXT:       u32 = 0xFFA6ADC8;
const ACCENT:        u32 = 0xFF89B4FA; // Blue
const GREEN:         u32 = 0xFFA6E3A1;
const RED:           u32 = 0xFFF38BA8;
const SEPARATOR:     u32 = 0xFF585B70;

const PANEL_H:       u32 = 40;
const TASK_W:        u32 = 120;
const TASK_H:        u32 = 30;
const TRAY_SLOT_W:   u32 = 32;
const LAUNCHER_W:    u32 = 44;
const CLOCK_CHARS:   u32 = 5; // "HH:MM"
const CLOCK_W:       u32 = CLOCK_CHARS * font::CHAR_W + 16;
const MARGIN:        u32 = 8;

const SEXDISPLAY_PD: u32 = 1;

// ─── Data structures ─────────────────────────────────────────────────────────

struct TaskEntry {
    id: u64,
    title: [u8; 32],
    title_len: usize,
    focused: bool,
}

struct TrayApplet {
    pd: u32,
    name: [u8; 32],
    text: [u8; 32],
    text_len: usize,
    state: u8, // 0=idle 1=active 2=alert
}

struct SilkBar {
    buf:          WindowBuffer,
    pfn_base:     u64,
    window_id:    u64,
    screen_w:     u32,
    tasks:        Vec<TaskEntry>,
    applets:      Vec<TrayApplet>,
    focused_id:   u64,
    clock_secs:   u64,
    dirty:        bool,
}

impl SilkBar {
    fn new() -> Option<Self> {
        // Query screen dimensions
        let info = unsafe { pdx_call(SEXDISPLAY_PD, PDX_GET_DISPLAY_INFO, 0, 0) };
        let screen_w = (info >> 32) as u32;
        let screen_h = info as u32;
        if screen_w == 0 || screen_h == 0 {
            return None;
        }

        // Allocate framebuffer
        let buf_size = (screen_w * PANEL_H * 4) as u64;
        let pfn = unsafe { pdx_call(0, PDX_ALLOCATE_MEMORY, buf_size, 0) };
        if pfn == u64::MAX { return None; }
        let virt = unsafe { pdx_call(0, PDX_MAP_MEMORY, pfn, buf_size) };
        if virt == u64::MAX { return None; }

        // Create compositor window
        let params = SexWindowCreateParams {
            x: 0,
            y: (screen_h - PANEL_H) as i32,
            width: screen_w,
            height: PANEL_H,
            pfn_base: pfn,
        };
        let wid = unsafe {
            pdx_call(SEXDISPLAY_PD, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0)
        };
        if wid == 0 { return None; }

        let buf = unsafe { WindowBuffer::new(virt, screen_w, PANEL_H, screen_w) };

        Some(SilkBar {
            buf,
            pfn_base: pfn,
            window_id: wid,
            screen_w,
            tasks: Vec::new(),
            applets: Vec::new(),
            focused_id: 0,
            clock_secs: 0,
            dirty: true,
        })
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    fn redraw(&mut self) {
        if !self.dirty { return; }
        self.dirty = false;

        unsafe { self.buf.clear(BG); }

        let cy = (PANEL_H - font::CHAR_H) / 2; // vertical centre of text

        // ── Launcher button ──────────────────────────────────────────────────
        self.draw_launcher_btn(MARGIN, cy);

        // ── Task buttons ─────────────────────────────────────────────────────
        let mut tx = MARGIN + LAUNCHER_W + MARGIN;
        for i in 0..self.tasks.len() {
            let focused = self.tasks[i].focused;
            let title = self.tasks[i].title;
            let tlen  = self.tasks[i].title_len;
            self.draw_task_btn(tx, 5, focused, &title[..tlen]);
            tx += TASK_W + 4;
        }

        // ── Right-side layout (tray + clock) — compute from screen edge ──────
        let clock_x = self.screen_w - CLOCK_W - MARGIN;
        let tray_x  = clock_x - (self.applets.len() as u32) * (TRAY_SLOT_W + 4) - MARGIN;

        // separator before tray
        unsafe {
            self.buf.draw_rect(
                Rect { x: tray_x as i32 - 2, y: 6, w: 1, h: PANEL_H - 12 },
                SEPARATOR,
            );
        }

        // ── Tray applets ─────────────────────────────────────────────────────
        let mut ax = tray_x;
        for i in 0..self.applets.len() {
            let state = self.applets[i].state;
            let text  = self.applets[i].text;
            let tlen  = self.applets[i].text_len;
            self.draw_tray_icon(ax, cy, state, &text[..tlen]);
            ax += TRAY_SLOT_W + 4;
        }

        // ── Clock ────────────────────────────────────────────────────────────
        self.draw_clock(clock_x, cy);

        self.commit();
    }

    fn draw_launcher_btn(&mut self, x: u32, _cy: u32) {
        // 3×3 dot grid (cosmic-panel style launcher)
        let dot = 4u32;
        let gap = 7u32;
        let oy  = (PANEL_H - (dot * 3 + gap * 2)) / 2;
        for row in 0..3u32 {
            for col in 0..3u32 {
                let rx = x + col * (dot + gap);
                let ry = oy + row * (dot + gap);
                unsafe {
                    self.buf.draw_rect(Rect { x: rx as i32, y: ry as i32, w: dot, h: dot }, ACCENT);
                }
            }
        }
    }

    fn draw_task_btn(&mut self, x: u32, y: u32, focused: bool, title: &[u8]) {
        let bg = if focused { SURFACE1 } else { SURFACE0 };
        unsafe {
            self.buf.draw_rect(Rect { x: x as i32, y: y as i32, w: TASK_W, h: TASK_H }, bg);
        }
        // focused accent bar at bottom
        if focused {
            unsafe {
                self.buf.draw_rect(
                    Rect { x: x as i32, y: (y + TASK_H - 2) as i32, w: TASK_W, h: 2 },
                    ACCENT,
                );
            }
        }
        // title text (truncate to fit)
        let max_chars = ((TASK_W - 8) / font::CHAR_W) as usize;
        let show = if title.len() > max_chars { max_chars } else { title.len() };
        let tx = x + 4;
        let ty = y + (TASK_H - font::CHAR_H) / 2;
        font::draw_str(&mut self.buf, tx, ty, &title[..show], TEXT, None);
    }

    fn draw_tray_icon(&mut self, x: u32, cy: u32, state: u8, text: &[u8]) {
        let col = match state {
            1 => GREEN,
            2 => RED,
            _ => SUBTEXT,
        };
        // small square icon placeholder
        unsafe {
            self.buf.draw_rect(Rect { x: x as i32, y: (cy + 1) as i32, w: 12, h: 12 }, col);
        }
        // short label beside icon
        let max = ((TRAY_SLOT_W - 16) / font::CHAR_W) as usize;
        let show = if text.len() > max { max } else { text.len() };
        if show > 0 {
            font::draw_str(&mut self.buf, x + 14, cy, &text[..show], col, None);
        }
    }

    fn draw_clock(&mut self, x: u32, cy: u32) {
        let secs  = self.clock_secs;
        let hh    = (secs / 3600) % 24;
        let mm    = (secs / 60) % 60;
        let mut s = [0u8; 5];
        s[0] = b'0' + (hh / 10) as u8;
        s[1] = b'0' + (hh % 10) as u8;
        s[2] = b':';
        s[3] = b'0' + (mm / 10) as u8;
        s[4] = b'0' + (mm % 10) as u8;
        font::draw_str(&mut self.buf, x + 8, cy, &s, TEXT, None);
    }

    fn commit(&self) {
        unsafe {
            pdx_call(
                SEXDISPLAY_PD,
                PDX_WINDOW_COMMIT_FRAME,
                self.window_id,
                self.pfn_base,
            );
        }
    }

    // ── Event handlers ───────────────────────────────────────────────────────

    fn handle_window_open(&mut self, id: u64, title_ptr: u64) {
        let mut entry = TaskEntry { id, title: [0; 32], title_len: 0, focused: false };
        if title_ptr != 0 {
            let slice = unsafe { core::slice::from_raw_parts(title_ptr as *const u8, 32) };
            let len = slice.iter().position(|&b| b == 0).unwrap_or(32);
            entry.title[..len].copy_from_slice(&slice[..len]);
            entry.title_len = len;
        }
        self.tasks.push(entry);
        self.dirty = true;
    }

    fn handle_window_close(&mut self, id: u64) {
        self.tasks.retain(|t| t.id != id);
        self.dirty = true;
    }

    fn handle_window_focus(&mut self, id: u64) {
        self.focused_id = id;
        for t in &mut self.tasks {
            t.focused = t.id == id;
        }
        self.dirty = true;
    }

    fn handle_applet_register(&mut self, caller: u32, params_ptr: u64) {
        if params_ptr == 0 { return; }
        let p = unsafe { &*(params_ptr as *const SilkbarRegisterParams) };
        let mut name = [0u8; 32];
        name.copy_from_slice(&p.name);
        let applet = TrayApplet {
            pd: caller,
            name,
            text: [0; 32],
            text_len: 0,
            state: 0,
        };
        self.applets.push(applet);
        self.dirty = true;
    }

    fn handle_applet_unregister(&mut self, caller: u32) {
        self.applets.retain(|a| a.pd != caller);
        self.dirty = true;
    }

    fn handle_applet_notify(&mut self, caller: u32, params_ptr: u64) {
        if params_ptr == 0 { return; }
        let p = unsafe { &*(params_ptr as *const SilkbarNotifyParams) };
        if let Some(a) = self.applets.iter_mut().find(|a| a.pd == caller) {
            a.state = p.icon_state;
            a.text.copy_from_slice(&p.text);
            let len = p.text.iter().position(|&b| b == 0).unwrap_or(32);
            a.text_len = len;
            self.dirty = true;
        }
    }

    fn handle_click(&mut self, px: i32, _py: i32) {
        // Check launcher
        if px < LAUNCHER_W as i32 + MARGIN as i32 * 2 {
            self.open_launcher();
            return;
        }
        // Check task buttons
        let mut tx = (MARGIN + LAUNCHER_W + MARGIN) as i32;
        for t in &self.tasks {
            if px >= tx && px < tx + TASK_W as i32 {
                let wid = t.id;
                unsafe { pdx_call(SEXDISPLAY_PD, sex_pdx::PDX_FOCUS_WINDOW, wid, 0); }
                return;
            }
            tx += (TASK_W + 4) as i32;
        }
    }

    fn open_launcher(&self) {
        // Spawn silk-launcher or sex-forge app menu
        // Placeholder: signal launcher PD (PD 10 by convention)
        unsafe { pdx_call(10, 0, 0, 0); }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut bar = loop {
        if let Some(b) = SilkBar::new() { break b; }
    };

    // Request initial clock value
    bar.clock_secs = unsafe { pdx_call(0, PDX_GET_TIME, 0, 0) };

    let mut tick: u64 = 0;
    const REDRAW_TICKS: u64 = 1000; // approximate: redraw every N iterations

    loop {
        bar.redraw();

        // Non-blocking poll for messages (flags = 1 = PDXLISTEN_POLL)
        let req: PdxRequest = pdx_listen(1);

        match req.num {
            PDX_SILKBAR_WINDOW_OPEN   => bar.handle_window_open(req.arg0, req.arg1),
            PDX_SILKBAR_WINDOW_CLOSE  => bar.handle_window_close(req.arg0),
            PDX_SILKBAR_WINDOW_FOCUS  => bar.handle_window_focus(req.arg0),
            PDX_SILKBAR_REGISTER      => {
                bar.handle_applet_register(req.caller_pd, req.arg0);
                unsafe { pdx_reply(req.caller_pd, 0); }
            }
            PDX_SILKBAR_UNREGISTER    => {
                bar.handle_applet_unregister(req.caller_pd);
                unsafe { pdx_reply(req.caller_pd, 0); }
            }
            PDX_SILKBAR_NOTIFY        => bar.handle_applet_notify(req.caller_pd, req.arg0),
            // HID mouse click event forwarded from sexinput
            0x10 /* HIDEvent click */ => {
                let ev_type = (req.arg0 >> 16) as u16;
                let value   = req.arg1 as i32;
                if ev_type == 1 && value == 1 {
                    let px = (req.arg0 & 0xFFFF) as i32;
                    let py = ((req.arg0 >> 32) & 0xFFFF) as i32;
                    bar.handle_click(px, py);
                }
            }
            _ => {}
        }

        // Periodic clock update
        tick = tick.wrapping_add(1);
        if tick % REDRAW_TICKS == 0 {
            let t = unsafe { pdx_call(0, PDX_GET_TIME, 0, 0) };
            if t != bar.clock_secs {
                bar.clock_secs = t;
                bar.dirty = true;
            }
        }
    }
}
