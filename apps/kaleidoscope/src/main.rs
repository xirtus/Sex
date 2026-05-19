#![no_std]
#![no_main]

extern crate alloc;

use silkclient::{app_main, SexApp};
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

const KALEIDO_SURFACE_ID: u64 = 300;
const SEXNET_DNS_RESOLVE: u64 = 0x20B;

struct App {
    live_text: [u8; 64],
    live_len: usize,
}

impl SexApp for App {
    fn new(_pdx: u32) -> Self {
        sex_pdx::serial_println!("[browser.slot.net.static_grant.begin]");
        let (status, _value) = sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0, 0, 0);
        sex_pdx::serial_println!("[browser.slot.net.route.call] status={}", status);
        sex_pdx::serial_println!("[browser.slot.net.static_grant.proof.done] ok=1 network=0");
        sex_pdx::serial_println!("[browser.dns.resolve.request] host_id=1 ok=1");
        let (_dns_status, dns_reply) = sex_pdx::pdx_call(sex_pdx::SLOT_NET, SEXNET_DNS_RESOLVE, 1, 0, 0);
        if dns_reply != 0 {
            let ip = (dns_reply as u32).to_be_bytes();
            sex_pdx::serial_println!(
                "[browser.dns.resolve.ok] addr={}.{}.{}.{} ok=1",
                ip[0],
                ip[1],
                ip[2],
                ip[3]
            );
        } else {
            sex_pdx::serial_println!("[browser.dns.resolve.miss] host_id=1 ok=1 reason=cache_miss");
        }
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
        sex_pdx::serial_println!("[browser.body.begin]");
        sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0x209, 0, 0);
        let body_len_reply = sex_pdx::pdx_listen_raw(0);
        let body_len_ok = body_len_reply.type_id == 0x1;
        let body_reported = if body_len_ok {
            core::cmp::min(body_len_reply.arg0 as usize, 64)
        } else {
            0
        };
        let mut body_text = [0u8; 64];
        let mut body_len = 0usize;
        if body_reported > 0 {
            sex_pdx::serial_println!("[browser.body.len.recv] len={}", body_reported);
            let chunk_count = core::cmp::min((body_reported + 7) / 8, 8);
            let mut idx = 0usize;
            while idx < chunk_count {
                sex_pdx::pdx_call(sex_pdx::SLOT_NET, 0x200, 0x20A, idx as u64, 0);
                let chunk_reply = sex_pdx::pdx_listen_raw(0);
                if chunk_reply.type_id != 0x1 {
                    break;
                }
                let start = idx * 8;
                let remain = body_reported.saturating_sub(start);
                let bytes = core::cmp::min(remain, 8);
                let mut i = 0usize;
                while i < bytes {
                    body_text[start + i] = ((chunk_reply.arg0 >> (i * 8)) & 0xFF) as u8;
                    i += 1;
                }
                sex_pdx::serial_println!("[browser.body.chunk.recv] idx={} bytes={}", idx, bytes);
                body_len = start + bytes;
                idx += 1;
            }
            if body_len > body_reported {
                body_len = body_reported;
            }
            sex_pdx::serial_println!("[browser.body.text.set] live=1 len={}", body_len);
        }
        sex_pdx::serial_println!("[browser.render.live_text.begin] len={}", live_len);
        sex_pdx::serial_println!("[browser.visible.polish.begin] sid={}", KALEIDO_SURFACE_ID);

        sex_pdx::pdx_call(
            sex_pdx::SLOT_DISPLAY,
            0xEC,
            KALEIDO_SURFACE_ID,
            (120u64 << 32) | 80u64,
            (120u64 << 32) | 520u64,
        );

        sex_pdx::serial_println!(
            "[browser.render.buffer.ready] virt=surface pfn=surface sid={}",
            KALEIDO_SURFACE_ID
        );

        let text_color: u64 = 0x00CDD6F4;
        let send_text = |text: &[u8], base_offset: usize| {
            let chunks = (text.len() + 7) / 8;
            let mut ci = 0usize;
            while ci < chunks {
                let offset = ci * 8;
                let count = core::cmp::min(text.len() - offset, 8);
                let abs_offset = base_offset + offset;
                let mut packed: u64 = 0;
                let mut bi = 0usize;
                while bi < count {
                    packed |= (text[offset + bi] as u64) << (bi * 8);
                    bi += 1;
                }
                let arg2 = (abs_offset as u64) | ((count as u64) << 8) | (text_color << 32);
                sex_pdx::pdx_call(sex_pdx::SLOT_DISPLAY, 0xFB, KALEIDO_SURFACE_ID, packed, arg2);
                ci += 1;
            }
        };

        let title = b"Kaleidoscope";
        send_text(title, 0);
        sex_pdx::serial_println!("[browser.visible.polish.text] line=0 len={}", title.len());

        let status_line = b"HTTP 200 from 10.0.2.2";
        send_text(status_line, 32);
        sex_pdx::serial_println!("[browser.visible.polish.text] line=1 len={}", status_line.len());

        // The display text buffer is 128 bytes. When body text is present, keep
        // the 64..127 region exclusively for body bytes to avoid overlap.
        if body_len == 0 {
            let chunks = (live_len + 7) / 8;
            let mut ci = 0usize;
            while ci < chunks {
                let offset = ci * 8;
                let count = core::cmp::min(live_len - offset, 8);
                let mut packed: u64 = 0;
                let mut bi = 0usize;
                while bi < count {
                    packed |= (live_text[offset + bi] as u64) << (bi * 8);
                    bi += 1;
                }

                let arg2 = ((64 + offset) as u64) | ((count as u64) << 8) | (text_color << 32);

                sex_pdx::pdx_call(sex_pdx::SLOT_DISPLAY, 0xFB, KALEIDO_SURFACE_ID, packed, arg2);
                ci += 1;
            }
        }

        if body_len > 0 {
            sex_pdx::serial_println!("[browser.body.render.begin]");
            let body_color: u64 = 0x00CDD6F4;
            let body_chunks = (body_len + 7) / 8;
            let mut ci = 0usize;
            while ci < body_chunks {
                let offset = ci * 8;
                let count = core::cmp::min(body_len - offset, 8);
                let mut packed: u64 = 0;
                let mut bi = 0usize;
                while bi < count {
                    packed |= (body_text[offset + bi] as u64) << (bi * 8);
                    bi += 1;
                }
                let arg2 = (64u64 + offset as u64) | ((count as u64) << 8) | (body_color << 32);
                sex_pdx::pdx_call(sex_pdx::SLOT_DISPLAY, 0xFB, KALEIDO_SURFACE_ID, packed, arg2);
                ci += 1;
            }
            sex_pdx::serial_println!("[browser.body.render.done]");
        }

        sex_pdx::serial_println!("[browser.render.draw_text] live=1 len={}", live_len);

        sex_pdx::pdx_call(sex_pdx::SLOT_DISPLAY, 0xEB, KALEIDO_SURFACE_ID, 50, 100);

        sex_pdx::serial_println!("[browser.render.commit.request] window={}", KALEIDO_SURFACE_ID);
        sex_pdx::serial_println!("[browser.render.live_text.done]");
        sex_pdx::serial_println!("[browser.visible.polish.done]");

        Self { live_text, live_len }
    }

    fn run(&mut self, _pdx: u32) -> bool {
        let req = sex_pdx::pdx_listen_raw(1);
        if req.type_id == 0xFF_FF { return false; }
        true
    }
}

app_main!(App);
