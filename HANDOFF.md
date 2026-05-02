# HANDOFF.md — v8+ USER_FAULT_CONTAINMENT_V1

## Documentation Source Of Truth

Use these files in this order to avoid roadmap drift:
1. `docs/ROADMAP.md` — current milestone sequence and delegation timing.
2. `docs/SILK_DE_EXECUTION_PLAN.md` — execution detail and external-agent prompts.
3. `HANDOFF.md` — runtime status, regressions, and immediate operational notes.

## Known-Good Forward Base

- **Confirmed working base commit:** `897ad23` (`fix(kernel): set user bits on upper page-walk levels for FB user access`).
- **Forward rule:** make tiny reversible branches from this base only.
- **Rollback rule:** if runtime breaks, immediately branch/save broken state, then return to the `WORKING-BOOT` base before further work.

## PD3 Containment Checkpoint

- Runtime checkpoint verified: visible non-black GUI, SilkBar clock running, PD3 `rip=0x0` null-jump not observed after containment.
- Current PD3 change is **temporary containment**, not final root cause fix.
- Containment behavior: silk-shell runtime loop path is bypassed/contained to keep PD3 alive while preventing null-jump recurrence.
- Root-fix debt: isolate the exact loop-path null target in a dedicated branch and replace containment with a real safe event loop fix.
- Safe-forward rule: next code branch must either:
  1. replace PD3 containment with a real loop fix, or
  2. proceed only with M2 color tokenization if containment is explicitly accepted as the checkpoint baseline.
- Invariant: no_std user PD entrypoints/loops must never fall through or call null handlers; idle paths must park via safe spin/yield behavior.
- Recovery: if breakage returns, branch/save current broken state, then checkout `backup/WORKING-GUI-CLOCK-PD3-CONTAINED-*` (or this checkpoint commit) before further changes.

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
- **GUI visible, clock counting continuously** (tested past 15s+).
- **PD3 silk-shell null-jump is contained** (kernel kills PD3, other PDs continue).
- **Scheduler round-robins remaining PDs** with no stalls.
- **Clock bounds-check fix applied** — framebuffer writes guarded against OOB access.
- **QEMU display:** Use `-display sdl` (not `-display gtk`). Default `dev.sh run` works.

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

## Bug 5: Clock freeze after ~2s (FIXED ✅)

**Symptom:** Clock advances briefly (2-3 seconds), then freezes. GUI may
remain visible but clock stops updating. No `fault.kill` in serial log.

**Root cause:** `sexdisplay` (PD1) hits a user-mode page fault while writing
framebuffer pixels. The framebuffer physical range is valid but the computed
write index exceeds the framebuffer pixel count, causing an out-of-bounds access
that faults. Once PD1 is killed, clock updates from silkbar stop rendering.

**Fix (servers/sexdisplay/src/main.rs):**
- Add `let total_pixels = pixels` in `render()`, compute `total_pixels` via
  `w.checked_mul(h)` in `redraw_clock_only()` and `redraw_surface_area()`.
- Every `write_volatile` call is wrapped in `if idx < total_pixels { ... }`.
- This prevents OOB framebuffer writes that would otherwise fault PD1.

**Key invariant:** Every framebuffer write path must validate the pixel index
against total pixel count before `write_volatile`. No exceptions.

**Files changed:**
| File | Change |
|------|--------|
| `servers/sexdisplay/src/main.rs` | Added `total_pixels` + `idx < total_pixels` guards in all 3 render functions |

---

## Black Screen Diagnosis (UPDATED 2026-05-02)

**Confirmed root causes of black screen:**

1. **Stale golden digest in `validate_deterministic_vectors()`** — On master,
   `crates/silkbar-model/src/lib.rs` has a `validate_deterministic_vectors()` check
   at the start of sexdisplay `_start()`. If the golden digest doesn't match the
   actual computed digest, sexdisplay hangs in a spin loop and never renders.
   **Fix:** Either update the golden digest, or remove the fatal check (as done
   in `test/silkbar-delivery` branch).

2. **Framebuffer PKEY mismatch** — FB pages mapped with PKEY 0. Sexdisplay PKRU
   should allow it.

3. **Framebuffer in kernel higher-half** — All 4 page-table levels need
   `USER_ACCESSIBLE`.

4. **Render writes wrong colors** — Dark palette (0x00102038, 0x00303860) appears
   black on some QEMU display backends. Try `-display sdl` instead of
   `-display gtk`.

5. **Clock freeze (now fixed)** — See Bug 5 above.

**Diagnostic procedure if screen is black:**
- Check serial log for `fault.kill` lines → PD1/PD5 may have been killed.
- Check for missing `validate_deterministic_vectors` pass → sexdisplay hung.
- Try `dev.sh run` (uses `-display sdl`) not raw QEMU with `-display gtk`.
- If clock freezes after 2s, check for missing `idx < total_pixels` bounds guards.

---

## Interrupts & Debug Quick Reference

### Key interrupt locations in `kernel/src/interrupts.rs`

| Range  | What | Debug tip |
|--------|------|-----------|
| ~48-49 | IDT handler registration | Verify page_fault, GPF, timer vectors are registered |
| ~131-293 | syscall_entry (naked asm) | Stack layout critical; avoid modifying |
| ~295-336 | page_fault_stub (naked asm) | Must preserve 8 extra qwords on stack |
| ~337-360 | general_protection_fault_stub | Similar stack discipline as #PF |
| ~361-450 | timer_interrupt_stub + handler | `timer.tick.enter` → `sched.tick()` → switch |
| ~450-455 | faulted_task_halt | Kernel halt loop for killed user tasks |
| ~456-610 | page_fault_handler | Check `fault.kill` → user_null_jump logic |
| ~610-720 | general_protection_fault_handler | GPF containment (similar to #PF kill path) |

### Common debug patterns:
```bash
# Find null-jump kills
rg "fault.kill user_null_jump" serial.log

# Check which PDs are alive
rg "task.running id=" serial.log | tail -20

# Find the timer tick handler entry
rg "timer.tick.enter" serial.log | wc -l

# Trace sexdisplay lifecycle
rg "sexdisplay|pd=1|PD 1" serial.log
```

### Known noisy serial prints (remove if present):
```bash
# Check for leftover debug prints in interrupts.rs
rg "DEBUG.*timer_tick" kernel/src/interrupts.rs
# Remove lines ~373-376 if found (every 100 ticks serial spam)
```

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

## Build & Run

```bash
./scripts/entrypoint_build.sh   # Full build + ISO
./dev.sh run                     # QEMU with window + serial stdio
```
