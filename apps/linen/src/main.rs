#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen, OP_WINDOW_CREATE};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

const PDX_WINDOW_COMMIT_FRAME: u64 = 0xDD;
const COMPOSITOR_SLOT: u32 = 5;

// x86_64-sex target does not wire compiler_builtins-mem; provide directly.
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let diff = *a.add(i) as i32 - *b.add(i) as i32;
        if diff != 0 { return diff; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n { *dest.add(i) = *src.add(i); }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n { *dest.add(i) = c as u8; }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest as usize <= src as usize {
        for i in 0..n { *dest.add(i) = *src.add(i); }
    } else {
        for i in (0..n).rev() { *dest.add(i) = *src.add(i); }
    }
    dest
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Wait for compositor to initialize before sending IPC.
    for _ in 0..2_000_000 {
        core::hint::spin_loop();
    }

    unsafe {
        // Request shared canvas from compositor.
        // Domain IPC is asynchronous; the return value of pdx_call is just a send-status.
        pdx_call(COMPOSITOR_SLOT, OP_WINDOW_CREATE, 0, 0, 0);

        let mut canvas_addr = 0;
        loop {
            let ev = pdx_listen(0);
            if ev.num == 0xF { // IPC Reply status
                canvas_addr = ev.arg0;
                break;
            }
            core::hint::spin_loop();
        }

        if canvas_addr != 0 {
            // Fill the top 1280x32 bar of canvas with WHITE — linen alive signal.
            let canvas = canvas_addr as *mut u32;
            for y in 0..32usize {
                for x in 0..1280usize {
                    core::ptr::write_volatile(canvas.add(y * 1280 + x), 0xFFFFFFFF);
                }
            }
            // Commit: blit silkbar region to physical framebuffer.
            pdx_call(COMPOSITOR_SLOT, PDX_WINDOW_COMMIT_FRAME, 0, 0, 0);
        }
    }

    loop { core::hint::spin_loop(); }
}
