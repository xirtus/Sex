use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;

#[repr(align(64))]
struct CacheAligned<T>(T);

/// A lockless MPSC (Multi Producer, Single Consumer) Ring Buffer.
///
/// BUG 2026-07-18: this was SPSC, but every client PD enqueues into a
/// server's single `message_ring` — multiple producers. Two producers could
/// load the same `write_idx`, write the same slot, and both return Ok: one
/// message silently lost (observed as storage requests vanishing while
/// sexfiles served another client; the caller then hung awaiting a reply).
/// A timer preemption between the index load and the store is enough — no
/// SMP needed.
///
/// Producers now claim a slot with CAS on `write_idx`, then publish it by
/// storing `pos + 1` into the slot's `seq`. The consumer only reads a slot
/// once its seq matches `read_idx + 1`, so a claimed-but-unwritten slot
/// reads as empty (bounded head-of-line wait) instead of garbage.
pub struct RingBuffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    /// Per-slot publish sequence: `pos + 1` once the item at absolute
    /// position `pos` is fully written. Absolute counters never wrap in
    /// practice (usize, 64-bit).
    seq: [AtomicUsize; N],

    // Aligned to separate cache lines to prevent false sharing.
    write_idx: CacheAligned<AtomicUsize>,
    read_idx: CacheAligned<AtomicUsize>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::uninit() }; N],
            seq: [const { AtomicUsize::new(0) }; N],
            write_idx: CacheAligned(AtomicUsize::new(0)),
            read_idx: CacheAligned(AtomicUsize::new(0)),
        }
    }

    /// Enqueues an item (multi-producer safe).
    pub fn enqueue(&self, item: T) -> Result<(), &'static str> {
        loop {
            let w = self.write_idx.0.load(Ordering::Acquire);
            let r = self.read_idx.0.load(Ordering::Acquire);

            if w.wrapping_sub(r) >= N {
                return Err("Ring: Buffer is full");
            }

            // Claim position w. On success no other producer can hold it:
            // r is monotonic, so the fullness check above still holds and
            // slot w % N has been consumed (its last use was pos w - N).
            if self
                .write_idx
                .0
                .compare_exchange_weak(w, w + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                unsafe {
                    let ptr = self.buffer[w % N].as_ptr() as *mut T;
                    ptr.write(item);
                }
                // Publish: consumer may read this slot only from here on.
                self.seq[w % N].store(w + 1, Ordering::Release);
                return Ok(());
            }
            // Lost the claim race — retry with fresh indices.
        }
    }

    /// Dequeues an item (single consumer).
    pub fn dequeue(&self) -> Option<T> {
        let r = self.read_idx.0.load(Ordering::Relaxed);
        let w = self.write_idx.0.load(Ordering::Acquire);

        if r == w {
            return None;
        }

        // Slot claimed but not yet published by its producer: treat as
        // empty rather than reading a partially written message.
        if self.seq[r % N].load(Ordering::Acquire) != r + 1 {
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
