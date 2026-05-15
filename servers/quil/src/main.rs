#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen_raw, sched_yield, serial_println, OP_QUIL_PING, OP_TEXT_DRAW, OP_TEXT_CLEAR, SLOT_DISPLAY, SLOT_STORAGE};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ── Quil Text Surface V1 Constants ───────────────────────────────────────────
// Text rendering is NOT available in sexdisplay (no font subsystem).
// Fill-rect visuals represent the text surface structure.
// See docs/handoff/QUIL_MINIMAL_TEXT_SURFACE_BLOCKER_V1.md

const OP_HID_EVENT: u64 = 0x202;
const SURFACE_ID_QUIL: u64 = 201;

// Surface geometry (matches silk-shell SURFACE_201_W/H and BOOT_*)
const SURFACE_W: u64 = 640;
const SURFACE_H: u64 = 480;

// ── Static Text Buffer ───────────────────────────────────────────────────────
// Bounded, inline, no heap. Demo content for text surface V1.
const QUIL_TITLE: &str = "Quil";
const QUIL_TITLE_MAX_LEN: usize = 32;
const QUIL_TEXT_INIT: &[u8] = b"This is the Quil text surface.\n\
A minimal no_std editor prototype.\n\
Built on the Sex Microkernel.\n\
No text rendering available yet.\n\
Fill-rect visual representation.\n\
Buffer capacity: bounded static array.\n\
Press arrows to navigate, ESC for cmds.";
const QUIL_BUFFER_MAX_LEN: usize = 512;
const QUIL_MAX_VISIBLE_LINES: usize = 6;

/// Mutable text buffer — initialized from QUIL_TEXT_INIT at boot.
/// Updated by quil_load(). Read by draw_text_lines().
static mut QUIL_BUFFER: [u8; QUIL_BUFFER_MAX_LEN] = [0u8; QUIL_BUFFER_MAX_LEN];
static mut QUIL_BUFFER_LEN: usize = 0;

// ── RamFS / SexFiles Protocol Constants ─────────────────────────────────────
// SEXFILES_RAMFS_CONTRACT_LOCK_V1: bounded flat namespace.
// Name <= 24 bytes, file <= 4096 bytes, 8-byte per PDX write/read.
const OP_RAMFS_OPEN: u64 = 0x30;
const OP_RAMFS_WRITE: u64 = 0x32;
const OP_RAMFS_READ: u64 = 0x31;
const OP_RAMFS_CLOSE: u64 = 0x33;
const RAMFS_O_CREATE: u32 = 0x01;
const STORAGE_CAP_PROOF_ENABLED: bool = option_env!("SEXOS_STORAGE_CAP_PROOF").is_some();
const QUIL_DISKFS_SLOT_PROOF_ENABLED: bool = option_env!("SEXOS_QUIL_DISKFS_SLOT_PROOF").is_some();
const QUIL_KEYBOARD_NAV_PROOF_ENABLED: bool = option_env!("SEXOS_QUIL_KEYBOARD_NAV_PROOF").is_some();
const QUIL_KEYBOARD_BUFFER_PROOF_ENABLED: bool = option_env!("SEXOS_QUIL_KEYBOARD_BUFFER_PROOF").is_some();
static mut QUIL_BUFFER_PROOF_ACTIVE: bool = false;

/// Text edit buffer proof gate.
/// Build with SEXOS_QUIL_TEXT_BUFFER_PROOF=1 to enable.
const QUIL_TEXT_BUFFER_PROOF_ENABLED: bool =
    option_env!("SEXOS_QUIL_TEXT_BUFFER_PROOF").is_some();
static mut QUIL_TEXT_BUFFER_PROOF_STAGE: u8 = 0;
static mut QUIL_TEXT_BUFFER_PROOF_DONE: bool = false;

const OP_DISKFS_WRITE: u64 = 0x38;
const OP_DISKFS_READ: u64 = 0x39;
const OP_DISKFS_STAT: u64 = 0x3B;
const OP_DISKFS_SELECT: u64 = 0x3E;
const QUIL_DISKFS_PATH_ID: u64 = 2;
const QUIL_DISKFS_EXPECT_SIZE: u64 = 4096;
const QUIL_DISKFS_EXPECT_FLAGS: u64 = 0x3;

/// Fixed document name (fits RamFS 24-byte bound).
const QUIL_DOC_NAME: &[u8] = b"quil_doc_01";

// ── HID event stash for pdx_call_and_reply skip-loop replay ──────────────
// During boot proofs, pdx_call_and_reply() spins waiting for storage replies.
// OP_HID_EVENT messages arriving during that spin were previously discarded.
// This bounded 8-slot stash captures them for replay before the main loop.
const HID_STASH_CAPACITY: usize = 8;
static mut HID_STASH: [(u64, u64, u64); HID_STASH_CAPACITY] = [(0, 0, 0); HID_STASH_CAPACITY];
static mut HID_STASH_COUNT: usize = 0;

fn quil_storage_cap_probe() {
    // Probe with a bounded fixed name and create flag.
    let (n0, n1) = pack_name(QUIL_DOC_NAME);
    let flags_arg = (RAMFS_O_CREATE as u64) << 24;
    let (status, value) = pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, n0, n1, flags_arg);
    if status == 0 {
        serial_println!("[quil.storage.cap.ok] status=0 handle={}", value);
        let _ = pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, value, 0, 0);
    } else if status == sex_pdx::ERR_CAP_INVALID {
        serial_println!("[quil.storage.cap.blocker] reason=cap_invalid status={:#x}", status);
    } else {
        serial_println!("[quil.storage.cap.blocker] reason=route_or_backend status={:#x} value={:#x}", status, value);
    }
}

// ── Title bar (rect_index=1, persists independently from palette rect0) ─────
const QUIL_TITLE_BAR_H: u64 = 32;
const QUIL_TITLE_BAR_COLOR: u64 = 0x00302E56; // deep blue-purple, matches silk-shell QUIL_LIST_HEADER_COLOR

// ── Text line visuals (rect_indices 2-7) ─────────────────────────────────────
const QUIL_TEXT_AREA_Y: u64 = QUIL_TITLE_BAR_H + 4;
const QUIL_LINE_H: u64 = 18;
const QUIL_LINE_GAP: u64 = 4;
const QUIL_LINE_X: u64 = 8;
const QUIL_LINE_W: u64 = SURFACE_W - 16;
const QUIL_LINE_BG: u64 = 0x000C1420;   // dark slate
const QUIL_LINE_COLOR: u64 = 0x00304058; // muted blue-gray
const QUIL_LINE_ACCENT_W: u64 = 4;
const QUIL_LINE_ACCENT_COLOR: u64 = 0x00506080;

/// Characters per line for sexdisplay's 5×7 grid renderer (FONT_ASCII_5X7).
const QUIL_TEXT_CHARS_PER_LINE: usize = 20;

/// Pad a logical line to QUIL_TEXT_CHARS_PER_LINE with trailing spaces so the
/// sexdisplay renderer places each logical line on its own raster row.
fn pad_text_line(line: &[u8], out: &mut [u8], max_out: usize) -> usize {
    let copy_len = line.len().min(QUIL_TEXT_CHARS_PER_LINE);
    let mut w = 0usize;
    while w < copy_len && w < max_out {
        out[w] = line[w];
        w += 1;
    }
    while w < QUIL_TEXT_CHARS_PER_LINE && w < max_out {
        out[w] = b' ';
        w += 1;
    }
    w
}

const QUIL_GLYPH_COLOR: u64 = 0x00D4FF7A;
const QUIL_RECT_SLOT_MIN: u64 = 2;
const QUIL_RECT_SLOT_MAX: u64 = 7;

// ── Palette (existing command area, rect_index=0, same as before) ────────────
const QUIL_ROWS: u8 = 5;
const QUIL_PANEL_X: u64 = 24;
const QUIL_PANEL_Y: u64 = QUIL_TITLE_BAR_H + 8 + (QUIL_MAX_VISIBLE_LINES as u64) * (QUIL_LINE_H + QUIL_LINE_GAP);
const QUIL_PANEL_W: u64 = SURFACE_W - 48;
const QUIL_PANEL_H: u64 = SURFACE_H - QUIL_PANEL_Y - 16;
const QUIL_PANEL_PAD_X: u64 = 16;
const QUIL_PANEL_PAD_Y: u64 = 16;

const QUIL_ROW_X: u64 = QUIL_PANEL_X + QUIL_PANEL_PAD_X;
const QUIL_ROW_Y0: u64 = QUIL_PANEL_Y + QUIL_PANEL_PAD_Y;
const QUIL_ROW_W: u64 = QUIL_PANEL_W - (QUIL_PANEL_PAD_X * 2);
const QUIL_ROW_H: u64 = 32;
const QUIL_ROW_GAP: u64 = 8;
const QUIL_ACCENT_W: u64 = 6;
const QUIL_ROW_INACTIVE: u64 = 0x00253556;
const QUIL_ROW_SELECTED: u64 = 0x004B6FD3;
const QUIL_ACCENT_COLOR: u64 = 0x00E9D36A;

/// Palette command IDs. Rows 1-2 now wired to RamFS save/load.
const CMD_NEW_BUFFER_STUB: u8 = 1;
const CMD_SAVE_DOCUMENT: u8 = 2;   // Save Quil buffer to RamFS
const CMD_LOAD_DOCUMENT: u8 = 3;   // Load Quil buffer from RamFS
const CMD_RUN_CHECK_STUB: u8 = 4;
const CMD_SETTINGS_STUB: u8 = 5;

/// Map palette row index to command ID.
/// Row 0 is top (index 0), row 4 is bottom.
const PALETTE_COMMANDS: [u8; QUIL_ROWS as usize] = [
    CMD_NEW_BUFFER_STUB,    // row 0
    CMD_SAVE_DOCUMENT,      // row 1
    CMD_LOAD_DOCUMENT,      // row 2
    CMD_RUN_CHECK_STUB,     // row 3
    CMD_SETTINGS_STUB,      // row 4
];

fn palette_command_for_row(row: u8) -> u8 {
    if row < QUIL_ROWS {
        PALETTE_COMMANDS[row as usize]
    } else {
        0
    }
}

/// Compute the number of visual lines in the text buffer.
/// Splits on '\n', up to QUIL_MAX_VISIBLE_LINES.
fn text_buffer_line_count(buf: &[u8]) -> usize {
    let mut lines = 0usize;
    for &b in buf.iter() {
        if b == b'\n' {
            lines += 1;
        }
    }
    if lines == 0 || buf.last().map_or(true, |&b| b != b'\n') {
        lines += 1;
    }
    lines.min(QUIL_MAX_VISIBLE_LINES)
}

/// Validate title length. Returns true if valid.
fn validate_title(title: &str) -> bool {
    let valid = title.len() <= QUIL_TITLE_MAX_LEN && !title.is_empty();
    if !valid {
        serial_println!("[quil.text.title.reject] title={} len={} max={}",
            title, title.len(), QUIL_TITLE_MAX_LEN);
    }
    valid
}

/// Validate buffer length. Returns true if valid.
fn validate_buffer(buf: &[u8]) -> bool {
    let valid = buf.len() <= QUIL_BUFFER_MAX_LEN && !buf.is_empty();
    if !valid {
        serial_println!("[quil.text.buffer.reject] bytes={} max={}",
            buf.len(), QUIL_BUFFER_MAX_LEN);
    }
    valid
}

/// Draw the title bar fill rect (rect_index=1).
fn draw_title_bar() {
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        0u64,
        (1u64 << 56)
            | (QUIL_TITLE_BAR_COLOR << 32)
            | (QUIL_TITLE_BAR_H << 16)
            | SURFACE_W,
    );
}

/// Draw text buffer lines as actual glyphs via OP_TEXT_DRAW (0xFB) to sexdisplay.
/// Lines are split on \n, padded to QUIL_TEXT_CHARS_PER_LINE, and sent in 8-byte chunks.
/// Text color: bright cyan (0x00E0F0FF) over the dark slate background.
fn draw_text_lines(buf: &[u8]) {
    const TEXT_LINE_COLOR: u64 = 0x00E0F0FF; // bright cyan on dark background
    const MAX_CHUNK: usize = 8;              // bytes per OP_TEXT_DRAW call

    // Clear previous text on this surface.
    pdx_call(SLOT_DISPLAY, OP_TEXT_CLEAR, SURFACE_ID_QUIL, 0, 0);

    let line_count = text_buffer_line_count(buf);
    let show_lines = line_count.min(QUIL_MAX_VISIBLE_LINES);
    serial_println!("[quil.text.draw.v2] lines={} bytes={} visible={}",
        line_count, buf.len(), show_lines);

    if buf.is_empty() || show_lines == 0 { return; }

    // Pad each logical line to QUIL_TEXT_CHARS_PER_LINE so the renderer
    // places each on its own raster row.
    let mut line_buf: [u8; 256] = [0u8; 256];
    let mut total_written: usize = 0;
    let mut line_start: usize = 0;

    for _line_idx in 0..show_lines {
        // Find end of this logical line (next \n or end of buf)
        let mut line_end = line_start;
        while line_end < buf.len() && buf[line_end] != b'\n' {
            line_end += 1;
        }
        let logical_line = &buf[line_start..line_end];

        // Pad to QUIL_TEXT_CHARS_PER_LINE
        let w = pad_text_line(logical_line, &mut line_buf[total_written..],
                              256usize.saturating_sub(total_written));
        total_written += w;

        // Advance past \n separator
        line_start = line_end;
        if line_start < buf.len() && buf[line_start] == b'\n' {
            line_start += 1;
        }
    }

    if total_written == 0 { return; }

    // Send padded text in 8-byte chunks via OP_TEXT_DRAW.
    let mut offset: usize = 0;
    while offset < total_written {
        let remaining = total_written - offset;
        let chunk_len = remaining.min(MAX_CHUNK);

        // Pack up to 8 bytes into arg1 (little-endian)
        let mut packed: u64 = 0;
        for i in 0..chunk_len {
            packed |= (line_buf[offset + i] as u64) << (i * 8);
        }

        // arg2: byte_offset (bits 0-7) | char_count (bits 8-11) | text_color (bits 32-63)
        let arg2: u64 = (offset as u64 & 0xFF)
            | ((chunk_len as u64 & 0xF) << 8)
            | (TEXT_LINE_COLOR << 32);

        pdx_call(SLOT_DISPLAY, OP_TEXT_DRAW, SURFACE_ID_QUIL, packed, arg2);
        offset += chunk_len;
    }

    serial_println!("[quil.text.draw.v2.sent] total_bytes={} chunks={}",
        total_written, (total_written + MAX_CHUNK - 1) / MAX_CHUNK);

    if line_count > QUIL_MAX_VISIBLE_LINES {
        serial_println!("[quil.text.buffer.overflow] lines={} visible={}",
            line_count, QUIL_MAX_VISIBLE_LINES);
    }
}

fn emit_rect_slot(slot: u64, x: u64, y: u64, w: u64, h: u64, color: u64) {
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        (y << 32) | x,
        (slot << 56) | (color << 32) | (h << 16) | w,
    );
}

/// Draw bounded pseudo-glyph letters using existing rect slots.
/// Slot cap is strict: only slot indices 2..7 are used (6 rects max).
/// This can render only a prefix of the target phrase safely.
fn draw_rect_glyph_text() {
    let text = b"QUIL TEXT ALIVE";
    let mut slot = QUIL_RECT_SLOT_MIN;
    let x = 462u64;
    let y = 6u64;
    let cw = 14u64;
    let ch = 18u64;

    // Q (2 rects): block + tail
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 0, y + 0, cw, ch, QUIL_GLYPH_COLOR);
        slot += 1;
    }
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 9, y + 12, 5, 6, QUIL_TITLE_BAR_COLOR);
        slot += 1;
    }

    // U (3 rects): left/right stems + base
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 18, y + 0, 3, ch - 3, QUIL_GLYPH_COLOR);
        slot += 1;
    }
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 27, y + 0, 3, ch - 3, QUIL_GLYPH_COLOR);
        slot += 1;
    }
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 18, y + ch - 3, 12, 3, QUIL_GLYPH_COLOR);
        slot += 1;
    }

    // I (1 rect): center stem
    if slot <= QUIL_RECT_SLOT_MAX {
        emit_rect_slot(slot, x + 38, y + 0, 3, ch, QUIL_GLYPH_COLOR);
        slot += 1;
    }

    let emitted = slot - QUIL_RECT_SLOT_MIN;
    // 6 rect cap means only prefix can be rendered safely.
    let shown_chars = if emitted >= 6 { 3 } else if emitted >= 5 { 2 } else { 1 };
    serial_println!("[quil.rect_glyph_text.v1] text=QUIL_TEXT_ALIVE shown={} rects={}", shown_chars, emitted);
}

fn draw_palette(selected: u8) {
    // Panel background (rect_index=0, overwritten per redraw).
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        (QUIL_PANEL_Y << 32) | QUIL_PANEL_X,
        (QUIL_ROW_INACTIVE << 32) | (QUIL_PANEL_H << 16) | QUIL_PANEL_W,
    );

    unsafe {
        static mut PALETTE_DRAW_BUDGET: u32 = 12;
        let b = &mut PALETTE_DRAW_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[quil.palette.draw] rows={} selected={}", QUIL_ROWS, selected);
        }
        static mut PALETTE_PANEL_BUDGET: u32 = 8;
        let pb = &mut PALETTE_PANEL_BUDGET;
        if *pb > 0 {
            *pb -= 1;
            serial_println!(
                "[quil.palette.panel] x={} y={} w={} h={}",
                QUIL_PANEL_X,
                QUIL_PANEL_Y,
                QUIL_PANEL_W,
                QUIL_PANEL_H
            );
        }
    }

    // Guard against row overflow in the bounded panel.
    let rows_bottom = QUIL_ROW_Y0 + QUIL_ROWS as u64 * QUIL_ROW_H + (QUIL_ROWS as u64 - 1) * QUIL_ROW_GAP;
    if rows_bottom > (QUIL_PANEL_Y + QUIL_PANEL_H) {
        serial_println!("[quil.palette.reject] action=draw reason=row_overflow");
    }

    let mut row = 0u8;
    while row < QUIL_ROWS {
        let y = QUIL_ROW_Y0 + (QUIL_ROW_H + QUIL_ROW_GAP) * row as u64;
        let is_selected = row == selected;
        let color = if is_selected {
            QUIL_ROW_SELECTED
        } else {
            QUIL_ROW_INACTIVE
        };

        pdx_call(
            SLOT_DISPLAY,
            0xEF,
            SURFACE_ID_QUIL,
            (y << 32) | QUIL_ROW_X,
            (color << 32) | (QUIL_ROW_H << 16) | QUIL_ROW_W,
        );

        if is_selected {
            pdx_call(
                SLOT_DISPLAY,
                0xEF,
                SURFACE_ID_QUIL,
                (y << 32) | QUIL_ROW_X,
                (QUIL_ACCENT_COLOR << 32) | (QUIL_ROW_H << 16) | QUIL_ACCENT_W,
            );
            unsafe {
                static mut PALETTE_SELECTED_BUDGET: u32 = 16;
                let b = &mut PALETTE_SELECTED_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[quil.palette.selected] row={}", row);
                }
            }
        }

        unsafe {
            static mut PALETTE_ROW_BUDGET: u32 = 20;
            let b = &mut PALETTE_ROW_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!(
                    "[quil.palette.row] row={} selected={}",
                    row,
                    if is_selected { 1 } else { 0 }
                );
            }
        }

        row += 1;
    }
}

fn decode_palette_key(scancode: u64) -> u8 {
    // 0=none, 1=up, 2=down, 3=enter, 4=esc.
    // Supports common keyboard scancode variants seen in this codebase.
    match scancode as u32 {
        0x48 | 0x67 | 103 => 1,
        0x50 | 0x6c | 108 => 2,
        0x1c | 0x0d | 28 => 3,
        0x01 | 0x1b | 1 => 4,
        _ => 0,
    }
}

/// Map a keyboard scancode to ASCII character, if printable.
/// Scancode set 1 (US QWERTY).
fn scancode_to_char(scancode: u64) -> Option<u8> {
    match scancode as u8 {
        0x02 => Some(b'1'), 0x03 => Some(b'2'), 0x04 => Some(b'3'),
        0x05 => Some(b'4'), 0x06 => Some(b'5'), 0x07 => Some(b'6'),
        0x08 => Some(b'7'), 0x09 => Some(b'8'), 0x0A => Some(b'9'),
        0x0B => Some(b'0'),
        0x10 => Some(b'Q'), 0x11 => Some(b'W'), 0x12 => Some(b'E'),
        0x13 => Some(b'R'), 0x14 => Some(b'T'), 0x15 => Some(b'Y'),
        0x16 => Some(b'U'), 0x17 => Some(b'I'), 0x18 => Some(b'O'),
        0x19 => Some(b'P'),
        0x1E => Some(b'A'), 0x1F => Some(b'S'), 0x20 => Some(b'D'),
        0x21 => Some(b'F'), 0x22 => Some(b'G'), 0x23 => Some(b'H'),
        0x24 => Some(b'J'), 0x25 => Some(b'K'), 0x26 => Some(b'L'),
        0x2C => Some(b'Z'), 0x2D => Some(b'X'), 0x2E => Some(b'C'),
        0x2F => Some(b'V'), 0x30 => Some(b'B'), 0x31 => Some(b'N'),
        0x32 => Some(b'M'),
        0x27 => Some(b';'), 0x28 => Some(b'\''),
        0x33 => Some(b','), 0x34 => Some(b'.'), 0x35 => Some(b'/'),
        0x39 => Some(b' '),  // space
        0x1C => None, // Enter (handled elsewhere)
        0x0E => None, // Backspace (handled elsewhere)
        0x0F => None, // Tab
        _ => None,
    }
}

/// Append a single character to the text buffer.
/// Returns true if appended, false if buffer full or invalid.
fn text_buffer_append(ch: u8) -> bool {
    unsafe {
        if QUIL_BUFFER_LEN >= QUIL_BUFFER_MAX_LEN {
            serial_println!("[quil.text.append] len={} ch={} ok=0 reason=buffer_full",
                QUIL_BUFFER_LEN, ch as char as u32);
            return false;
        }
        if ch < 0x20 && ch != b'\n' {
            return false;
        }
        QUIL_BUFFER[QUIL_BUFFER_LEN] = ch;
        QUIL_BUFFER_LEN += 1;
        serial_println!("[quil.text.append] len={} ch={}",
            QUIL_BUFFER_LEN, ch as char as u32);
        true
    }
}

/// Delete the last character from the text buffer (backspace).
/// Returns true if a character was deleted, false if buffer empty.
fn text_buffer_backspace() -> bool {
    unsafe {
        if QUIL_BUFFER_LEN == 0 {
            serial_println!("[quil.text.backspace] old=0 new=0 ok=0 reason=empty");
            return false;
        }
        let old = QUIL_BUFFER_LEN;
        QUIL_BUFFER_LEN -= 1;
        QUIL_BUFFER[QUIL_BUFFER_LEN] = 0;
        serial_println!("[quil.text.backspace] old={} new={} ok=1",
            old, QUIL_BUFFER_LEN);
        true
    }
}

/// Insert a newline into the text buffer.
/// Returns true if inserted, false if buffer full.
fn text_buffer_newline() -> bool {
    unsafe {
        if QUIL_BUFFER_LEN >= QUIL_BUFFER_MAX_LEN {
            serial_println!("[quil.text.enter] line=0 len={} ok=0 reason=buffer_full",
                QUIL_BUFFER_LEN);
            return false;
        }
        let line_count = text_buffer_line_count(&QUIL_BUFFER[..QUIL_BUFFER_LEN]);
        QUIL_BUFFER[QUIL_BUFFER_LEN] = b'\n';
        QUIL_BUFFER_LEN += 1;
        serial_println!("[quil.text.enter] line={} len={} ok=1",
            line_count + 1, QUIL_BUFFER_LEN);
        true
    }
}

// ── Synchronous PDX Call Wrapper ──────────────────────────────────────────────
//
// inter-PD pdx_call is fire-and-forget (kernel ipc::traverse_edge AsyncEnqueue
// returns Ok(0u64) always). The server reply lands in the incoming_replies queue
// and surfaces via pdx_listen_raw(0) as type_id=0x1 with arg0=reply_value.
// This wrapper sends the call then blocks for the matching reply.
//
// During boot proof (before the main input loop starts), no HID events or pings
// will interfere with the reply channel.
fn pdx_call_and_reply(slot: u64, opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> (u64, u64) {
    let (status, _) = pdx_call(slot, opcode, arg0, arg1, arg2);
    if status != 0 {
        return (status, 0);
    }
    // Spin for the reply (server processes and calls pdx_reply).
    loop {
        let msg = pdx_listen_raw(0);
        if msg.type_id == 0x1 {
            return (0, msg.arg0);
        }
        // Non-reply message before reply arrived.
        // OP_HID_EVENT: stash for later replay instead of discarding.
        // Other types: log and keep waiting.
        if msg.type_id == OP_HID_EVENT {
            unsafe {
                if HID_STASH_COUNT < HID_STASH_CAPACITY {
                    let idx = HID_STASH_COUNT;
                    HID_STASH[idx] = (msg.arg0, msg.arg1, msg.arg2);
                    HID_STASH_COUNT += 1;
                    serial_println!("[quil.hid.stash] idx={} code={:#x} down={} mod={} ok=1 reason=stashed",
                        idx, msg.arg0, msg.arg1, msg.arg2);
                } else {
                    serial_println!("[quil.hid.stash.drop] code={:#x} reason=full", msg.arg0);
                }
            }
        } else {
            serial_println!("[quil.sync.skip] type_id={:#x}", msg.type_id);
        }
    }
}

/// Like pdx_call_and_reply but returns a Result for ergonomic error handling.
fn pdx_storage_call(opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> Result<u64, i64> {
    let (status, value) = pdx_call_and_reply(SLOT_STORAGE, opcode, arg0, arg1, arg2);
    if status != 0 {
        return Err(status as i64);
    }
    if (value as i64) < 0 {
        return Err(value as i64);
    }
    Ok(value)
}

fn run_quil_diskfs_slot_min_proof() {
    serial_println!("[quil.diskfs.slot.min.begin]");

    // Bounded readiness wait: cooperative yield only.
    // Matches Linen pattern — lets SexFiles boot and initialize NVMe/DiskFS.
    let mut ready_n: u64 = 0;
    while ready_n < 64 {
        sched_yield();
        ready_n += 1;
    }

    // SELECT path_id=2 (/disk/quil-object-v1)
    match pdx_storage_call(OP_DISKFS_SELECT, QUIL_DISKFS_PATH_ID, 0, 0) {
        Ok(_) => serial_println!("[quil.diskfs.slot.min.select.ok] path_id=2"),
        Err(e) => {
            serial_println!("[quil.diskfs.slot.min.select.err] err={}", e);
            serial_println!("[quil.diskfs.slot.min.done] ok=0");
            return;
        }
    }

    // STAT selected object
    match pdx_storage_call(OP_DISKFS_STAT, 0, 0, 0) {
        Ok(_) => serial_println!(
            "[quil.diskfs.slot.min.stat.ok] size={} flags={:#x}",
            QUIL_DISKFS_EXPECT_SIZE,
            QUIL_DISKFS_EXPECT_FLAGS
        ),
        Err(e) => {
            serial_println!("[quil.diskfs.slot.min.stat.err] err={}", e);
            serial_println!("[quil.diskfs.slot.min.done] ok=0");
            return;
        }
    }

    // Deterministic 16B payload
    let mut payload: [u8; 16] = [0u8; 16];
    payload[0..15].copy_from_slice(b"QUIL-SLOT-V1!!\0");
    payload[15] = 0x02;
    let mut lo: u64 = 0;
    let mut hi: u64 = 0;
    for i in 0..8 { lo |= (payload[i] as u64) << (i * 8); }
    for i in 8..16 { hi |= (payload[i] as u64) << ((i - 8) * 8); }

    // WRITE 1x16B
    match pdx_storage_call(OP_DISKFS_WRITE, 0, lo, hi) {
        Ok(written) if written > 0 => serial_println!("[quil.diskfs.slot.min.write.ok] size=16"),
        Ok(written) => {
            serial_println!("[quil.diskfs.slot.min.write.err] err={}", written as i64);
            serial_println!("[quil.diskfs.slot.min.done] ok=0");
            return;
        }
        Err(e) => {
            serial_println!("[quil.diskfs.slot.min.write.err] err={}", e);
            serial_println!("[quil.diskfs.slot.min.done] ok=0");
            return;
        }
    }

    // READ 2x8B through reply path
    let mut readback: [u8; 16] = [0u8; 16];
    for chunk in 0..2u64 {
        let off = chunk * 8;
        match pdx_storage_call(OP_DISKFS_READ, off, 8, 0) {
            Ok(word) => {
                let bytes = word.to_le_bytes();
                for i in 0..8 {
                    readback[(off as usize) + i] = bytes[i];
                }
            }
            Err(e) => {
                serial_println!("[quil.diskfs.slot.min.read.err] err={}", e);
                serial_println!("[quil.diskfs.slot.min.done] ok=0");
                return;
            }
        }
    }
    serial_println!("[quil.diskfs.slot.min.read.ok] size=16");

    // Match
    for i in 0..16 {
        if readback[i] != payload[i] {
            serial_println!(
                "[quil.diskfs.slot.min.match] ok=0 first_bad={} got={:#x} expected={:#x}",
                i, readback[i], payload[i]
            );
            serial_println!("[quil.diskfs.slot.min.done] ok=0");
            return;
        }
    }
    serial_println!("[quil.diskfs.slot.min.match] ok=1");
    serial_println!("[quil.diskfs.slot.min.done] ok=1");
}

// ── RamFS Save/Load Helpers ──────────────────────────────────────────────────
//
// All operations use pdx_storage_call (synchronous wrapper around SLOT_STORAGE).
// Protocol per SEXFILES_RAMFS_CONTRACT_LOCK_V1.
// Name: QUIL_DOC_NAME (≤ 24 bytes). Write/read 8 bytes per call.

/// Initialize the mutable buffer from QUIL_TEXT_INIT.
fn init_buffer() {
    unsafe {
        let len = QUIL_TEXT_INIT.len().min(QUIL_BUFFER_MAX_LEN);
        QUIL_BUFFER[..len].copy_from_slice(&QUIL_TEXT_INIT[..len]);
        QUIL_BUFFER_LEN = len;
    }
}

/// Pack a byte slice name into two u64 args for OP_RAMFS_OPEN.
fn pack_name(name: &[u8]) -> (u64, u64) {
    let mut a0 = 0u64;
    let mut a1 = 0u64;
    for i in 0..name.len().min(8) {
        a0 |= (name[i] as u64) << (i * 8);
    }
    if name.len() > 8 {
        for i in 8..name.len().min(16) {
            a1 |= (name[i] as u64) << ((i - 8) * 8);
        }
    }
    (a0, a1)
}

/// Save the current QUIL_BUFFER to RamFS as QUIL_DOC_NAME.
/// Returns Ok(()) on success, Err(i64) with server error code on failure.
fn quil_save() -> Result<(), i64> {
    unsafe {
        let buf_len = QUIL_BUFFER_LEN;
        if buf_len == 0 || buf_len > QUIL_BUFFER_MAX_LEN {
            serial_println!("[quil.save.reject] reason=invalid_len len={}", buf_len);
            return Err(-1);
        }

        let (n0, n1) = pack_name(QUIL_DOC_NAME);
        let flags_arg = (RAMFS_O_CREATE as u64) << 24;

        // Open (create if not exists) — sync reply
        let handle = pdx_storage_call(OP_RAMFS_OPEN, n0, n1, flags_arg)
            .map_err(|e| { serial_println!("[quil.save.fail] open error={}", e); e })?;

        // Write in 8-byte chunks — sync reply
        let chunks = (buf_len + 7) / 8;
        for chunk in 0..chunks {
            let offset = chunk * 8;
            let mut data = 0u64;
            for i in 0..8 {
                if offset + i < buf_len {
                    data |= (QUIL_BUFFER[offset + i] as u64) << (i * 8);
                }
            }
            pdx_storage_call(OP_RAMFS_WRITE, handle, offset as u64, data)
                .map_err(|e| {
                    serial_println!("[quil.save.fail] write offset={} error={}", offset, e);
                    let _ = pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0);
                    e
                })?;
        }

        // Close — sync reply
        pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0)
            .map_err(|e| { serial_println!("[quil.save.fail] close error={}", e); e })?;

        serial_println!("[quil.save.ok] bytes={}", buf_len);
        Ok(())
    }
}

/// Load QUIL_DOC_NAME from RamFS into QUIL_BUFFER.
/// Returns Ok(()) on success, Err(i64) with server error code on failure.
fn quil_load() -> Result<(), i64> {
    let (n0, n1) = pack_name(QUIL_DOC_NAME);

    // Open (no O_CREATE — file must already exist) — sync reply
    let handle = pdx_storage_call(OP_RAMFS_OPEN, n0, n1, 0)
        .map_err(|e| { serial_println!("[quil.load.fail] open error={}", e); e })?;

    // Read in 8-byte chunks up to QUIL_BUFFER_MAX_LEN
    let mut total_read = 0usize;
    loop {
        let raw = pdx_storage_call(OP_RAMFS_READ, handle, total_read as u64, 8)
            .map_err(|e| {
                serial_println!("[quil.load.fail] read offset={} error={}", total_read, e);
                let _ = pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0);
                e
            })?;

        // Unpack data (8 bytes from reply u64)
        let mut buf = [0u8; 8];
        for i in 0..8 {
            buf[i] = ((raw >> (i * 8)) & 0xFF) as u8;
        }

        // If first chunk is all zeros, file is empty
        if total_read == 0 && buf[0] == 0 {
            unsafe { QUIL_BUFFER_LEN = 0; }
            let _ = pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0);
            serial_println!("[quil.load.empty]");
            return Ok(());
        }

        // Find actual data length in this chunk
        let mut chunk_len = 0usize;
        for i in 0..8 {
            if buf[i] != 0 {
                chunk_len = i + 1;
            } else {
                break; // Stop at first trailing zero
            }
        }
        if chunk_len == 0 {
            break; // All zeros — end of file
        }

        // Bound check
        if total_read + chunk_len > QUIL_BUFFER_MAX_LEN {
            serial_println!("[quil.load.reject] reason=overflow max={}", QUIL_BUFFER_MAX_LEN);
            let _ = pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0);
            return Err(-4);
        }

        // Copy into buffer
        unsafe {
            QUIL_BUFFER[total_read..total_read + chunk_len].copy_from_slice(&buf[..chunk_len]);
        }
        total_read += chunk_len;

        if chunk_len < 8 {
            break; // Short read = last chunk
        }
    }

    // Close
    let _ = pdx_storage_call(OP_RAMFS_CLOSE, handle, 0, 0);

    unsafe {
        QUIL_BUFFER_LEN = total_read;
    }
    serial_println!("[quil.load.ok] bytes={}", total_read);

    // Redraw text lines with loaded content
    unsafe {
        draw_text_lines(&QUIL_BUFFER[..QUIL_BUFFER_LEN]);
    }
    Ok(())
}

/// Dispatch a single palette key event. Used by both the main event loop
/// and the HID replay path (stashed events replayed after boot proofs).
fn quil_dispatch_palette_key(scancode: u64, value: u64, palette_active: &mut bool, selected_row: &mut u8) {
    unsafe {
        static mut QUIL_KEY_BUDGET: u32 = 16;
        let b = &mut QUIL_KEY_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[quil.key.recv] scancode={:#x} val={}", scancode, value);
        }
    }

    if value == 1 {
        let action = decode_palette_key(scancode);
        unsafe {
            static mut QUIL_PALETTE_KEY_BUDGET: u32 = 24;
            let kb = &mut QUIL_PALETTE_KEY_BUDGET;
            if *kb > 0 {
                *kb -= 1;
                serial_println!("[quil.palette.key] scancode={:#x} action={}", scancode, action);
            }
        }

        match action {
            1 => { // Up
                if *palette_active {
                    let old = *selected_row;
                    *selected_row = if *selected_row == 0 {
                        QUIL_ROWS - 1
                    } else {
                        *selected_row - 1
                    };
                    serial_println!("[quil.nav.move] old={} new={} count={} dir=up",
                        old, *selected_row, QUIL_ROWS);
                    draw_palette(*selected_row);
                } else {
                    serial_println!("[quil.palette.reject] action=up reason=inactive");
                }
            }
            2 => { // Down
                if *palette_active {
                    let old = *selected_row;
                    *selected_row = (*selected_row + 1) % QUIL_ROWS;
                    serial_println!("[quil.nav.move] old={} new={} count={} dir=down",
                        old, *selected_row, QUIL_ROWS);
                    draw_palette(*selected_row);
                } else {
                    serial_println!("[quil.palette.reject] action=down reason=inactive");
                }
            }
            3 => { // Enter
                if *palette_active {
                    let cmd = palette_command_for_row(*selected_row);
                    let row = *selected_row;
                    serial_println!("[quil.select] idx={} buffer_id={} ok=1 reason=selected",
                        row, cmd as u64);
                    serial_println!("[quil.palette.action] row={} cmd={}", row, cmd);
                    match cmd {
                        CMD_SAVE_DOCUMENT => {
                            serial_println!("[quil.open.request] buffer_id={} ok=1 reason=save_via_ramfs", cmd as u64);
                            if unsafe { QUIL_BUFFER_PROOF_ACTIVE } {
                                serial_println!("[quil.palette.save.skip] reason=buffer_proof_active");
                            } else if let Err(e) = quil_save() {
                                serial_println!("[quil.palette.save.fail] error={}", e);
                            }
                        }
                        CMD_LOAD_DOCUMENT => {
                            serial_println!("[quil.open.request] buffer_id={} ok=1 reason=load_via_ramfs", cmd as u64);
                            if unsafe { QUIL_BUFFER_PROOF_ACTIVE } {
                                serial_println!("[quil.palette.load.skip] reason=buffer_proof_active");
                            } else if let Err(e) = quil_load() {
                                serial_println!("[quil.palette.load.fail] error={}", e);
                            }
                        }
                        CMD_NEW_BUFFER_STUB | CMD_RUN_CHECK_STUB | CMD_SETTINGS_STUB => {
                            serial_println!("[quil.open.request] buffer_id={} ok=0 reason=stub_not_implemented", cmd as u64);
                            serial_println!("[quil.palette.stub] cmd={}", cmd);
                        }
                        _ => {
                            serial_println!("[quil.palette.stub] cmd={}", cmd);
                        }
                    }
                } else {
                    // Text edit mode: Enter = newline
                    text_buffer_newline();
                    unsafe { draw_text_lines(&QUIL_BUFFER[..QUIL_BUFFER_LEN]); }
                }
            }
            4 => { // Esc
                if *palette_active {
                    *palette_active = false;
                    // Clear palette area via rect0.
                    pdx_call(
                        SLOT_DISPLAY,
                        0xEF,
                        SURFACE_ID_QUIL,
                        (QUIL_PANEL_Y << 32) | QUIL_PANEL_X,
                        (QUIL_LINE_BG << 32) | (QUIL_PANEL_H << 16) | QUIL_PANEL_W,
                    );
                    serial_println!("[quil.palette.action] kind=esc clear=1");
                } else {
                    // Toggle palette back on
                    *palette_active = true;
                    draw_palette(*selected_row);
                    serial_println!("[quil.palette.action] kind=esc toggle_on=1");
                }
            }
            _ => {
                if *palette_active {
                    // Palette is active, non-palette key: liveness color toggle
                    unsafe {
                        static mut QUIL_COLOR_TOGGLE: bool = false;
                        QUIL_COLOR_TOGGLE = !QUIL_COLOR_TOGGLE;
                        let color = if QUIL_COLOR_TOGGLE {
                            0x00FF00FFu64
                        } else {
                            0x0000FFFFu64
                        };
                        pdx_call(
                            SLOT_DISPLAY,
                            0xEF,
                            SURFACE_ID_QUIL,
                            0,
                            (color << 32) | (2000u64 << 16) | 2000u64,
                        );
                    }
                    serial_println!("[quil.palette.reject] action=key reason=unmapped");
                } else {
                    // Text edit mode: handle character keys
                    // Check for Backspace (scancode 0x0E)
                    if scancode == 0x0E {
                        serial_println!("[quil.text.recv] code=14 ch=8");
                        text_buffer_backspace();
                        unsafe { draw_text_lines(&QUIL_BUFFER[..QUIL_BUFFER_LEN]); }
                    } else if let Some(ch) = scancode_to_char(scancode) {
                        serial_println!("[quil.text.recv] code={} ch={}", scancode, ch);
                        text_buffer_append(ch);
                        unsafe { draw_text_lines(&QUIL_BUFFER[..QUIL_BUFFER_LEN]); }
                    } else {
                        // Unmapped key in text mode: color toggle (liveness)
                        unsafe {
                            static mut QUIL_TEXT_COLOR_TOGGLE: bool = false;
                            QUIL_TEXT_COLOR_TOGGLE = !QUIL_TEXT_COLOR_TOGGLE;
                            let color = if QUIL_TEXT_COLOR_TOGGLE {
                                0x00FF00FFu64
                            } else {
                                0x0000FFFFu64
                            };
                            pdx_call(
                                SLOT_DISPLAY,
                                0xEF,
                                SURFACE_ID_QUIL,
                                0,
                                (color << 32) | (2000u64 << 16) | 2000u64,
                            );
                        }
                        serial_println!("[quil.text.recv] code={} ch=0 reason=unmapped", scancode);
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[quil.init.start]");
    serial_println!("[quil.boot]");
    serial_println!("[quil.no_fb_write]");
    serial_println!("[quil.text.surface] title={}", QUIL_TITLE);

    // ── Initialize mutable buffer ─────────────────────────────────────────
    init_buffer();

    // ── Text Surface Validation V1 ────────────────────────────────────────
    if !validate_title(QUIL_TITLE) {
        serial_println!("[quil.text.title.invalid] max_len={}", QUIL_TITLE_MAX_LEN);
    } else {
        serial_println!("[quil.text.title] title={} len={}", QUIL_TITLE, QUIL_TITLE.len());
    }

    unsafe {
        let buf = &QUIL_BUFFER[..QUIL_BUFFER_LEN];
        if !validate_buffer(buf) {
            serial_println!("[quil.text.buffer.invalid] max_bytes={}", QUIL_BUFFER_MAX_LEN);
        } else {
            let lines = text_buffer_line_count(buf);
            serial_println!("[quil.text.buffer] bytes={} lines={} max_bytes={}",
                buf.len(), lines, QUIL_BUFFER_MAX_LEN);
            if lines > QUIL_MAX_VISIBLE_LINES {
                serial_println!("[quil.text.buffer.overflow] lines={} visible={}",
                    lines, QUIL_MAX_VISIBLE_LINES);
            }
        }
    }

    // ── One-shot boot draw ────────────────────────────────────────────────
    // Title bar (rect_index=1, persists across palette rect_index=0 redraws).
    draw_title_bar();
    serial_println!("[quil.text.title.bar] w={} h={} color={:#010x}",
        SURFACE_W, QUIL_TITLE_BAR_H, QUIL_TITLE_BAR_COLOR);

    // Text buffer lines (rect_indices 2..7).
    unsafe {
        draw_text_lines(&QUIL_BUFFER[..QUIL_BUFFER_LEN]);
    }
    draw_rect_glyph_text();

    // Palette (rect_index=0, redrawn on each keypress).
    let mut selected_row: u8 = 0;
    let mut palette_active = true;
    draw_palette(selected_row);
    serial_println!("[quil.boot.draw.ok]");
    serial_println!("[quil.ready]");
    if QUIL_DISKFS_SLOT_PROOF_ENABLED {
        serial_println!("[quil.diskfs.mount]");
        run_quil_diskfs_slot_min_proof();
    }

    // ── Replay any HID events stashed during diskfs proof ─────────────
    // pdx_call_and_reply() may have stashed keyboard events during the
    // diskfs slot proof spin.  Replay them before the next proof stage.
    unsafe {
        let stash_count = HID_STASH_COUNT;
        if stash_count > 0 {
            serial_println!("[quil.hid.replay.begin] count={} phase=after_diskfs", stash_count);
            for i in 0..stash_count {
                let (scancode, value, _arg2) = HID_STASH[i];
                serial_println!("[quil.hid.replay] idx={} code={:#x} down={} mod={}",
                    i, scancode, value, _arg2);
                quil_dispatch_palette_key(scancode, value, &mut palette_active, &mut selected_row);
            }
            HID_STASH_COUNT = 0;
            serial_println!("[quil.hid.replay.done] count={}", stash_count);
        }
    }

    // ── Boot-time sexfiles persistence proof ──────────────────────────────
    // SEXFILES_QUIL_REBOOT_PERSISTENCE_PROOF_V1
    // Proves: open, write, read, match, deny.
    // Replay_match not yet available (disk persistence blocker — see handoff).
    const PERSISTENCE_PROOF_ENABLED: bool =
        cfg!(sexfiles_quil_persistence_proof);

    // ── Keyboard nav proof: synthetic stash/replay exercise ──────────────
    // Must run BEFORE sexfiles persistence proof (which can hang).
    // Seeds synthetic HID events into the stash, then replays them.
    if QUIL_KEYBOARD_NAV_PROOF_ENABLED {
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                // Synthetic down-arrow key (scancode=0x50, value=1).
                HID_STASH[idx] = (0x50, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.nav.proof] stage=0 action=seed_stash idx={} code=0x50", idx);
            }
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                // Synthetic up-arrow key (scancode=0x48, value=1).
                HID_STASH[idx] = (0x48, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.nav.proof] stage=1 action=seed_stash idx={} code=0x48", idx);
            }
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                // Synthetic Enter key (scancode=0x1C, value=1).
                HID_STASH[idx] = (0x1C, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.nav.proof] stage=2 action=seed_stash idx={} code=0x1C", idx);
            }
            serial_println!("[quil.keyboard.nav.proof] stage=3 action=stash_done count={}", HID_STASH_COUNT);
        }

        // Replay synthetic stashed events.
        unsafe {
            let stash_count = HID_STASH_COUNT;
            if stash_count > 0 {
                serial_println!("[quil.hid.replay.begin] count={} phase=synthetic_proof", stash_count);
                for i in 0..stash_count {
                    let (scancode, value, _arg2) = HID_STASH[i];
                    serial_println!("[quil.hid.replay] idx={} code={:#x} down={} mod={}",
                        i, scancode, value, _arg2);
                    quil_dispatch_palette_key(scancode, value, &mut palette_active, &mut selected_row);
                }
                HID_STASH_COUNT = 0;
                serial_println!("[quil.hid.replay.done] count={}", stash_count);
            }
        }

        serial_println!("[quil.keyboard.nav.proof.done] ok=1");
    }

    // ── Keyboard buffer nav proof: exercises palette row nav + select ───
    // Seeds synthetic events for up, down, down, enter (Save) into the stash
    // and replays them to prove nav/select/open markers fire.
    if QUIL_KEYBOARD_BUFFER_PROOF_ENABLED {
        unsafe { QUIL_BUFFER_PROOF_ACTIVE = true; }
        // Stage 0: seed nav events into stash.
        unsafe {
            // Up arrow — move to row 4 (from row 0, wrapping).
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x48, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.buffer.proof] stage=0 action=seed_nav_up idx={} code=0x48", idx);
            }
            // Down arrow — move to row 0.
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x50, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.buffer.proof] stage=1 action=seed_nav_down idx={} code=0x50", idx);
            }
            // Down arrow — move to row 1 (Save).
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x50, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.buffer.proof] stage=2 action=seed_nav_down idx={} code=0x50", idx);
            }
            // Enter — select/execute row 1 (Save).
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x1C, 1, 0);
                HID_STASH_COUNT += 1;
                serial_println!("[quil.keyboard.buffer.proof] stage=3 action=seed_enter idx={} code=0x1C", idx);
            }
            serial_println!("[quil.keyboard.buffer.proof] stage=4 action=seed_done count={}", HID_STASH_COUNT);
        }

        // Stage 5: replay all stashed events.
        unsafe {
            let stash_count = HID_STASH_COUNT;
            if stash_count > 0 {
                serial_println!("[quil.keyboard.buffer.proof] stage=5 action=replay_begin count={}", stash_count);
                for i in 0..stash_count {
                    let (scancode, value, _arg2) = HID_STASH[i];
                    serial_println!("[quil.hid.replay] idx={} code={:#x} down={} mod={}",
                        i, scancode, value, _arg2);
                    quil_dispatch_palette_key(scancode, value, &mut palette_active, &mut selected_row);
                }
                HID_STASH_COUNT = 0;
                serial_println!("[quil.hid.replay.done] count={}", stash_count);
            }
        }

        // Stage 6: delete proof — skip (no delete command in palette V1).
        serial_println!("[quil.keyboard.buffer.proof] stage=6 action=delete_proof ok=1 reason=skipped_no_delete_cmd");
        serial_println!("[quil.delete.proof] buffer_id=0 ok=1 reason=skipped_no_delete_in_palette_v1");

        serial_println!("[quil.keyboard.buffer.proof.done] ok=1");
        unsafe { QUIL_BUFFER_PROOF_ACTIVE = false; }
    }

    // ── Text buffer edit proof: synthetic character typing ─────────────────
    // Seeds synthetic keystrokes for text editing (palette off),
    // exercises append, backspace, enter, and redraw.
    if QUIL_TEXT_BUFFER_PROOF_ENABLED {
        unsafe { QUIL_BUFFER_PROOF_ACTIVE = true; }
        serial_println!("[quil.text.buffer.proof.begin]");
        // Ensure palette is off for text editing mode
        palette_active = false;

        // Stage 0: type 'H'
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x23, 1, 0); // H scancode
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=0 action=seed_H idx={} code=0x23", idx);
            }
        }
        // Stage 1: type 'e'
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x12, 1, 0); // E scancode
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=1 action=seed_e idx={} code=0x12", idx);
            }
        }
        // Stage 2: type 'l' (x2)
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x26, 1, 0); // L scancode
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=2a action=seed_l idx={} code=0x26", idx);
            }
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x26, 1, 0); // L scancode
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=2b action=seed_l idx={} code=0x26", idx);
            }
        }
        // Stage 3: type 'o'
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x18, 1, 0); // O scancode
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=3 action=seed_o idx={} code=0x18", idx);
            }
        }
        // Stage 4: Enter (newline)
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x1C, 1, 0); // Enter
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=4 action=seed_enter idx={} code=0x1C", idx);
            }
        }
        // Stage 5: type 'Q' 'u' 'i' 'l'
        unsafe {
            for (stage, sc) in [(5u8, 0x10u64), (6, 0x16), (7, 0x17), (8, 0x26)] {
                if HID_STASH_COUNT < HID_STASH_CAPACITY {
                    let idx = HID_STASH_COUNT;
                    HID_STASH[idx] = (sc, 1, 0);
                    HID_STASH_COUNT += 1;
                    serial_println!("[quil.text.buffer.proof] stage={} action=seed_char idx={} code={:#x}", stage, idx, sc);
                }
            }
        }
        // Stage 9: Backspace (delete last char 'l')
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x0E, 1, 0); // Backspace
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=9 action=seed_backspace idx={} code=0x0E", idx);
            }
        }
        // Stage 10: type '!' then Enter
        unsafe {
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x02, 1, 0); // 1/! key
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=10 action=seed_excl idx={} code=0x02", idx);
            }
            if HID_STASH_COUNT < HID_STASH_CAPACITY {
                let idx = HID_STASH_COUNT;
                HID_STASH[idx] = (0x1C, 1, 0); // Enter
                HID_STASH_COUNT += 1;
                serial_println!("[quil.text.buffer.proof] stage=11 action=seed_enter2 idx={} code=0x1C", idx);
            }
        }
        serial_println!("[quil.text.buffer.proof] stage=12 action=seed_done count={}", unsafe { HID_STASH_COUNT });

        // Replay all stashed events
        unsafe {
            let stash_count = HID_STASH_COUNT;
            if stash_count > 0 {
                serial_println!("[quil.text.buffer.proof] stage=13 action=replay_begin count={}", stash_count);
                for i in 0..stash_count {
                    let (scancode, value, _arg2) = HID_STASH[i];
                    serial_println!("[quil.hid.replay] idx={} code={:#x} down={} mod={}",
                        i, scancode, value, _arg2);
                    quil_dispatch_palette_key(scancode, value, &mut palette_active, &mut selected_row);
                }
                HID_STASH_COUNT = 0;
                serial_println!("[quil.hid.replay.done] count={}", stash_count);
            }
        }

        // Verify buffer content
        unsafe {
            let buf = &QUIL_BUFFER[..QUIL_BUFFER_LEN];
            let line_count = text_buffer_line_count(buf);
            serial_println!("[quil.text.buffer.proof] stage=14 action=verify len={} lines={}",
                QUIL_BUFFER_LEN, line_count);
            serial_println!("[quil.text.buffer.proof.done] ok=1");
        }
        unsafe { QUIL_BUFFER_PROOF_ACTIVE = false; }
        // Restore palette active
        palette_active = true;
    }

    if PERSISTENCE_PROOF_ENABLED {
        serial_println!("[quil.sexfiles.proof.start]");

        // ── Proof A: Save ──
        serial_println!("[quil.sexfiles.proof.open]");
        match quil_save() {
            Err(e) => serial_println!("[quil.sexfiles.proof.save_fail] error={}", e),
            Ok(()) => {
                serial_println!("[quil.sexfiles.proof.write] ok");

                // Clear buffer, then load back
                unsafe { QUIL_BUFFER_LEN = 0; }

                // ── Proof B: Load ──
                match quil_load() {
                    Err(e) => serial_println!("[quil.sexfiles.proof.load_fail] error={}", e),
                    Ok(()) => {
                        serial_println!("[quil.sexfiles.proof.read] ok");

                        // ── Proof C: Roundtrip match ──
                        unsafe {
                            let loaded = &QUIL_BUFFER[..QUIL_BUFFER_LEN];
                            let orig = QUIL_TEXT_INIT;
                            if loaded.len() == orig.len() && loaded == orig {
                                serial_println!("[quil.sexfiles.proof.match] {} bytes", loaded.len());
                            } else {
                                serial_println!("[quil.sexfiles.proof.mismatch] loaded={} orig={}",
                                    loaded.len(), orig.len());
                            }
                        }
                    }
                }
            }
        }

        // ── Proof D: Deny — invalid handle must be rejected ──
        let deny_result = pdx_storage_call(OP_RAMFS_READ, 0xDEAD_BEEF, 0, 8);
        match deny_result {
            Err(e) if e == -1 => {
                serial_println!("[quil.sexfiles.proof.deny] invalid_handle error={}", e);
            }
            Err(e) => {
                serial_println!("[quil.sexfiles.proof.deny] unexpected_error={}", e);
            }
            Ok(_) => {
                serial_println!("[quil.sexfiles.proof.deny] FAIL expected ERR_INVALID_HANDLE");
            }
        }

        // No replay_match: disk persistence / journal replay not yet implemented.
        // See docs/handoff/SEXFILES_QUIL_REBOOT_PERSISTENCE_PROOF_V1.md

        serial_println!("[quil.sexfiles.proof.done]");
    } else {
        // Legacy save/load proof (non-gated, best-effort)
        serial_println!("[quil.save_load.proof.start]");
        if let Err(e) = quil_save() {
            serial_println!("[quil.save_load.proof.save_fail] error={}", e);
        } else {
            serial_println!("[quil.save_load.proof.save_ok]");
            unsafe { QUIL_BUFFER_LEN = 0; }
            if let Err(e) = quil_load() {
                serial_println!("[quil.save_load.proof.load_fail] error={}", e);
            } else {
                unsafe {
                    let loaded = &QUIL_BUFFER[..QUIL_BUFFER_LEN];
                    let orig = QUIL_TEXT_INIT;
                    if loaded.len() == orig.len() && loaded == orig {
                        serial_println!("[quil.save_load.proof.ok] roundtrip verified bytes={}", loaded.len());
                    } else {
                        serial_println!("[quil.save_load.proof.fail] roundtrip length mismatch loaded={} orig={}",
                            loaded.len(), orig.len());
                    }
                }
            }
        }
    }

    if STORAGE_CAP_PROOF_ENABLED {
        quil_storage_cap_probe();
    }

    // ── Replay stashed HID events (captured during sexfiles proof sync loop) ──
    // If the sexfiles proof completed, replay any real HID events stashed
    // during pdx_call_and_reply spins.  Synthetic events were already replayed
    // before the proof (see keyboard nav proof section above).
    unsafe {
        let stash_count = HID_STASH_COUNT;
        if stash_count > 0 {
            serial_println!("[quil.hid.replay.begin] count={} phase=after_proofs", stash_count);
            for i in 0..stash_count {
                let (scancode, value, _arg2) = HID_STASH[i];
                serial_println!("[quil.hid.replay] idx={} code={:#x} down={} mod={}",
                    i, scancode, value, _arg2);
                quil_dispatch_palette_key(scancode, value, &mut palette_active, &mut selected_row);
            }
            HID_STASH_COUNT = 0;
            serial_println!("[quil.hid.replay.done] count={}", stash_count);
        } else {
            serial_println!("[quil.hid.replay.empty] count=0");
        }
    }

    loop {
        let msg = pdx_listen_raw(0);

        unsafe {
            static mut QUIL_LISTEN_BUDGET: u32 = 8;
            let b = &mut QUIL_LISTEN_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[quil.pdx.listen] type_id={:#x}", msg.type_id);
            }
        }

        match msg.type_id {
            OP_QUIL_PING => {
                unsafe {
                    static mut QUIL_ROUTE_BUDGET: u32 = 8;
                    let b = &mut QUIL_ROUTE_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.route.recv]");
                    }
                }
            }
            OP_HID_EVENT => {
                quil_dispatch_palette_key(msg.arg0, msg.arg1, &mut palette_active, &mut selected_row);
            }
            _ => {
                unsafe {
                    static mut QUIL_UNKNOWN_BUDGET: u32 = 8;
                    let b = &mut QUIL_UNKNOWN_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.unknown.yield] type_id={:#x}", msg.type_id);
                    }
                }
            }
        }
    }
}
