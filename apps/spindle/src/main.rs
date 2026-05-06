//! Spindle V1 — SexOS native command console
//!
//! Architecture:
//!   • Static bounded text surface via sex-graphics CP437 font
//!   • Window created via PDX OP_WINDOW_CREATE on sexdisplay slot 5
//!   • Fixed PFN base for framebuffer (matches sexsh convention)
//!   • Bounded line editor with synthetic input proof gate
//!   • No real HID delivery yet — Spindle not kernel-spawned (no PDX slot)
//!   • No command execution yet
//!   • No terminal emulation (Spindle is NOT sexsh)
//!
//! Contract: docs/handoff/SPINDLE_APP_CONTRACT_V1.md
//! Next: SPINDLE_SCROLLBACK_RING_V1

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // no-op
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use sex_pdx::{
    pdx_call, serial_println,
    OP_WINDOW_CREATE, SLOT_DISPLAY,
};
use sex_graphics::{WindowBuffer, font};

// ── Local window create params (not yet in sex-pdx) ────────────────────────

/// Window creation parameters sent to sexdisplay via OP_WINDOW_CREATE.
/// Matches the struct defined in crates/silk-client/src/lib.rs and used by sexsh.
#[repr(C)]
struct WindowCreateParams {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pfn_base: u64,
}

// ── Spindle surface geometry ───────────────────────────────────────────────

const CELL_W: u32 = 8;
const CELL_H: u32 = 8;
const COLS: u32 = 80;
const ROWS: u32 = 24;
const WIN_W: u32 = COLS * CELL_W;  // 640
const WIN_H: u32 = ROWS * CELL_H;  // 192

/// Framebuffer lives at 256 MiB physical; kernel maps it on window create.
/// Matches sexsh convention.
const FB_PFN_BASE: u64 = 0x0000_0004_0000;

// ── Catppuccin Mocha palette ───────────────────────────────────────────────

const BG:     u32 = 0xFF1E1E2E; // Base
const FG:     u32 = 0xFFCDD6F4; // Text
const ACCENT: u32 = 0xFF89B4FA; // Blue
const GREEN:  u32 = 0xFFA6E3A1; // Green
const YELLOW: u32 = 0xFFF9E2AF; // Yellow
const CURSOR: u32 = 0xFFF5E0DC; // Rosewater (cursor color)

// ── Bounded line editor ────────────────────────────────────────────────────

const CMD_MAX: usize = 256;
const PROMPT: &[u8] = b"sex> ";
const PROMPT_LEN: usize = PROMPT.len();

struct CmdLine {
    buf: [u8; CMD_MAX],
    len: usize,
}

impl CmdLine {
    const fn new() -> Self { CmdLine { buf: [0u8; CMD_MAX], len: 0 } }

    /// Append a printable ASCII byte. Rejects if full or non-printable.
    fn push(&mut self, b: u8) {
        if self.len < CMD_MAX && b >= 0x20 && b <= 0x7E {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    /// Delete one character before cursor.
    fn backspace(&mut self) {
        if self.len > 0 { self.len -= 1; }
    }

    /// Clear the buffer entirely.
    fn clear(&mut self) { self.len = 0; }

    /// Return the current line as a byte slice.
    fn as_bytes(&self) -> &[u8] { &self.buf[..self.len] }
}

// ── Prompt redraw ──────────────────────────────────────────────────────────

/// Redraw the prompt line (row 23). Clears the row, draws "sex> " + current line + cursor.
unsafe fn redraw_prompt(fb: &mut WindowBuffer, line: &CmdLine) {
    // Clear entire prompt row
    fb.draw_rect(sex_pdx::Rect { x: 0, y: CELL_H * 23, width: WIN_W, height: CELL_H }, BG);

    // Draw prompt prefix
    font::draw_str(fb, 4, CELL_H * 23 + 4, PROMPT, GREEN, None);

    // Draw current line
    if line.len > 0 {
        let prompt_px = (PROMPT_LEN as u32) * CELL_W;
        // Max visible chars = COLS - PROMPT_LEN - 1 (cursor)
        let max_vis = (COLS as usize).saturating_sub(PROMPT_LEN + 1);
        let start = line.len.saturating_sub(max_vis);
        let visible = &line.as_bytes()[start..];
        font::draw_str(fb, 4 + prompt_px, CELL_H * 23 + 4, visible, FG, None);

        // Cursor block at end of input
        let cursor_col = PROMPT_LEN as u32 + visible.len() as u32;
        font::draw_char(fb, cursor_col * CELL_W, CELL_H * 23, b' ', CURSOR, Some(CURSOR));
    } else {
        // Cursor block at prompt position when empty
        let cursor_col = PROMPT_LEN as u32;
        font::draw_char(fb, cursor_col * CELL_W, CELL_H * 23, b' ', CURSOR, Some(CURSOR));
    }
}

// ── Entry ──────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[spindle.boot]");

    // ── Create window on sexdisplay ──
    let params = WindowCreateParams {
        x: 40, y: 200,          // below silkbar, above bottom
        width: WIN_W,
        height: WIN_H,
        pfn_base: FB_PFN_BASE,
    };
    unsafe {
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE,
            &params as *const _ as u64, 0, 0);
    }
    serial_println!("[spindle.surface.req] w={} h={}", WIN_W, WIN_H);

    // ── Draw static content ──
    let mut fb = unsafe {
        WindowBuffer::new((FB_PFN_BASE << 12) as u64, WIN_W, WIN_H, WIN_W)
    };

    unsafe {
        fb.clear(BG);

        // Title row (row 0)
        font::draw_str(&mut fb, 4, 4, b"Spindle", ACCENT, None);

        // Separator (row 1)
        for col in 0..COLS {
            unsafe { fb.draw_pixel(col * CELL_W, (CELL_H * 1) + (CELL_H / 2) - 1, ACCENT); }
        }

        // Info lines (rows 2-3)
        font::draw_str(&mut fb, 4, CELL_H * 2 + 4, b"SexOS native command console", FG, None);
        font::draw_str(&mut fb, 4, CELL_H * 3 + 4, b"Type help for commands.", FG, None);

        // Separator (row 4)
        for col in 0..COLS {
            unsafe { fb.draw_pixel(col * CELL_W, (CELL_H * 4) + (CELL_H / 2) - 1, ACCENT); }
        }

        // Output area rows 5-22 (empty, scrollback will fill these)
        // Bottom row (row 23): prompt line
        font::draw_str(&mut fb, 4, CELL_H * 23 + 4, b"sex> ", GREEN, None);
    }
    serial_println!("[spindle.surface.ok] content drawn");

    // ── Input proof gate (compile-time, synthetic input) ──
    const INPUT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SPINDLE_INPUT_PROOF").is_some();
    if INPUT_PROOF_ENABLED {
        unsafe { run_input_proof(&mut fb); }
    }

    // ── Idle loop (HID delivery blocked — Spindle not kernel-spawned) ──
    loop {
        unsafe {
            sex_pdx::pdx_listen_raw(0);
        }
    }
}

// ── Synthetic input proof ──────────────────────────────────────────────────

/// Run a synthetic input proof exercising all line editor operations.
/// Activated by SEXOS_SPINDLE_INPUT_PROOF=1.
///
/// Since Spindle is not kernel-spawned, it has no PDX slot for silk-shell
/// to forward HID events to. This proof gate injects synthetic keystrokes
/// directly, proving the line editor logic is correct. Real HID delivery
/// will be wired when Spindle gets kernel-spawned (STOP FIRST).
unsafe fn run_input_proof(fb: &mut WindowBuffer) {
    serial_println!("[spindle.input.proof.start]");

    let mut line = CmdLine::new();

    // ── Stage 1: Append printable characters ──
    let test_str = b"hello";
    for &ch in test_str {
        line.push(ch);
        redraw_prompt(fb, &line);
        serial_println!("[spindle.line.append] ch={} len={}", ch as char, line.len);
    }
    let stage1_ok = line.len == 5 && line.as_bytes() == b"hello";
    serial_println!("[spindle.input.proof.append] ok={} len={}", stage1_ok as u8, line.len);

    // ── Stage 2: Backspace ──
    line.backspace();
    redraw_prompt(fb, &line);
    serial_println!("[spindle.line.backspace] len={}", line.len);
    line.backspace();
    redraw_prompt(fb, &line);
    let stage2_ok = line.len == 3 && line.as_bytes() == b"hel";
    serial_println!("[spindle.input.proof.backspace] ok={} len={}", stage2_ok as u8, line.len);

    // ── Stage 3: Overflow rejection ──
    // Fill to max capacity
    while line.len < CMD_MAX {
        line.push(b'X');
    }
    // One more must be rejected (len stays at CMD_MAX)
    line.push(b'Y');
    let stage3_ok = line.len == CMD_MAX;
    serial_println!("[spindle.input.proof.overflow] ok={} len={} max={}", stage3_ok as u8, line.len, CMD_MAX);

    // ── Stage 4: Non-printable rejection ──
    line.clear();
    line.push(0x01); // control-A
    line.push(0x00); // null
    line.push(0x7F); // DEL
    line.push(b'\n'); // newline
    let stage4_ok = line.len == 0;
    serial_println!("[spindle.input.proof.nonprintable] ok={} len={}", stage4_ok as u8, line.len);

    // ── Stage 5: Enter (clear + redraw prompt) ──
    line.push(b't'); line.push(b'e'); line.push(b's'); line.push(b't');
    // Simulate Enter: append current line to output (for now, just redraw)
    redraw_prompt(fb, &line); // show what was typed
    serial_println!("[spindle.line.enter] text={:?}", core::str::from_utf8(line.as_bytes()).unwrap_or("?"));
    line.clear();
    redraw_prompt(fb, &line);
    let stage5_ok = line.len == 0;
    serial_println!("[spindle.input.proof.enter] ok={} len={}", stage5_ok as u8, line.len);

    // ── Stage 6: Empty backspace is no-op ──
    line.backspace();
    line.backspace();
    let stage6_ok = line.len == 0;
    serial_println!("[spindle.input.proof.empty_backspace] ok={}", stage6_ok as u8);

    let all_ok = stage1_ok && stage2_ok && stage3_ok && stage4_ok && stage5_ok && stage6_ok;
    serial_println!("[spindle.input.proof.done] ok={}", all_ok as u8);
}
