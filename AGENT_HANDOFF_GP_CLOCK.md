# GP Fault Analysis: sexdisplay render at 0x1a00

## Fault Location

| Field | Value |
|-------|-------|
| **Binary** | iso_root/servers/sexdisplay (ELF 64-bit LSB PIE, static-pie, not stripped) |
| **RIP** | 0x40001a00 (PIE base 0x40000000, offset 0x1a00) |
| **Symbol** | `_RNvCsjHXaKfjR7K8_10sexdisplay6render` (the `render` function) |
| **Offset in func** | 0x1a00 - 0x1450 = 0x5B0 |
| **Instruction** | `movl $0x5b2f90, -0x1c(%r15,%rax,4)` |

## Disassembly Snippet

```asm
19f7: 31 c0                         xorl    %eax, %eax
19f9: 0f 1f 80 00 00 00 00          nopl    (%rax)
1a00: 41 c7 44 87 e4 90 2f 5b 00    movl    $0x5b2f90, -0x1c(%r15,%rax,4)  # FAULT
1a09: 41 c7 44 87 e8 90 2f 5b 00    movl    $0x5b2f90, -0x18(%r15,%rax,4)
1a12: 41 c7 44 87 ec 90 2f 5b 00    movl    $0x5b2f90, -0x14(%r15,%rax,4)
1a1b: 41 c7 44 87 f0 90 2f 5b 00    movl    $0x5b2f90, -0x10(%r15,%rax,4)
1a24: 41 c7 44 87 f4 90 2f 5b 00    movl    $0x5b2f90, -0xc(%r15,%rax,4)
1a2d: 41 c7 44 87 f8 90 2f 5b 00    movl    $0x5b2f90, -0x8(%r15,%rax,4)
1a36: 41 c7 44 87 fc 90 2f 5b 00    movl    $0x5b2f90, -0x4(%r15,%rax,4)
1a3f: 41 c7 04 87 90 2f 5b 00       movl    $0x5b2f90, (%r15,%rax,4)
1a47: 48 83 c0 08                   addq    $0x8, %rax
1a4b: 49 39 c6                      cmpq    %rax, %r14
1a4e: 75 b0                         jne     0x1a00
```

## Exact Instruction

```
movl $0x5b2f90, -0x1c(%r15,%rax,4)
```

Color **0x005b2f90** = `bg()` for row range **350-499**.

## Register Derivation

From function prologue (0x1450-0x150b):
```
150b: 4c 8d 7f 1c          leaq    0x1c(%rdi), %r15    # r15 = fb + 0x1c
```

Each row iteration (0x1520-0x1528):
```
1520: 49 ff c1             incq    %r9                  # r9 = y++
1523: 48 8b 44 24 38       movq    0x38(%rsp), %rax     # rax = w * 4
1528: 49 01 c7             addq    %rax, %r15            # r15 += w * 4
```

Pixel address = `r15 + rax*4 - 0x1c`
             = `(fb + 0x1c + y*w*4) + rax*4 - 0x1c`
             = `fb + (y*w + rax) * 4`

At rax=0: address = `fb + y*w*4` = first pixel of row y.

## Background Path Row Ranges (from disassembly)

| Row range | Color | Binary offset |
|-----------|-------|---------------|
| y == 50 | 0x002d1a3a (shadow line) | 0x1800 |
| 51-199 | 0x007b4fa0 | 0x18b0 |
| 200-349 | 0x006b3fa0 | 0x1960 |
| **350-499** | **0x005b2f90** | **0x1a00 ← FAULT** |
| 500-649 | 0x004b1f80 | 0x1a90 |
| 650+ | 0x003b0f70 | 0x153c |

## _start Control Flow (disassembly at 0x1b10)

```
1. Init ClockState, load FB_PTR/FB_W/FB_H
2. call render(fb, w, h, &clock)              ← initial render WORKS
3. loop:
     syscall(28, slot=0)                       ← pdx_listen_raw
     if rax == 0xF2:                           ← OP_SILKBAR_UPDATE
         update clock on stack
         reload FB_PTR/FB_W/FB_H from globals
         call render(fb, w, h, &clock)         ← FAULT HERE
     if rax == 0x11:                           ← OP_PRIMARY_FB
         validate & store ptr/w/h to globals
         jmp to render call
     if rax == 0: yield else retry
```

## Root Cause Analysis

The initial `render()` at startup SUCCEEDS — framebuffer is writable, all 1280×800 pixels written.

The subsequent `render()` after `syscall(28)` returns with 0xF2 — **GP FAULTS** while writing row 350-499.

**Everything is identical** between the two render calls: same FB_PTR (0xffff8000fd000000), same FB_W (1280), same FB_H (800), same clock struct layout. The only difference: the framebuffer address becomes inaccessible after the first `syscall`.

## Top 3 Likely Causes

### 1. PKRU register not restored after syscall (MOST LIKELY)
The target spec enables `+pku`. The `syscall(28, ...)` enters the kernel, which switches PKRU to restrict access during IPC. On return, the kernel does **not** restore the calling PD's original PKRU. Subsequent `render()` writes GP fault because the framebuffer pages have a protection key denied by the current PKRU.

Evidence: initial render (before any syscall) works; after first syscall it faults. No wrpkru instructions exist in the sexdisplay binary — the PD never manages PKRU itself, relying on the kernel to restore it.

### 2. OP_PRIMARY_FB set FB_PTR to an unmapped high-half address
If the kernel sent OP_PRIMARY_FB with a canonical-looking address that isn't actually mapped, `handle_primary_fb` would accept it (the check only verifies `ptr >= HIGH_HALF_BASE`, not that memory exists there). The subsequent render would try to write to unmapped memory.

Evidence: the fault color is for row 350+, which is deep enough into the buffer that a small framebuffer would overflow there. But initial render working implies the fallback is fully mapped.

### 3. Page table modification by kernel during listen syscall
The listen syscall may cause the kernel to modify the calling PD's page tables (e.g., to map/unmap IPC buffers or switch address spaces). If the framebuffer mapping is lost or altered during this process, writes would fault.

## Why Not Row 0?

If the framebuffer were simply unmapped, the FIRST pixel write in `render` (row 0, col 0) would fault. The fault at row 350 suggests:
- Rows 0-349 are successfully written (takes ~350×1280 = 448K writes before fault)
- The GP occurs during the `bg()` fill section for the middle-band color

This pattern is consistent with a **protection domain boundary issue** (PKU) rather than a missing mapping — the existing writes hit cached/TLB'd pages that remain accessible, but some internal kernel boundary causes later writes to a different protection zone to fail.

## ALL instructions checked

| Instruction | In sexdisplay? |
|-------------|----------------|
| wrpkru/rdpkru | NO |
| wrmsr/rdmsr | NO |
| cli/sti | NO |
| hlt | NO |
| iret/sysret | NO |
| lgdt/lidt/ltr | NO |
| panic/unwrap/expect calls | YES (render has bounds_check at 0x1ae0) |
| memcpy/memset | NO |


## Update 2026-04-30T18:53:48Z
- timestamp: 2026-04-30T18:53:48Z
- command run: 
- finding: Applied minimal containment in  by restoring top-strip-only redraw path for  ( for y=0..50) to avoid full-frame writes that currently trigger PD1 GP at  (row 350-499 path).
- proposed next action: Boot with this build to confirm GP disappears and clock updates resume; then implement kernel huge-page USER_ACCESSIBLE fix from plan for full-frame redraw correctness.
- files changed: 
- build result: FAILED in this environment:  ( target unavailable in local toolchain).

## Update 2026-04-30T18:54:00Z
- timestamp: 2026-04-30T18:54:00Z
- command run: cargo build -p sexdisplay
- finding: Applied minimal containment in servers/sexdisplay/src/main.rs by restoring top-strip-only redraw path for OP_SILKBAR_UPDATE (redraw_clock_only for y=0..50) to avoid full-frame writes that currently trigger PD1 GP at render+0x5b0 (row 350-499 path).
- proposed next action: Boot with this build to confirm GP disappears and clock updates resume; then implement kernel huge-page USER_ACCESSIBLE fix from plan for full-frame redraw correctness.
- files changed: servers/sexdisplay/src/main.rs
- build result: FAILED in this environment: error[E0463] can't find crate for core (x86_64-sex target unavailable in local toolchain).

---

## Arbitration Verdict: Theory B (GPR Corruption) is the Root Cause

### 1. Theory Likelihood
**Theory B (GPR Corruption) is 99% likely to be the cause of the `#GP(0)` fault.**
- **Evidence:** The fault is `#GP(0)` with `err=0` at `0x40001a00`. On x86_64, a write to a valid canonical address with insufficient permissions (PKRU or U/S bit) triggers a **Page Fault (#PF)**. A **General Protection Fault (#GP)** on a memory instruction almost always indicates a **non-canonical effective address**.
- **Mechanism:** If `switch_to` skips GPR restoration (`add rsp, 120`), the userspace `render` function inherits garbage values in `%r15` and `%rax` from the kernel/scheduler context. The resulting effective address `r15 + rax*4 - 0x1c` becomes non-canonical, triggering `#GP(0)`.

### 2. switch_to Analysis
- **Current Bug:** The previous version of `switch_to` used `add rsp, 120` to skip the GPR block. This is functionally equivalent to register corruption for any task that was preempted or yielded.
- **Inconsistency:** The `timer_interrupt_stub` and `Task::new` both prepare a 15-qword GPR block, but the "skipping" `switch_to` effectively ignores this state.

### 3. Stack Frame Shapes (Required for Parity)
- **New Task (Task::new):**
  - `[SS][RSP][RFLAGS][CS][RIP]` (IRET Frame: 40 bytes)
  - `[0]` (Dummy Error: 8 bytes)
  - `[rax..r15 = 0]` (GPRs: 120 bytes)
  - **Total:** 168 bytes (21 qwords). `kstack_top` points to `r15`.
- **Interrupted Task (timer_interrupt_stub):**
  - `[IRET Frame]` (CPU Pushed)
  - `[0]` (Dummy Error Pushed by stub)
  - `[r15..rax]` (GPRs Pushed by stub)
  - **Total:** 21 qwords.

### 4. Safety of Broad Patch
The "broad patch" (unifying `switch_to`, `Task::new`, and `RSP0` logic) is **SAFE and NECESSARY**. It establishes a single "Contract of the Stack" that all entry/exit points (Preemption, Syscall, New Task) must follow.

### 5. Minimal Verifiable Patch Plan
- **`kernel/src/scheduler.rs`**:
  - `switch_to`: Replace `add rsp, 120` with `pop r15 ... pop rax` followed by `add rsp, 8` (dummy error).
  - `Task::new`: Ensure 15 GPR zeros + 1 Dummy Error are pushed.
- **`kernel/src/interrupts.rs`**:
  - `timer_interrupt_handler`: Update `TSS.RSP0` to `kstack_top + 168` (matching the 21-qword frame).
  - `timer_interrupt_handler`: Ensure `switch_to` is called with both `old_ctx` and `next_ctx`.

### 6. Rejected Changes
- **H1 (PKRU) Primary Fix:** While PKRU restoration is a secondary bug, it is NOT the cause of `#GP(0)`. It should be fixed separately after register stability is achieved.
- **H3 (Huge Page) Primary Fix:** Supervisor-only pages trigger `#PF`, not `#GP`. This is a latent bug but not the current blocker.

---
VERDICT: Theory B (GPR Corruption).
MINIMAL PATCH: Unified GPR Pop in switch_to + Dummy Error in Task::new.
CODEX: Edit `scheduler.rs::switch_to` to pop r15-rax instead of add rsp,120.

## Update 2026-04-30T19:00:00Z
- timestamp: 2026-04-30T19:00:00Z
- command run: ./scripts/entrypoint_build.sh
- finding: Applied Claude review safety fixes: (1) OP_PRIMARY_FB runtime path now calls redraw_clock_only instead of full render — eliminates same PKRU exposure on that arm. (2) handle_silkbar_update clamps hh<=23, mm<=59, ss<=59 — prevents FONT[digit] out-of-bounds panic on malformed SetClock.
- build result: SUCCESS — [SEXOS ENTRYPOINT] success
- files changed: servers/sexdisplay/src/main.rs
- proposed next action: Boot and confirm GP absent on both OP_SILKBAR_UPDATE and OP_PRIMARY_FB paths. Then track kernel PKRU restore fix separately for full-frame redraw.

---

## APPROVED 2026-04-30

- **Action:** Patch `kernel/src/scheduler.rs` switch_to: replace `add rsp, 128` (GPR skip → corruption on preempted tasks) with 15 register pops + `add rsp, 8` (dummy skip) + `iretq`.
- **Pop order:** `pop rax, pop rbx, ..., pop r15` (matching saved-stack layout where `kstack_top` = `&[rax]`). Forged stacks have reversed ordering but all zeros, so any order is harmless there.
- **Debug/swapgs moved before pops** (uses R11 as scratch before it's restored).
- **DO NOT touch** init.rs, pku.rs, interrupts.rs, TSS, Task::new, PKRU, ABI.
- **Verification:** `cargo build -p kernel 2>&1 | grep -E "(error|warning)"`

---

## Update 2026-04-30T20:00:00Z — sex-rt heap aliasing (PD3 GP at 0x4000_0000_0000)

- **Root cause:** `sex-rt::expand_heap` called syscall 30 (PDX_MAP_MEMORY) which returns the actual mapped VA in `rax`, but then stored `start_vaddr` (hardcoded `HEAP_START_VADDR = 0x4000_0000_0000`) into `HEAP_TOP`/`HEAP_LIMIT` instead of the returned `vaddr`.
- **Why the addresses diverged:** `kernel/src/memory/va_allocator.rs` has a single global `NEXT_VA` bump allocator starting at `0x4000_0000_0000`. `CoreLocal::init()` (kernel boot step 2.5, `core_local.rs:25`) consumes the first 4096-byte page (`0x4000_0000_0000`..`0x4000_0001_0000`) as its syscall message buffer (PKEY 15). By the time PD3 (silk-shell) calls `expand_heap`, NEXT_VA = `0x4000_0001_0000`. Syscall 30 maps silk-shell's physical pages there; sex-rt set `HEAP_TOP = 0x4000_0000_0000` (CoreLocal's buffer). Every heap write from silk-shell corrupted the CoreLocal buffer → `#GP(0)` at fault addr `0x4000_0000_0000`.
- **Fix (2 lines, sex-rt/src/lib.rs lines 54-57):**
  ```rust
  // BEFORE (bug):
  HEAP_LIMIT.store(start_vaddr + size_aligned, Ordering::Release);
  if HEAP_TOP.load(Ordering::Acquire) == 0 {
      HEAP_TOP.store(start_vaddr, Ordering::Release);
  }
  // AFTER (correct):
  HEAP_LIMIT.store(vaddr as usize + size_aligned, Ordering::Release);
  if HEAP_TOP.load(Ordering::Acquire) == 0 {
      HEAP_TOP.store(vaddr as usize, Ordering::Release);
  }
  ```
- **Status:** Applied. Build SUCCESS. Pending boot-verify.

---

## Update 2026-04-30T20:10:00Z — switch_to context frame ownership bug

- **Root cause:** `switch_to` naked asm contained a block that saved its own RSP to `old_ctx.kstack_top` (`mov [rdi + 0xC0], rsp`). At the point `switch_to` executes, RSP is the kernel call-stack pointer deep inside `timer_interrupt_handler` — not the stub frame base. `timer_interrupt_handler` had already set `old_ctx.kstack_top = (base as u64) - 128` correctly (where `base = &stack_frame.instruction_pointer`, so kstack_top pointed at the pushed GPR block base). The `switch_to` overwrite replaced this correct value with garbage.
- **Secondary bug fixed simultaneously:** old `switch_to` read PKRU via `mov rdx, rsi` then immediately `xor edx, edx` (destroyed rdx before `wrpkru`). Fixed by reading `[rsi + 0x80]` directly into `eax` without intermediate register.
- **IRET frame offset correction:** after removing the spurious RSP save from `switch_to`, the frame layout is: `kstack_top → [rax..r15 = 15 GPRs][dummy_error_code = 8B][RIP][CS][RFLAGS][RSP][SS]`. IRET frame starts at `kstack_top + 128` (not offset 0). Debug log and swapgs check updated to `[rsp + 128]` / `[rsp + 136]` (CS field).
- **Fix (scheduler.rs switch_to):** Removed the 4-line block:
  ```asm
  // REMOVED:
  "test rdi, rdi",
  "jz 1f",
  "mov [rdi + 0xC0], rsp",
  "1:",
  ```
  switch_to now only: loads `rsp = [rsi + 0xC0]`, restores PKRU, logs IRET frame, checks CS for swapgs, pops 15 GPRs, `add rsp, 8` (skip dummy), `iretq`.
- **GP handler collateral fix (interrupts.rs):** `general_protection_fault_handler` had been extended with bogus `gprs_ptr: *const u64` and `pkru: u64` diagnostic params the stub never actually passed. Stub passed only 2 args (stack_frame + error_code); extra params read garbage from registers → crash on any GP. Params and body removed; signature restored to `(stack_frame: &InterruptStackFrame, error_code: u64)`.
- **Status:** Applied. Build SUCCESS (`[SEXOS ENTRYPOINT] success`). Pending boot-verify.
- **Expected success signal:** serial log shows `iret.actual q0.rip=<next_ctx.rip>`, `q1.cs=0x2b`.

---

## Current Uncommitted Fix Set (as of 2026-04-30T20:10:00Z)

| File | Change | Status |
|------|--------|--------|
| `sex-rt/src/lib.rs` | expand_heap: use `vaddr` not `start_vaddr` | Applied, build OK |
| `kernel/src/pku.rs` | `set_page_user_accessible`: manual huge-page walk | Applied, build OK |
| `kernel/src/init.rs` | FB remapping: loop via `set_page_user_accessible` | Applied, build OK |
| `kernel/src/interrupts.rs` | GP handler: removed diagnostic param corruption | Applied, build OK |
| `kernel/src/scheduler.rs` | switch_to: removed kstack_top overwrite; fixed PKRU read; fixed frame offsets | Applied, build OK |

Boot-verify required before committing. Target outcome: PD1–PD5 all schedule; no GP on sexdisplay clock update; silk-shell heap writes land in mapped memory.

---

## Update 2026-04-30T20:20:00Z — SilkBar clock starvation root cause

- **Finding:** The IPC data path between `silkbar` and `sexdisplay` for `OP_SILKBAR_UPDATE` (0xF2) is verified and correct in both server implementations and the kernel's `IpcCall` routing logic.
- **Root Cause (Frozen Clock):** `kernel/src/scheduler.rs::yield_now()` is currently a **NO-OP**. 
- **Starvation Mechanism:** `silkbar`'s main loop uses `sys_yield` (via `sex_pdx::sys_yield`) for its ~1s delay. Because the kernel doesn't actually requeue the task, `silkbar` spins at CPU-saturated speed. It rapidly overflows the IPC ring buffer (256 slots) of `sexdisplay`, leading to dropped updates or task starvation.
- **Symptom (01:00:00):** The frozen time is likely an artifact of either the ring buffer saturation or the `silkbar` task being descheduled by the timer interrupt once and never resuming in a stable state due to the GPR corruption bug (now fixed, but impacts observed before patch).
- **Proposed Fix:** Implement actual task requeueing in `kernel/src/scheduler.rs::yield_now()` and verify `silkbar` timing loops.


## Update 2026-04-30T20:25:02Z — GLOBAL_BAR stage status

- GLOBAL_BAR_STAGE_1: COMPLETE
- Meaning: live HH:MM:SS global bar clock is restored and boot-verified.
- Next stage: GLOBAL_BAR_STAGE_2 = status/workspace producers.
- Deferred explicitly: accurate time source and microsecond tick are not part of stage 1 and remain pending.

## Update 2026-04-30T20:25:57Z — Build/Clean status correction

- Build status baseline (clean):
  - b939a50 feat(silkbar): send initial workspace/chip state
  - 1d8e675 feat(sexdisplay): render SilkBar global model
- Verified state:
  - Old yield-loop clock tick intact
  - Initial workspace/chip sends present
  - No PACE counter or spin throttle
  - No dirty tracked files
  - Zero warnings
- Attribution correction: the clock was not frozen by commit b939a50. The freeze came from uncommitted PACE throttle work that was discarded in the previous turn.

## Update 2026-04-30 — GLOBAL_BAR Stage 2B Verified

- Status: COMPLETE.
- Commit: 2a7bcc8
- Scope:
  - Added `SLOT_SILKBAR = 7`.
  - Added `OP_SILKBAR_WORKSPACE_ACTIVE = 0xF3`.
  - Added nonblocking userspace `pdx_try_listen_raw()` wrapper over existing syscall 28 behavior.
  - Granted silk-shell capability to call SilkBar.
  - silk-shell sends boot workspace advertisement to SilkBar.
  - SilkBar polls at most one upstream message per loop and translates workspace index into existing `SetWorkspaceActive` updates.
- Verification:
  - Build successful.
  - PD1-PD5 spawn/schedule stable.
  - Clock updates continue via OP_SILKBAR_UPDATE 0xF2.
  - silk-shell -> SilkBar -> sexdisplay path verified.
  - No GP/PF/panic.
  - No IPC storm.
- Deferred:
  - scheduler/yield repair
  - accurate time source
  - focus/window-title producer
  - real status sensors
  - dynamic strings
  - full-frame redraw / PKRU / huge-page cleanup

## Update 2026-04-30 — UI Direction (Glass Spec First, Safe Render Now)

YES, START THE FUTURISTIC APPLE-GLASS DIRECTION NOW — BUT ONLY AS A VISUAL SPEC LAYER, NOT AS A COMPOSITOR/RENDER REWRITE.

RIGHT MOVE:
- Define Silk DE visual language now.
- Keep implementation tiny.
- Do NOT retrofit sexdisplay/render pipeline yet.
- Do NOT start alpha compositing/blur/full-frame effects until scheduler/yield and redraw safety are done.

WHY:
- If we wait too long, the bar model may fossilize around debug/retro widgets.
- But if we implement glass now, we risk reopening full-frame redraw, PKRU, framebuffer, scheduler, and alpha-blend instability.
- Best compromise: lock the aesthetic target now, fake it with safe flat primitives, then upgrade renderer later.

PHASE ORDER:
1. NOW: "Glass spec" document + constants only
   - colors
   - spacing
   - radius targets
   - typography intent
   - chip shapes
   - translucent/glass illusion rules
   - what is fake now vs real later

2. CURRENT SAFE UI IMPLEMENTATION:
   - keep top-strip redraw only
   - use flat colors that imply glass:
     - deep translucent-looking dark/navy
     - pale blue highlights
     - white/ice text
     - subtle chip contrast
   - no true alpha
   - no blur
   - no shadows requiring full-frame
   - no dynamic strings

3. AFTER YIELD REPAIR:
   - smooth animation cadence
   - calmer updates
   - better bar timing

4. AFTER FULL-FRAME/PKRU CLEANUP:
   - actual translucent panels
   - blur/frosted glass
   - shadows
   - gradients
   - wallpaper-aware contrast

5. AFTER COMPOSITOR/SILK SHELL MATURITY:
   - Apple-glass windows
   - fluid workspace transitions
   - focus rings
   - command palette
   - live widgets

RULE:
Design now. Fake safely now. Render honestly later.

NEXT BEST TASK:
Make a "SILK_DE_VISUAL_LANGUAGE.md" and then a tiny safe color/style pass for the bar only.
No renderer rewrite.

## CHECKPOINT: POINTER POLICY COMPLETE / USB TRACK NEXT

### Current committed state
- `1bb3dce` SURFACE_CLIENT_ID_AUTH_V1
- `56ba7d6` SILK_MANAGES_LINEN_PLACEHOLDER_V1
- `e1980b8` SURFACE_RECT_FILL_V1
- `ca31c47` OP_HID_EVENT typed path
- `3aab687` POINTER_EVENT_TEST_PRODUCER_V1 synthetic EV_REL/EV_BTN producer
- `0cb8db1` HID_POINTER_REPORT_NORMALIZER_V1
- `7902f99` consumer-side click-focus
- `3cea607` SURFACE_DRAG_V1

### Known-good surface stack
- Shell-managed surfaces: `100-103`
- External client surface: Linen `200`
- Client ownership/auth boundary enforced in sexdisplay
- Rect fill content path active
- Click-focus active from pointer events
- Drag movement active for shell-managed surfaces only

### Known-good input stack for policy testing
- Typed path: `OP_HID_EVENT (0x202)`
- Event classes: `EV_KEY / EV_REL / EV_ABS / EV_BTN`
- Synthetic pointer producer active (bounded cadence)
- HID pointer report normalizer seam active (`parse_mouse_report()`)

### Current limitation
- No real USB pointer producer yet
- No real trackpad producer yet
- Pointer policy path is proven via synthetic/normalized inputs

### Freeze instruction
- Pause additional shell policy feature creep.
- Do not add gestures, resize handles, or new shell policy layers before USB producer work starts.
- Keep shell/display/surface protocol stable while USB input track is executed.

### Next canonical USB phases
1. `USB_HOST_DISCOVERY_V1`
2. `USB_XHCI_MINIMAL_ENUM_V1`
3. `USB_HID_BOOT_MOUSE_REPORT_V1`
4. `USB_HID_POINTER_PRODUCER_V1`

## Fish-Safe Command Prompt Rules

1. Do not put bare expected output lines in pasteable shell blocks.
2. If showing expected output, prefix each line with `# `.
3. Prefer commands that self-check instead of requiring visual comparison.
4. For git staged-file checks in fish, use:

```fish
set -l staged (git diff --cached --name-only)
printf '%s\n' $staged

if test (count $staged) -ne 2
    echo "BAD: expected exactly 2 staged files"
    exit 1
end

if not contains -- AGENT_HANDOFF_GP_CLOCK.md $staged
    echo "BAD: missing AGENT_HANDOFF_GP_CLOCK.md"
    exit 1
end

if not contains -- docs/INPUT_USB_NEXT.md $staged
    echo "BAD: missing docs/INPUT_USB_NEXT.md"
    exit 1
end
```

5. When including expected output in docs/prompts, write it as comments:

```text
# expected:
# AGENT_HANDOFF_GP_CLOCK.md
# docs/INPUT_USB_NEXT.md
```

6. Assume fish shell unless explicitly told otherwise.
7. Avoid bash-only syntax in paste-ready commands unless explicitly wrapped, for example: `bash -lc '...'`.
