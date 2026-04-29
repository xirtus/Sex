#![no_std]
#![no_main]

use silkbar_model::*;

// ── Clock Simulation State ──────────────────────────────────────────────────

static mut FAKE_HH: u8 = 10;
static mut FAKE_MM: u8 = 42;

// ── Public API (v7: local only, no PDX transport yet) ───────────────────────

/// Enqueue a `SilkBarUpdate` into the local ring buffer.
/// Returns `false` if the queue is full.
pub fn enqueue_update(queue: &mut SilkBarUpdateQueue, update: SilkBarUpdate) -> bool {
    queue.push(update)
}

/// Generate a fake clock tick: increment time by one minute, push a `SetClock` update.
/// Returns `false` if the queue is full.
pub fn tick_clock_fake(queue: &mut SilkBarUpdateQueue) -> bool {
    unsafe {
        FAKE_MM = FAKE_MM.wrapping_add(1);
        if FAKE_MM >= 60 {
            FAKE_MM = 0;
            FAKE_HH = (FAKE_HH + 1) % 24;
        }
        let update = SilkBarUpdate::new(4, 0, FAKE_HH as u32, FAKE_MM as u32);
        queue.push(update)
    }
}

/// Run all queue invariant tests from the model crate.
pub fn validate_invariants() -> bool {
    silkbar_model::validate_invariants()
}

/// Send a typed SilkBar update to sexdisplay via PDX.
/// Encodes wire format: arg0=kind, arg1=(index << 32)|a, arg2=b.
fn send_update(kind: UpdateKind, index: u8, a: u32, b: u32) {
    let _ = sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        kind as u32 as u64,
        ((index as u64) << 32) | a as u64,
        b as u64,
    );
}

/// Send boot-time demo updates to sexdisplay via PDX.
/// Five updates queued before sexdisplay first drains:
///   1. SetClock 10:43
///   2. SetClock 10:44
///   3. SetChipVisible index=1 visible=false
///   4. SetWorkspaceActive index=4 true
///   5. SetWorkspaceActive index=2 false
fn send_boot_demo_updates() {
    send_update(UpdateKind::SetClock, 0, 10, 43);
    send_update(UpdateKind::SetClock, 0, 10, 44);
    send_update(UpdateKind::SetChipVisible, 1, 0, 0);
    send_update(UpdateKind::SetWorkspaceActive, 4, 1, 0);
    send_update(UpdateKind::SetWorkspaceActive, 2, 0, 0);
}

// ── Entry Point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut _queue = SilkBarUpdateQueue::empty();

    // Validate model invariants at boot
    if !validate_invariants() {
        // Invariant failure - halt silently
        loop { core::hint::spin_loop(); }
    }

    // Send boot demo updates to prove SilkBarUpdateQueue draining.
    send_boot_demo_updates();

    // Future: listen for events, push updates, send to sexdisplay.
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
