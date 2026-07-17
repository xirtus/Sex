# SCHEDULER_TICK_PD8_PF_FLAKE_V1 — Root Cause Report

## PHASE 3 — TRUE ROOT CAUSE FOUND AND FIXED (2026-07-18)

**Root cause: RSP0 ratchet leak in `yield_and_switch` (scheduler.rs ~line 630).**

The yield path set `update_tss_rsp0((*next).kstack_top)` with NO `+168`. Its
comment claimed the value was `Task.kstack_top` (alloc top) — type confusion:
`next` is `*const TaskContext` (tick()'s return), so the read is
`ctx.kstack_top` = **saved-frame BASE** (rax slot) — the very field the
timer/page-fault/lib.rs paths all take `+168` on. (TaskContext sits at Task
offset 0, so `*const Task`/`*const TaskContext` alias numerically; the field
read at +0xC0 is ctx.kstack_top either way.)

Mechanism (the "ratchet"): task saved with frame base K → yield-path dispatch
sets RSP0 = K instead of K+168 → next ring3 interrupt pushes its IRET+GPR
frame from aligned(K) downward → task re-saved with base ≈ K−168. Every
yield-path dispatch of a previously-saved task permanently lowers its kernel
stack by ~168–176 bytes; nothing ever resets it (only the task's own
yield-forge does, at Task.kstack_top−168). Busy, frequently-preempted PDs
under input storms burn ~390 such events → the 64 KiB kstack is exhausted →
interrupt frames push straight past the allocation base into the adjacent
heap object BELOW — which, given Task::new's alloc order (kstack vec, then
Task box), is another PD's **Task struct**.

This explains every phase-2 observation:
- pointer-rich Task overwrites (sprayed GPR/IRET frames, incl. userland ptrs);
- reproducible victim addresses (deterministic heap: quil Task box at
  0x444444608d00 sits directly below sexbell's kstack; silkbar's box below
  linen's kstack — the two observed victims);
- pd_ptr=0 / partial overwrites (which fields die depends on where the frame
  lands in the descent);
- run-2's silkbar "IRET frame corrupt" + `rsp.align16=8` (mid-ratchet frames;
  note align16=8 at the IRET base is NORMAL — HW 16-aligns then pushes 40
  bytes — the earlier "off-by-8 frame shift" hypothesis was a misread);
- fault-or-silent-stall variance and correlation with QMP input volume.

**Hardware-watchpoint proof** (qemu -gdb, `watch -l` on quil's
`context.pd_ptr` at 0x444444608db8, two independent captures): write hit in
`timer_interrupt_stub`'s GPR pushes with **RSP already inside quil's Task
struct** (rsp = the watched address itself at the faulting `push`), interrupt
entry point ≈ Task box interior — i.e., RSP0 had descended below the
neighboring kstack's base. Victim ctx dump showed pd_id=9/rip/cs/rflags/
rsp/ss intact, pd_ptr and kstack_top freshly clobbered — matching phase-2
run 2 exactly.

**Fix (one line + comment):** yield path now sets RSP0 = `kstack_top + 168`
(frame top), identical to the timer path. One-shot marker
`[scheduler.pd8.flake.fix.ok] reason=yield_rsp0_frame_base_ratchet` on first
yield-path dispatch. Phase-1 defenses (set_pd null-guard, steal reject +
drain-retry) stay armed as tripwires — they should now stay silent.

Files: `kernel/src/scheduler.rs` (backup `.bak.pd8_flake_v2`),
`kernel/src/init.rs` (backup `.bak.pd8_flake_v2` — boot instrumentation:
`scheduler.enqueue` now prints task ptr + pd_ptr field addr for watchpoint
work; kept, 14 lines once at boot).

Proof-run table: see PHASE 3 VERIFICATION at the end of this file.

---

Date: 2026-07-05
Fault signature:
```
EXCEPTION: PAGE FAULT at 0x58 (RIP: 0xffffffff80220a38, RSP: 0x4444446804e0, ERR: 0x0)
KERNEL PAGE FAULT HALT: addr=0x58 rip=0xffffffff80220a38 rsp=0x4444446804e0 err=0x0 pd=8
```
Capture: `logs/qemu-latest.log.bak.linen_zero_name_storm_v1:6651` (2026-07-02).
Reproduction today: 0/3 attempts clean (task bcdss8bla); prior lane observation ~70%.
Flake rate is environment/timing sensitive.

---

## A. Current Status

- One faulting capture on disk; today's `qemu-latest.log` clean.
- RIP symbolizes (current `iso_root/sexos-kernel`, rebuilt 2026-07-05 18:09, symbol
  still matches) to `<sex_kernel::scheduler::Scheduler>::tick`.
- No `task.faulted` or `fault.kill` markers anywhere in the faulting log — no
  userland fault preceded the crash. Preceding ~90 lines are steady-state noise
  (`lifecycle.focusref.make sid=100` spam, clock redraws, silkbar iter).

## B. Evidence Chain

1. **Disassembly at fault RIP** (objdump, `tick+~0x1c8`):
   ```
   mov rcx, [r15+0xb8]      ; rcx = next_task->context.pd_ptr   (TaskContext offset 0xB8)
   mov [rdx+rax], rcx       ; CoreLocal.current_pd_ptr = pd_ptr (store — succeeds)
   8b 49 58  mov ecx, [rcx+0x58]   ; ← FAULT RIP 0xffffffff80220a38
   mov [rdx+rax+8], ecx     ; CoreLocal.current_pd_id = id
   ```
   This is `CoreLocal::set_pd()` inlined into `tick()` (scheduler.rs:239 →
   core_local.rs:87-91): store ptr, then read `(*pd_ptr).id`.
2. **Fault address math:** addr `0x58` = `pd_ptr + offset_of(ProtectionDomain.id)`
   with **pd_ptr == NULL**. ERR=0x0 = supervisor read of not-present page. The
   earlier session's "pd_ptr=0x10" estimate was close but wrong — exact decode
   shows null base, `.id` field offset 0x58.
3. **TaskContext offsets verified:** repr(C), 15 GPRs 0x00-0x70, dummy 0x78,
   pkru 0x80, pd_id 0x88, rip 0x90 … pd_ptr 0xB8, kstack_top 0xC0 (compile
   assert scheduler.rs:22). Disasm `[r15+0xb8]` / `[r15+0x88]` match exactly.
4. **The read `[r15+0xb8]` itself did NOT fault** — the stolen `next_task`
   pointer references mapped memory whose bytes are zero. RSP 0x4444446804e0
   is in kernel heap (`HEAP_START = 0x4444_4444_0000`, lib.rs:51) — expected,
   per-task kstacks are heap `vec![0u8; 65536]`.

## C. Root Cause (confirmed mechanism)

`Scheduler::tick()` stole a task whose `context.pd_ptr` reads as NULL, then
`set_pd()` dereferenced it with **no null check** (core_local.rs:88-91).
Sequence:

1. Timer IRQ from user mode → `timer_interrupt_handler` → `sched.tick()`.
2. `steal()` returns pointer P; P is mapped but its Task bytes are zero
   (or P is not a live Task at all).
3. `tick()` line 236-239: reads `context.pd_ptr` = 0, calls `core.set_pd(0)`.
4. `set_pd` stores `current_pd_ptr = NULL` (succeeds), then reads
   `(*NULL).id` → kernel #PF at 0x58 → `KERNEL PAGE FAULT HALT`.

## D. Why "pd=8" Is a Red Herring

`set_pd` stores the (null) ptr FIRST, faults on the `.id` read SECOND —
`current_pd_id` never updates. The PF handler prints the STALE id: the last
successfully-bound PD. pd=8 = sexstore = victim of timing, not culprit.
Corroboration: sexstore behaved normally (spawned line 734, handoff 1587, ran
blocked-in-listen at rip 0x47005480). Its apparent 4400-line "silence" before
the fault is log-budget exhaustion (`TASK_LIFECYCLE_LOG_BUDGET=128`,
`SCHED_PICK_NEXT_LOG_BUDGET=32` — all lifecycle markers stop early), not
descheduling. Any PD could wear this fault's label.

## E. Injection Paths for the Bogus Task Pointer (unproven, ranked)

The single capture cannot pin which path put P in the queue. Candidates:

1. **Kstack clobber of a Task struct.** Tasks and their 64KiB kstacks are
   adjacent heap allocations. A kstack deep-write (the timer save path writes
   `old_ctx.kstack_top = base-128` frames; `yield_and_switch` rebuilds forged
   frames at `Task.kstack_top - 168`) that lands below its allocation zeroes a
   neighboring Task → pd_ptr=0 while the queue still holds its pointer.
   Fits "mapped but zero" evidence best.
2. **WorkStealingQueue ABA/wrap.** `steal()` loads `buffer[t & MASK]` then CAS
   on top; after 512 wraps a stale slot value can be returned if push/steal
   indices race. Single-core today (timer skips tick when interrupting kernel
   mode, so queue ops look serialized) — but `yield_and_switch` (syscall
   context) and `page_fault` handler (exception context) both push+tick;
   any future IF-enabled window in those paths makes this live.
3. **Double-push of the same task** via `yield_now`/`yield_and_switch`
   (push + clear current_task) racing a timer-tick requeue of the same task —
   would need an interleave that current CS.RPL guard should exclude, but the
   guard's coverage of the PF-handler tick path is unaudited.

## F. Fix Plan — IMPLEMENTED 2026-07-05 (approved)

Minimal, two files, no ABI change:

1. **core_local.rs `set_pd`:** null-guard added. On null: no store, emit
   `[sched.set_pd.null] refused core=<n>`, return. Kernel keeps running with
   previous PD binding; flake becomes a logged event instead of a halt.
2. **scheduler.rs `tick`:** stolen-task validation added between
   steal/attempt_steal and the `current_task` swap:
   - ptr within kernel heap (`HEAP_START .. +HEAP_SIZE`) — checked BEFORE any
     deref, so an out-of-heap garbage ptr is never touched;
   - `context.pd_ptr` non-null AND `context.pd_id < MAX_DOMAINS`.
   On failure: `[sched.steal.reject] ptr=… reason=out_of_heap` or
   `[sched.steal.reject] ptr=… pd_ptr=… pd_id=… reason=corrupt_task`, task
   treated as null → tick degrades to "no runnable", interrupted task resumes
   via its original IRET frame. The corrupt slot is dropped from the queue.
   Marker payload names the injection path on next occurrence
   (out_of_heap = queue corruption/ABA; corrupt_task = neighboring-allocation
   clobber of a live Task).
3. Canary word in `Task` — NOT implemented (deferred until a reject marker
   actually fires; avoid speculative TCB growth).

**Verification (2026-07-05):** `cargo build --release` clean (pre-existing
warnings only); full `entrypoint_build.sh` success; `gate_0_2.sh` run:
BUILD/BOOT/FAULT_REGRESSION/INPUT_OWNERSHIP PASS, 85 `task.running` markers,
all PDs cycling, ZERO false-positive reject/null markers, zero kernel PF.
POINTER/KEYBOARD_LIVE FAILs match the pre-fix baseline in
GATE_0_2_LAST_RUN.md (known input-lane state, unrelated to this change).
Note: flake was ~70% historical but 0/3 on 2026-07-05 even unfixed — one
clean run does not prove the corrupt-steal path is gone, it proves the halt
is now impossible and the diagnostics are armed.

## F2. Phase-2 Evidence (2026-07-05, usb_path_gate.sh lane runs)

The diagnostics fired the same day. Two independent captures:

1. **Run 1:** `[sched.steal.reject] ptr=0x444444608d00 pd_ptr=0x0
   pd_id=1174428183 reason=corrupt_task` — in-heap Task, pd_ptr zeroed,
   pd_id = 0x46005A17 (a linen-range userland address). Task memory
   overwritten with pointer-rich data, NOT zeros.
   Exposed a fix gap: single reject returned None while a runnable task sat
   behind the corrupt entry; `yield_and_switch` parks on None (timer skips
   kernel-mode ticks) → userland deadlocked with kernel alive (PS/2 IRQs
   only). **Fixed:** tick now drain-and-retries rejects, bounded by
   QUEUE_SIZE (`[sched.steal.reject.exhausted]` if the whole queue is bad).
2. **Run 2 (post-retry-fix):** same ptr `0x444444608d00`, pd_ptr=0x0 but
   pd_id=9 (quil) INTACT — partial overwrite of quil's Task struct at a
   REPRODUCIBLE heap address. Kernel kept running ~4800 log lines further,
   then died downstream with new, richer evidence:
   - silkbar (pd=6) IRET frame corrupt on its kstack:
     `q0.rip=0x45003167 q1.cs=0x4444445f8ad8 q2.rflags=0x0 q4.ss=0x0` —
     a kernel-stack ADDRESS sitting in the CS slot, `rsp.align16=8`
     (frame shifted by 8 bytes);
   - resulting #GP, then #PF inside general_protection_fault_handler
     (RIP 0xffffffff8021f65b: GDT-style indexed read with the garbage
     selector) → `KERNEL PAGE FAULT HALT addr=0x4443c48badd8 pd=6`.

**Working hypothesis (phase 2):** an 8-byte frame-offset bug in a context
save/restore path (timer save writes GPRs at fixed offsets from
stack_frame; yield_and_switch forges frames at Task.kstack_top-168; the
two must agree exactly). An off-by-8 both shifts IRET frames
(rsp.align16=8, CS slot holding a stack address) and sprays saved register
values (userland pointers — matching run 1's pd_id=0x46005A17) across
adjacent heap allocations, clobbering Task structs. Next step: audit the
three frame layouts (syscall stub push order, timer stub push order,
yield_and_switch forge) against each other; instrument a canary qword at
kstack_top-176.

## G. Repro / Gate

- Lane: boot QMP input-proof lane repeatedly (historically ~70% fault rate;
  today 0/3 — rerun ≥10 boots for signal).
- Gate addition after fix: FAIL on `KERNEL PAGE FAULT HALT`, WARN-collect on
  `[sched.steal.reject]` / `[sched.set_pd.null]` markers with their pointer
  payloads; those payloads close phase 2 of this investigation.
- Note: `claude-references/BUG_HISTORY.md` (per CLAUDE.md self-update rule)
  does not exist in this repo — this doc is the canonical record.

---

## PHASE 3 VERIFICATION (2026-07-18, post-fix)

Build: `./scripts/entrypoint_build.sh` PASS (twice: instrumentation, then fix).

6/6 runtime boots clean, every one driven with QMP input storms (the
historical trigger; pre-fix ~2/3 of identical lanes died by fault or silent
input stall):

| # | Lane | Stimulus | Watchpoint hits | Faults/rejects/set_pd.null | Result |
|---|------|----------|-----------------|----------------------------|--------|
| 1 | watchpoint (4 hw watches on Task pd_ptr fields) | 40 rounds keys+abs pointer, 300s | 0 | 0 | PASS |
| 2 | watchpoint | same | 0 | 0 | PASS |
| 3 | watchpoint | same | 0 | 0 | PASS |
| 4 | spindle visible/typing | scroll_lock,a,b,c,ret,a,right,backspace + screendump | n/a | 0 | all marker rows PASS, glyph px visible |
| 5 | spindle visible/typing | same | n/a | 0 | all rows PASS |
| 6 | spindle visible/typing | same | n/a | 0 | all rows PASS |

Every boot: `[scheduler.pd8.flake.fix.ok] reason=yield_rsp0_frame_base_ratchet`
exactly once; `[spindle.ghost.accept]`, `[spindle.history.push]`,
`[spindle.input.echo.ok]` all still live; cursor/keyboard path unchanged.
USER_FAULT_CONTAINMENT untouched (no fault-path edits; phase-1 tick/set_pd
tripwires remain armed and stayed silent). No new serial output besides the
two boot-time markers.

Pre-fix control (same day, same lanes): 1 of 3 watchpoint lanes caught the
Task-struct overwrite live in `timer_interrupt_stub` pushes; spindle lanes
historically died ~4/6 (fault line or silent input stall).

---

## REGRESSION GATE (RSP0_RATCHET_REGRESSION_GATE_V1, 2026-07-18)

```sh
./scripts/rsp0_regression_gate.sh                      # static source rows only
./scripts/rsp0_regression_gate.sh <serial-log>         # + runtime rows
```

Static rows: yield_and_switch has exactly one RSP0 assignment, it reads
`kstack_top + 168`, no bare `kstack_top` variant, and the comment still names
the `saved-frame BASE` trap. Runtime rows (log given): fix marker present,
zero faults, zero phase-1 tripwires (`sched.steal.reject` / `sched.set_pd.null`).
Negative-tested: stripping `+ 168` from the source fails the gate.

Note: the `scheduler.enqueue pd_id=N task=0x... pdptr_field=0x...` boot lines
(init.rs) are intentional permanent diagnostics — 14 lines once per boot,
they bootstrap hardware-watchpoint capture for any future Task-corruption
investigation. Do not strip.
