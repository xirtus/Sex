//! silk-shell — Desktop Shell for Sex microkernel SASOS
//! Protected by physical Intel MPK/PKU/PKEY domains (Phase 25)

#![no_std]

use core::slice;
use sex_pdx::{SLOT_DISPLAY, OP_SET_BG, OP_RENDER_BAR, OP_WINDOW_CREATE};

pub const PANEL_HEIGHT: u32 = 48;
pub const LAUNCHER_WIDTH: u32 = 320;

#[derive(Default)]
pub struct ShellState {
    pub panel_window_id: u32,
    pub launcher_window_id: u32,
    pub bg_color: u32,          // 0xFF1E1E2E (SexOS dark)
    pub is_launcher_open: bool,
    pub current_mouse_x: i32,
    pub current_mouse_y: i32,
}

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

// PDX compositor client (typed, no magic)
pub struct PdxCompositorClient;

impl PdxCompositorClient {
    pub fn create_window(&self, x: i32, y: i32, w: u32, h: u32) -> u32 {
        // Uses sex-pdx constants — hardware PKEY 1 (sexdisplay) locked
        unsafe { sex_rt::pdx_call(SLOT_DISPLAY as u32, OP_WINDOW_CREATE, x as u64, y as u64, w as u64, h as u64) as u32 }
    }

    pub fn set_bg(&self, color: u32) {
        unsafe { sex_rt::pdx_call(SLOT_DISPLAY as u32, OP_SET_BG, color as u64, 0, 0, 0); }
    }

    pub fn render_bar(&self, window_id: u32) {
        unsafe { sex_rt::pdx_call(SLOT_DISPLAY as u32, OP_RENDER_BAR, window_id as u64, 0, 0, 0); }
    }
}
