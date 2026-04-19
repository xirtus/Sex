use linked_list_allocator::LockedHeap;
use crate::{MEMMAP_REQUEST, HHDM_REQUEST};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap() {
    let mmap = MEMMAP_REQUEST.response().expect("MMAP Fail");
    let hhdm = HHDM_REQUEST.response().expect("HHDM Fail");
    
    // We use a direct match on the entry type. 
    // This is the most resilient way to find the variant in a no_std environment.
    let usable_region = mmap.entries().iter()
        .find(|e| {
            // We inspect the variant through pattern matching 
            // Most Limine bindings use 'Usable' or 'USABLE' as the first variant (usually 0)
            let is_usable = match e.type_ {
                t if unsafe { core::mem::transmute::<_, u64>(t) } == 0 => true,
                _ => false,
            };
            is_usable && e.length >= (HEAP_SIZE as u64)
        })
        .expect("No usable memory region found");

    let virt_addr = usable_region.base + hhdm.offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
}
