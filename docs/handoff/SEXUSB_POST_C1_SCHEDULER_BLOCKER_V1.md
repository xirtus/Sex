# SEXUSB_POST_C1_SCHEDULER_BLOCKER_V1

**Date:** 2026-05-07
**Status:** DOCS — USB parked at C1, no code changes

## 1. Safe Baseline (C1)

SEXUSB_SLOT2_POLL_START_V1 queues exactly one interrupt-IN Normal TRB on
slot2's transfer ring and rings the doorbell.  This proves:

- Slot2 XHCI slot enabled and addressed
- Slot2 device + config descriptors fetched
- Slot2 HID role classified (PointerTablet for QEMU usb-tablet)
- Slot2 SET_CONFIGURATION issued
- Slot2 Configure Endpoint command succeeded
- Slot2 interrupt ring + report buffer allocated
- Slot2 endpoint context built and active
- One TRB completion event appears on the shared event ring

The slot2 event is consumed as "unrelated" by the slot1 poll loop.
No dispatch, no rearm, no classification — zero overhead after boot.

**C1 is proven safe:** clock advances, windows open, runtime gate passes.

## 2. C2 Regression

### C2A — HidDevice Table (benign but unnecessary without C2B/C2C)

Added `HidDevice` struct + `devices` array.  Populated during enumeration,
never read in the poll loop.  Likely benign on its own.

### C2B — Event Match Helper (suspect)

Added `for midx in 0..device_count` loop in the innermost event-wait
spin of the poll loop.  Executes on EVERY Transfer Event completion.

### C2C — Slot2 Report Classification (suspect)

Added 3× `volatile` reads + conditional branches in the `else` branch
of the slot1 match check.  Executes on every slot2 event.

### Regression Mechanism

The cooperative scheduler relies on each PD's event loop to `sys_yield()`
frequently.  The sexusb poll loop is the hottest path — it spins on the
event ring with `sys_yield()` only when no event is pending.  Under load
(keyboard events arriving), the loop rarely yields.

Adding work inside this loop (C2B match scan, C2C volatile reads) increases
the per-iteration cost, delaying `sys_yield()` calls.  Other PDs (sexdisplay,
silk-shell, quil) starve waiting for scheduler ticks.

**Result:** clock freezes, windows never open, runtime gate FAIL.

## 3. Blocked Phases

| Phase | Description | Blocked by |
|-------|-------------|------------|
| C2A | HidDevice table | Applied but unused without C2B/C2C |
| C2B | Event match helper | Scheduler starvation |
| C2C | Slot2 classify (no dispatch) | Scheduler starvation |
| C2D | Slot2 rearm | Depends on C2B+C2C |
| C2E | Slot2 forward to sexinput | Depends on C2D |
| C2F | Click-focus real device proof | Depends on C2E + non-zero pointer data |

## 4. Future Unblock Options

### A. Preemptive Scheduler / Timer Fix

Fix the cooperative scheduler so sexusb's poll loop can't starve other
PDs.  Requires kernel `scheduler.rs` changes — out of scope for USB agent.

**Effort:** High (kernel)
**Risk:** Medium (timing-sensitive)
**Unblocks:** All C2 phases

### B. Budgeted Poll-Loop Yielding

Add an iteration counter to sexusb's poll loop and force `sys_yield()`
every N iterations (e.g., every 64).  This lets the scheduler run other
PDs even under high USB event load.

**Effort:** Low (~5 lines in poll loop)
**Risk:** Low (may miss USB events during yield gap — acceptable for
keyboard, problematic for high-rate pointer)
**Unblocks:** C2B, C2C (with caveats)

### C. Separate sexusb2 PD

Register a second USB protection domain for slot2 enumeration + polling.
Requires kernel `devmgr` to grant a second `SLOT_USB_HOST` capability
and sexusb2 to handle slot2 independently.

**Effort:** High (kernel + new PD)
**Risk:** Medium
**Unblocks:** All C2 phases (slot2 runs in its own scheduler quantum)

### D. Real Hardware Timing Proof

Boot on real hardware.  Real xHCI interrupt timing and hardware CPU
speed may mean the poll loop never starves the scheduler.  QEMU's
emulated timing may be the root cause of the starvation.

**Effort:** Medium (hardware boot setup)
**Risk:** Low (no code changes)
**Unblocks:** C2B-C2E (if hardware proves no starvation)

### E. Synthetic Slot2 Report Gate

Use the existing `SEXUSB_SYNTHETIC` gate (dead code at line 3570) to
inject slot2 pointer reports via `OP_USB_MOUSE_REPORT` directly into
the PDX ring, bypassing the poll loop entirely.  This proves the
pointer pipeline end-to-end without changing the hot path.

**Effort:** Low (~30 lines in dead synthetic block)
**Risk:** Low (dead code when gate off, no poll-loop change)
**Unblocks:** C2E (bypasses C2B-C2D)

## 5. Recommendation

**Option E (Synthetic Slot2 Gate)** is the fastest path to prove the
pointer pipeline.  It requires zero poll-loop changes, reuses the
existing synthetic infrastructure, and unblocks the click-focus real
device proof phase.

**Option B (Budgeted Yield)** is the simplest structural fix and should
be attempted after Option E proves the pipeline.

**Options A/C/D** are long-term but not urgent — the guest pipeline is
already proven via the synthetic drag proof (INPUT_PHASE_CLOSEOUT_V1).

## 6. Current State

| Component | Status |
|-----------|--------|
| C1 slot2 poll start | ✅ Proven |
| C2 demux | ❌ Blocked (scheduler) |
| C2A table | ❌ Reverted |
| C2B match | ❌ Reverted |
| C2C classify | ❌ Reverted |
| Synthetic pointer pipeline | ✅ Proven (INPUT_PHASE_CLOSEOUT_V1) |
| Real pointer data | ❌ QEMU host-routing gap |
| Build | ✅ PASS, 1761 sectors |

---

*End of SEXUSB_POST_C1_SCHEDULER_BLOCKER_V1.md*
