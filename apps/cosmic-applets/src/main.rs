#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use sex_pdx::{pdx_call, PDX_SEX_WINDOW_CREATE, SexWindowCreateParams, DummyAllocator};
use sex_pdx::serial_println;

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! { loop {} }

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let params = SexWindowCreateParams { x: 0, y: 0, width: 1280, height: 720, pfn_base: 0 };
    serial_println!("cosmic-applets: spawning window");
    let _ = unsafe { pdx_call(5, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0, 0) };
    loop {}
}
