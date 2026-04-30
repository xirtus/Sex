#![no_std]
#![no_main]

use silkbar_model::{SilkBarUpdate, UpdateKind, ChipKind, OP_SILKBAR_UPDATE};

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
    let mut focus_state: u8 = 0;
    let mut last_focus_state: u8 = 0xFF;
    let mut chip_phase: u8 = 0;

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

        // Stage 2C: bounded internal status-chip stub (no new ABI, no floods).
        // Phases 0-2 emit one chip-kind update; phase 3 emits three.
        if ss % 20 == 0 {
            match chip_phase {
                0 => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 1, ChipKind::Wifi as u32, 0,
                    ));
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
                        UpdateKind::SetChipKind as u32, 1, ChipKind::Net as u32, 0,
                    ));
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, 2, ChipKind::Wifi as u32, 0,
                    ));
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
