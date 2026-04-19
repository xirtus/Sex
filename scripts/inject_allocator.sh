#!/bin/bash
set -euo pipefail

# Define the Allocator Block
# Note: Using a static 1MB heap for each server. 
# In production SASOS, this will be replaced by a libsys mmap call.
ALLOCATOR_BLOCK="
use core::alloc::{GlobalAlloc, Layout};
struct SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc;
unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    loop {}
}
"

for file in servers/tuxedo/src/lib.rs servers/sexdisplay/src/main.rs servers/sexgemini/src/main.rs; do
    echo "Injecting Allocator into $file"
    echo "$ALLOCATOR_BLOCK" >> "$file"
done
