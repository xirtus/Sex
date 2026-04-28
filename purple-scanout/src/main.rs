#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::write_volatile;
use sex_pdx::{pdx_call_checked, SLOT_DISPLAY, serial_println};

const OP_WINDOW_CREATE: u64 = 0xDE;
const OP_WINDOW_SUBMIT: u64 = 0xDD; // submit/commit path

#[repr(C)]
struct SexWindowCreateParams {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pfn_base: u64,
}

#[repr(align(4096))]
struct Page([u32; 1024]); // one 4KiB page

static mut FB_PAGE: Page = Page([0; 1024]);
static mut PFN_LIST: [u64; 1] = [0];

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}", info);
    loop { core::hint::spin_loop(); }
}

#[inline(always)]
fn debug_syscall_probe() {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0u64,
            in("rdi") 0u64,
            in("rsi") 0x5151u64,
            in("rdx") 0u64,
            in("r10") 0u64,
            in("r8")  0u64,
            lateout("rax") _,
            lateout("rsi") _,
            out("rcx") _,
            out("r11") _,
        );
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_syscall_probe();
    serial_println!("[purple] single-pixel scanout test start");

    let fb_ptr = unsafe { FB_PAGE.0.as_mut_ptr() as u64 };
    let params = SexWindowCreateParams {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        pfn_base: fb_ptr,
    };

    let window_id = match pdx_call_checked(SLOT_DISPLAY, OP_WINDOW_CREATE, &params as *const _ as u64, 0, 0) {
        Ok(id) => id,
        Err(code) => {
            serial_println!("[purple] create failed status={:#x}", code);
            loop { sex_pdx::sys_yield(); }
        }
    };

    // Exactly one pixel at (0,0) = magenta.
    unsafe {
        write_volatile(FB_PAGE.0.as_mut_ptr(), 0x00FF00FF);
        PFN_LIST[0] = fb_ptr;
    }

    if let Err(code) = pdx_call_checked(SLOT_DISPLAY, OP_WINDOW_SUBMIT, window_id, unsafe { PFN_LIST.as_ptr() as u64 }, 1) {
        serial_println!("[purple] submit failed status={:#x}", code);
        loop { sex_pdx::sys_yield(); }
    }

    serial_println!("[purple] submitted single magenta pixel");
    loop {
        sex_pdx::sys_yield();
    }
}
