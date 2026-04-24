#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen, SLOT_DISPLAY, OP_WINDOW_CREATE, OP_WINDOW_PAINT};

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
    for _ in 0..2_000_000 {
        core::hint::spin_loop();
    }

    // PHASE 32: WARDEN STRESS TEST
    // Violation: Read from PKEY 1 (sexdisplay) from current domain (linen, PKEY 3)
    unsafe {
        let sexdisplay_base = 0x2020_0000 as *const u64;
        let _val = core::ptr::read_volatile(sexdisplay_base);
    }

    unsafe {
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, 0, 0, 0);

        loop {
            let msg = pdx_listen();
            if msg.type_id != 0 { break; }
            core::hint::spin_loop();
        }

        pdx_call(SLOT_DISPLAY, OP_WINDOW_PAINT, 0, 0, 0);
    }

    loop { core::hint::spin_loop(); }
}