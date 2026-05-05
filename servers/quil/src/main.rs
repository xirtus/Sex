#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_listen_raw, serial_println};

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
        let msg = pdx_listen_raw(0);

        unsafe {
            static mut QUIL_LISTEN_BUDGET: u32 = 8;
            let b = &mut QUIL_LISTEN_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[quil.pdx.listen] type_id={:#x}", msg.type_id);
            }
        }

        match msg.type_id {
            _ => {
                unsafe {
                    static mut QUIL_UNKNOWN_BUDGET: u32 = 8;
                    let b = &mut QUIL_UNKNOWN_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.unknown.yield] type_id={:#x}", msg.type_id);
                    }
                }
            }
        }
    }
}
