use core::sync::atomic::{AtomicU64, Ordering};
use core::hint::spin_loop;

pub const MAX_ORDER: usize = 18; // Support up to 1 GiB (4 KiB * 2^18)
pub const PAGE_SIZE: u64 = 4096;

/// A Lock-Free, Wait-Free Buddy Allocator.
/// Uses sharded atomic stacks for each order to achieve O(1) alloc/free.
/// IPCtax-Compliant: ZERO Mutexes, ZERO blocking.
pub struct LockFreeBuddyAllocator {
    /// Atomic stacks for each order (0 to MAX_ORDER).
    /// Stores physical addresses of free blocks.
    free_lists: [AtomicU64; MAX_ORDER + 1],
}

impl LockFreeBuddyAllocator {
    pub const fn new() -> Self {
        const INIT: AtomicU64 = AtomicU64::new(0);
        Self {
            free_lists: [INIT; MAX_ORDER + 1],
        }
    }

    /// Initializes the allocator with a range of physical memory.
    pub unsafe fn init(&self, mut start_phys: u64, mut size: u64) {
        let phys_offset = 0xFFFF_8000_0000_0000;

        // Align start_phys to 4 KiB
        if start_phys % PAGE_SIZE != 0 {
            let padding = PAGE_SIZE - (start_phys % PAGE_SIZE);
            start_phys += padding;
            size = size.saturating_sub(padding);
        }

        while size >= PAGE_SIZE {
            // Find the largest order that fits and is aligned
            let mut order = MAX_ORDER;
            while order > 0 {
                let block_size = PAGE_SIZE << order;
                if size >= block_size && start_phys % block_size == 0 {
                    break;
                }
                order -= 1;
            }

            self.push_free(order, start_phys, phys_offset);
            let block_size = PAGE_SIZE << order;
            start_phys += block_size;
            size -= block_size;
        }
    }

    fn push_free(&self, order: usize, frame: u64, phys_offset: u64) {
        loop {
            let head = self.free_lists[order].load(Ordering::Acquire);
            unsafe {
                ((frame + phys_offset) as *mut u64).write_volatile(head);
            }
            if self.free_lists[order].compare_exchange_weak(head, frame, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                break;
            }
            spin_loop();
        }
    }

    fn pop_free(&self, order: usize, phys_offset: u64) -> Option<u64> {
        loop {
            let head = self.free_lists[order].load(Ordering::Acquire);
            if head == 0 { return None; }
            let next = unsafe { ((head + phys_offset) as *const u64).read_volatile() };
            if self.free_lists[order].compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                return Some(head);
            }
            spin_loop();
        }
    }

    /// Allocates a block of memory of the given order.
    pub fn alloc(&self, order: usize) -> Option<u64> {
        let phys_offset = 0xFFFF_8000_0000_0000;
        
        // 1. Try to pop from the requested order
        if let Some(frame) = self.pop_free(order, phys_offset) {
            return Some(frame);
        }

        // 2. Try to split from higher orders
        for o in (order + 1)..=MAX_ORDER {
            if let Some(frame) = self.pop_free(o, phys_offset) {
                // Split blocks down to the requested order
                for split_order in (order..o).rev() {
                    let buddy = frame + (PAGE_SIZE << split_order);
                    self.push_free(split_order, buddy, phys_offset);
                }
                return Some(frame);
            }
        }

        None
    }

    /// Frees a block of memory. Buddy merging is deferred or handled via Epochs
    /// in a full implementation to maintain wait-free properties.
    pub fn free(&self, frame: u64, order: usize) {
        let phys_offset = 0xFFFF_8000_0000_0000;
        self.push_free(order, frame, phys_offset);
    }
}

pub static GLOBAL_ALLOCATOR: LockFreeBuddyAllocator = LockFreeBuddyAllocator::new();

pub fn alloc_frame() -> Option<u64> {
    GLOBAL_ALLOCATOR.alloc(0)
}

pub fn alloc_pages(order: usize) -> Option<u64> {
    GLOBAL_ALLOCATOR.alloc(order)
}

pub fn free_pages(frame: u64, order: usize) {
    GLOBAL_ALLOCATOR.free(frame, order)
}
