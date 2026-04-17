#![no_std]
#![no_main]
#![allow(unsafe_code)]

mod messages;

use sex_pdx::{pdx_listen, pdx_reply, pdx_call, MessageType, DisplayProtocol, PageHandover, Rect, OrbitalEvent};
use core::sync::atomic::{AtomicU64, Ordering};

/// Phase 20: Sex-Orbital Display Server
/// Pure asynchronous PDX messages, Lock-free, zero-copy window buffers via PKU.

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24);
    }
}

const MAX_WINDOWS: usize = 64;

struct Window {
    id: u32,
    rect: Rect,
    pfn: u64,
    pku_key: u8,
    active: bool,
    owner_pd: u32,
}

struct Compositor {
    windows: [Option<Window>; MAX_WINDOWS],
    screen_w: u32,
    screen_h: u32,
    next_win_id: u32,
    pub frame_flips: AtomicU64,
}

impl Compositor {
    pub fn new() -> Self {
        const INIT_WIN: Option<Window> = None;
        Self {
            windows: [INIT_WIN; MAX_WINDOWS],
            screen_w: 1920,
            screen_h: 1080,
            next_win_id: 1,
            frame_flips: AtomicU64::new(0),
        }
    }

    pub fn create_window(&mut self, owner_pd: u32, rect: Rect) -> u32 {
        let id = self.next_win_id;
        self.next_win_id += 1;

        // Allocate physical pages for the buffer (1024x1024x4 = 4MB approx)
        let pages = ((rect.w * rect.h * 4) + 4095) / 4096;
        let pfn = pdx_call(1, 12 /* ALLOC_PHYS */, pages as u64, 0);
        let pku_key = 6; // Application-specific buffer key

        for slot in self.windows.iter_mut() {
            if slot.is_none() {
                *slot = Some(Window { id, rect, pfn, pku_key, active: true, owner_pd });
                break;
            }
        }
        id
    }

    pub fn get_window_buffer(&self, window_id: u32, caller_pd: u32) -> u64 {
        for win in self.windows.iter().flatten() {
            if win.id == window_id && win.owner_pd == caller_pd {
                // Return virtual address mapped into caller's PD
                return pdx_call(1, 13 /* MAP_INTO_PD */, caller_pd as u64, win.pfn);
            }
        }
        0
    }

    pub fn composite(&self) {
        self.frame_flips.fetch_add(1, Ordering::Relaxed);
        for win in self.windows.iter().flatten() {
            if win.active {
                // Zero-copy blit via PDX call to hardware scanout/tuxedo
                pdx_call(10 /* TRANNY */, 0 /* BLIT */, win.pfn, win.pku_key as u64);
            }
        }
    }

    pub fn destroy_window(&mut self, window_id: u32, caller_pd: u32) {
        for slot in self.windows.iter_mut() {
            if let Some(win) = slot {
                if win.id == window_id && win.owner_pd == caller_pd {
                    pdx_call(1, 14 /* FREE_PHYS */, win.pfn, 0);
                    *slot = None;
                    break;
                }
            }
        }
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
                DisplayProtocol::CreateWindow { x, y, w, h, .. } => {
                    let id = compositor.create_window(req.caller_pd, Rect { x, y, w, h });
                    pdx_reply(req.caller_pd, id as u64);
                },
                DisplayProtocol::RequestBuffer { window_id } => {
                    let addr = compositor.get_window_buffer(window_id, req.caller_pd);
                    pdx_reply(req.caller_pd, addr);
                },
                DisplayProtocol::CommitDamage { window_id, .. } => {
                    compositor.composite();
                    pdx_reply(req.caller_pd, 0);
                },
                DisplayProtocol::DmaBufferSubmit { page, offset, len } => {
                    // Forward to sexdrive (Slot 11) for GPU processing
                    // Zero-copy DMA: we just pass the PageHandover (PFN)
                    let res = pdx_call(11 /* sexdrive */, 0x100 /* GPU_SUBMIT */, page.pfn, (*offset as u64) << 32 | *len as u64);
                    pdx_reply(req.caller_pd, res);
                },
                DisplayProtocol::FenceWait { fence_id } => {
                    let res = pdx_call(11, 0x101 /* FENCE_WAIT */, *fence_id, 0);
                    pdx_reply(req.caller_pd, res);
                },
                DisplayProtocol::DestroyWindow { window_id } => {
                    compositor.destroy_window(window_id, req.caller_pd);
                    pdx_reply(req.caller_pd, 0);
                },
                DisplayProtocol::Stats => {
                    let flips = compositor.frame_flips.load(Ordering::Relaxed);
                    pdx_reply(req.caller_pd, flips);
                },
                _ => { pdx_reply(req.caller_pd, u64::MAX); }
            },
            _ => { pdx_reply(req.caller_pd, u64::MAX); }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("pause"); } }
}
