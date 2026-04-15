#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Real lock-free park using syscall (simulated PDX)
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_YIELD/PARK */) };
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
