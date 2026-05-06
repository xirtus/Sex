#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen_raw, serial_println, SLOT_DISPLAY};

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
const OP_HID_EVENT: u64 = 0x202;

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

    // Fill rect: local (20, 20, 80, 60), coral color
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        (20u64 << 32) | 20u64,
        (0x00FF6464u64 << 32) | (60u64 << 16) | 80u64);
    serial_println!("[linen] Fill rect 0xEF sent to sexdisplay");

    loop {
        let msg = pdx_listen_raw(0);
        
        match msg.type_id {
            OP_HID_EVENT => {
                let scancode = msg.arg0;
                let value = msg.arg1;
                unsafe {
                    static mut LINEN_KEY_BUDGET: u32 = 16;
                    let b = &mut LINEN_KEY_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[linen.key.recv] scancode={:#x} val={}", scancode, value);
                    }

                    // Track C3: visible proof
                    static mut LINEN_COLOR_TOGGLE: bool = false;
                    if value == 1 { // On key press
                        LINEN_COLOR_TOGGLE = !LINEN_COLOR_TOGGLE;
                        let color = if LINEN_COLOR_TOGGLE { 0x0000FF00 } else { 0x00FF6464 }; // Green/Coral toggle
                        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN, 
                            (20u64 << 32) | 20u64, 
                            (color << 32) | (60u64 << 16) | 80u64);
                        
                        static mut LINEN_VISUAL_BUDGET: u32 = 16;
                        let vb = &mut LINEN_VISUAL_BUDGET;
                        if *vb > 0 {
                            *vb -= 1;
                            serial_println!("[linen.focus.visual_update] color={:#x}", color);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
