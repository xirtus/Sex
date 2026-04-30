#![no_std]
#![no_main]

use silkbar_model::{SilkBarUpdate, UpdateKind, OP_SILKBAR_UPDATE};

fn send_update(update: SilkBarUpdate) {
    let _ = sex_pdx::pdx_call(
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

    // INIT: full GLOBAL_BAR state — workspace activation, chip visibility, clock
    // Workspace 3 active (index 2), others inactive
    for ws_idx in 0..5 {
        send_update(SilkBarUpdate::new(
            UpdateKind::SetWorkspaceActive as u32, ws_idx, if ws_idx == 2 { 1 } else { 0 }, 0,
        ));
    }
    // All four status chips visible
    for chip_idx in 0..4 {
        send_update(SilkBarUpdate::new(
            UpdateKind::SetChipVisible as u32, chip_idx, 1, 0,
        ));
    }
    // Initial clock
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
