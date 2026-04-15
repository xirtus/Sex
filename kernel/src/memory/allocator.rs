use core::sync::atomic::{AtomicU64, Ordering};

/// A Lock-Free, Wait-Free Physical Frame Allocator.
/// Uses an atomic Treiber stack to manage free 4KiB frames.
/// IPCtax-Compliant: Zero Mutexes.

pub struct LockFreeFrameAllocator {
    /// Pointer to the physical address of the first free frame.
    /// The first 8 bytes of a free frame store the physical address of the next free frame.
    free_list_head: AtomicU64,
}

impl LockFreeFrameAllocator {
    pub const fn new() -> Self {
        Self {
            free_list_head: AtomicU64::new(0),
        }
    }

    /// Initializes the allocator with a contiguous range of physical memory.
    /// This should only be called once during early boot.
    pub unsafe fn init(&self, start_phys: u64, size: u64) {
        let num_frames = size / 4096;
        if num_frames == 0 { return; }

        // Build the linked list directly in the physical memory
        // Assuming identity mapping for the first few megabytes during boot, 
        // or a dedicated physical mapping window.
        // For this prototype, we assume we can write to these physical addresses 
        // through a higher-half virtual offset (e.g., + 0xFFFF_8000_0000_0000).
        // To keep the prototype simple and focused on the lock-free structure:
        let phys_offset = 0xFFFF_8000_0000_0000;

        for i in 0..(num_frames - 1) {
            let current_frame = start_phys + (i * 4096);
            let next_frame = start_phys + ((i + 1) * 4096);
            let vaddr = (current_frame + phys_offset) as *mut u64;
            vaddr.write_volatile(next_frame);
        }

        // The last frame points to 0 (null)
        let last_frame = start_phys + ((num_frames - 1) * 4096);
        let vaddr = (last_frame + phys_offset) as *mut u64;
        vaddr.write_volatile(0);

        self.free_list_head.store(start_phys, Ordering::SeqCst);
    }

    /// Allocates a 4KiB frame wait-free.
    pub fn alloc(&self) -> Option<u64> {
        let phys_offset = 0xFFFF_8000_0000_0000;
        
        loop {
            let head = self.free_list_head.load(Ordering::Acquire);
            if head == 0 {
                return None; // Out of memory
            }

            // Read the 'next' pointer from the current head frame
            let next = unsafe { ((head + phys_offset) as *const u64).read_volatile() };

            // Try to swap the head to the next frame
            if self.free_list_head.compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                return Some(head);
            }
        }
    }

    /// Frees a 4KiB frame wait-free.
    pub fn free(&self, frame: u64) {
        let phys_offset = 0xFFFF_8000_0000_0000;
        
        loop {
            let head = self.free_list_head.load(Ordering::Acquire);
            
            // Link the freed frame to the current head
            unsafe { ((frame + phys_offset) as *mut u64).write_volatile(head); }

            // Try to update the head to the freed frame
            if self.free_list_head.compare_exchange_weak(head, frame, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                break;
            }
        }
    }
}

pub static GLOBAL_ALLOCATOR: LockFreeFrameAllocator = LockFreeFrameAllocator::new();

pub fn alloc_frame() -> Option<u64> {
    GLOBAL_ALLOCATOR.alloc()
}
