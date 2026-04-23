// kernel/src/memory/va_allocator.rs
use core::sync::atomic::{AtomicU64, Ordering};

static NEXT_VA: AtomicU64 = AtomicU64::new(0x_4000_0000_0000); // SASOS Shared User Region

const PAGE_SIZE: u64 = 4096;

pub fn allocate_va(size: usize) -> Option<u64> {
    let size = ((size as u64 + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE; // page-align up

    let mut current = NEXT_VA.load(Ordering::Relaxed);
    loop {
        let next = current + size;
        match NEXT_VA.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => return Some(current),
            Err(v) => current = v,
        }
    }
}

// Optional: debug helper (remove in final daily-driver)
pub fn current_va_cursor() -> u64 {
    NEXT_VA.load(Ordering::Relaxed)
}
