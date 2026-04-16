#![no_std]
#![no_main]
#![forbid(unsafe_code)]

mod messages;

use sex_pdx::{pdx_listen, pdx_reply, pdx_call, MessageType, DisplayProtocol, PageHandover};
use core::sync::atomic::{AtomicU64, Ordering};

/// Phase 20: Display Server (Zero-Copy PageHandover)
/// Pure asynchronous PDX messages, Lock-free, PKU key dance.

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut compositor = Compositor::new();
    
    loop {
        sys_park();
        
        let req = pdx_listen(0);
        
        // Safety: Abstracted PDX layer assumes valid MessageType in arg0.
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::Display(proto) => match proto {
                DisplayProtocol::DisplayBufferAlloc { width, height, format } => {
                    let page = compositor.allocate_buffer(width, height, format);
                    pdx_reply(req.caller_pd, &page as *const _ as u64);
                },
                DisplayProtocol::DisplayBufferCommit { page } => {
                    compositor.commit_buffer(page);
                    pdx_reply(req.caller_pd, 0);
                },
                DisplayProtocol::Stats => {
                    let flips = compositor.frame_flips.load(Ordering::Relaxed);
                    pdx_reply(req.caller_pd, flips);
                }
            },
            MessageType::DisplayModeset { width, height, refresh } => {
                compositor.modeset(width, height, refresh);
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::DisplayCursor { x, y, visible, buffer_id: _ } => {
                compositor.update_cursor(x, y, visible);
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
    pub frame_flips: AtomicU64,
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
            frame_flips: AtomicU64::new(0),
        }
    }

    pub fn allocate_buffer(&mut self, width: u32, height: u32, _format: u32) -> PageHandover {
        // Request physical page from kernel (Slot 1, RESOLVE_PHYS 12)
        // Simplified: one large page handover for the framebuffer
        let pfn = pdx_call(1, 12, 0, 0);
        let pku_key = 5; // Display domain key
        
        PageHandover { pfn, pku_key }
    }

    pub fn commit_buffer(&mut self, page: PageHandover) {
        self.frame_flips.fetch_add(1, Ordering::Relaxed);

        // Atomic flip with PKU key swap:
        // 1. Revoke write access from caller
        pdx_call(1, 30 /* REVOKE_KEY */, page.pku_key as u64, 0);
        
        // 2. Forward to tuxedo (Slot 10) for scanout
        pdx_call(10, 0, page.pfn, page.pku_key as u64);
    }

    pub fn modeset(&mut self, w: u32, h: u32, r: u32) {
        self.width = w;
        self.height = h;
        self.refresh = r;
    }

    pub fn update_cursor(&mut self, x: i32, y: i32, visible: bool) {
        self.cursor_x = x;
        self.cursor_y = y;
        self.cursor_visible = visible;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("pause"); } }
}
