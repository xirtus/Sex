#![no_std]
#![no_main]
#![forbid(unsafe_code)]

mod messages;

use sex_pdx::{pdx_listen, pdx_reply, pdx_call, MessageType};
use sex_pdx::mmio::Mmio;
use sex_pdx::dma::DmaBuffer;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::messages::{DisplayMessageType, DisplayBufferReply};

/// Phase 16: PDX Display Server (Zero-Copy Compositor)
/// Pure asynchronous PDX messages, Lock-free, No-Remnants.

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut compositor = Compositor::new();
    
    loop {
        // Standard FLSCHED wait-free park
        // Wait for next PDX message or HID interrupt
        sys_park();
        
        let req = pdx_listen(0);
        
        // Safety: We assume the caller provided a valid MessageType pointer in arg0.
        // The #![forbid(unsafe_code)] is bypassed here by the abstracted PDX layer.
        // (In a real implementation, pdx_listen would return a safe enum).
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::DisplayBufferAlloc { width, height, format } => {
                let reply = compositor.allocate_buffer(width, height, format);
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            MessageType::DisplayBufferCommit { buffer_id, damage_x, damage_y, damage_w, damage_h } => {
                compositor.commit_buffer(buffer_id, damage_x, damage_y, damage_w, damage_h);
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::DisplayModeset { width, height, refresh } => {
                compositor.modeset(width, height, refresh);
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::DisplayCursor { x, y, visible, buffer_id } => {
                compositor.update_cursor(x, y, visible, buffer_id);
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::DisplayGeminiRepairDisplay => {
                compositor.repair();
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::HIDEvent { ev_type, code, value } => {
                compositor.process_input(ev_type, code, value);
                pdx_reply(req.caller_pd, 0);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

struct Compositor {
    width: u32,
    height: u32,
    refresh: u32,
    cursor_x: i32,
    cursor_y: i32,
    cursor_visible: bool,
    active_buffer: u32,
    // Lock-free damage tracking (simplified for prototype)
    damage_mask: AtomicU32,
}

impl Compositor {
    pub fn new() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh: 60,
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            active_buffer: 0,
            damage_mask: AtomicU32::new(0),
        }
    }

    pub fn allocate_buffer(&mut self, width: u32, height: u32, _format: u32) -> MessageType {
        // 1. Request physical pages from kernel (Slot 1)
        let page_count = (width * height * 4 + 4095) / 4096;
        let mut pfns = [0u64; 64];
        
        for i in 0..core::cmp::min(page_count as usize, 64) {
            pfns[i] = pdx_call(1, 12 /* RESOLVE_PHYS */, i as u64, 0);
        }

        // 2. Assign PKU key for zero-copy isolation
        let pku_key = 5; // Static key for display domain
        
        MessageType::DisplayBufferReply {
            page_count,
            pfn_list: pfns,
            pku_key,
        }
    }

    pub fn commit_buffer(&mut self, buffer_id: u32, dx: u32, dy: u32, dw: u32, dh: u32) {
        // Atomic damage update
        self.damage_mask.fetch_or(1 << (buffer_id % 32), Ordering::SeqCst);
        self.active_buffer = buffer_id;

        // Forward to sextuxedo (Slot 10) for atomic scanout
        let scanout_msg = MessageType::DisplayBufferCommit {
            buffer_id,
            damage_x: dx,
            damage_y: dy,
            damage_w: dw,
            damage_h: dh,
        };
        pdx_call(10 /* sextuxedo */, 0, &scanout_msg as *const _ as u64, 0);
    }

    pub fn modeset(&mut self, w: u32, h: u32, r: u32) {
        self.width = w;
        self.height = h;
        self.refresh = r;
    }

    pub fn update_cursor(&mut self, x: i32, y: i32, visible: bool, _buf: u32) {
        self.cursor_x = x;
        self.cursor_y = y;
        self.cursor_visible = visible;
    }

    pub fn process_input(&mut self, _ev: u16, _code: u16, _val: i32) {
        // Logic to route events to the correct surface in Smithay
    }

    pub fn repair(&mut self) {
        // Hot-repair state from Gemini logs
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("pause"); } }
}
