#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::Pdx;
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

const BG:      u32 = 0xFF1E1E2E; // Catppuccin Mocha Base
const FG:      u32 = 0xFFCDD6F4; // Catppuccin Mocha Text
const CURSOR:  u32 = 0xFFF5E0DC; // Catppuccin Mocha Rosewater

struct App {
    window: SilkWindow,
    buffer: WindowBuffer,
}

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let window = SilkWindow::create("COSMIC Edit Port", 800, 600, 1).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 800, 600, 800) };
        
        let mut app = Self { window, buffer };
        app.draw();
        app
    }

    fn run(&mut self, _pdx: Pdx) -> bool {
        // Handle events (stub)
        let req = sex_pdx::pdx_listen(1);
        if req.num == 0xFF_FF { // Window close
            return false;
        }
        true
    }
}

impl App {
    fn draw(&mut self) {
        unsafe {
            self.buffer.clear(BG);
            font::draw_str(&mut self.buffer, 10, 10, b"COSMIC Edit Port Stub", FG, None);
            
            // Draw a cursor
            self.buffer.draw_rect(sex_pdx::Rect { x: 10, y: 30, w: 2, h: 16 }, CURSOR);
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
