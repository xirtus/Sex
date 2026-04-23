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
const BUTTON:  u32 = 0xFF313244; // Catppuccin Mocha Surface0
const DISPLAY: u32 = 0xFF11111B; // Catppuccin Mocha Crust
const ACCENT:  u32 = 0xFF89B4FA; // Catppuccin Mocha Blue

struct App {
    window: SilkWindow,
    buffer: WindowBuffer,
}

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let window = SilkWindow::create("Redox Calc Port", 400, 500, 1).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 400, 500, 400) };
        
        let mut app = Self { window, buffer };
        app.draw();
        app
    }

    fn run(&mut self, _pdx: Pdx) -> bool {
        // Handle events (stub)
        let req = sex_pdx::pdx_listen_raw(1);
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
            
            // Display
            self.buffer.draw_rect(sex_pdx::Rect { x: 10, y: 10, w: 380, h: 80 }, DISPLAY);
            font::draw_str(&mut self.buffer, 360, 40, b"0", FG, None);
            
            // Grid of buttons
            let button_w: u32 = 80;
            let button_h: u32 = 60;
            let gap: u32 = 10;
            
            for row in 0..5 {
                for col in 0..4 {
                    let bx = (10 + col * (button_w + gap)) as i32;
                    let by = (110 + row * (button_h + gap)) as i32;
                    self.buffer.draw_rect(sex_pdx::Rect { x: bx, y: by, w: button_w, h: button_h }, BUTTON);
                }
            }
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
