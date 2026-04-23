#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::Pdx;
use core::sync::atomic::{AtomicUsize, Ordering};

// --- Bump allocator ---
const HEAP_START: usize = 0x6000_0000;
const HEAP_END:   usize = HEAP_START + 16 * 1024 * 1024;
static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);
struct Bump;
unsafe impl core::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: core::alloc::Layout) -> *mut u8 {
        let mut c = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let a = (c + l.align() - 1) & !(l.align() - 1);
            let n = a + l.size();
            if n > HEAP_END { return core::ptr::null_mut(); }
            match HEAP_TOP.compare_exchange_weak(c, n, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => return a as *mut u8,
                Err(x) => c = x,
            }
        }
    }
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {}
}
#[global_allocator]
static ALLOC: Bump = Bump;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

// This is a stub for a full port of COSMIC Files.
// A real port would involve significant work to adapt the original codebase.

struct App;

impl SexApp for App {
    fn new(_pdx: Pdx) -> Self {
        let _window = SilkWindow::create("COSMIC Files Port", 800, 600, 1).unwrap();
        // In a real port, you would initialize the COSMIC Files UI here,
        // passing it the window buffer.
        Self
    }

    fn run(&mut self, _pdx: Pdx) -> bool {
        // Handle events and update the UI
        true
    }
}

app_main!(App);
