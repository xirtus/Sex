pub mod allocator;

pub fn init() {
    // Correcting E0061: Remove the argument as init_heap is now zero-arg
    allocator::init_heap();
}
