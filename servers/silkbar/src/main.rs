#![no_std]
#![no_main]

use silkbar_model::*;

// ── Public API ──────────────────────────────────────────────────────────────

/// Run all queue invariant tests from the model crate.
pub fn validate_invariants() -> bool {
    silkbar_model::validate_invariants()
}

/// Send a typed `SilkBarUpdate` to sexdisplay via PDX.
/// Wire format: arg0=kind, arg1=(index << 32)|a, arg2=b
fn send_update(update: SilkBarUpdate) {
    let _ = sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
}

/// Temporary snapshot producer — sends initial model state to sexdisplay.
/// Replaced later by real modules (clock driver, workspace manager, etc.)
/// that push incremental updates on state changes.
///
/// Five updates:
///   1. SetClock 10:43
///   2. SetClock 10:44
///   3. SetChipVisible index=1 visible=false
///   4. SetWorkspaceActive index=4 true
///   5. SetWorkspaceActive index=2 false
fn send_initial_state_snapshot() {
    send_update(SilkBarUpdate::new(4, 0, 10, 43));
    send_update(SilkBarUpdate::new(4, 0, 10, 44));
    send_update(SilkBarUpdate::new(2, 1, 0, 0));
    send_update(SilkBarUpdate::new(0, 4, 1, 0));
    send_update(SilkBarUpdate::new(0, 2, 0, 0));
}

// ── Clock Tick ────────────────────────────────────────────────────────────

/// Clock tick interval in spin-loop iterations (~0.5–1 s on 2 GHz with `pause`).
const CLOCK_TICK_INTERVAL: u64 = 10_000_000;

// ── Entry Point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Validate model invariants at boot
    if !validate_invariants() {
        // Invariant failure - halt silently
        loop { core::hint::spin_loop(); }
    }

    // Send initial state snapshot to prove typed SilkBarUpdate transport.
    // Temporary — real modules replace this later.
    send_initial_state_snapshot();

    // ── Idle server loop with fake clock tick ────────────────────────────────
    // TODO: workspace producer — listen for WM events, push SetWorkspace*
    // TODO: status producer — poll net/wifi/battery, push SetChip*
    // TODO: input/action listener — receive click events, dispatch actions

    // Local clock state — starts at 10:44:00 (matches end of initial snapshot).
    let mut hh: u8 = 10;
    let mut mm: u8 = 44;
    let mut ss: u8 = 0;
    let mut tick: u64 = 0;

    loop {
        tick += 1;
        if tick % CLOCK_TICK_INTERVAL == 0 {
            // Advance one second, rolling up through minutes and hours
            ss += 1;
            if ss >= 60 {
                ss = 0;
                mm += 1;
                if mm >= 60 {
                    mm = 0;
                    hh += 1;
                    if hh >= 24 {
                        hh = 0;
                    }
                }
            }
            send_update(SilkBarUpdate::new(4, 0, hh as u32, mm as u32 | ((ss as u32) << 8)));
        }
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
