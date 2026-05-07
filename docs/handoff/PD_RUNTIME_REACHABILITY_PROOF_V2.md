# PD_RUNTIME_REACHABILITY_PROOF_V2

## 1. Runtime timeline (from `.gate_master/serial.log`)

1. LAPIC timer calibration and init occurs:
- `APIC: LAPIC timer calibrated: ...`
- `timer.init.done ...`

2. PD spawn completes for all target domains:
- `✓ Spawned PD 1..12`
- Includes `PD 3 (silk-shell)`, `PD 4 (sexinput)`, `PD 11 (sexfiles)`.

3. Capability wiring completes, including storage route setup:
- `[kernel.cap.block] sexfiles->sexdrive slot=15`

4. Scheduler bootstrap and enqueue completes:
- `scheduler.enqueue pd_id=1` ... `scheduler.enqueue pd_id=12`
- `init: Ready for Scheduler.`

5. First scheduling event occurs:
- `scheduler.tick.enter core=0 phase=4 rq_depth=12`
- `task.running id=1 pd_id=1 ...`
- `scheduler.pick_next pd_id=1`
- `first scheduled pd_id=1`

6. Context switch to PD1 succeeds and userland starts:
- `context_switch.before_switch_to ... pd_id=1`
- `iret.frame.check ... rflags.if=true`
- `user.entry.bytes ...`
- then PD1 app logs continue (`[sexdisplay.ready]`, render proof markers).

7. Missing after first switch:
- No subsequent `timer.tick.enter`
- No subsequent `scheduler.tick.enter`
- No `task.running ... pd_id=3/4/11`
- No `sexfiles.ready`
- No `sexdrive.ready`
- No `fault.kill/#PF/#GP/panic` marker hit in this captured run.

## 2. Spawn/enqueue/running table

| PD | Service | Spawned | Enqueued | Running seen | Ready marker seen |
|---|---|---|---|---|---|
| 1 | sexdisplay | Yes | Yes | Yes (`task.running pd_id=1`) | Yes (`sexdisplay.ready`) |
| 2 | sexdrive | Yes | Yes | No | No (`sexdrive.ready` absent) |
| 3 | silk-shell | Yes | Yes | No | N/A |
| 4 | sexinput | Yes | Yes | No | N/A |
| 11 | sexfiles | Yes | Yes | No | No (`sexfiles.ready` absent) |
| 5/6/7/8/9/10/12 | other spawned PDs | Yes | Yes | No | N/A |

## 3. Exact first dead hop

**First dead hop: after successful first context switch into PD1, periodic preemption/timer-driven re-entry into scheduler is not observed in the probe log.**

Evidence chain:
- Runqueue depth is nonzero (`rq_depth=12`).
- Scheduler clearly has runnable tasks (`scheduler.enqueue pd_id=1..12`).
- First dispatch works (`task.running pd_id=1`, context switch + user entry valid).
- Then no second tick/scheduler entry is logged, so other queued PDs never get CPU time in-window.

Classification against mission matrix:
- PD3/PD4/PD11 are **C: enqueued but not scheduled** (within the observed probe window), with strongest immediate suspect being missing periodic preemption after first dispatch.

## 4. No-behavior-change statement

This mission performed **audit/proof only**.
No kernel behavior, scheduler behavior, interrupts behavior, task states, ABI, sex-pdx, or storage protocol was changed.

## 5. Exact next STOP FIRST patch prompt

```text
MISSION: PD1_YIELD_OR_TIMER_PREEMPT_PROOF_V1

Goal:
Prove why scheduling does not return after first PD1 dispatch, without changing scheduler algorithm.

Scope:
- kernel/src/interrupts.rs (timer path only, narrow ranges)
- kernel/src/apic.rs (timer programming only)
- kernel/src/scheduler.rs (tick entry/exit observability only)

Rules:
- proof markers only, no behavior changes
- no ABI/sex-pdx/storage edits
- no scheduler policy rewrite
- STOP FIRST before any control-flow change

Add minimal markers:
- timer.tick.enter count=<n>
- timer.eoi.sent
- scheduler.tick.exit switched=<0|1>
- scheduler.current pd_id=<id>

Success:
- show whether timer IRQ fires repeatedly after PD1 enters userland
- if timer fires but scheduler not switching, identify exact branch
- if timer does not fire, isolate APIC timer programming/EOI dead point
- produce smallest next behavioral patch prompt only after proof
```

## 6. Commands used / reproducible grep

```bash
grep -E 'pd\.spawn|task\.enqueue|task\.running|sexfiles\.ready|sexdrive\.ready|scheduler\.no_next|fault\.kill|#PF|#GP|panic|timer|tick' .gate_master/serial.log || true

rg -n 'Spawned PD|kernel\.spawn|scheduler\.enqueue|task\.running|scheduler\.pick_next|task\.requeued|sched\.no_runnable|timer\.tick\.enter|scheduler\.tick\.enter|sexfiles\.ready|sexdrive\.ready|fault\.kill|#PF|#GP|panic|FATAL' .gate_master/serial.log
```
