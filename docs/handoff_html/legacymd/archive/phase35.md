🧱 What the “automation MD” should do

Think of it like this (no poetry, just structure):

1. Deterministic build order
kernel first (clock + smp + ap trampoline)
then ipc layer
then servers (sexdisplay / silk-shell)
2. Hard invariants (machine-checkable)
GS must be valid before STI
TSC_FREQ must be set before any deadline programming
IPC must be Acquire/Release enforced
APs must reach READY before BSP enables interrupts
3. Boot phases as a state machine

Instead of “Phase 32.2 / 33 / 34 / 35” in your head:

BOOT_STATE:
  0 INIT
  1 TSC_CALIBRATED
  2 CORE_ENUMERATED
  3 GS_BOUND
  4 SMP_TRAMPOLINE_LOADED
  5 AP_SYNCED
  6 FRAME_ENGAGED
  7 USERLAND_RUNNING
4. Single command execution flow

You want:

make sasos-run
make sasos-test
make sasos-qemu-smp

not manual step chains anymore.

📦 What the MD should contain

Here’s the clean structure you want:

PHASE_32_TO_35_AUTOMATION.md
1. Boot Pipeline
ACPI MADT parse
TSC calibration
CoreTable init
GS binding per AP
AP trampoline load
SIPI broadcast
barrier wait
frame enable
2. Hard Invariants (DO NOT BREAK)
GS must be per-core before interrupts
IPC must never use relaxed ordering
frame_gate must be monotonic
TSC must be globally consistent across cores (calibrated delta max threshold)
no AP enters userland before READY latch
3. Memory Model Contract
SPSC per IPC lane only
Acquire on dequeue
Release on enqueue
sfence only on frame commit
no mfence in hot path
4. SMP ignition state machine
BSP:
  init -> calibrate -> enumerate -> load_trampoline -> sipi -> wait

AP:
  trampoline -> long_mode -> gs_bind -> ready -> idle_wait

SYNC:
  CORES_READY == N
  THEN enable_interrupts_global()
5. Build + Run automation
make bios
make kernel
make initrd
make smp-image
make run-smp-qemu
6. Failure modes (important)
AP missing GS → #GP crash
wrong TSC → frame drift
STI too early → race interrupt corruption
SIPI timing too fast → AP hang
🧠 Why this matters (no fluff)

Right now your system is:

“correct architecture expressed in incremental reasoning”

That breaks the moment:

you compile differently
reorder init steps
or forget one invariant in SMP

The MD turns it into:

“deterministic boot machine”
