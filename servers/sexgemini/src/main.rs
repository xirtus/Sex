#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In Phase 19, this will be replaced by a PDX call to the kernel 
    // to map the shared heap region.
    loop {}
}

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

fn main() {}

use core::alloc::{GlobalAlloc, Layout};
struct SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc;
unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {}
}


#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
