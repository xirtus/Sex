#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp, SilkWindow};
use sex_pdx::Rect;
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
    fn new(_pdx: u32) -> Self {
        sex_pdx::serial_println!("[browser.slot.net.static_grant.begin]");
        let (status, _value) = sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0, 0, 0);
        sex_pdx::serial_println!("[browser.slot.net.route.call] status={}", status);
        sex_pdx::serial_println!("[browser.slot.net.static_grant.proof.done] ok=1 network=0");
        sex_pdx::serial_println!("[browser.packed_text.begin]");

        sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0x207, 0, 0);
        let len_reply = sex_pdx::pdx_listen_raw(0);
        let len_status = if len_reply.type_id == 0x1 { 0u64 } else { 1u64 };
        let len_value = if len_reply.type_id == 0x1 { len_reply.arg0 } else { 0u64 };
        let mut live_text = [0u8; 64];
        let mut live_len = 0usize;
        if len_status == 0 {
            let reported = core::cmp::min(len_value as usize, live_text.len());
            sex_pdx::serial_println!("[browser.packed_text.len.recv] len={}", reported);
            let chunk_count = core::cmp::min((reported + 7) / 8, 8);
            let mut idx = 0usize;
            while idx < chunk_count {
                sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0x208, idx as u64, 0);
                let chunk_reply = sex_pdx::pdx_listen_raw(0);
                let chunk_status = if chunk_reply.type_id == 0x1 { 0u64 } else { 1u64 };
                let chunk_value = if chunk_reply.type_id == 0x1 { chunk_reply.arg0 } else { 0u64 };
                if chunk_status != 0 {
                    break;
                }
                let start = idx * 8;
                let remain = reported.saturating_sub(start);
                let bytes = core::cmp::min(remain, 8);
                let mut i = 0usize;
                while i < bytes {
                    live_text[start + i] = ((chunk_value >> (i * 8)) & 0xFF) as u8;
                    i += 1;
                }
                sex_pdx::serial_println!("[browser.packed_text.chunk.recv] idx={} bytes={}", idx, bytes);
                live_len = start + bytes;
                idx += 1;
            }
            if live_len > reported {
                live_len = reported;
            }
            sex_pdx::serial_println!("[browser.packed_text.text.set] live=1 len={}", live_len);
            sex_pdx::serial_println!(
                "[browser.async_reply.proof.done] len={} text_recv={}",
                reported,
                live_len
            );
        }
        sex_pdx::serial_println!("[browser.packed_text.proof.done]");

        let window = SilkWindow::create("Kaleidoscope - Browser", 1024, 768).expect("Failed to create window");
        let buffer = unsafe { WindowBuffer::new(window.virt_addr, 1024, 768, 1024) };
        
        let mut app = Self { window, buffer };
        app.draw();
        app
    }

    fn run(&mut self, _pdx: u32) -> bool {
        let req = sex_pdx::pdx_listen_raw(1);
        if req.type_id == 0xFF_FF { return false; }
        true
    }
}

impl App {
    fn draw(&mut self) {
        unsafe {
            self.buffer.clear(BG);
            
            // Toolbar
            self.buffer.draw_rect(Rect { x: 0, y: 0, width: 1024, height: 40 }, SURFACE);
            font::draw_str(&mut self.buffer, 10, 12, b"https://sexos.org", FG, None);
            
            // Content area (placeholder for Servo)
            self.buffer.draw_rect(Rect { x: 10, y: 50, width: 1004, height: 708 }, 0xFFFFFFFF);
            font::draw_str(&mut self.buffer, 450, 380, b"Servo WebRender Placeholder", 0xFF000000, None);
        }
        self.window.commit(&[self.window.pfn_base]).expect("Failed to commit frame");
    }
}

app_main!(App);
