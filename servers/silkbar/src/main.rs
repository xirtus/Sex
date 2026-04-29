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
    send_update(SilkBarUpdate::new(4, 0, 10, 43 << 8)); // hh=10, mm=43, ss=0
    send_update(SilkBarUpdate::new(4, 0, 10, 44 << 8)); // hh=10, mm=44, ss=0
    send_update(SilkBarUpdate::new(2, 1, 0, 0));
    send_update(SilkBarUpdate::new(0, 4, 1, 0));
    send_update(SilkBarUpdate::new(0, 2, 0, 0));
}

// ── Clock Module ─────────────────────────────────────────────────────────

/// Fake clock module — produces one SetClock update per second.
/// Replaced later by a real RTC-driven clock producer.
struct ClockModule {
    hh: u8,
    mm: u8,
    ss: u8,
    spin: u64,
}

/// Clock tick interval in spin-loop iterations (~0.5–1 s on 2 GHz with `pause`).
const CLOCK_TICK_INTERVAL: u64 = 10_000_000;

impl ClockModule {
    /// Create a new clock module at the given wall time.
    fn new(hh: u8, mm: u8, ss: u8) -> Self {
        ClockModule { hh, mm, ss, spin: 0 }
    }

    /// Advance one spin-loop iteration.
    /// Returns `Some(SilkBarUpdate)` when the clock ticks (once per interval).
    fn tick(&mut self) -> Option<SilkBarUpdate> {
        self.spin += 1;
        if self.spin % CLOCK_TICK_INTERVAL != 0 {
            return None;
        }
        // Advance one second, rolling up through minutes and hours
        self.ss += 1;
        if self.ss >= 60 {
            self.ss = 0;
            self.mm += 1;
            if self.mm >= 60 {
                self.mm = 0;
                self.hh += 1;
                if self.hh >= 24 {
                    self.hh = 0;
                }
            }
        }
        Some(SilkBarUpdate::new(
            4, 0,
            self.hh as u32,
            ((self.mm as u32) << 8) | self.ss as u32,
        ))
    }
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

    // ── Idle server loop with fake clock tick ────────────────────────────────
    // TODO: workspace producer — listen for WM events, push SetWorkspace*
    // TODO: status producer — poll net/wifi/battery, push SetChip*
    // TODO: input/action listener — receive click events, dispatch actions

    // Clock module starts at 10:44:00 (matches end of initial snapshot).
    let mut clock = ClockModule::new(10, 44, 0);

    loop {
        if let Some(update) = clock.tick() {
            send_update(update);
        }
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
