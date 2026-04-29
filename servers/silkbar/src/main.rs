#![no_std]
#![no_main]

use silkbar_model::{SilkBarUpdate, UpdateKind, OP_SILKBAR_UPDATE};

fn send_update(update: SilkBarUpdate) {
    sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut hh: u8 = 10;
    let mut mm: u8 = 44;
    let mut ss: u8 = 0;

    // INIT: show initial time
    send_update(SilkBarUpdate::new(
        UpdateKind::SetClock as u32, 0, hh as u32, ((mm as u32) << 8) | ss as u32,
    ));

    loop {
        // ~1s via yield (no rdtsc — freezes under QEMU TCG)
        for _ in 0..100 {
            sex_pdx::sys_yield();
        }

        // Advance clock
        ss += 1;
        if ss >= 60 {
            ss = 0;
            mm += 1;
            if mm >= 60 {
                mm = 0;
                hh = if hh >= 23 { 0 } else { hh + 1 };
            }
        }

        send_update(SilkBarUpdate::new(
            UpdateKind::SetClock as u32, 0, hh as u32, ((mm as u32) << 8) | ss as u32,
        ));
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
