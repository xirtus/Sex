# HANDOFF.md — v8+ USER_FAULT_CONTAINMENT_V1

## 2026-05-02 Clock Freeze Regression (FIXED ✅)

**Symptom:** Clock advanced briefly (~2 seconds) then froze after merging `debug/silkbar-delivery` into `master`.

**Root cause:** `sexdisplay` (PD1) hit a user-mode page fault while writing framebuffer pixels in redraw paths. Once PD1 died, clock updates from `silkbar` no longer appeared even though scheduler kept running other PDs.

**Fix:** Added strict per-pixel framebuffer index bounds checks (`idx < w*h`) in all redraw/write loops.

**Files changed:**
| File | Change |
|------|--------|
| `servers/sexdisplay/src/main.rs` | Added write guards in `render`, `redraw_clock_only`, and `redraw_surface_area` |

**Regression guard (must keep):** Any framebuffer write path in userland renderers must validate write index against total pixel count before `write_volatile`.

## Current Runtime State ✅

- **All 6 PDs spawn and run** (sexdisplay, sexdrive, silk-shell, sexinput, silkbar, linen).
- **Zero page faults, zero panics** after fix (was infinite RIP=0x0 loop).
- **Scheduler round-robins all 6 PDs** with no stalls.
- **Clock counts in SilkBar** (no freeze after PD fault).
- **QEMU window appears** (removed `-display gtk` override).
- **Known: screen may still appear black** — see diagnosis below.

---

## Bug 4: User null-jump at RIP=0x0 freezes scheduler (FIXED ✅)

**USER_FAULT_CONTAINMENT_V1** — `kernel/src/interrupts.rs`

**Root cause:** When a user task faulted at RIP=0x0, the page fault handler
set the task to STATE_BLOCKED and returned via iretq — which loaded the
unchanged poisoned IRET frame (RIP=0x0), causing an immediate re-fault.
Interrupts are disabled during exception handling (IF cleared by #PF), so
the timer could never fire to preempt the loop. The system appeared frozen.

**Fix:**
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

**Files changed:**
| File | Change |
|------|--------|
| `kernel/src/interrupts.rs` | `faulted_task_halt()`, Exited state, IRET redirect on kill |
| `kernel/src/init.rs` | Removed temporary silk-shell spawn skip |
| `kernel/src/scheduler.rs` | Removed noisy bind_next traces |
| `kernel/src/syscalls/mod.rs` | Removed noisy syscall.exit trace, SYSCALL_LOG_BUDGET |
| `servers/silk-shell/src/main.rs` | WINDOWS[1] get_mut hardening, pre-create FOCUS_ID=2 |
| `dev.sh` | Removed `-display gtk` so QEMU window appears |
| `sexos_build_spec.toml` | Updated abi_version_hash |

---

## Agent Infrastructure (Committed)

### CLAUDE.md — Interrupts Reading Discipline
Every future agent is instructed to NOT read `interrupts.rs` whole. Use `rg` + `sed -n 'N,Mp'` for targeted reads.

### docs/INTERRUPTS_QUICKMAP.md
Full section index with line ranges, critical invariants (poisoned IRET, stub stack layout, switch_to contract), and `rg` patterns for common debug entry points.

### AGENTS.md
Added explicit "Interrupts Discipline" section with landmark table and `rg` cheat sheet.

### docs/manual_sex.md
USER_FAULT_CONTAINMENT_V1 section added with root cause, fix, and invariant.

---

## Black Screen Diagnosis (not yet fixed)

Sexdisplay DOES receive OP_PRIMARY_FB and DOES render. QEMU window appears but may be black.

1. **Framebuffer PKEY mismatch** — FB pages mapped with PKEY 0. Sexdisplay PKRU should allow it.
2. **Framebuffer in kernel higher-half** — All 4 page-table levels need USER_ACCESSIBLE.
3. **Render writes wrong colors** — Dark palette appears black on some QEMU configs.
4. **CLOCK STILL FREEZES after 4s** — If clock freezes, check `fault.kill` in serial log. If RIP=0x0, containment fired. If no faults, investigate sexdisplay render loop.

---

## Build & Run

```bash
./scripts/entrypoint_build.sh   # Full build + ISO
./dev.sh run                     # QEMU with window + serial stdio
```
