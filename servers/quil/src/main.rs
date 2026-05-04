#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{sys_yield, serial_println};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[quil.boot]");
    serial_println!("[quil.no_fb_write]");
    loop {
        sys_yield();
    }
}
