//! silknet — WiFi/VPN GUI applet for Silk Desktop Environment.
//!
//! Dual-mode: tray icon embedded in silkbar, or full windowed settings GUI.
//! Based on cosmic-applet-network architecture; uses silk-client surfaces
//! and PDX capability-aware VPN sandboxing.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use sex_pdx::{
    pdx_call,
    PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY, PDX_MAP_MEMORY,
    PDX_WINDOW_COMMIT_FRAME,
    PDX_SILKBAR_REGISTER, PDX_SILKBAR_NOTIFY,
    SexWindowCreateParams, SilkbarRegisterParams, SilkbarNotifyParams,
    Rect,
};
use sex_graphics::{WindowBuffer, font};

const SEXDISPLAY_PD:    u32 = 1;
const SILKBAR_PD:       u32 = 2;
const SEXNET_PD:        u32 = 5;

pub const SEXNET_GET_STATUS:  u64 = 0x200;
pub const SEXNET_SCAN_WIFI:   u64 = 0x201;
pub const SEXNET_CONNECT:     u64 = 0x202;
pub const SEXNET_DISCONNECT:  u64 = 0x203;
pub const SEXNET_VPN_UP:      u64 = 0x204;
pub const SEXNET_VPN_DOWN:    u64 = 0x205;

// ─── Network state ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetState {
    Disconnected,
    Connecting,
    Connected,
    VpnActive,
    Error,
}

#[derive(Clone)]
pub struct WifiNetwork {
    pub ssid:     [u8; 32],
    pub ssid_len: usize,
    pub signal:   u8,
    pub secured:  bool,
}

pub struct SilkNetState {
    pub state:        NetState,
    pub ssid:         [u8; 32],
    pub ssid_len:     usize,
    pub vpn_active:   bool,
    pub scan_results: Vec<WifiNetwork>,
}

impl Default for SilkNetState {
    fn default() -> Self {
        SilkNetState {
            state: NetState::Disconnected,
            ssid: [0; 32],
            ssid_len: 0,
            vpn_active: false,
            scan_results: Vec::new(),
        }
    }
}

// ─── Tray applet ─────────────────────────────────────────────────────────────

pub struct SilkNetApplet {
    pub net: SilkNetState,
}

impl SilkNetApplet {
    pub fn new() -> Self {
        SilkNetApplet { net: SilkNetState::default() }
    }

    pub fn register_with_silkbar(&self, own_pd: u32) {
        let mut p = SilkbarRegisterParams { name: [0; 32], applet_pd: own_pd };
        let n = b"silknet";
        p.name[..n.len()].copy_from_slice(n);
        unsafe { pdx_call(SILKBAR_PD, PDX_SILKBAR_REGISTER, &p as *const _ as u64, 0); }
    }

    pub fn poll_status(&mut self) {
        let res = unsafe { pdx_call(SEXNET_PD, SEXNET_GET_STATUS, 0, 0) };
        self.net.state = match (res >> 62) & 0x3 {
            0 => NetState::Disconnected,
            1 => NetState::Connecting,
            2 => NetState::Connected,
            3 => NetState::VpnActive,
            _ => NetState::Error,
        };
        self.net.vpn_active = self.net.state == NetState::VpnActive;
    }

    pub fn notify_silkbar(&self, own_pd: u32) {
        let mut p = SilkbarNotifyParams {
            applet_pd: own_pd,
            text: [0; 32],
            icon_state: match self.net.state {
                NetState::Connected | NetState::VpnActive => 1,
                NetState::Error => 2,
                _ => 0,
            },
        };
        let label: &[u8] = match self.net.state {
            NetState::Connected    => b"WiFi",
            NetState::VpnActive   => b"VPN",
            NetState::Connecting  => b"...",
            NetState::Error       => b"ERR",
            NetState::Disconnected => b"Off",
        };
        p.text[..label.len()].copy_from_slice(label);
        unsafe { pdx_call(SILKBAR_PD, PDX_SILKBAR_NOTIFY, &p as *const _ as u64, 0); }
    }

    pub fn scan_wifi(&self) {
        unsafe { pdx_call(SEXNET_PD, SEXNET_SCAN_WIFI, 0, 0); }
    }

    pub fn connect(&self, ssid: &[u8], pass: &[u8]) {
        unsafe { pdx_call(SEXNET_PD, SEXNET_CONNECT, ssid.as_ptr() as u64, pass.as_ptr() as u64); }
    }

    pub fn vpn_up(&self, config_pfn: u64) {
        unsafe { pdx_call(SEXNET_PD, SEXNET_VPN_UP, config_pfn, 0); }
    }

    pub fn vpn_down(&self) {
        unsafe { pdx_call(SEXNET_PD, SEXNET_VPN_DOWN, 0, 0); }
    }
}

// ─── Full windowed settings GUI ──────────────────────────────────────────────

const WIN_W: u32 = 360;
const WIN_H: u32 = 480;

const BG:       u32 = 0xFF1E1E2E;
const SURFACE0: u32 = 0xFF313244;
const SURFACE1: u32 = 0xFF45475A;
const TEXT:     u32 = 0xFFCDD6F4;
const SUBTEXT:  u32 = 0xFFA6ADC8;
const ACCENT:   u32 = 0xFF89B4FA;
const GREEN:    u32 = 0xFFA6E3A1;
const RED:      u32 = 0xFFF38BA8;

pub struct SilkNetWindow {
    pub applet:  SilkNetApplet,
    buf:         WindowBuffer,
    pfn_base:    u64,
    window_id:   u64,
    pub selected: Option<usize>,
}

impl SilkNetWindow {
    pub fn open() -> Option<Self> {
        let sz   = (WIN_W * WIN_H * 4) as u64;
        let pfn  = unsafe { pdx_call(0, PDX_ALLOCATE_MEMORY, sz, 0) };
        if pfn == u64::MAX { return None; }
        let virt = unsafe { pdx_call(0, PDX_MAP_MEMORY, pfn, sz) };
        if virt == u64::MAX { return None; }

        let params = SexWindowCreateParams { x: 200, y: 200, width: WIN_W, height: WIN_H, pfn_base: pfn };
        let wid = unsafe {
            pdx_call(SEXDISPLAY_PD, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0)
        };
        if wid == 0 { return None; }

        let mut w = SilkNetWindow {
            applet: SilkNetApplet::new(),
            buf: unsafe { WindowBuffer::new(virt, WIN_W, WIN_H, WIN_W) },
            pfn_base: pfn,
            window_id: wid,
            selected: None,
        };
        w.applet.poll_status();
        w.applet.scan_wifi();
        Some(w)
    }

    pub fn draw(&mut self) {
        unsafe { self.buf.clear(BG); }

        // Title bar
        unsafe { self.buf.draw_rect(Rect { x: 0, y: 0, w: WIN_W, h: 40 }, SURFACE0); }
        font::draw_str(&mut self.buf, 16, 14, b"Network Settings", TEXT, None);

        // Status card
        unsafe { self.buf.draw_rect(Rect { x: 12, y: 52, w: WIN_W - 24, h: 48 }, SURFACE0); }
        let (status_label, status_col): (&[u8], u32) = match self.applet.net.state {
            NetState::Connected    => (b"Connected",     GREEN),
            NetState::VpnActive   => (b"VPN Active",    GREEN),
            NetState::Connecting  => (b"Connecting...", ACCENT),
            NetState::Disconnected => (b"Disconnected", SUBTEXT),
            NetState::Error       => (b"Error",         RED),
        };
        font::draw_str(&mut self.buf, 24, 60, b"Status:", SUBTEXT, None);
        font::draw_str(&mut self.buf, 24, 76, status_label, status_col, None);

        // VPN toggle
        let (vpn_txt, vpn_col) = if self.applet.net.vpn_active {
            (b"VPN: ON " as &[u8], GREEN)
        } else {
            (b"VPN: OFF" as &[u8], SUBTEXT)
        };
        unsafe { self.buf.draw_rect(Rect { x: WIN_W as i32 - 100, y: 52, w: 88, h: 28 }, SURFACE1); }
        font::draw_str(&mut self.buf, WIN_W - 96, 62, vpn_txt, vpn_col, None);

        // Network list header
        font::draw_str(&mut self.buf, 16, 116, b"Available Networks", SUBTEXT, None);
        unsafe { self.buf.draw_rect(Rect { x: 12, y: 128, w: WIN_W - 24, h: 1 }, SURFACE1); }

        // Network rows — collect row data first to avoid borrow conflict
        struct RowInfo { ssid: [u8; 32], slen: usize, secured: bool, signal: u8 }
        let rows: alloc::vec::Vec<RowInfo> = self.applet.net.scan_results.iter().map(|n| RowInfo {
            ssid: n.ssid, slen: n.ssid_len.min(22), secured: n.secured, signal: n.signal,
        }).collect();

        let mut ly = 136u32;
        for (i, row) in rows.iter().enumerate() {
            if ly + 36 > WIN_H - 56 { break; }
            let row_bg = if self.selected == Some(i) { SURFACE1 } else { SURFACE0 };
            unsafe { self.buf.draw_rect(Rect { x: 12, y: ly as i32, w: WIN_W - 24, h: 32 }, row_bg); }
            font::draw_str(&mut self.buf, 24, ly + 10, &row.ssid[..row.slen], TEXT, None);
            if row.secured {
                font::draw_char(&mut self.buf, WIN_W - 64, ly + 10, b'*', SUBTEXT, None);
            }
            self.draw_signal_bars(WIN_W - 48, ly + 10, row.signal);
            ly += 36;
        }

        // Action buttons
        unsafe { self.buf.draw_rect(Rect { x: 12, y: WIN_H as i32 - 48, w: 80, h: 32 }, ACCENT); }
        font::draw_str(&mut self.buf, 28, WIN_H - 38, b"Scan", BG, None);

        if self.selected.is_some() {
            unsafe { self.buf.draw_rect(Rect { x: 104, y: WIN_H as i32 - 48, w: 100, h: 32 }, ACCENT); }
            font::draw_str(&mut self.buf, 116, WIN_H - 38, b"Connect", BG, None);
        }

        unsafe { pdx_call(SEXDISPLAY_PD, PDX_WINDOW_COMMIT_FRAME, self.window_id, self.pfn_base); }
    }

    fn draw_signal_bars(&mut self, x: u32, y: u32, signal: u8) {
        let bars = (signal / 25).min(4) as u32;
        for b in 0..4u32 {
            let bh = (b + 1) * 3;
            let col = if b < bars { ACCENT } else { SURFACE1 };
            unsafe {
                self.buf.draw_rect(
                    Rect { x: (x + b * 6) as i32, y: (y + 12 - bh) as i32, w: 4, h: bh },
                    col,
                );
            }
        }
    }

    pub fn handle_click(&mut self, px: i32, py: i32) {
        // Scan button
        if px >= 12 && px < 92 && py >= WIN_H as i32 - 48 && py < WIN_H as i32 - 16 {
            self.applet.scan_wifi();
            return;
        }
        // VPN toggle
        if px >= WIN_W as i32 - 100 && px < WIN_W as i32 && py >= 52 && py < 80 {
            if self.applet.net.vpn_active {
                self.applet.vpn_down();
                self.applet.net.vpn_active = false;
            } else {
                self.applet.vpn_up(0);
                self.applet.net.vpn_active = true;
            }
            return;
        }
        // Row selection
        let mut ly = 136i32;
        for i in 0..self.applet.net.scan_results.len() {
            if py >= ly && py < ly + 32 { self.selected = Some(i); return; }
            ly += 36;
        }
    }
}
