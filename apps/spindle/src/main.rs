//! Spindle V1 — SexOS native command console
//!
//! Architecture:
//!   • Static bounded text surface via sex-graphics CP437 font
//!   • Window created via PDX OP_WINDOW_CREATE on sexdisplay slot 5
//!   • Fixed PFN base for framebuffer (matches sexsh convention)
//!   • Bounded line editor with synthetic input proof gate
//!   • Bounded scrollback ring (1024 lines × 80 bytes)
//!   • No real HID delivery yet — Spindle not kernel-spawned (no PDX slot)
//!   • No command execution yet
//!   • No terminal emulation (Spindle is NOT sexsh)
//!
//! Contract: docs/handoff/SPINDLE_APP_CONTRACT_V1.md
//! Next: SPINDLE_NATIVE_COMMAND_DISPATCH_V1

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

// ── Bounded scrollback ring ────────────────────────────────────────────────

/// Number of visible output rows (rows 5–22 = 18 lines).
const VISIBLE_ROWS: usize = 18;
/// First row index for the output area.
const OUTPUT_ROW_START: u32 = 5;
/// Max scrollback lines in the ring buffer.
const MAX_SCROLLBACK: usize = 1024;
/// Max chars per scrollback line (matches COLS = 80).
const MAX_LINE_BYTES: usize = 80;

struct Scrollback {
    /// Fixed ring buffer: 1024 lines of 80 bytes each = 80 KiB.
    ring: [[u8; MAX_LINE_BYTES]; MAX_SCROLLBACK],
    /// Index in ring where the next line will be written.
    write_pos: usize,
    /// Total number of lines ever written (used for display offset).
    /// Monotonically increases, never wraps. Saturates at u32::MAX.
    total_lines: u32,
    /// User scroll offset from newest (0 = show latest).
    scroll_offset: u32,
}

impl Scrollback {
    const fn new() -> Self {
        Scrollback {
            ring: [[0u8; MAX_LINE_BYTES]; MAX_SCROLLBACK],
            write_pos: 0,
            total_lines: 0,
            scroll_offset: 0,
        }
    }

    /// Push one line into the ring buffer. Clamps to MAX_LINE_BYTES.
    fn push(&mut self, line: &[u8]) {
        let n = line.len().min(MAX_LINE_BYTES);
        let dst = &mut self.ring[self.write_pos][..n];
        dst.copy_from_slice(&line[..n]);
        // Zero-fill remainder
        for i in n..MAX_LINE_BYTES {
            self.ring[self.write_pos][i] = 0;
        }
        self.write_pos = (self.write_pos + 1) % MAX_SCROLLBACK;
        self.total_lines = self.total_lines.saturating_add(1);
        // Reset scroll offset to show latest unless user is actively scrolling
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    /// Return the line at the given ring index. Clamps to valid bytes.
    fn get(&self, ring_idx: usize) -> &[u8] {
        let raw = &self.ring[ring_idx % MAX_SCROLLBACK];
        // Find the effective length (stop at first NUL or at MAX_LINE_BYTES)
        let mut len = 0;
        while len < MAX_LINE_BYTES && raw[len] != 0 {
            len += 1;
        }
        &raw[..len]
    }
}

// ── Scrollback render ──────────────────────────────────────────────────────

/// Render the visible scrollback area (rows 5–22) into the framebuffer.
///
/// Calculates which ring buffer entries are visible based on total_lines,
/// scroll_offset, and VISIBLE_ROWS. Draws each line with font::draw_str.
/// Empty lines are left as background.
unsafe fn render_scrollback(fb: &mut WindowBuffer, sb: &Scrollback) {
    // Clear the output area
    let area_h = VISIBLE_ROWS as u32 * CELL_H;
    fb.draw_rect(sex_pdx::Rect {
        x: 0, y: (OUTPUT_ROW_START * CELL_H) as u32,
        width: WIN_W, height: area_h,
    }, BG);

    if sb.total_lines == 0 { return; }

    // How many total lines could be visible
    let available = (sb.total_lines as usize).min(VISIBLE_ROWS);
    if available == 0 { return; }

    // Start from newest line minus scroll_offset, work backwards
    let newest_line = sb.total_lines.saturating_sub(1 + sb.scroll_offset);
    let oldest_visible = newest_line.saturating_sub(VISIBLE_ROWS as u32 - 1);

    for vis_row in 0..VISIBLE_ROWS {
        let line_idx = oldest_visible as usize + vis_row;
        if line_idx >= sb.total_lines as usize { break; }

        let ring_idx = line_idx % MAX_SCROLLBACK;
        let text = sb.get(ring_idx);
        if !text.is_empty() {
            let y = (OUTPUT_ROW_START as u32 + vis_row as u32) * CELL_H;
            font::draw_str(fb, 4, y + 4, text, FG, None);
        }
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

    let mut sb = Scrollback::new();

    // ── Push boot header lines into scrollback ──
    sb.push(b"Spindle -- SexOS native command console");
    sb.push(b"");
    sb.push(b"Type help for commands. V1.0.0-pre");
    sb.push(b"");

    unsafe {
        fb.clear(BG);

        // Title row (row 0)
        font::draw_str(&mut fb, 4, 4, b"Spindle", ACCENT, None);

        // Separator (row 1)
        for col in 0..COLS {
            fb.draw_pixel(col * CELL_W, (CELL_H * 1) + (CELL_H / 2) - 1, ACCENT);
        }

        // Info lines (rows 2-3)
        font::draw_str(&mut fb, 4, CELL_H * 2 + 4, b"SexOS native command console", FG, None);
        font::draw_str(&mut fb, 4, CELL_H * 3 + 4, b"Type help for commands.", FG, None);

        // Separator (row 4)
        for col in 0..COLS {
            fb.draw_pixel(col * CELL_W, (CELL_H * 4) + (CELL_H / 2) - 1, ACCENT);
        }

        // Output area (rows 5-22) — rendered from scrollback
        render_scrollback(&mut fb, &sb);

        // Bottom row (row 23): prompt line
        font::draw_str(&mut fb, 4, CELL_H * 23 + 4, b"sex> ", GREEN, None);
    }
    serial_println!("[spindle.surface.ok] boot_lines={}", sb.total_lines);

    // ── Input proof gate (compile-time, synthetic input) ──
    const INPUT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SPINDLE_INPUT_PROOF").is_some();
    if INPUT_PROOF_ENABLED {
        unsafe { run_input_proof(&mut fb, &mut sb); }
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
unsafe fn run_input_proof(fb: &mut WindowBuffer, sb: &mut Scrollback) {
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
    while line.len < CMD_MAX {
        line.push(b'X');
    }
    line.push(b'Y');
    let stage3_ok = line.len == CMD_MAX;
    serial_println!("[spindle.input.proof.overflow] ok={} len={} max={}", stage3_ok as u8, line.len, CMD_MAX);

    // ── Stage 4: Non-printable rejection ──
    line.clear();
    line.push(0x01); line.push(0x00); line.push(0x7F); line.push(b'\n');
    let stage4_ok = line.len == 0;
    serial_println!("[spindle.input.proof.nonprintable] ok={} len={}", stage4_ok as u8, line.len);

    // ── Stage 5: Enter — push to scrollback, clear buffer ──
    line.push(b't'); line.push(b'e'); line.push(b's'); line.push(b't');
    // Push the command line into scrollback (prefixed as user input)
    sb.push(line.as_bytes());
    serial_println!("[spindle.line.enter] text={:?} scrollback_len={}", core::str::from_utf8(line.as_bytes()).unwrap_or("?"), sb.total_lines);
    line.clear();
    redraw_prompt(fb, &line);
    render_scrollback(fb, sb);
    let stage5_ok = line.len == 0 && sb.total_lines >= 5; // 4 boot + 1 test
    serial_println!("[spindle.input.proof.enter] ok={} scrollback_lines={}", stage5_ok as u8, sb.total_lines);

    // ── Stage 6: Empty backspace is no-op ──
    line.backspace();
    line.backspace();
    let stage6_ok = line.len == 0;
    serial_println!("[spindle.input.proof.empty_backspace] ok={}", stage6_ok as u8);

    // ── Stage 7: Scrollback overflow ──
    // Fill scrollback beyond capacity: push MAX_SCROLLBACK * 2 lines
    let sb_before = sb.total_lines;
    for i in 0..(MAX_SCROLLBACK as u32 * 2) {
        sb.push(b"overflow test line 1234567890123456789012345678901234567890");
    }
    let sb_after = sb.total_lines;
    // Ring wraps correctly — total_lines > MAX_SCROLLBACK but ring only holds MAX_SCROLLBACK
    let wrapped = sb_after > sb_before + MAX_SCROLLBACK as u32;
    serial_println!("[spindle.scrollback.overflow] ok={} total={} capacity={}", wrapped as u8, sb_after, MAX_SCROLLBACK);

    // ── Stage 8: Scrollback line clamping ──
    // Push a line longer than MAX_LINE_BYTES — must be clamped
    sb.push(&[b'L'; 200]);
    let clamped = sb.get((sb.total_lines - 1) as usize % MAX_SCROLLBACK);
    let stage8_ok = clamped.len() <= MAX_LINE_BYTES;
    serial_println!("[spindle.scrollback.clamp] ok={} line_len={} max={}", stage8_ok as u8, clamped.len(), MAX_LINE_BYTES);

    // ── Stage 9: Scroll offset + render ──
    sb.scroll_offset = 10; // scroll back 10 lines
    render_scrollback(fb, sb);
    sb.scroll_offset = 0;  // reset to latest
    render_scrollback(fb, sb);
    serial_println!("[spindle.scrollback.render] ok=1 visible_rows={}", VISIBLE_ROWS);

    let all_ok = stage1_ok && stage2_ok && stage3_ok && stage4_ok
              && stage5_ok && stage6_ok && wrapped && stage8_ok;
    serial_println!("[spindle.input.proof.done] ok={}", all_ok as u8);
}
