#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::{Pdx, Rect};
use sex_graphics::{WindowBuffer, font};
use core::sync::atomic::{AtomicUsize, Ordering};

// --- Bump allocator ---
const HEAP_START: usize = 0x6000_0000;
const HEAP_END:   usize = HEAP_START + 16 * 1024 * 1024;
static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);

struct BumpAlloc;
unsafe impl core::alloc::GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut cur = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let aligned = (cur + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();
            if next > HEAP_END { return core::ptr::null_mut(); }
            match HEAP_TOP.compare_exchange_weak(cur, next, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => return aligned as *mut u8,
                Err(x) => cur = x,
            }
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAlloc = BumpAlloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const BG:      u32 = 0xFF1E1E2E; 
const FG:      u32 = 0xFFCDD6F4;
const SURFACE: u32 = 0xFF313244;
const ACCENT:  u32 = 0xFF89B4FA;
const RED:     u32 = 0xFFF38BA8;

struct App {
    window: SilkWindow,
    buffer: WindowBuffer,
}

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let window = SilkWindow::create("Qupid - Media Player", 800, 200, 1).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 800, 200, 800) };
        
        let mut app = Self { window, buffer };
        app.draw();
        app
    }

    fn run(&mut self, _pdx: Pdx) -> bool {
        let req = sex_pdx::pdx_listen_raw(1);
        if req.num == 0xFF_FF { return false; }
        true
    }
}

impl App {
    fn draw(&mut self) {
        unsafe {
            self.buffer.clear(BG);
            
            // Progress bar
            self.buffer.draw_rect(Rect { x: 20, y: 140, w: 760, h: 4 }, SURFACE);
            self.buffer.draw_rect(Rect { x: 20, y: 140, w: 320, h: 4 }, ACCENT);
            
            // Controls
            let cx = 400;
            let cy = 80;
            self.buffer.draw_rect(Rect { x: cx - 50, y: cy - 20, w: 40, h: 40 }, SURFACE); // Prev
            self.buffer.draw_rect(Rect { x: cx - 20, y: cy - 20, w: 40, h: 40 }, ACCENT);  // Play
            self.buffer.draw_rect(Rect { x: cx + 30, y: cy - 20, w: 40, h: 40 }, SURFACE); // Next
            
            font::draw_str(&mut self.buffer, 395, 72, b"II", BG, None);
            
            font::draw_str(&mut self.buffer, 20, 160, b"Now Playing: COSMIC Radio - Lofi Beats", FG, None);
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
