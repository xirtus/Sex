use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;

#[repr(align(64))]
struct CacheAligned<T>(T);

/// A lockless, SPSC (Single Producer, Single Consumer) Ring Buffer.
/// Designed for "Silicon Physics" speed with cache-line alignment.
pub struct RingBuffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    
    // Aligned to separate cache lines to prevent false sharing.
    write_idx: CacheAligned<AtomicUsize>,
    read_idx: CacheAligned<AtomicUsize>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::uninit() }; N],
            write_idx: CacheAligned(AtomicUsize::new(0)),
            read_idx: CacheAligned(AtomicUsize::new(0)),
        }
    }

    /// Enqueues an item (Producer side).
    pub fn enqueue(&self, item: T) -> Result<(), &'static str> {
        let w = self.write_idx.0.load(Ordering::Relaxed);
        let r = self.read_idx.0.load(Ordering::Acquire);
        
        if w - r == N {
            return Err("Ring: Buffer is full");
        }
        
        unsafe {
            let ptr = self.buffer[w % N].as_ptr() as *mut T;
            ptr.write(item);
        }
        
        self.write_idx.0.store(w + 1, Ordering::Release);
        Ok(())
    }

    /// Dequeues an item (Consumer side).
    pub fn dequeue(&self) -> Option<T> {
        let r = self.read_idx.0.load(Ordering::Relaxed);
        let w = self.write_idx.0.load(Ordering::Acquire);
        
        if r == w {
            return None;
        }
        
        let item = unsafe {
            self.buffer[r % N].as_ptr().read()
        };
        
        self.read_idx.0.store(r + 1, Ordering::Release);
        Some(item)
    }

    /// Returns the number of queued items.
    pub fn len(&self) -> usize {
        let w = self.write_idx.0.load(Ordering::Acquire);
        let r = self.read_idx.0.load(Ordering::Acquire);
        w.saturating_sub(r)
    }
}

pub type SpscRing<T> = RingBuffer<T, 256>;
