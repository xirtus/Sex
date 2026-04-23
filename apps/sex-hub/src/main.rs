#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::{Pdx, Rect, pdx_spawn_pd};
use sex_graphics::{WindowBuffer, font};
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;

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

struct AppEntry {
    name: &'static str,
    exec: &'static [u8],
}

const APPS: &[AppEntry] = &[
    AppEntry { name: "Files",      exec: b"sex-files" },
    AppEntry { name: "Editor",     exec: b"sex-edit" },
    AppEntry { name: "Calculator", exec: b"sex-calc" },
    AppEntry { name: "Browser",    exec: b"kaleidoscope" },
    AppEntry { name: "Player",     exec: b"qupid" },
];

struct App {
    window: SilkWindow,
    buffer: WindowBuffer,
    selected: usize,
}

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let window = SilkWindow::create("SexHub - App Store", 600, 400, 1).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 600, 400, 600) };
        
        let mut app = Self { window, buffer, selected: 0 };
        app.draw();
        app
    }

    fn run(&mut self, _pdx: Pdx) -> bool {
        let req = sex_pdx::pdx_listen_raw(1);
        match req.num {
            0xFF_FF => return false,
            0x11 => { // HID Key
                let code = (req.arg0 & 0xFFFF) as u16;
                let value = req.arg1 as i32;
                if value == 1 { // Pressed
                    match code {
                        103 => { // Up
                            if self.selected > 0 { self.selected -= 1; self.draw(); }
                        }
                        108 => { // Down
                            if self.selected + 1 < APPS.len() { self.selected += 1; self.draw(); }
                        }
                        28 => { // Enter
                            let _ = pdx_spawn_pd(APPS[self.selected].exec);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        true
    }
}

impl App {
    fn draw(&mut self) {
        unsafe {
            self.buffer.clear(BG);
            font::draw_str(&mut self.buffer, 20, 20, b"SexHub Application Store", ACCENT, None);
            
            for (i, app) in APPS.iter().enumerate() {
                let y = 60 + i as i32 * 40;
                let bg = if i == self.selected { SURFACE } else { BG };
                self.buffer.draw_rect(Rect { x: 20, y, w: 560, h: 36 }, bg);
                font::draw_str(&mut self.buffer, 40, (y + 10) as u32, app.name.as_bytes(), FG, None);
            }
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
