#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        sex_pdx::OP_SILKBAR_UPDATE,
        4,   // SetClock
        10,  // hour
        44,  // minute
    );

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop();
    }
}
