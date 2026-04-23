# SexOS Microkernel — Claude Session Memory

This file is read automatically by Claude Code at session start.
It encodes project invariants, ABI contracts, and debugging history.
**Never delete or contradict entries here without updating the date.**

---

## Standing Orders for Claude Code Sessions

### Token Discipline
- **Read files before searching.** If the answer is likely in a source file already
  known from this document, read that file directly. Do not web search for things
  that are defined in this codebase.
- **No speculative reads.** Only open files directly relevant to the current task.
  Do not read files "just in case."
- **No redundant builds.** Do not run `cargo build` more than once per fix unless
  the first build produced an unexpected error requiring re-diagnosis.
- **Prefer targeted edits.** Use `str_replace` on the exact lines that need changing.
  Do not rewrite whole files to fix a 2-line bug.
- **State assumptions explicitly.** If unsure about something, say so and ask rather
  than searching or reading multiple files to guess.

### Self-Update Rule (CRITICAL)
When you discover something that was **blocking progress** — a wrong assumption, a
missing invariant, a bug root cause, a correct ABI detail — update this file before
ending the session:
- Add confirmed bug root causes to "Known Fixed Bugs" once fixed
- Update "Current Status" to reflect what changed
- Add new ABI facts, memory layout details, or invariants to the relevant section
- Remove resolved items from debugging checklists

This file is the only persistent memory across sessions. If it is not updated,
the next session starts blind and wastes tokens re-discovering the same things.

---

## Project Overview

SexOS is a SASOS (Single Address Space OS) microkernel written in Rust targeting
`x86_64-unknown-none`. It uses Limine as its bootloader and QEMU for development.

**Build pipeline:**
```
./build_payload.sh && make iso && make run-sasos
```
QEMU flags: `-M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio`

**Workspace layout:**
```
kernel/          — sex-kernel crate (ring 0)
servers/
  sexdisplay/    — framebuffer/compositor server (PDX)
  silk-shell/    — shell server (PDX)
apps/
  linen/         — first userland app (PDX)
crates/
  sex-pdx/       — shared PDX calling convention crate
```

**Cargo resolver:** workspace uses resolver = "1" (edition 2021 members exist but
workspace Cargo.toml has not been updated). Do not change this without testing.

---

## Memory Layout (SASOS Map)

All components share one PML4. Isolation is via PKU keys, not address separation.

| Component          | Virtual Address          | Notes |
|--------------------|--------------------------|-------|
| Kernel binary      | Higher-half (linker.ld)  | `no_std` core + Limine requests |
| Userland stubs     | `0x4000_0000`            | Mock entry point for `sex-ld` library mapping |
| Translated native  | `0x4000_1000`            | Target entry for translated ELFs via `sexnode` |
| System heap        | `0x4444_4444_0000`       | 128 MiB (HEAP_SIZE in lib.rs), mapped at boot |
| sexdisplay FB      | Dynamic (passed via IPC) | Framebuffer pages tagged PKEY 1, handed over at spawn |

- HHDM offset: `0xffff800000000000`
- All userland segments mapped via `GlobalVas::map_pku_range` which applies PD key
  to Level-1 page table entries
- Page tables: `OffsetPageTable` via x86_64 crate
- PD structs live on system heap at `0x4444_4444_0000`
- PD load bases: `0x2000_0000 + (pku_key * 0x20_0000)` — sexdisplay at `0x2020_0000`
- User stacks: `0x7000_0000_0000 + (pku_key * 0x100_0000)`, 64KB each

---

## Protection Keys (PKU/MPK) — CRITICAL

PKU is enabled in CR4. Every PDX domain has an assigned PKEY.

**Known PKEY assignments (from init.rs):**
| PDX          | PKEY | PKRU value (computed)      |
|--------------|------|----------------------------|
| sexdisplay   | 1    | `0x3FFF_FFF0` (allows 0,1,15) |
| silk-shell   | 2    | `0x3FFF_FFCC` (allows 0,2,15) |
| linen        | 3    | `0x3FFF_FF0F` (allows 0,3,15) — NEXT_PKEY starts at 2, linen gets 3 |

**PKRU formula (from ProtectionDomain::new):**
```rust
let mut pkru_mask: u32 = 0xFFFF_FFFF;
pkru_mask &= !(0b11 << (pku_key * 2));  // allow own key
pkru_mask &= !0b11;                      // allow PKEY 0 (kernel/default)
pkru_mask &= !(0b11 << 30);             // allow PKEY 15 (shared IPC)
```
Kernel heap (PKEY 0) is accessible from ALL PKRUs. No wrpkru bug in switch_to.

**PKRU policy:**
- Kernel entry (syscall/interrupt): `xor eax,eax; xor edx,edx; xor ecx,ecx; wrpkru`
  (in both syscall_entry and timer_interrupt_stub — opens ALL keys).
- **Never use** `core::arch::x86_64::_wrpkru` directly — use `crate::pku::wrpkru`.
- `kernel/src/memory/pku.rs` was deleted. The only PKU file is `kernel/src/pku.rs`.
- `serial_println!` in sex-pdx uses **direct asm `syscall` with rax=69** — NOT
  a null deref, NOT a bridge call. Kernel dispatch handles rax=69 natively.

---

## Syscall ABI

- Entry via `SYSCALL`/`SYSRET` (LSTAR/STAR configured in GDT init)
- SFMask clears IF on SYSCALL → interrupts disabled throughout syscall handler
- Syscall number in rax; arguments: rdi=slot, rsi=opcode, rdx=arg0, r10=arg1, r8=arg2
- **CRITICAL: `syscall_entry` `pop rax` restores original rax (syscall number), NOT
  the handler return value.** Kernel modifies `regs.rax` via the regs pointer if it
  wants to return a value to userland. Simply returning from `dispatch()` does NOT
  set userland rax — you must write `regs.rax = value` before returning.
  - Current dispatch() returns u64 but it's DISCARDED by `pop rax`. Every syscall
    that needs to return a value must do `regs.rax = result` explicitly.
  - `pdx_call(0, 0x03, ...)` currently returns 27 (syscall number) instead of 0,
    causing sexdisplay to think DisplayInfo query failed.
- **Never reference `opcode` in syscalls/mod.rs** — it does not exist. Use `num`
  (bound from rsi at dispatch entry via `let rsi = regs.rsi`).
- Arguments in dispatch: `let rdi=regs.rdi; let rsi=regs.rsi; ...`

**SyscallRegs layout (kernel/src/interrupts.rs):**
```
Push order: r11,rcx (1st save), r9,r8,r10,rdx,rsi,rdi,rax (SyscallRegs)
Memory:     [rsp+0]=rax, [+8]=rdi, [+16]=rsi, [+24]=rdx, [+32]=r10,
            [+40]=r8, [+48]=r9, [+56]=rcx, [+64]=r11
Then:       r15..rbp (callee-saved), then rax=pkru
sysretq:    rcx→RIP, r11→RFLAGS (restored from 2nd pop of first-save copies)
```

---

## PDX (Protection Domain eXtension) ABI

### Calling Convention (Phase 21+ standardized, 5-argument arity)

```rust
pdx_call(slot: u32, syscall: u64, arg0: u64, arg1: u64, arg2: u64) -> u64
```

**Register mapping:**
| Argument | Register | Notes |
|----------|----------|-------|
| slot     | rdi      | Capability slot index |
| syscall  | rsi      | Opcode (bound as rsi in dispatch) |
| arg0     | rdx      | |
| arg1     | r10      | |
| arg2     | r8       | |

- **Never use 4-argument arity** — causes stack misalignment on `sysretq`/`iretq`.
- `serial_println!` in PDX context: uses direct `syscall` with rax=69. **NOT** a
  null deref. Kernel handles rax=69 at top-level dispatch AND at slot=0/num=69.

### IPC Slot Convention

| Slot | Service |
|------|---------|
| 0    | Kernel direct (handled inline in dispatch, no safe_pdx_call) |
| 1    | Primary system service (`sexfiles` VFS) |
| 2    | Secondary (`sext` allocator or `sexnode`) |
| 4    | Network manager (`sexnet`) |
| 5    | Compositor / display server (`sexdisplay`) |

### Capability Table Structure

Each `ProtectionDomain` struct contains a `CapabilityTable` with `CapabilityData` entries.
Cap table is accessed via raw pointer in `init.rs` — `unsafe` is intentional.

`init.rs` inserts `CapabilityData::Domain(sexdisp_id)` at slot 5 for PDs 1..=4.

---

## Scheduler — CONFIRMED BUGS (2026-04-23)

- Round-robin via `WorkStealingQueue`. Uses `steal()` on local queue (should be
  `pop()` but functionally identical on single core).
- Timer IRQ fires → `timer_interrupt_stub` → `timer_interrupt_handler`.

### BUG 1 (CRITICAL — causes kernel panic on any pdx_listen/safe_pdx_call):
**`current_pd_id` is NEVER updated by the scheduler.**
`set_pd()` is only called from `jump_to_userland()` which is NEVER called (dead code).
`current_pd_id` stays 0 forever. Any call path that hits `CoreLocal::current_pd_ref()`
(syscall 28 = pdx_listen, `safe_pdx_call` for slot>0) does:
```rust
DOMAIN_REGISTRY.get(0)  // domains[0] is null — PDs start at ID 1
    .expect("CoreLocal: Current PD lost")  // KERNEL PANIC
```
**Fix:** In `timer_interrupt_handler`, after `sched.tick()` returns `(old, next)`,
add `crate::core_local::CoreLocal::get().set_pd(unsafe { (*next_ctx_ptr).pd_id });`
before calling `switch_to`.

### BUG 2 (CRITICAL — corrupts callee-saved registers on context restore):
**`switch_to` saves KERNEL callee-saved registers into old task context, not user's.**
When timer fires from userland, `timer_interrupt_stub` pushes user registers to kernel
stack but DOES NOT restore them to the CPU register file before calling
`timer_interrupt_handler` → `switch_to`. The naked `switch_to` does:
```asm
"mov [rdi + 0x00], r15"  // saves KERNEL r15, not user r15!
```
User r15-rbp are sitting on the kernel stack (pushed by stub) but switch_to ignores them.
On restore, user gets kernel garbage in r15-rbp.
**Fix:** In `timer_interrupt_handler`, before calling `switch_to`, extract the user
callee-saved registers from the kernel stack frame (they were pushed by the stub at
known offsets relative to `stack_frame`) and write them into `old_ctx.r15` etc.
OR: have the stub pass a pointer to the pushed regs as a second argument.

### BUG 3 (CRITICAL — pdx_call always returns wrong value):
**`syscall_entry` discards `dispatch()` return value.** `pop rax` after
`call syscall_handler` restores the PUSHED original rax (= syscall number),
NOT the Rust function's return value. Dispatch must write `regs.rax = result`
to communicate return values to userland. Currently dispatch() returns u64 but
the return convention is wrong — `regs.rax` is not written.
Effect: `pdx_call(0, 0x03, ...)` returns 27 (not 0), sexdisplay enters error loop.
**Fix:** In `dispatch()`, write results via `regs.rax = value` and return 0,
OR restructure syscall_entry to use the function return value.

### BUG 4 (minor — potential layout mismatch):
**`TaskContext` lacks `#[repr(C)]`** but `switch_to` uses hardcoded offsets.
Works in practice (Rust preserves order when no alignment benefit from reordering)
but is fragile. Add `#[repr(C)]` to `TaskContext`.

### Known panic pattern:
`KERNEL PANIC: Userland Null Pointer Jump at RIP: 0x0` — page fault at address 0
with RIP=0 means null instruction fetch. Caused by: iretq with RIP=0 in frame
(task context.rip=0), OR sysretq with rcx=0 (return addr corrupted), OR null
function pointer call in userland.

---

## Known Fixed Bugs (do not reintroduce)

| File                        | Bug                                               | Fix |
|-----------------------------|---------------------------------------------------|-----|
| `kernel/src/interrupts.rs`  | `_wrpkru` used directly                           | Use `crate::pku::wrpkru` |
| `kernel/src/syscalls/mod.rs`| `opcode` referenced (undefined)                   | Use `num` |
| `kernel/src/gdt.rs`         | `kernel_tss_selector` used (wrong name)           | Use `tss_selector` |
| `kernel/src/memory/manager.rs` | `let next += 1` (syntax error)               | Use `self.next += 1` |
| `kernel/src/memory/manager.rs` | Unused imports `MEMMAP_REQUEST`, `HHDM_REQUEST` | Line deleted |
| `kernel/src/gdt.rs`         | `unsafe {}` around `interrupts::disable()`        | Remove unsafe block |
| `kernel/src/elf.rs`         | `let mut flags` (flags never mutated)             | Remove `mut` |
| `CLAUDE.md` (old note)      | "serial_println! must go through pdx_call(0,69)" | WRONG: sex-pdx uses direct asm syscall rax=69. Kernel handles natively. |

---

## ELF Loader Notes

`kernel/src/elf.rs::load_elf_for_pd`:
- Loads segments at `load_base + ph.p_vaddr`
- Returns entry point as `load_base + header.entry`
- For PIE ELFs (p_vaddr=0): correct — segments at `load_base`, entry at `load_base + elf_entry_offset`
- For fixed-address ELFs (p_vaddr=link_addr): segments at `load_base + link_addr` which is
  likely WRONG (double-offset). Ensure all PDX binaries are built as PIE.

---

## Tiny TCB Policy

- Minimize unsafe blocks. If `unsafe` is flagged as unnecessary, remove it.
- Exception: raw pointer dereferences on `cap_table` in `init.rs` genuinely
  require unsafe — do not remove that block.
- Keep the kernel small. Don't add abstractions that aren't needed for the
  current phase.

---

## Workspace Cargo Warnings (expected, non-fatal)

These warnings appear on every build and are harmless:
- "profiles for the non root package will be ignored" (silk-shell, linen)
- "virtual workspace defaulting to resolver = 1"
- `lib.no_std` unused manifest key in `sex-pdx/Cargo.toml`

Do not attempt to fix these without understanding the full workspace layout.

---

## Display Bring-up Checklist (Phase 24+)

When the screen is black:
1. Confirm Limine framebuffer request is fulfilled before sexdisplay spawns
2. Pass framebuffer address/width/height/pitch to sexdisplay at spawn time
3. Verify sexdisplay's PKEY (1) is assigned to the framebuffer mapping
4. Verify PKRU allows writes to key 1 when sexdisplay is active
5. Check sexdisplay isn't blocked on IPC recv() waiting for silk-shell
6. Kernel-side sanity check: write `0x00FF00FF` (magenta) directly to framebuffer
   from init.rs before spawning any PDX — if magenta appears, framebuffer is fine
7. Check for `function_casts_as_integer` warnings in interrupts.rs — stub
   addresses being cast incorrectly can cause bad handler entry points
8. Confirm `dispatch()` writes `regs.rax = 0` for syscall 0x03 success — otherwise
   sexdisplay thinks DisplayInfo query failed and enters error spin loop

---

## Current Status (last updated 2026-04-23)

- Phase 25 — PDX ABI standardized, capability table documented
- Kernel boots, GDT/IDT/PKU/Syscalls all initialize correctly
- Scheduler runs round-robin over 3 PDs (sexdisplay=PD1, silk-shell=PD2, linen=PD3)
- **3 critical bugs FIXED (2026-04-23):**
  1. `current_pd_id` never set — fixed: `timer_interrupt_handler` now calls `set_pd(pd_id)` before `switch_to`
  2. `switch_to` saved kernel r15-rbp — fixed: r15-rbp now extracted from stub's kernel stack frame in `timer_interrupt_handler`; `switch_to` only saves PKRU
  3. `dispatch()` return discarded — fixed: `dispatch()` now writes `regs.rax = result` so `pop rax` loads correct value
- **Expected next issue:** sexdisplay may now reach pdx_listen loop and wait for IPC. Silk-shell sends OP_SET_BG + OP_WINDOW_COMMIT after 2M spin iterations. Linen sends OP_WINDOW_CREATE. These go through `safe_pdx_call` → `CapabilityData::Domain(sexdisp_id)` → enqueues to sexdisplay's message_ring.
- **Goal:** Verify red screen appears (sexdisplay proof-of-life), then get silk-shell IPC flowing
