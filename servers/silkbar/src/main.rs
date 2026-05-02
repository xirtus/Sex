#![no_std]
#![no_main]

use silkbar_model::{
    SilkBarUpdate, UpdateKind, ChipKind, OP_SILKBAR_UPDATE, validate_contract,
    validate_deterministic_vectors,
};

fn send_update(update: SilkBarUpdate) {
    let result = sex_pdx::pdx_call_checked(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
    if let Err(status) = result {
        // Rate-limited: log roughly every 64th failure
        unsafe {
            static mut DROP_COUNTER: u64 = 0;
            let count = DROP_COUNTER.wrapping_add(1);
            DROP_COUNTER = count;
            if count & 0x3F == 0 {
                sex_pdx::serial_println!(
                    "[silkbar] drop: kind={} idx={} status={} count={}",
                    update.kind, update.index, status, count,
                );
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    if !validate_contract() || !validate_deterministic_vectors() {
        loop { core::hint::spin_loop(); }
    }

    let mut focus_state: u8 = 0;
    let mut last_focus_state: u8 = 0xFF;
    let mut chip_phase: u8 = 0;
    let mut chip0_net: bool = true;

    /// Approximate LAPIC timer ticks per second (divide=16, init_count=1_000_000).
    /// Not calibrated — yields monotonic uptime, not wall-clock accuracy.
    const LAPIC_TICKS_PER_SECOND_APPROX: u64 = 62;

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
    // Initial clock — derived from kernel uptime
    {
        let ticks = sex_pdx::get_ticks();
        let uptime_seconds = ticks / LAPIC_TICKS_PER_SECOND_APPROX;
        let hh0 = ((uptime_seconds / 3600) % 24) as u8;
        let mm0 = ((uptime_seconds / 60) % 60) as u8;
        let ss0 = (uptime_seconds % 60) as u8;
        send_update(SilkBarUpdate::new(
            UpdateKind::SetClock as u32, 0, hh0 as u32, ((mm0 as u32) << 8) | ss0 as u32,
        ));
    }

    loop {
        // Stage 2B: poll at most one upstream message (non-blocking)
        if let Some(msg) = sex_pdx::pdx_try_listen_raw(0) {
            if msg.type_id == sex_pdx::OP_SILKBAR_WORKSPACE_ACTIVE {
                let ws = (msg.arg0 as u8).min(4);
                for i in 0..5 {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetWorkspaceActive as u32, i, if i == ws { 1 } else { 0 }, 0,
                    ));
                }
            } else if msg.type_id == sex_pdx::OP_SILKBAR_FOCUS_STATE {
                // Clamp invalid producer values to debug(3) to keep update space bounded.
                focus_state = (msg.arg0 as u8).min(3);
            }
        }

        if focus_state != last_focus_state {
            // Temporary Stage 2C focus->urgent visual stub.
            // none: clear all; shell/app/debug => ws0/ws1/ws2 urgent respectively.
            let urgent_ws = match focus_state {
                1 => Some(0u8),
                2 => Some(1u8),
                3 => Some(2u8),
                _ => None,
            };
            for ws in 0..5u8 {
                let urgent = if Some(ws) == urgent_ws { 1 } else { 0 };
                send_update(SilkBarUpdate::new(
                    UpdateKind::SetWorkspaceUrgent as u32, ws, urgent, 0,
                ));
            }
            last_focus_state = focus_state;
        }

        // ~1s via yield (no rdtsc — freezes under QEMU TCG)
        for _ in 0..100 {
            sex_pdx::sys_yield();
        }

        // Read kernel uptime ticks for clock and chip cadence
        let ticks = sex_pdx::get_ticks();
        let uptime_seconds = ticks / LAPIC_TICKS_PER_SECOND_APPROX;
        let hh = ((uptime_seconds / 3600) % 24) as u8;
        let mm = ((uptime_seconds / 60) % 60) as u8;
        let ss = (uptime_seconds % 60) as u8;

        // Stage 2C: bounded internal status-chip stub (no new ABI, no floods).
        // Slow cadence: every 120 seconds.
        if uptime_seconds % 120 == 0 {
            match chip_phase {
                0 => {
                    let chip0_kind = if chip0_net { ChipKind::Net } else { ChipKind::Wifi };
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 0, chip0_kind as u32, 0,
                    ));
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 1, ChipKind::Wifi as u32, 0,
                    ));
                    chip0_net = !chip0_net;
                }
                1 => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 2, ChipKind::Battery as u32, 0,
                    ));
                }
                2 => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 3, ChipKind::Net as u32, 0,
                    ));
                }
                _ => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 3, ChipKind::Battery as u32, 0,
                    ));
                }
            }
            chip_phase = (chip_phase + 1) & 0x3;
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
