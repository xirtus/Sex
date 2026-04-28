#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, SLOT_SHELL};

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

// Local Opcode
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Wait for display/shell to be ready
    for _ in 0..10_000_000 {
        core::hint::spin_loop();
    }

    // Allocate shared buffer (1024x768x4 = 3MB)
    let fb_size = 1024 * 768 * 4;
    let shared_addr: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 40u64,
            in("rdi") fb_size as u64,
            in("rsi") 1u64, // Consumer: SexDisplay (Domain 1)
            lateout("rax") shared_addr,
        );
    }

    if shared_addr == 0 {
        loop { core::hint::spin_loop(); }
    }

    // Zero-copy handoff: Notify AUTHORITY (Silk-Shell) about shared buffer
    unsafe {
        pdx_call(SLOT_SHELL, OP_SHELL_BIND_BUFFER, shared_addr, 0, 0);
    }

    let mut frame: u32 = 0;
    loop {
        frame += 1;
        let ptr = shared_addr as *mut u32;
        for y in 0..768 {
            for x in 0..1024 {
                let color = (x as u32 ^ y as u32).wrapping_add(frame);
                unsafe {
                    *ptr.add(y * 1024 + x) = color;
                }
            }
        }

        // Throttle
        for _ in 0..2_000_000 {
            core::hint::spin_loop();
        }
    }
}
