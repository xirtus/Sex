//! toys — Silk Desktop Environment pluggable widget framework.
//!
//! Based on the COSMIC applet system (libcosmic + cosmic-applets).
//! Widgets render into sub-regions of WindowBuffer via sex-graphics.
//! Pluggable into silkbar or as standalone desktop widgets.
//!
//! Widgets: Clock · CPU · RAM · Network · Calendar · Weather(stub)

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use sex_pdx::{pdx_call, PDX_GET_TIME, Rect};
use sex_graphics::{WindowBuffer, font};

// ─── Colour palette ───────────────────────────────────────────────────────────
const TEXT:     u32 = 0xFFCDD6F4;
const SUBTEXT:  u32 = 0xFFA6ADC8;
const ACCENT:   u32 = 0xFF89B4FA;
const GREEN:    u32 = 0xFFA6E3A1;
const YELLOW:   u32 = 0xFFF9E2AF;
const RED:      u32 = 0xFFF38BA8;
const SURFACE0: u32 = 0xFF313244;
const SURFACE1: u32 = 0xFF45475A;
const BG:       u32 = 0xFF1E1E2E;

// ─── Widget trait ─────────────────────────────────────────────────────────────

/// A widget that can render itself into a region of a WindowBuffer.
pub trait Widget {
    /// Update internal state (called before draw on each frame cycle).
    fn update(&mut self);
    /// Render into the given sub-region of buf.
    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, w: u32, h: u32);
    /// Preferred width in pixels.
    fn preferred_w(&self) -> u32;
    /// Preferred height in pixels.
    fn preferred_h(&self) -> u32;
    /// Handle a mouse click at (px, py) relative to widget origin.
    fn on_click(&mut self, _px: i32, _py: i32) {}
}

// ─── Clock widget ─────────────────────────────────────────────────────────────

pub struct ClockWidget {
    pub secs: u64,
    pub show_date: bool,
}

impl ClockWidget {
    pub fn new() -> Self { ClockWidget { secs: 0, show_date: false } }

    fn hhmm(&self) -> [u8; 5] {
        let hh = (self.secs / 3600) % 24;
        let mm = (self.secs / 60) % 60;
        [
            b'0' + (hh / 10) as u8,
            b'0' + (hh % 10) as u8,
            b':',
            b'0' + (mm / 10) as u8,
            b'0' + (mm % 10) as u8,
        ]
    }
}

impl Widget for ClockWidget {
    fn update(&mut self) {
        self.secs = unsafe { pdx_call(0, PDX_GET_TIME, 0, 0) };
    }
    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, _w: u32, h: u32) {
        let cy = y + (h.saturating_sub(font::CHAR_H)) / 2;
        let t = self.hhmm();
        font::draw_str(buf, x, cy, &t, TEXT, None);
    }
    fn preferred_w(&self) -> u32 { font::str_width(5) + 4 }
    fn preferred_h(&self) -> u32 { font::CHAR_H + 4 }
}

// ─── Bar widget (generic progress bar) ───────────────────────────────────────

pub struct BarWidget {
    pub label:  [u8; 8],
    pub llen:   usize,
    pub value:  u8,   // 0–100
    pub color:  u32,
}

impl BarWidget {
    pub fn new(label: &[u8], color: u32) -> Self {
        let mut l = [0u8; 8];
        let n = label.len().min(8);
        l[..n].copy_from_slice(&label[..n]);
        BarWidget { label: l, llen: n, value: 0, color }
    }
}

impl Widget for BarWidget {
    fn update(&mut self) {} // caller sets .value
    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, w: u32, h: u32) {
        // label
        font::draw_str(buf, x, y + 2, &self.label[..self.llen], SUBTEXT, None);
        let bx = x + font::str_width(self.llen) + 4;
        let bw = w.saturating_sub(bx - x + 4);
        if bw == 0 { return; }
        // track
        unsafe { buf.draw_rect(Rect { x: bx as i32, y: (y + 4) as i32, w: bw, h: h - 8 }, SURFACE0); }
        // fill
        let fill = (bw * self.value as u32) / 100;
        if fill > 0 {
            unsafe { buf.draw_rect(Rect { x: bx as i32, y: (y + 4) as i32, w: fill, h: h - 8 }, self.color); }
        }
        // percent text
        let pct = self.value;
        let mut pstr = [0u8; 4]; // "XXX%"
        let mut n = pct as u32;
        let mut plen = 0usize;
        if n == 0 { pstr[0] = b'0'; plen = 1; } else {
            let mut tmp = [0u8; 3];
            while n > 0 { tmp[plen] = b'0' + (n % 10) as u8; plen += 1; n /= 10; }
            for i in 0..plen { pstr[i] = tmp[plen - 1 - i]; }
        }
        pstr[plen] = b'%';
        plen += 1;
        let px = bx + bw + 4;
        font::draw_str(buf, px, y + 2, &pstr[..plen], TEXT, None);
    }
    fn preferred_w(&self) -> u32 { 140 }
    fn preferred_h(&self) -> u32 { font::CHAR_H + 8 }
}

// ─── CPU widget ───────────────────────────────────────────────────────────────

const PDX_GET_CPU_USAGE: u64 = 0x300; // Returns usage 0–100 as u64

pub struct CpuWidget {
    inner: BarWidget,
    history: [u8; 32],
    hist_pos: usize,
}

impl CpuWidget {
    pub fn new() -> Self {
        CpuWidget {
            inner: BarWidget::new(b"CPU", GREEN),
            history: [0; 32],
            hist_pos: 0,
        }
    }
}

impl Widget for CpuWidget {
    fn update(&mut self) {
        let v = unsafe { pdx_call(0, PDX_GET_CPU_USAGE, 0, 0) } as u8;
        self.inner.value = v;
        self.history[self.hist_pos % 32] = v;
        self.hist_pos = self.hist_pos.wrapping_add(1);
    }
    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, w: u32, h: u32) {
        self.inner.draw(buf, x, y, w, h / 2);
        // sparkline (history graph)
        self.draw_sparkline(buf, x, y + h / 2, w, h / 2);
    }
    fn preferred_w(&self) -> u32 { 160 }
    fn preferred_h(&self) -> u32 { 40 }
}

impl CpuWidget {
    fn draw_sparkline(&self, buf: &mut WindowBuffer, x: u32, y: u32, w: u32, h: u32) {
        if w < 2 || h < 2 { return; }
        let n = 32usize.min(w as usize);
        let step = w / n as u32;
        for i in 0..n {
            let idx = (self.hist_pos + i) % 32;
            let v = self.history[idx] as u32;
            let bh = (h * v) / 100;
            if bh > 0 {
                unsafe {
                    buf.draw_rect(
                        Rect {
                            x: (x + i as u32 * step) as i32,
                            y: (y + h - bh) as i32,
                            w: step.max(1),
                            h: bh,
                        },
                        GREEN,
                    );
                }
            }
        }
    }
}

// ─── RAM widget ───────────────────────────────────────────────────────────────

const PDX_GET_MEM_USAGE: u64 = 0x301; // Returns used_mb << 32 | total_mb

pub struct RamWidget {
    inner: BarWidget,
    used_mb: u32,
    total_mb: u32,
}

impl RamWidget {
    pub fn new() -> Self {
        RamWidget {
            inner: BarWidget::new(b"RAM", ACCENT),
            used_mb: 0,
            total_mb: 0,
        }
    }
}

impl Widget for RamWidget {
    fn update(&mut self) {
        let v = unsafe { pdx_call(0, PDX_GET_MEM_USAGE, 0, 0) };
        self.used_mb  = (v >> 32) as u32;
        self.total_mb = v as u32;
        if self.total_mb > 0 {
            self.inner.value = ((self.used_mb as u64 * 100) / self.total_mb as u64) as u8;
        }
    }
    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, w: u32, h: u32) {
        self.inner.draw(buf, x, y, w, h);
    }
    fn preferred_w(&self) -> u32 { 160 }
    fn preferred_h(&self) -> u32 { font::CHAR_H + 8 }
}

// ─── Calendar widget ──────────────────────────────────────────────────────────

pub struct CalendarWidget {
    pub day:   u8,
    pub month: u8,
    pub year:  u16,
}

impl CalendarWidget {
    pub fn new() -> Self { CalendarWidget { day: 1, month: 1, year: 2025 } }
}

impl Widget for CalendarWidget {
    fn update(&mut self) {
        // PDX_GET_DATE returns day | (month << 8) | (year << 16)
        let v = unsafe { pdx_call(0, 0x302, 0, 0) };
        self.day   = (v & 0xFF) as u8;
        self.month = ((v >> 8) & 0xFF) as u8;
        self.year  = ((v >> 16) & 0xFFFF) as u16;
    }

    fn draw(&self, buf: &mut WindowBuffer, x: u32, y: u32, _w: u32, _h: u32) {
        let months: [&[u8]; 12] = [
            b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun",
            b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
        ];
        let mi = self.month.saturating_sub(1).min(11) as usize;
        // "DD Mon YYYY"
        let d = self.day;
        let y_val = self.year;

        let mut s = [0u8; 12];
        s[0] = b'0' + (d / 10) as u8;
        s[1] = b'0' + (d % 10) as u8;
        s[2] = b' ';
        let mn = months[mi];
        s[3..3 + mn.len()].copy_from_slice(mn);
        s[6] = b' ';
        s[7] = b'0' + ((y_val / 1000) % 10) as u8;
        s[8] = b'0' + ((y_val / 100) % 10) as u8;
        s[9] = b'0' + ((y_val / 10) % 10) as u8;
        s[10]= b'0' + (y_val % 10) as u8;

        font::draw_str(buf, x, y, &s[..11], TEXT, None);
    }

    fn preferred_w(&self) -> u32 { font::str_width(11) + 4 }
    fn preferred_h(&self) -> u32 { font::CHAR_H + 4 }
}

// ─── Desktop widget host ─────────────────────────────────────────────────────
// Manages a collection of widgets at fixed desktop positions.

pub struct DesktopWidget {
    pub widget: alloc::boxed::Box<dyn Widget>,
    pub x: u32,
    pub y: u32,
}

pub struct DesktopWidgetHost {
    pub widgets: Vec<DesktopWidget>,
}

impl DesktopWidgetHost {
    pub fn new() -> Self { DesktopWidgetHost { widgets: Vec::new() } }

    pub fn add(&mut self, widget: alloc::boxed::Box<dyn Widget>, x: u32, y: u32) {
        self.widgets.push(DesktopWidget { widget, x, y });
    }

    pub fn update_all(&mut self) {
        for dw in &mut self.widgets { dw.widget.update(); }
    }

    pub fn draw_all(&self, buf: &mut WindowBuffer) {
        for dw in &self.widgets {
            let w = dw.widget.preferred_w();
            let h = dw.widget.preferred_h();
            dw.widget.draw(buf, dw.x, dw.y, w, h);
        }
    }

    pub fn dispatch_click(&mut self, px: i32, py: i32) {
        for dw in &mut self.widgets {
            let x = dw.x as i32;
            let y = dw.y as i32;
            let w = dw.widget.preferred_w() as i32;
            let h = dw.widget.preferred_h() as i32;
            if px >= x && px < x + w && py >= y && py < y + h {
                dw.widget.on_click(px - x, py - y);
            }
        }
    }
}
