//! sexsh v2 — Silk terminal emulator
//!
//! Architecture:
//!   • VT100/ANSI escape parser (no_std, state machine)
//!   • Cell grid rendered via sex-graphics CP437 font onto zero-copy PDX surface
//!   • Input from sexinput HID events via PDX
//!   • Built-in command set; external commands via sex-ld launcher PDX calls

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

use sex_pdx::{
    pdx_call, pdx_listen,
    PDX_SEX_WINDOW_CREATE, SexWindowCreateParams,
};
use sex_graphics::WindowBuffer;
use sex_graphics::font;

// --------------------------------------------------------------------------
// Terminal geometry
// --------------------------------------------------------------------------

const CELL_W: u32 = 8;
const CELL_H: u32 = 8;
const COLS: u32 = 100;
const ROWS: u32 = 36;
const WIN_W: u32 = COLS * CELL_W;  // 800
const WIN_H: u32 = ROWS * CELL_H;  // 288

// Framebuffer lives at 256 MiB physical; kernel maps it on window create.
const FB_PFN_BASE: u64 = 0x0000_0004_0000;

// --------------------------------------------------------------------------
// Catppuccin Mocha palette
// --------------------------------------------------------------------------

const BG:      u32 = 0xFF1E1E2E;
const FG:      u32 = 0xFFCDD6F4;
const CURSOR:  u32 = 0xFFF5E0DC;
const C_RED:   u32 = 0xFFF38BA8;
const C_GREEN: u32 = 0xFFA6E3A1;
const C_YELLW: u32 = 0xFFF9E2AF;
const C_BLUE:  u32 = 0xFF89B4FA;
const C_MAGEN: u32 = 0xFFCBA4F7;
const C_CYAN:  u32 = 0xFF89DCEB;
const C_WHITE: u32 = 0xFFBAC2DE;

const ANSI_FG: [u32; 8] = [BG, C_RED, C_GREEN, C_YELLW, C_BLUE, C_MAGEN, C_CYAN, C_WHITE];
const ANSI_BG: [u32; 8] = [BG, C_RED, C_GREEN, C_YELLW, C_BLUE, C_MAGEN, C_CYAN, C_WHITE];

// --------------------------------------------------------------------------
// Cell
// --------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Cell {
    ch: u8,
    fg: u32,
    bg: u32,
    dirty: bool,
}

impl Cell {
    const fn blank() -> Self {
        Cell { ch: b' ', fg: FG, bg: BG, dirty: true }
    }
}

// --------------------------------------------------------------------------
// VT100 parser
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum ParseState { Normal, Escape, CsiParam }

struct Vt100 {
    cells: [[Cell; COLS as usize]; ROWS as usize],
    cx: u32,
    cy: u32,
    saved_cx: u32,
    saved_cy: u32,
    fg: u32,
    bg: u32,
    state: ParseState,
    csi_buf: [u8; 32],
    csi_len: usize,
    dirty_all: bool,
}

impl Vt100 {
    fn new() -> Self {
        Vt100 {
            cells: [[Cell::blank(); COLS as usize]; ROWS as usize],
            cx: 0, cy: 0,
            saved_cx: 0, saved_cy: 0,
            fg: FG, bg: BG,
            state: ParseState::Normal,
            csi_buf: [0u8; 32],
            csi_len: 0,
            dirty_all: true,
        }
    }

    fn put_char(&mut self, ch: u8) {
        if self.cx >= COLS { self.cx = 0; self.advance_row(); }
        self.cells[self.cy as usize][self.cx as usize] =
            Cell { ch, fg: self.fg, bg: self.bg, dirty: true };
        self.cx += 1;
    }

    fn advance_row(&mut self) {
        self.cy += 1;
        if self.cy >= ROWS { self.scroll_up(); self.cy = ROWS - 1; }
    }

    fn scroll_up(&mut self) {
        for r in 0..(ROWS as usize - 1) {
            self.cells[r] = self.cells[r + 1];
            for c in 0..COLS as usize { self.cells[r][c].dirty = true; }
        }
        for c in 0..COLS as usize {
            self.cells[(ROWS - 1) as usize][c] = Cell::blank();
        }
    }

    fn clear_screen(&mut self) {
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() { *cell = Cell::blank(); }
        }
        self.dirty_all = true;
    }

    fn clear_eol(&mut self) {
        for c in self.cx as usize..COLS as usize {
            self.cells[self.cy as usize][c] = Cell::blank();
        }
    }

    fn apply_sgr(&mut self, params: &[u8], np: usize) {
        if np == 0 { self.fg = FG; self.bg = BG; return; }
        for i in 0..np {
            match params[i] {
                0 => { self.fg = FG; self.bg = BG; }
                30..=37 => { self.fg = ANSI_FG[(params[i] - 30) as usize]; }
                39 => { self.fg = FG; }
                40..=47 => { self.bg = ANSI_BG[(params[i] - 40) as usize]; }
                49 => { self.bg = BG; }
                _ => {}
            }
        }
    }

    fn feed(&mut self, b: u8) {
        match self.state {
            ParseState::Normal => match b {
                0x1B => { self.state = ParseState::Escape; }
                b'\n' => { self.advance_row(); }
                b'\r' => { self.cx = 0; }
                0x08 => { if self.cx > 0 { self.cx -= 1; } }
                _ => { self.put_char(b); }
            },
            ParseState::Escape => {
                match b {
                    b'[' => { self.state = ParseState::CsiParam; self.csi_len = 0; }
                    b'7' => { self.saved_cx = self.cx; self.saved_cy = self.cy; self.state = ParseState::Normal; }
                    b'8' => { self.cx = self.saved_cx; self.cy = self.saved_cy; self.state = ParseState::Normal; }
                    _ => { self.state = ParseState::Normal; }
                }
            },
            ParseState::CsiParam => {
                if b.is_ascii_digit() || b == b';' {
                    if self.csi_len < 31 { self.csi_buf[self.csi_len] = b; self.csi_len += 1; }
                } else {
                    self.dispatch_csi(b);
                    self.state = ParseState::Normal;
                }
            }
        }
    }

    fn dispatch_csi(&mut self, cmd: u8) {
        let mut params = [0u8; 8];
        let mut np = 0usize;
        let mut cur = 0u32;
        for i in 0..self.csi_len {
            let b = self.csi_buf[i];
            if b == b';' {
                if np < 8 { params[np] = cur.min(255) as u8; np += 1; }
                cur = 0;
            } else if b.is_ascii_digit() {
                cur = cur * 10 + (b - b'0') as u32;
            }
        }
        if np < 8 { params[np] = cur.min(255) as u8; np += 1; }

        let p0 = params[0] as u32;
        let p1 = params[1] as u32;

        match cmd {
            b'A' => { self.cy = self.cy.saturating_sub(p0.max(1)); }
            b'B' => { self.cy = (self.cy + p0.max(1)).min(ROWS - 1); }
            b'C' => { self.cx = (self.cx + p0.max(1)).min(COLS - 1); }
            b'D' => { self.cx = self.cx.saturating_sub(p0.max(1)); }
            b'H' | b'f' => {
                self.cy = p0.saturating_sub(1).min(ROWS - 1);
                self.cx = p1.saturating_sub(1).min(COLS - 1);
            }
            b'J' => match p0 {
                0 => for r in self.cy as usize..ROWS as usize { for c in 0..COLS as usize { self.cells[r][c] = Cell::blank(); } },
                1 => for r in 0..=self.cy as usize { for c in 0..COLS as usize { self.cells[r][c] = Cell::blank(); } },
                _ => self.clear_screen(),
            },
            b'K' => self.clear_eol(),
            b'm' => self.apply_sgr(&params, np),
            _ => {}
        }
    }
}

// --------------------------------------------------------------------------
// Command line buffer
// --------------------------------------------------------------------------

struct CmdLine {
    buf: [u8; 256],
    len: usize,
}

impl CmdLine {
    fn new() -> Self { CmdLine { buf: [0u8; 256], len: 0 } }
    fn push(&mut self, b: u8) { if self.len < 255 { self.buf[self.len] = b; self.len += 1; } }
    fn backspace(&mut self) { if self.len > 0 { self.len -= 1; } }
    fn clear(&mut self) { self.len = 0; }
    fn as_bytes(&self) -> &[u8] { &self.buf[..self.len] }
}

// --------------------------------------------------------------------------
// Render
// --------------------------------------------------------------------------

unsafe fn render(vt: &mut Vt100, fb: &mut WindowBuffer) {
    if vt.dirty_all {
        fb.clear(BG);
        vt.dirty_all = false;
        for row in vt.cells.iter_mut() {
            for cell in row.iter_mut() { cell.dirty = true; }
        }
    }

    for r in 0..ROWS as usize {
        for c in 0..COLS as usize {
            let cell = &mut vt.cells[r][c];
            if !cell.dirty { continue; }
            let px = (c as u32) * CELL_W;
            let py = (r as u32) * CELL_H;
            font::draw_char(fb, px, py, cell.ch, cell.fg, Some(cell.bg));
            cell.dirty = false;
        }
    }

    // Cursor
    let cx = vt.cx.min(COLS - 1);
    let cy = vt.cy.min(ROWS - 1);
    let under = vt.cells[cy as usize][cx as usize].ch;
    font::draw_char(fb, cx * CELL_W, cy * CELL_H, under, BG, Some(CURSOR));
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn write_vt(vt: &mut Vt100, s: &[u8]) {
    for &b in s { vt.feed(b); }
}

fn print_prompt(vt: &mut Vt100) {
    write_vt(vt, b"\x1B[32m>\x1B[0m ");
}

fn trim(s: &[u8]) -> &[u8] {
    let start = match s.iter().position(|&b| b != b' ') { Some(i) => i, None => return &[] };
    let end = s.iter().rposition(|&b| b != b' ').unwrap_or(start);
    &s[start..=end]
}

fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

fn starts_with(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len() && &hay[..needle.len()] == needle
}

fn run_cmd(vt: &mut Vt100, cmd: &[u8]) {
    let cmd = trim(cmd);
    if bytes_eq(cmd, b"help") {
        write_vt(vt, b"Built-ins: help, clear, echo <text>, uname, whoami, exit\r\n");
    } else if bytes_eq(cmd, b"clear") {
        vt.clear_screen(); vt.cx = 0; vt.cy = 0;
    } else if bytes_eq(cmd, b"uname") {
        write_vt(vt, b"SexOS 0.1.0-silk x86_64\r\n");
    } else if bytes_eq(cmd, b"whoami") {
        write_vt(vt, b"root\r\n");
    } else if bytes_eq(cmd, b"exit") {
        write_vt(vt, b"logout\r\n");
        loop {}
    } else if starts_with(cmd, b"echo ") {
        write_vt(vt, &cmd[5..]);
        write_vt(vt, b"\r\n");
    } else if cmd.is_empty() {
        // nothing
    } else {
        write_vt(vt, b"\x1B[31mcommand not found:\x1B[0m ");
        write_vt(vt, cmd);
        write_vt(vt, b"\r\n");
    }
}

fn handle_key(vt: &mut Vt100, line: &mut CmdLine, code: u16) {
    match code {
        28 => { // Enter
            vt.feed(b'\r'); vt.feed(b'\n');
            run_cmd(vt, line.as_bytes());
            line.clear();
            print_prompt(vt);
        }
        14 => { // Backspace
            if line.len > 0 {
                line.backspace();
                write_vt(vt, b"\x1B[D \x1B[D");
            }
        }
        _ => {
            if let Some(ch) = scancode_to_ascii(code) {
                line.push(ch);
                vt.feed(ch);
            }
        }
    }
}

fn scancode_to_ascii(code: u16) -> Option<u8> {
    const TABLE: &[(u16, u8)] = &[
        (2,b'1'),(3,b'2'),(4,b'3'),(5,b'4'),(6,b'5'),(7,b'6'),(8,b'7'),(9,b'8'),(10,b'9'),(11,b'0'),
        (16,b'q'),(17,b'w'),(18,b'e'),(19,b'r'),(20,b't'),(21,b'y'),(22,b'u'),(23,b'i'),(24,b'o'),(25,b'p'),
        (30,b'a'),(31,b's'),(32,b'd'),(33,b'f'),(34,b'g'),(35,b'h'),(36,b'j'),(37,b'k'),(38,b'l'),
        (44,b'z'),(45,b'x'),(46,b'c'),(47,b'v'),(48,b'b'),(49,b'n'),(50,b'm'),
        (57,b' '),(12,b'-'),(13,b'='),(26,b'['),(27,b']'),(43,b'\\'),(39,b';'),(40,b'\''),(41,b'`'),
        (51,b','),(52,b'.'),(53,b'/'),
    ];
    TABLE.iter().find(|&&(k, _)| k == code).map(|&(_, ch)| ch)
}

// --------------------------------------------------------------------------
// Entry
// --------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let params = SexWindowCreateParams {
        x: 40, y: 40,
        width: WIN_W,
        height: WIN_H,
        pfn_base: FB_PFN_BASE,
    };
    pdx_call(0, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0, 0);

    let mut fb = unsafe {
        WindowBuffer::new((FB_PFN_BASE << 12) as u64, WIN_W, WIN_H, WIN_W)
    };

    let mut vt = Vt100::new();
    let mut line = CmdLine::new();

    write_vt(&mut vt, b"\x1B[32msexsh v2\x1B[0m \xE2\x80\x94 SexOS Silk terminal\r\n");
    write_vt(&mut vt, b"Type \x1B[33mhelp\x1B[0m for commands.\r\n\n");
    print_prompt(&mut vt);

    unsafe { render(&mut vt, &mut fb); }

    loop {
        let req = pdx_listen(0);
        // HID events: arg0=ev_type, arg1=code, arg2=value
        let ev_type = req.arg0 as u16;
        let code    = req.arg1 as u16;
        let value   = req.arg2 as i32;

        if ev_type == 1 /* EV_KEY */ && (value == 1 || value == 2) {
            handle_key(&mut vt, &mut line, code);
            unsafe { render(&mut vt, &mut fb); }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
