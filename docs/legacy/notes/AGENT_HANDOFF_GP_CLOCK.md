# AGENT_HANDOFF_GP_CLOCK.md — Master Baseline Handoff Ledger

> **Status**: Current as of master `9da9897`.
> **Policy**: Read this first in every session. It is the canonical handoff.

---

## Current Master Baseline

**Branch:** `master` → `origin/master`
**Tip commit:** `9da9897` — `build(gate): remove stale apps/quil workspace member and sync abi_version_hash`
**Parent:** `d497c76` — `fix(kernel): contain user null-jump faults`

**Working tree:** Clean (only untracked `an/` — ignore unless asked).

**All 6 PDs spawn and run** (sexdisplay, sexdrive, silk-shell, sexinput, silkbar, linen):
- Zero page faults, zero panics on normal boot
- Scheduler round-robins all 6 PDs with no stalls
- Clock counts in SilkBar (no freeze after PD fault)
- QEMU window appears (no `-display gtk` override)

**Known: screen may appear black** — see Black Screen Diagnosis in `HANDOFF.md`.

---

## USER_FAULT_CONTAINMENT_V1

**What it is:** The canonical kernel fix for user null-jump faults.

**Root cause:** When a user task faulted at RIP=0x0, the page fault handler
set the task to STATE_BLOCKED and returned via iretq — which loaded the
unchanged poisoned IRET frame (RIP=0x0), causing an immediate re-fault.
Interrupts are disabled during exception handling (IF cleared by #PF), so
the timer could never fire to preempt the loop. The system appeared frozen.

**Fix (kernel/src/interrupts.rs):**
- User null-jump tasks are set to Exited (3) instead of BLOCKED
- When `tick()` returns None and the task is Exited, the raw IRET frame is
  rewritten to return to a kernel-mode halt loop (`faulted_task_halt`)
  with IF=1 (interrupts enabled)
- The timer interrupt fires in the halt loop, scheduler picks remaining
  tasks, system continues

**Invariant:** User exception handlers must never return to an unchanged
poisoned user IRET frame. A fatal user fault transitions the task to
Exited and redirects execution to a kernel-safe halt path with interrupts
enabled, allowing the scheduler to continue other PDs.

**Key files changed:**
| File | Change |
|------|--------|
| `kernel/src/interrupts.rs` | `faulted_task_halt()`, Exited state, IRET redirect on kill |
| `kernel/src/init.rs` | Removed temporary silk-shell spawn skip |
| `kernel/src/scheduler.rs` | Removed noisy bind_next traces |
| `kernel/src/syscalls/mod.rs` | Removed noisy syscall.exit trace, SYSCALL_LOG_BUDGET |
| `servers/silk-shell/src/main.rs` | WINDOWS[1] get_mut hardening, pre-create FOCUS_ID=2 |
| `dev.sh` | Removed `-display gtk` so QEMU window appears |

---

## Build Gate Status

- `./scripts/entrypoint_build.sh` — passes
- ISO packaging succeeds — produces `sexos-v1.0.0.iso`
- `xorriso` ISO is bootable under QEMU
- `abi_version_hash` is synced (last update: `bc536bdd6d1601c3e9bcc8c3386614230692c5eaa1cecd88a2c3bff2057322fe`)

---

## Next Work Queue

### A. Run runtime from clean master
Boot QEMU from clean master build to verify:
- All 6 PDs spawn
- Clock counts past 4s (the freeze boundary)
- SilkBar/clock stable for 30s+
- No serial fault lines

### B. Re-check silk-shell pointer safety fix
Verify whether `WINDOWS[1]` `get_mut` hardening + `FOCUS_ID=2` pre-creation
is already present in `servers/silk-shell/src/main.rs`. If not present in
master (cherry-pick needed from `debug/silkbar-delivery`), reapply cleanly.

### C. If a PD still dies, identify and fix userland
If a PD faults at runtime:
1. Identify which PD from serial log (`fault.kill` line)
2. Investigate root cause (null pointer, bad capability, etc.)
3. Fix userland code while containment keeps OS alive
4. Do not weaken containment to tolerate more faults — fix the cause

### D. Agent infrastructure docs
- `docs/INTERRUPTS_QUICKMAP.md` — full section index with line ranges
  for interrupts.rs (created on `debug/silkbar-delivery`, cherry-pick
  or recreate on master)
- Optional: Extract debug tracing helpers from `interrupts.rs` into
  `kernel/src/debug_trace.rs` (after active debugging is stable)

### E. WINDOWS → Shoji rename
Postpone until after crash debugging. Cosmetic rename of the window
management system. Not during active fault investigation.

---

## Agent Rules

### Interrupts Reading Discipline
**Do not read all of `kernel/src/interrupts.rs`.** It is large and every
agent that opens it wastes context budget. Instead:

```bash
rg "page_fault_handler|timer_interrupt|switch_to|faulted_task_halt|page_fault_stub|general_protection|send_eoi" kernel/src/interrupts.rs -n
```

Then open only the line ranges you need:
```bash
sed -n 'N,Mp' kernel/src/interrupts.rs   # N..M from rg output
```

**Key landmarks:**

| Range  | What |
|--------|------|
| 48–49  | IDT handler registration (page_fault, GPF, timer) |
| 131–293 | syscall_entry (naked asm) |
| 295–336 | page_fault_stub (naked asm — stack layout) |
| 337–360 | general_protection_fault_stub |
| 361–456 | timer_interrupt_stub + handler |
| 458–465 | faulted_task_halt |
| 466–618 | page_fault_handler |
| 620–725 | general_protection_fault_handler |

### Fish-Safe Command Rule
Pasteable command blocks must not contain bare expected-output lines.
Expected output must be comments:

```bash
# GOOD: expected output is a comment
echo "hello"
# → hello

# BAD: bare expected output that fish would try to parse
echo "hello"
hello
```

### General Discipline
- No mixed refactor+feature commits
- No kernel ABI changes unless explicitly approved
- No sex-pdx ABI changes unless explicitly approved
- No pointer IPC, shared memory, backing buffers in Track 1 phases
- Untracked `an/` directory is ignored unless explicitly requested
