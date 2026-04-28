//! silk-shell — Execution Orchestration Layer
//!
//! silk-shell is NOT a service. It is part of the SexOS execution topology:
//! - Capsule lifecycle manager: spawn, suspend, resume, destroy
//! - Runtime orchestration layer
//! - Execution composition system
//! - Domain execution entry point for interactive sessions
//!
//! silk-shell is a node in the capability graph, not a peripheral utility.
//! All capsule execution flows through silk-shell's orchestration context.
//! Authority for capsule operations is granted via sex-pdx capability — not ambient.

#![no_std]

use core::slice;
use sex_pdx::{pdx_call, SLOT_DISPLAY};

pub const PANEL_HEIGHT: u32 = 48;
pub const LAUNCHER_WIDTH: u32 = 320;
pub const SCREEN_WIDTH: u32 = 1280;
pub const SCREEN_HEIGHT: u32 = 720;
pub const BG_COLOR: u32 = 0xFF1E1E2E;

// Local Opcodes
pub const OP_WINDOW_CREATE: u64 = 0xE4; // Legacy stub for lib.rs compilation
pub const OP_SET_BG: u64 = 0x100;
pub const OP_RENDER_BAR: u64 = 0x101;

/// Orchestration state for silk-shell's active execution context.
/// Tracks the composition topology: active capsules, display geometry, input focus.
#[derive(Default)]
pub struct ShellState {
    pub panel_window_id: u32,
    pub launcher_window_id: u32,
    pub bg_color: u32,          // 0xFF1E1E2E (SexOS dark)
    pub is_launcher_open: bool,
    pub current_mouse_x: i32,
    pub current_mouse_y: i32,
}

/// Handle to an isolated execution capsule managed by silk-shell.
/// Capsules are the unit of execution composition in the orchestration layer.
/// A capsule is bound to a capability domain and has a defined execution lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapsuleHandle(pub u32);

pub struct Canvas {
    fb: &'static mut [u32],
    width: u32,
    height: u32,
}

impl Canvas {
    pub fn new(fb_ptr: *mut u32, w: u32, h: u32) -> Self {
        // Safe slice wrapper — eliminates raw pointer math disaster
        let fb = unsafe { slice::from_raw_parts_mut(fb_ptr, (w * h) as usize) };
        Self { fb, width: w, height: h }
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let y_end = (y + h).min(self.height);
        let x_end = (x + w).min(self.width);
        for py in y..y_end {
            for px in x..x_end {
                let idx = (py * self.width + px) as usize;
                if idx < self.fb.len() {
                    self.fb[idx] = color;
                }
            }
        }
    }

    pub fn draw_panel(&mut self, state: &ShellState) {
        // silkbar at top
        self.fill_rect(0, 0, self.width, PANEL_HEIGHT, 0xFF0A0A14);
        // launcher area when open
        if state.is_launcher_open {
            self.fill_rect(0, PANEL_HEIGHT, LAUNCHER_WIDTH, 400, 0xFF1E1E2E);
        }
    }
}

/// PDX client for the SexDisplay compositor capability (SLOT_DISPLAY).
/// silk-shell uses this to submit display work on behalf of capsules it orchestrates.
pub struct PdxCompositorClient;

impl PdxCompositorClient {
    pub fn create_window(&self, x: i32, y: i32, w: u32, h: u32) -> u32 {
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, x as u64, y as u64, w as u64).1 as u32
    }

    pub fn set_bg(&self, color: u32) {
        let _ = pdx_call(SLOT_DISPLAY, OP_SET_BG, color as u64, 0, 0);
    }

    pub fn render_bar(&self, window_id: u32) {
        let _ = pdx_call(SLOT_DISPLAY, OP_RENDER_BAR, window_id as u64, 0, 0);
    }
}
