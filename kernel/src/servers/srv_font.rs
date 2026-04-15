use crate::serial_println;
use crate::ipc_ring::SpscRing;
use alloc::sync::Arc;
use spin::Mutex;

/// srv_font: TrueType/OpenType Font Rendering Server.
/// Provides glyph bitmaps via zero-copy SAS shared memory.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlyphRequest {
    pub font_id: u32,
    pub char_code: u32,
    pub size: u32,
    pub target_vaddr: u64, // Virtual address in SAS to render the bitmap into
}

pub struct FontServer {
    pub name: &'static str,
    pub request_queue: SpscRing<GlyphRequest>,
}

impl FontServer {
    pub fn new() -> Self {
        Self {
            name: "srv_font",
            request_queue: SpscRing::new(),
        }
    }

    /// The main event loop for the font server.
    pub fn run_loop(&self) {
        serial_println!("srv_font: Font Server loop started.");
        loop {
            if let Some(req) = self.request_queue.dequeue() {
                serial_println!("srv_font: Rendering Glyph {:#x} (Size: {}) to {:#x}", 
                    req.char_code, req.size, req.target_vaddr);
                
                // 1. In a production system, use Freetype or Fontdue here.
                // 2. Render the glyph into the target virtual address.
                self.render_mock_glyph(req.target_vaddr, req.size);
            }
            
            // Wait for more requests
            x86_64::instructions::hlt();
        }
    }

    /// Simulated glyph rendering (Draws a simple box for the prototype).
    fn render_mock_glyph(&self, vaddr: u64, size: u32) {
        let ptr = vaddr as *mut u8;
        let dim = size as usize;
        unsafe {
            for y in 0..dim {
                for x in 0..dim {
                    // Draw a simple border
                    if x == 0 || x == dim-1 || y == 0 || y == dim-1 {
                        *ptr.add(y * dim + x) = 0xFF; // White
                    } else {
                        *ptr.add(y * dim + x) = 0x00; // Black
                    }
                }
            }
        }
    }
}

pub extern "C" fn font_entry(arg: u64) -> u64 {
    serial_println!("srv_font PDX: Received request {:#x}", arg);
    0
}
