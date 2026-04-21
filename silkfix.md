#![no_std]
#![no_main]

use sex_pdx::{pdx_call, PdxCall, PdxResult};
use sex_graphics::WindowBuffer;

// ─────────────────────────────────────────────────────────────────────────────
// 1. Core Types — River Tag Model (frozen Phase 20A)
// ─────────────────────────────────────────────────────────────────────────────
pub type TagMask = u32;

#[derive(Clone)]
pub struct Window {
    pub id: u64,
    pub tags: TagMask,
    pub buffer: WindowBuffer,
    pub mode: WindowMode,
    // add more fields only via incremental patches
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindowMode {
    Tiled,
    Floating,
    Monocle,
}

#[derive(Clone)]
pub struct OutputState {
    pub active_tags: TagMask,
}

#[derive(Clone, Copy)]
pub struct SexWindowCreateParams {
    pub tags: TagMask,
    // … other params from Phase 19
}

pub enum WindowMode { /* existing from Phase 19 */ }

// ─────────────────────────────────────────────────────────────────────────────
// 2. SexCompositor — single source of truth
// ─────────────────────────────────────────────────────────────────────────────
pub struct SexCompositor {
    windows: alloc::vec::Vec<Window>,  // heap allowed only inside kernel crate
    outputs: alloc::vec::Vec<OutputState>,
    // NO stored UI state — only kernel truth
}

impl SexCompositor {
    pub fn new() -> Self {
        Self {
            windows: alloc::vec::Vec::new(),
            outputs: alloc::vec![OutputState { active_tags: 1 }],
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // 3. Visibility — pure River math (single source)
    // ─────────────────────────────────────────────────────────────────────
    fn is_visible(&self, window: &Window) -> bool {
        window.tags & self.outputs[0].active_tags != 0
    }

    // ─────────────────────────────────────────────────────────────────────
    // 4. Core PDX Syscalls (Phase 19 + 20A)
    // ─────────────────────────────────────────────────────────────────────
    pub fn handle_pdx_call(&mut self, call: PdxCall) -> PdxResult {
        match call {
            PdxCall::SexWindowCreate(params) => {
                let window = Window {
                    id: self.next_id(),
                    tags: params.tags,           // inherit current view
                    buffer: WindowBuffer::new(),
                    mode: WindowMode::Tiled,
                };
                self.windows.push(window);
                Ok(0)
            }
            PdxCall::SexWindowSetTags { window_id, tags } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
                    w.tags = tags;
                }
                Ok(0)
            }
            PdxCall::SexViewSetTags { tags } => {
                self.outputs[0].active_tags = tags;
                Ok(0)
            }
            PdxCall::SexViewToggleTag { tag_bit } => {
                self.outputs[0].active_tags ^= tag_bit;
                Ok(0)
            }
            _ => Ok(0), // other Phase 19 calls
        }
    }

    fn next_id(&self) -> u64 { /* simple counter */ 0 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry for sexdisplay main loop (Phase 19)
// ─────────────────────────────────────────────────────────────────────────────
pub fn sexdisplay_main() {
    let mut compositor = SexCompositor::new();
    // pdx_listen loop + event forwarding already wired in Phase 19
    // render loop will call evaluate_frame() once pipeline is added incrementally
}
