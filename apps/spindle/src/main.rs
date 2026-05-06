//! Spindle V1 — SexOS native command console scaffold
//!
//! Architecture:
//!   • Static bounded text surface via sex-graphics CP437 font
//!   • Window created via PDX OP_WINDOW_CREATE on sexdisplay slot 5
//!   • Fixed PFN base for framebuffer (matches sexsh convention)
//!   • No input handling yet — static content only
//!   • No command execution yet
//!   • No terminal emulation (Spindle is NOT sexsh)
//!
//! Contract: docs/handoff/SPINDLE_APP_CONTRACT_V1.md
//! Next: SPINDLE_KEYBOARD_INPUT_LINE_V1

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

    // ── Idle loop (no input handling yet) ──
    loop {
        // Yield — future: listen for HID events, keyboard input, command dispatch
        unsafe {
            sex_pdx::pdx_listen_raw(0);
        }
    }
}
