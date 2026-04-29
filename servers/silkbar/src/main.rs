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

// ── Entry Point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut _queue = SilkBarUpdateQueue::empty();

    // Validate model invariants at boot
    if !validate_invariants() {
        // Invariant failure - halt silently
        loop { core::hint::spin_loop(); }
    }

    // v8: Send one fake clock update (SetClock 10:44) to sexdisplay via scalar PDX.
    // Wire format: arg0=kind, arg1=(index << 32)|a, arg2=b
    let (_status, _value) = sex_pdx::pdx_call(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        4,                         // arg0: kind = SetClock
        (0u64 << 32) | 10,         // arg1: index=0, a=10 (hour)
        44,                        // arg2: b=44 (minute)
    );
    // If PDX fails (ERR_SERVICE_NOT_READY, ring full), no visual change — guaranteed.

    // v7: owns _queue, no PDX transport yet.
    // v8: _queue becomes mut, tick_clock_fake called periodically.
    // Future: listen for events, push updates, send to sexdisplay.
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
