# SexOS Microkernel — Claude Session Memory

> Canonical team policy now lives in [CREW.md](/home/xirtus_arch/Documents/microkernel/CREW.md).
> Keep this file for Claude-specific memory and deep invariants; do not drift from CREW policy.
> Build authority is sealed to `scripts/entrypoint_build.sh` + `sexos_build_spec.toml`.

This file is read automatically by Claude Code at session start.
It encodes project invariants, ABI contracts, and debugging history.
**Never delete or contradict entries here without updating the date.**

---


### 3-Ring Rule
**Context:** SexOS Phase 21/25 (SASOS, Ring-3 Handoff). Do not violate these x86_64 hardware and crate limits.

**1. GDT & TSS Array Limits (The 16-Byte Rule)**
The `x86_64` crate has a strict 8-slot GDT limit. A Task State Segment (TSS) in long mode is a "System Segment" requiring **two contiguous 8-byte slots** (16 bytes). 
* **MANDATORY GDT ORDER:** * Slot 0: Null 
    * Slot 1: Kernel Code
    * Slot 2: Kernel Data
    * Slot 3 & 4: TSS (MUST be inserted here, before User segments, to prevent array overflow).
    * Slot 5: User Data (SS)
    * Slot 6: User Code (CS)

**2. SYSRET Mathematical Offsets**
The `syscall` instruction strictly calculates segments. `x86_64::registers::model_specific::Star::write` will throw a `SysretOffset` panic if indices violate this math:
* Kernel SS Index MUST be `Kernel CS + 1` (Index 2 = 1 + 1).
* User CS Index MUST be `User SS + 1` (Index 5 = 4 + 1 — hardware confirmed).
* *Never* pass `user_data_selector` as the Kernel SS parameter.

**3. Ring-3 Context Switch (IRETQ)**
* **The RPL Drop:** When forging the interrupt stack frame in `Task::new()`, user selectors MUST explicitly be bitwise-OR'd with the Ring Privilege Level 3 (`| 3`). 
    * `User CS` must evaluate to `0x2B` (GDT index 5 | RPL3). **NOT 0x33** — index 6 is TSS → `#GP(0x30)`.
    * `User SS` must evaluate to `0x23` (GDT index 4 | RPL3).
    * Confirmed by hardware: CS=0x33 → `#GP Error: 0x30` → CPU saw TSS at index 6, not code segment.
* **Actual GDT user segment layout (hardware-confirmed):**
    * Index 4: User Data (SS) → selector `0x20`, with RPL3 = `0x23`
    * Index 5: User Code (CS) → selector `0x28`, with RPL3 = `0x2B`
    * Index 6-7: TSS (system segment, 2 slots)
    * SYSRET math: User CS Index (5) = User SS Index (4) + 1 ✓
* **The Stack Bomb:** If using a custom stub (e.g., `timer_interrupt_stub`) before `iretq`, `Task::new()` must push exactly 15 dummy zeros onto the task stack *on top* of the hardware frame. Otherwise, the stub's `pop r15 ... pop rdi` sequence will literally eat the `iretq` frame, misaligning the stack.

*** ### Why this works for LLMs:
* **The "MANDATORY" phrasing:** AI models are trained to follow explicit negative constraints ("Do not violate", "MANDATORY").
* **Pre-empting the Math:** Explaining *why* `Star::write` panics prevents the LLM from trying to "hack" the GDT order in a way that breaks the sysret math.
* **Consolidated Fixes:** It packages the RPL fix, the Stack alignment, the GDT limit, and the Syscall offsets into one token-light summary.


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

SexOS is a SASOS microkernel (Rust, x86_64). Memory model: ARCHITECTURE.md §0.
Bootloader: Limine. Dev target: QEMU.

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

**Cargo resolver:** workspace uses resolver = "2".

---

## Memory Layout (SASOS Map)

All components share one PML4 (GLOBAL_VAS). Authority model: ARCHITECTURE.md §0.

| Component          | Virtual Address          | Notes |
|--------------------|--------------------------|-------|
| Kernel binary      | Higher-half (linker.ld)  | `no_std` core + Limine requests |
| Userland stubs     | `0x4000_0000`            | Mock entry point for `sex-ld` library mapping |
| Translated native  | `0x4000_1000`            | Target entry for translated ELFs via `sexnode` |
| System heap        | `0x4444_4444_0000`       | 128 MiB (HEAP_SIZE in lib.rs), mapped at boot |
| sexdisplay FB      | Dynamic (passed via IPC as OP_PRIMARY_FB) | Framebuffer pages tagged PKEY 1 |

- HHDM offset: `0xffff800000000000`
- All userland segments mapped via `GlobalVas::map_pku_range` which applies PD key
  to Level-1 page table entries
- Page tables: `OffsetPageTable` via x86_64 crate
- PD structs live on system heap at `0x4444_4444_0000`
- PD load bases: `0x4000_0000 + ((domain_id - 1) * 0x0100_0000)`
  - domain 1 (sexdisplay): 0x40000000
  - domain 2 (sexdrive):   0x41000000
  - domain 3 (silk-shell): 0x42000000
  - domain 4 (sexinput):   0x43000000
  - domain 5 (silkbar):    0x44000000
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
| 1    | sexfiles VFS |
| 2    | sext (demand pager) |
| 3    | sexinput (HID input ring) |
| 4    | Audio server |
| **5** | **sexdisplay (compositor)** — also silkbar's SLOT_DISPLAY target |
| 6    | silk-shell orchestration | |

### Capability Table Structure

Each `ProtectionDomain` struct contains a `CapabilityTable` with `CapabilityData` entries.
Cap table is accessed via raw pointer in `init.rs` — `unsafe` is intentional.

`init.rs` inserts `CapabilityData::Domain(sexdisp_id)` at slot 5 for PDs 1..=4.

---

## Scheduler — BUG HISTORY & ACTIVE STALL

- Round-robin via `WorkStealingQueue`. Uses `steal()` on local queue (should be
  `pop()` but functionally identical on single core).
- Timer IRQ fires → `timer_interrupt_stub` → `timer_interrupt_handler`.
- **"Fresh Frame" model enforced (Phase 28):** `switch_to` loads `kstack_top` as clean slate, pushes IRETQ frame manually. `add rsp, 8` removed. `TaskContext` offsets 0x90-0x98.

### BUG 1 (FIXED 2026-04-23 — was: kernel panic on any pdx_listen/safe_pdx_call):
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

### BUG 2 (FIXED 2026-04-23 — was: corrupts callee-saved registers on context restore):
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

### BUG 3 (FIXED 2026-04-23 — was: pdx_call always returns wrong value):
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

### BUG 5 (ACTIVE — Phase 28 stall — scheduler returns None every tick):
**`Scheduler::tick()` never finds a task to switch to.** `steal()` returns `None`
for all cores despite `pdx_spawn` logging successful task registration.
`SWITCH` log lines never appear. `timer_tick` spam continues indefinitely.
Diagnosis: runqueue push and steal/pop operate on different state, or tasks
are registered after scheduler init but before runqueue is live.
**Next:** Instrument `WorkStealingQueue::push()`, `steal()`, `attempt_steal()` —
verify tasks actually land in the queue and are visible to the scheduler's steal path.

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
- Loads segments at `load_base + (p_vaddr - min_vaddr)` where min_vaddr is the smallest
  vaddr across all PT_LOAD segments.
- Returns entry point as `load_base + (header.entry - min_vaddr)`.
- For PIE ELFs (p_vaddr=0, min_vaddr=0): segments at `load_base`, entry at `load_base + elf_entry` — correct.
- For fixed-address ELFs (p_vaddr=0x200000, min_vaddr=0x200000): segments at `load_base`, entry at `load_base + (entry - 0x200000)` — correct.
- **CRITICAL: Does NOT process `.rela.dyn` or `.rela.plt`.** Any absolute address reference
  (GOT entry for cross-crate `pub static`) retains the ELF's original address. Use `const`
  instead of `static` for shared data to force compile-time inlining.
- **CRITICAL: Does NOT check for lower-half vaddr ranges** — the string "ELF lower half phdrs
  are not allowed" does NOT exist in the kernel source. Segments with vaddr in the 0x0000-0x3FFF
  range are loaded at `load_base + delta` without rejection (as of this writing).
- **GOT relocation gap (BURNDOWN):** When sexdisplay/silkbar references `DEFAULT_SILK_BAR` (a
  `pub static` from another crate), the compiler generates a GOT entry. At link time (PIE), the
  GOT entry receives the ELF's pre-relocation address (e.g., 0x2001d8). The kernel loads the
  segment at a different base (e.g., 0x44000000) but never fixes GOT entries. Result: page fault
  at the stale lower-half address. **Fix: use `pub const` instead of `pub static`** — forces
  compile-time inlining, no GOT entry needed.

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
- "profiles for the non root package will be ignored" (silk-shell, sexinput, silkbar)
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

## Current Status (last updated 2026-04-29 — v8 SilkBar PDX Integration)

- **Scheduler stall is FIXED.** All 5 PDs spawn and schedule correctly: sexdisplay (PD1), sexdrive (PD2), silk-shell (PD3), sexinput (PD4), silkbar (PD5).
- **Current task:** Integrate silkbar → sexdisplay PDX clock update (v8 scalar protocol).
- **Active bug:** Cross-crate `static` references from silkbar-model crate produce unrelocated GOT entries. Kernel ELF loader doesn't do `.rela.dyn` processing.
  - Fix: Changed `pub static` → `pub const` for `DEFAULT_SILK_BAR` and `DEFAULT_THEME` in silkbar-model.
  - sexdisplay/main.rs rewritten as PDX-aware renderer: listens for OP_PRIMARY_FB (0x11) and OP_SILKBAR_UPDATE (0xF2).
- **Next action:** Rebuild and boot-verify.

## Critical ABI Facts (discovered this session)

1. **No GOT relocation in ELF loader.** `kernel/src/elf.rs` copies segments but does NOT apply `.rela.dyn` relocations. Cross-crate `pub static` references produce stale GOT entries.
2. **Fix: use `const` not `static`** for shared data across PDX crates. Const values are inlined, no GOT involved.
3. **OP_PRIMARY_FB (0x11) message format:** arg0=fb_addr, arg1=(width | height<<32), arg2=pitch (pixels/row). Sent by kernel to sexdisplay's message ring before scheduler runs sexdisplay.
4. **OP_SILKBAR_UPDATE (0xF2) message format:** arg0=kind(4=SetClock), arg1=(index<<32 | a), arg2=b.
5. **silkbar PDX call:** `pdx_call(SLOT_DISPLAY, OP_SILKBAR_UPDATE, 4, (0<<32)|10, 44)` sends SetClock(10:44) to sexdisplay.
6. **Pixel format:** 0x00RRGGBB (32-bit RGB, alpha ignored).

## Domain/PD Layout

| Domain | PD ID | Base       | Name          |
|--------|-------|------------|---------------|
| 1      | 1     | 0x40000000 | sexdisplay    |
| 2      | 2     | 0x41000000 | sexdrive      |
| 3      | 3     | 0x42000000 | silk-shell    |
| 4      | 4     | 0x43000000 | sexinput      |
| 5      | 5     | 0x44000000 | silkbar       |

## Interrupts Reading Discipline

**Do not read all of `kernel/src/interrupts.rs`.** It is large (~740 lines)
and every agent that opens it wastes context budget. Instead:

1. Use `rg` to find the symbol you need:
   ```
   rg "page_fault_handler|timer_interrupt|switch_to|faulted_task_halt" kernel/src/interrupts.rs -n
   ```
2. Open only ±80 lines around the match:
   ```
   sed -n '460,540p' kernel/src/interrupts.rs
   ```
3. See `docs/INTERRUPTS_QUICKMAP.md` for the full section index with line
   ranges, critical invariants, and rg patterns for common debug entry points.

Key landmarks in interrupts.rs:

| Lines  | What |
|--------|------|
| 48-49  | IDT handler registration (page_fault, GPF, timer) |
| 131-293| `syscall_entry` naked asm |
| 295-336| `page_fault_stub` naked asm (stack layout) |
| 361-456| `timer_interrupt_stub` + `timer_interrupt_handler` |
| 458-465| `faulted_task_halt()` kernel halt trampoline |
| 466-618| `page_fault_handler` (#PF dispatch) |
| 620-725| `general_protection_fault_handler` |

## SilkBar ABI

- `SilkBarUpdate`: `#[repr(C)]` 16 bytes: kind(u32), index(u8), a(u32), b(u32)
- Update kinds: 0=SetWorkspaceActive, 1=SetWorkspaceUrgent, 2=SetChipVisible, 3=SetChipKind, 4=SetClock, 5=SetThemeToken
- `silkbar-model` crate provides: types, `DEFAULT_SILK_BAR` (const), `DEFAULT_THEME` (const), `apply_update()`, `SilkBarUpdateQueue`
- sexdisplay imports `silkbar-model` for types; renders clock chip at position CHIP_X3=1090, CHIP_Y=18
