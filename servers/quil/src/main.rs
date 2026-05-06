#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_listen_raw, pdx_call, serial_println, OP_QUIL_PING, SLOT_DISPLAY};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

const OP_HID_EVENT: u64 = 0x202;
const SURFACE_ID_QUIL: u64 = 201;

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
            OP_QUIL_PING => {
                unsafe {
                    static mut QUIL_ROUTE_BUDGET: u32 = 8;
                    let b = &mut QUIL_ROUTE_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.route.recv]");
                    }
                }
            }
            OP_HID_EVENT => {
                let scancode = msg.arg0;
                let value = msg.arg1;
                unsafe {
                    static mut QUIL_KEY_BUDGET: u32 = 16;
                    let b = &mut QUIL_KEY_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.key.recv] scancode={:#x} val={}", scancode, value);
                    }

                    // Track C3: visible proof
                    static mut QUIL_COLOR_TOGGLE: bool = false;
                    if value == 1 { // On key press
                        QUIL_COLOR_TOGGLE = !QUIL_COLOR_TOGGLE;
                        let color = if QUIL_COLOR_TOGGLE { 0x00FF00FF } else { 0x0000FFFF }; // Magenta/Cyan toggle
                        pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0, (color << 32) | (2000u64 << 16) | 2000u64); // Oversized to fill
                        
                        static mut QUIL_VISUAL_BUDGET: u32 = 16;
                        let vb = &mut QUIL_VISUAL_BUDGET;
                        if *vb > 0 {
                            *vb -= 1;
                            serial_println!("[silk-shell.focus.visual_update] color={:#x}", color);
                        }
                    }
                }
            }
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
