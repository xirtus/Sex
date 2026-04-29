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

    // Future: listen for events, push updates, send to sexdisplay.
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
