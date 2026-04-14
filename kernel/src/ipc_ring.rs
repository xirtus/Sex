use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;

/// A lockless, SPSC (Single Producer, Single Consumer) Ring Buffer.
/// Designed for "Silicon Physics" speed with cache-line alignment.
#[repr(C, align(64))]
pub struct RingBuffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    
    // Aligned to separate cache lines to prevent false sharing.
    #[repr(align(64))]
    write_idx: AtomicUsize,
    
    #[repr(align(64))]
    read_idx: AtomicUsize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        const UNINIT: MaybeUninit<u64> = MaybeUninit::uninit();
        // This is a workaround since MaybeUninit<T> isn't always const copy.
        // In a real system, we'd use a more robust const initialization.
        unsafe {
            Self {
                buffer: core::mem::transmute([UNINIT; N]),
                write_idx: AtomicUsize::new(0),
                read_idx: AtomicUsize::new(0),
            }
        }
    }

    /// Enqueues an item (Producer side).
    pub fn enqueue(&self, item: T) -> Result<(), &'static str> {
        let w = self.write_idx.load(Ordering::Relaxed);
        let r = self.read_idx.load(Ordering::Acquire);
        
        if w - r == N {
            return Err("Ring: Buffer is full");
        }
        
        unsafe {
            let ptr = self.buffer[w % N].as_ptr() as *mut T;
            ptr.write(item);
        }
        
        self.write_idx.store(w + 1, Ordering::Release);
        Ok(())
    }

    /// Dequeues an item (Consumer side).
    pub fn dequeue(&self) -> Option<T> {
        let r = self.read_idx.load(Ordering::Relaxed);
        let w = self.write_idx.load(Ordering::Acquire);
        
        if r == w {
            return None;
        }
        
        let item = unsafe {
            self.buffer[r % N].as_ptr().read()
        };
        
        self.read_idx.store(r + 1, Ordering::Release);
        Some(item)
    }
}
