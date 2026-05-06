#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen_raw, serial_println, OP_QUIL_PING, SLOT_DISPLAY};

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
const QUIL_TEXT_BUFFER: &[u8] = b"This is the Quil text surface.\n\
A minimal no_std editor prototype.\n\
Built on the Sex Microkernel.\n\
No text rendering available yet.\n\
Fill-rect visual representation.\n\
Buffer capacity: bounded static array.\n\
Press arrows to navigate, ESC for cmds.";
const QUIL_BUFFER_MAX_LEN: usize = 512;
const QUIL_MAX_VISIBLE_LINES: usize = 6;

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

/// Palette command IDs. No execution — marker stubs only.
const CMD_NEW_BUFFER_STUB: u8 = 1;
const CMD_OPEN_OBJECT_STUB: u8 = 2;
const CMD_AGENT_REVIEW_STUB: u8 = 3;
const CMD_RUN_CHECK_STUB: u8 = 4;
const CMD_SETTINGS_STUB: u8 = 5;

/// Map palette row index to command ID.
/// Row 0 is top (index 0), row 4 is bottom.
const PALETTE_COMMANDS: [u8; QUIL_ROWS as usize] = [
    CMD_NEW_BUFFER_STUB,    // row 0
    CMD_OPEN_OBJECT_STUB,   // row 1
    CMD_AGENT_REVIEW_STUB,  // row 2
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

/// Draw text buffer lines as visual fill rects (rect_indices 2..7).
fn draw_text_lines(buf: &[u8]) {
    let line_count = text_buffer_line_count(buf);
    serial_println!("[quil.text.lines] count={} bytes={}", line_count, buf.len());

    let text_area_h = QUIL_MAX_VISIBLE_LINES as u64 * (QUIL_LINE_H + QUIL_LINE_GAP);
    // Text area background (rect_index=2).
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        ((QUIL_TEXT_AREA_Y as u64) << 32) | 0u64,
        (2u64 << 56)
            | (QUIL_LINE_BG << 32)
            | (text_area_h << 16)
            | SURFACE_W,
    );
    serial_println!("[quil.text.bg] y={} h={}", QUIL_TEXT_AREA_Y, text_area_h);

    let show_lines = line_count.min(QUIL_MAX_VISIBLE_LINES);
    for i in 0..show_lines {
        if i >= 5 { // rect_indices 3-7 cover 5 lines max
            serial_println!("[quil.text.line.skip] index={} reason=max_rects", i);
            break;
        }
        let rect_index = (3 + i) as u64; // bits 56-59: 3, 4, 5, 6, 7
        let y = QUIL_TEXT_AREA_Y + (i as u64) * (QUIL_LINE_H + QUIL_LINE_GAP);

        // Line fill.
        pdx_call(
            SLOT_DISPLAY,
            0xEF,
            SURFACE_ID_QUIL,
            (y << 32) | QUIL_LINE_X as u64,
            (rect_index << 56)
                | (QUIL_LINE_COLOR << 32)
                | (QUIL_LINE_H << 16)
                | QUIL_LINE_W as u64,
        );
        serial_println!("[quil.text.line] index={} rect={} y={}", i, rect_index, y);

        // Left accent overrides the line's left edge (same rect_index).
        pdx_call(
            SLOT_DISPLAY,
            0xEF,
            SURFACE_ID_QUIL,
            (y << 32) | QUIL_LINE_X as u64,
            (rect_index << 56)
                | (QUIL_LINE_ACCENT_COLOR << 32)
                | (QUIL_LINE_H << 16)
                | QUIL_LINE_ACCENT_W as u64,
        );
    }

    if line_count > QUIL_MAX_VISIBLE_LINES {
        serial_println!("[quil.text.buffer.overflow] lines={} visible={}",
            line_count, QUIL_MAX_VISIBLE_LINES);
    }
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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[quil.boot]");
    serial_println!("[quil.no_fb_write]");
    serial_println!("[quil.text.surface] title={}", QUIL_TITLE);

    // ── Text Surface Validation V1 ────────────────────────────────────────
    if !validate_title(QUIL_TITLE) {
        serial_println!("[quil.text.title.invalid] max_len={}", QUIL_TITLE_MAX_LEN);
    } else {
        serial_println!("[quil.text.title] title={} len={}", QUIL_TITLE, QUIL_TITLE.len());
    }

    if !validate_buffer(QUIL_TEXT_BUFFER) {
        serial_println!("[quil.text.buffer.invalid] max_bytes={}", QUIL_BUFFER_MAX_LEN);
    } else {
        let lines = text_buffer_line_count(QUIL_TEXT_BUFFER);
        serial_println!("[quil.text.buffer] bytes={} lines={} max_bytes={}",
            QUIL_TEXT_BUFFER.len(), lines, QUIL_BUFFER_MAX_LEN);
        if lines > QUIL_MAX_VISIBLE_LINES {
            serial_println!("[quil.text.buffer.overflow] lines={} visible={}",
                lines, QUIL_MAX_VISIBLE_LINES);
        }
    }

    // ── One-shot boot draw ────────────────────────────────────────────────
    // Title bar (rect_index=1, persists across palette rect_index=0 redraws).
    draw_title_bar();
    serial_println!("[quil.text.title.bar] w={} h={} color={:#010x}",
        SURFACE_W, QUIL_TITLE_BAR_H, QUIL_TITLE_BAR_COLOR);

    // Text buffer lines (rect_indices 2..7).
    draw_text_lines(QUIL_TEXT_BUFFER);

    // Palette (rect_index=0, redrawn on each keypress).
    let mut selected_row: u8 = 0;
    draw_palette(selected_row);
    serial_println!("[quil.boot.draw.ok]");

    let mut palette_active = true;

    serial_println!("[quil.ready]");

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
                let scancode = msg.arg0;
                let value = msg.arg1;
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
                        1 => {
                            if palette_active {
                                selected_row = if selected_row == 0 {
                                    QUIL_ROWS - 1
                                } else {
                                    selected_row - 1
                                };
                                draw_palette(selected_row);
                            } else {
                                serial_println!("[quil.palette.reject] action=up reason=inactive");
                            }
                        }
                        2 => {
                            if palette_active {
                                selected_row = (selected_row + 1) % QUIL_ROWS;
                                draw_palette(selected_row);
                            } else {
                                serial_println!("[quil.palette.reject] action=down reason=inactive");
                            }
                        }
                        3 => {
                            if palette_active {
                                let cmd = palette_command_for_row(selected_row);
                                serial_println!("[quil.palette.action] row={} cmd={}", selected_row, cmd);
                            } else {
                                serial_println!("[quil.palette.reject] action=enter reason=inactive");
                            }
                        }
                        4 => {
                            if palette_active {
                                palette_active = false;
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
                                serial_println!("[quil.palette.reject] action=esc reason=inactive");
                            }
                        }
                        _ => {
                            // Existing liveness fallback for non-palette keys.
                            static mut QUIL_COLOR_TOGGLE: bool = false;
                            unsafe {
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
                        }
                    }
                }
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
