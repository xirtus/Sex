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

## H3: Framebuffer huge-page USER_ACCESSIBLE bug (2026-04-30)

Confirmed root cause via x86_64 crate source analysis:

1. `init.rs:137` calls `mapper.update_flags(Page<Size4KiB>)` on framebuffer.
2. Limine framebuffer is mapped as 2MiB huge pages (bootloader convention — avoids 512 PTEs).
3. `MappedPageTable::update_flags` for `Size4KiB` (`x86_64-0.15.4/.../mapped_page_table.rs:431-454`) walks `p4→p3→p2→p1`. At `next_table_mut(&mut p2[page.p2_index()])`, the PDE has HUGE_PAGE/PS=1, so it returns `Err(PageAlreadyMappedToHugePage)`, propagated as `Err(FlagUpdateError::ParentEntryHugePage)`.
4. `init.rs` swallows via `if let Ok(tlb)` — silent failure.
5. `USER_ACCESSIBLE` never gets set on the framebuffer mapping.
6. Ring-3 sexdisplay writes to supervisor-only pages → #GP.

### Why initial render with FALLBACK_PTR appears to work

`FALLBACK_PTR = 0xffff8000fd000000` is a hardcoded HHDM alias for physical `0xfd000000` (VGA LFB MMIO region). The HHDM direct map has different page table entries than the separate Limine framebuffer mapping. The initial render writes to VGA LFB memory (possibly wrong pixels or silently absorbed by MMIO) and doesn't crash. After the kernel provides the real Limine framebuffer address via `DisplayPrimaryFramebuffer` message, sexdisplay tries to write to the supervisor-only Limine mapping → #GP.

### PKRU traced correct

Full lifecycle trace: syscall entry/exit (saves/restores user PKRU), timer interrupts (enters God Mode), context switch (loads `TaskContext.pkru`), PD creation (stores `0xEFFFFFF0`). SexDisplay PKRU `0xEFFFFFF0` grants PKEY 0 = RW, matching the framebuffer PTE (PKEY=0). If USER_ACCESSIBLE were set, the write would succeed.

### Secondary bug: `tag_virtual_address` walks past PS bit

`pku.rs:tag_virtual_address` walks to PTE level without checking PS bit on PDE/PDPTE. If called on a huge page mapping, it would corrupt arbitrary page table entries by interpreting the terminal PDE's physical address field as a page table pointer. Currently only called after `map_to` (which creates 4KiB PTEs), so latent but dangerous.

### Fix strategy

Replace `mapper.update_flags(Page<Size4KiB>)` with a manual page-table walk that sets `USER_ACCESSIBLE|WRITABLE|PKEY` on the terminal entry at whatever level it exists (PDPTE for 1GiB, PDE for 2MiB, PTE for 4KiB). The walk follows the same pattern as `log_page_walk` in `memory/manager.rs` and `tag_virtual_address` in `pku.rs`, but checks PS at each level and stops at the terminal entry.

### Files to fix

1. `kernel/src/init.rs:131-141` — replace `update_flags` loop
2. `kernel/src/pku.rs:118-144` — add PS checks to `tag_virtual_address`
3. New helper in `kernel/src/pku.rs` or `memory/manager.rs` — `set_user_accessible(va, pkey)` that walks page table flags and sets U/S at terminal level

## Update 2026-04-30T19:00:00Z
- timestamp: 2026-04-30T19:00:00Z
- command run: ./scripts/entrypoint_build.sh
- finding: Applied Claude review safety fixes: (1) OP_PRIMARY_FB runtime path now calls redraw_clock_only instead of full render — eliminates same PKRU exposure on that arm. (2) handle_silkbar_update clamps hh<=23, mm<=59, ss<=59 — prevents FONT[digit] out-of-bounds panic on malformed SetClock.
- build result: SUCCESS — [SEXOS ENTRYPOINT] success
- files changed: servers/sexdisplay/src/main.rs
- proposed next action: Boot and confirm GP absent on both OP_SILKBAR_UPDATE and OP_PRIMARY_FB paths. Then track kernel PKRU restore fix separately for full-frame redraw.
