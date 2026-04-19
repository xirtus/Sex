#![no_std]
#![no_main]

use sex_kernel::init;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Jump into the library initialization
    init::kernel_init();

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
