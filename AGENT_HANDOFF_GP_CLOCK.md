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
