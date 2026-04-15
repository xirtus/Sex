use crate::memory::allocator::GLOBAL_ALLOCATOR;
use crate::ipc::pagefault::forward_page_fault;

#[test]
fn test_memory_and_pagefault() {
    // 1. Test Buddy Allocator
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    let frame = allocator.alloc(0);
    assert!(frame.is_some(), "Buddy allocation failed");
    
    // 2. Test Page Fault Forwarding (Synthetic)
    let res = forward_page_fault(0x_dead_beef_000, 0, 3000);
    assert!(res.is_ok(), "Page fault forwarding failed");
}
