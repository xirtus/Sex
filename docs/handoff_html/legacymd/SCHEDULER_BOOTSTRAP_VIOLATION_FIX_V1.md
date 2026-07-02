# SCHEDULER_BOOTSTRAP_VIOLATION_FIX_V1

**Date:** 2026-05-25
**Baseline HEAD:** `7ca20e6a`
**Scope:** Scheduler panic fix — INITIALIZED race between LAPIC timer and init_first_core

---

## A) Outcome

**PASS** — scheduler bootstrap violation eliminated. No panic. All 14 PDs schedule and run.

---

## B) Root Cause

### Panic location
`kernel/src/scheduler.rs:204`:
```rust
assert!(
    crate::core_local::INITIALIZED.load(Ordering::Acquire) == true,
    "SCHEDULER_BOOTSTRAP_VIOLATION"
);
```

### Race window

1. `lib.rs`: `hal::init_advanced()` → `apic::init()` → `interrupts::enable()` at `apic.rs:333`
   **LAPIC periodic timer now running.**
2. `lib.rs`: `init::init()` spawns 14 PDs (added `RESERVED_SHARED` + `kaleidoscope`) —
   phase advances to `BootPhase::SchedulerRunning` at `init.rs:714`
3. Timer fires. `interrupts.rs` phase guard: `phase >= SchedulerRunning` → **passes**.
4. `sched.tick()` → `INITIALIZED == false` → **PANIC**
5. `init_first_core()` (which sets `INITIALIZED = true`) never reached.

### Why now

`module_paths` array grew from 13 to 15 entries (`RESERVED_SHARED`, `kaleidoscope`).
Wider `init::init()` window gave LAPIC timer more chances to fire after
`BootPhase::SchedulerRunning` advanced but before `init_first_core` ran.

Visible QEMU (GTK display) also has slightly different timing than headless,
further widening the race.

---

## C) Fix

**File:** `kernel/src/lib.rs`

Add `interrupts::disable()` before the bind sequence so the LAPIC timer cannot
fire between `BootPhase::SchedulerRunning` and `init_first_core`:

```rust
// 5. Start Scheduler (Phase 21: Preemptive Multi-tasking)
// APIC calibration leaves interrupts enabled; disable here so the LAPIC timer
// cannot fire before init_first_core sets INITIALIZED=true.
x86_64::instructions::interrupts::disable();
serial_println!("scheduler.bind.before");
```

Interrupts re-enable at the existing `x86_64::instructions::interrupts::enable()` call
later in `lib.rs` (or via `iretq` rflags restore when `switch_to` fires).

**Invariant preserved:** `SCHEDULER_BOOTSTRAP_VIOLATION` still fires if anything calls
`sched.tick()` before `INITIALIZED = true`. The fix just closes the race window; it
does not weaken the assertion.

---

## D) Files Changed

| File | Change |
|------|--------|
| `kernel/src/lib.rs` | Add `interrupts::disable()` before `scheduler.bind.before` (3 lines) |

---

## E) Proof Log Markers (headless verify run)

```
scheduler.bind.before          ← no panic before this
scheduler.bind.target_pd_id=1
scheduler.bind.after           ← init_first_core completed, INITIALIZED=true
scheduler.tick.enter core=0 phase=4 rq_depth=14   ← passes INITIALIZED check
task.running id=1 pd_id=1 rip=0x40007890          ← sexdisplay scheduled
context_switch.begin
...
[sexusb.ready]                 ← USB pipeline reaches ready state
```

---

## F) Fault Scan

**faults_zero: PASS** — 0 occurrences of:
- `BOOTSTRAP_VIOLATION`
- `SCHEDULER_RUNNING_VIOLATION`
- `panic` / `PANIC`
- `#PF` / `page_fault`
- `#GP` / `general_protection`

---

## G) Next Prompt

**USB_HID_BOOT_KEYBOARD_HUMAN_OPERATOR_PASS** — scheduler now boots cleanly.
Re-run visible QEMU operator proof:

```bash
SEXOS_QEMU_DISPLAY=gtk SEXUSB_QEMU_DEVICE=kbd SEXOS_QEMU_I8042=off \
  ./dev.sh run 2>&1 | tee logs/qemu-usb-kbd-operator.log
```

Operator: click QEMU GTK window, type `test`, close window.
Then verify markers and update `USB_HID_BOOT_KEYBOARD_PROOF_V1.md`.
