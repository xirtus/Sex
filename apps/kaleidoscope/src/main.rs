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

struct App {
    window: SilkWindow,
    buffer: WindowBuffer,
}

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let window = SilkWindow::create("Kaleidoscope - Browser", 1024, 768, 1).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 1024, 768, 1024) };
        
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
            
            // Toolbar
            self.buffer.draw_rect(Rect { x: 0, y: 0, w: 1024, h: 40 }, SURFACE);
            font::draw_str(&mut self.buffer, 10, 12, b"https://sexos.org", FG, None);
            
            // Content area (placeholder for Servo)
            self.buffer.draw_rect(Rect { x: 10, y: 50, w: 1004, h: 708 }, 0xFFFFFFFF);
            font::draw_str(&mut self.buffer, 450, 380, b"Servo WebRender Placeholder", 0xFF000000, None);
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
