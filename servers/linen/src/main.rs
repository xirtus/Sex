#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, sys_yield, serial_println, SLOT_DISPLAY};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

const SURFACE_ID_LINEN: u64 = 200;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Brief delay to ensure sexdisplay is ready to receive
    for _ in 0..5_000_000 { core::hint::spin_loop(); }

    // Create placeholder surface on sexdisplay (0xEC upsert by id)
    // arg1 = (y<<32)|x, arg2 = (h<<32)|w
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_LINEN,
        (500u64 << 32) | 900u64,  // x=900, y=500
        (150u64 << 32) | 300u64); // w=300, h=150
    serial_println!("[linen] Placeholder surface 200 created via 0xEC");

    loop {
        sys_yield();
    }
}
