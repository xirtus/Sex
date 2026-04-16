use core::alloc::Layout;
use core::ptr::NonNull;
use spin::Mutex;
use alloc::vec::Vec;

/// A simple Slab Allocator for fixed-size kernel objects.
/// Improves performance and reduces fragmentation for common structures.

pub struct Slab {
    pub object_size: usize,
    pub free_list: Vec<*mut u8>,
}

unsafe impl Send for Slab {}
unsafe impl Sync for Slab {}

impl Slab {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            free_list: Vec::new(),
        }
    }

    /// Refills the slab by allocating a new 4KiB page.
    pub fn refill(&mut self, allocator: &mut impl x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>) {
        use x86_64::structures::paging::FrameAllocator;
        if let Some(frame) = allocator.allocate_frame() {
            // In a SASOS, we can assume identity mapping or use the phys_offset
            let page_ptr = (frame.start_address().as_u64() + 0x_0000_0000_0000) as *mut u8; // Placeholder offset
            
            for i in 0..(4096 / self.object_size) {
                unsafe {
                    self.free_list.push(page_ptr.add(i * self.object_size));
                }
            }
        }
    }

    pub fn alloc(&mut self) -> Option<*mut u8> {
        self.free_list.pop()
    }

    pub fn free(&mut self, ptr: *mut u8) {
        self.free_list.push(ptr);
    }
}

pub struct SlabAllocator {
    pub slab_64: Mutex<Slab>,
    pub slab_128: Mutex<Slab>,
    pub slab_512: Mutex<Slab>,
}

impl SlabAllocator {
    pub fn new() -> Self {
        Self {
            slab_64: Mutex::new(Slab::new(64)),
            slab_128: Mutex::new(Slab::new(128)),
            slab_512: Mutex::new(Slab::new(512)),
        }
    }

    pub fn allocate(&self, layout: Layout) -> Option<*mut u8> {
        if layout.size() <= 64 && layout.align() <= 64 {
            self.slab_64.lock().alloc()
        } else if layout.size() <= 128 && layout.align() <= 128 {
            self.slab_128.lock().alloc()
        } else if layout.size() <= 512 && layout.align() <= 512 {
            self.slab_512.lock().alloc()
        } else {
            None // Fallback to standard heap
        }
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_SLAB: SlabAllocator = SlabAllocator::new();
}
