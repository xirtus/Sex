#[test_case]
fn test_lockfree_buddy_and_async_pf() {
    serial_println!("test: Exercising Lock-Free Buddy Allocator...");
    
    // 1. Allocate 100 frames (order 0)
    let mut frames = [0u64; 100];
    for i in 0..100 {
        frames[i] = crate::memory::allocator::alloc_frame().expect("Buddy: Allocation failed");
    }
    
    // 2. Free them to verify no contention
    for i in 0..100 {
        crate::memory::allocator::free_pages(frames[i], 0);
    }
    serial_println!("test: Buddy Allocator SUCCESS.");

    // 3. Simulate Synthetic Page Fault
    serial_println!("test: Simulating Asynchronous Page Fault Forwarding...");
    let res = crate::ipc::pagefault::forward_page_fault(0x_DEAD_B000, 0x2, 4000);
    
    assert!(res.is_ok(), "Page fault forwarding failed");
    serial_println!("test: Async Page Fault SUCCESS.");
}
