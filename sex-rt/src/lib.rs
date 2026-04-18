#![no_std]

// Fake sex_pdx crate for illustration (will be replaced by real sex-pdx crate soon)
pub mod sex_pdx {
    pub struct PdxMessage {
        #[allow(dead_code)]  // temporary placeholder for future PDX message tagging
        id: usize,
    }
    impl PdxMessage {
        pub fn new(id: usize) -> Self { Self { id } }
        pub fn append_arg(&mut self, _arg: usize) {}
        pub fn append_slice(&mut self, _buf: &[u8]) {}
        pub fn append_mut_slice(&mut self, _buf: &mut [u8]) {}
    }
    pub struct PdxChannel;
    impl PdxChannel {
        pub fn open(_name: &str) -> Option<Self> { Some(Self) }
        pub fn send_sync(&self, _msg: &PdxMessage) -> usize { 0 }
    }
}

use sex_pdx::{PdxChannel, PdxMessage};

// ─────────────────────────────────────────────────────────────
// Safe, lock-free bump allocator (Rust 2024 compliant)
// No static_mut_refs, no UB, compare_exchange_weak loop
// ─────────────────────────────────────────────────────────────
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_START: usize = 0x4000_0000;  // matches your prototype range
const HEAP_END:   usize = 0x4000_0000 + (256 * 1024 * 1024);  // 256 MiB arena

static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);

#[inline(always)]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator;

pub struct SimpleAllocator;

unsafe impl core::alloc::GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        if size == 0 {
            return core::ptr::null_mut();
        }

        let mut current = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let aligned = align_up(current, align);
            let next = aligned.wrapping_add(size);

            if next < aligned || next > HEAP_END {
                return core::ptr::null_mut(); // OOM — sexgemini will catch later
            }

            match HEAP_TOP.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(actual) => current = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Leak for prototype (same as before — we will add proper reclaim later)
    }
}

// ─────────────────────────────────────────────────────────────
// Rest of the Sex runtime (unchanged)
// ─────────────────────────────────────────────────────────────
#[inline]
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    // PKU zero-copy buffer map to sexfiles
    let mut msg = PdxMessage::new(1 /* WRITE */);
    msg.append_arg(fd);
    msg.append_slice(buf);

    let channel = PdxChannel::open("sexfiles").unwrap();
    channel.send_sync(&msg) as isize
}

#[inline]
pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    let mut msg = PdxMessage::new(0 /* READ */);
    msg.append_arg(fd);
    msg.append_mut_slice(buf);

    let channel = PdxChannel::open("sexfiles").unwrap();
    channel.send_sync(&msg) as isize
}

pub fn sys_exit(status: usize) -> ! {
    let mut msg = PdxMessage::new(60 /* EXIT */);
    msg.append_arg(status);
    let channel = PdxChannel::open("sexproc").unwrap();
    channel.send_sync(&msg);
    loop {}
}

pub fn heap_init() {
    // No-op — AtomicUsize is already initialized at static creation time
}
