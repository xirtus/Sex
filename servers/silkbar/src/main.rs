#![no_std]
#![no_main]

use silkbar_model::{SilkBarUpdate, UpdateKind, OP_SILKBAR_UPDATE};

fn send_update(update: &SilkBarUpdate) {
    sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
}

fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    (lo as u64) | ((hi as u64) << 32)
}

fn wait_approx_1s() {
    let start = rdtsc();
    // ~1.5 GHz × 1 sec = 1.5e9 reference cycles
    let target = 1_500_000_000u64;
    loop {
        if rdtsc().wrapping_sub(start) >= target {
            break;
        }
        sex_pdx::sys_yield();
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut hh: u8 = 10;
    let mut mm: u8 = 44;
    let mut ss: u8 = 0;
    let mut tick: u64 = 0;
    let mut chip_idx: u8 = 0;
    let mut ws_idx: u8 = 0;

    // Initial state
    send_update(&SilkBarUpdate::new(
        UpdateKind::SetClock as u32, 0, hh as u32, mm as u32,
    ));
    send_update(&SilkBarUpdate::new(
        UpdateKind::SetWorkspaceActive as u32, 0, 1, 0,
    ));

    loop {
        // Approximate 1-second delay via rdtsc + yield.
        // Actual wall time varies with CPU frequency (~0.5–2 s typical).
        wait_approx_1s();
        tick += 1;

        // Advance seconds; roll to minute/hour on overflow
        ss += 1;
        if ss >= 60 {
            ss = 0;
            mm += 1;
            if mm >= 60 {
                mm = 0;
                hh = if hh >= 23 { 0 } else { hh + 1 };
            }
        }
        // Send clock every tick (HH:MM only — seconds are local to this loop)
        send_update(&SilkBarUpdate::new(
            UpdateKind::SetClock as u32, 0, hh as u32, mm as u32,
        ));

        // Every 5 ticks: toggle one chip's visibility, rotate index
        if tick % 5 == 0 {
            let vis = if (tick / 5) % 2 == 0 { 1 } else { 0 };
            send_update(&SilkBarUpdate::new(
                UpdateKind::SetChipVisible as u32, chip_idx, vis, 0,
            ));
            chip_idx = (chip_idx + 1) % 4;
        }

        // Every 8 ticks: advance active workspace
        if tick % 8 == 0 {
            send_update(&SilkBarUpdate::new(
                UpdateKind::SetWorkspaceActive as u32, ws_idx, 0, 0,
            ));
            ws_idx = (ws_idx + 1) % 5;
            send_update(&SilkBarUpdate::new(
                UpdateKind::SetWorkspaceActive as u32, ws_idx, 1, 0,
            ));
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
