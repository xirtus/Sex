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

### NEXT_BOUNDARY_HARDENING_PLAN_V1 Anti-Scope-Creep Rule
**STOP FIRST:** Reject patches touching USB + shell + display + kernel + sex-pdx together. Any patch spanning more than two major domains must STOP FIRST before implementation.
- Use `rg "NEXT_BOUNDARY_HARDENING_PLAN_V1|USB_BUTTON_CLICK_PROOF_V1|SHELL_FOCUS_CONTRACT_V1|SURFACE_OWNERSHIP_CONTRACT_V1|DOCK_OVERLAYBAR_MODEL_V1|BELL_CAPABILITY_ATTENTION_V1|LINEN_STATIC_SURFACE_V1|SHELL_GLOBAL_INTERACTION_CONTRACT_V1|SHELL_INTERACTION_STATE_V1|HIT_TEST_PRIORITY_V1|EVENT_ORDERING_CONTRACT_V1|SURFACE_ID_LIFETIME_V1|CHROME_MODE_ARBITRATION_V1|DEAD_PD_SURFACE_CLEANUP_V1|INTEGRATED_SCENARIO_PROOF_V1" -n docs CLAUDE.md` to confirm the plan.
- Ensure every feature proves exactly ONE boundary.

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
| `servers/sexusb/src/main.rs` | xHCI interrupt-IN Transfer Ring dequeue stuck at slot 1 forever | Circular ring: 15 Normal slots + Link TRB at slot 15 with TC=1. Track `intr_prod`/`intr_pcs`. See §xHCI Interrupt Ring below. |
| `servers/sexusb/src/main.rs` | Bounded 512-attempt outer poll exhausted before user interaction | Changed to unbounded `loop` with wrapping `u32` counter. |
| `servers/sexinput/src/main.rs` | Synthetic drag proof wraps forever via `% 3`, storms shell with drag.start/move/end every 120 ticks | Added `SYNTHETIC_DRAG_PROOF_DONE` one-shot gate; stage 2 sets `DONE=true`, block guarded by `!DONE`. See `docs/handoff/INPUT_REPLAY_STORM_FIX_V1.md`. |

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

## Current Status (last updated 2026-05-03 — SILK_DE_M2_ASSERT_PATCH_V1)

- **M2 audit assert patch (SILK_DE_M2_ASSERT_PATCH_V1):**
  - F3: sexdisplay apply_update() return value now captured; invalid updates logged with [silkde.m2.assert.bad] and do NOT trigger redraw
  - F4: ChipSlot discriminant invariant added to validate_contract() -- Chip0/Chip1/Chip2/Clock discriminants must match CHIP_SLOTS indices 0/1/2/3
  - F1 (queue overflow) and F2 (stale clock watchdog) deferred to separate boundary decisions
  - Files: crates/silkbar-model/src/lib.rs (+8 lines), servers/sexdisplay/src/main.rs (+6/-6 lines)
  - Zero kernel/sex-pdx/silk-shell/sexinput edits. Zero ABI changes. Zero warnings.
- **Renderer conformance cleanup (RENDERER_CONFORMANCE_CLEANUP_V1, commit e9596eb):**
  - 11 magic color literals replaced with DEFAULT_THEME fields (values identical)
  - Remaining custom colors (Wifi/Battery chips, launcher dot, bg gradient) have no theme mapping
  - Top-strip hash confirmed unchanged: `0x3c8d391f6e312fca`
- **Top-strip render proof live (SILK_TOP_STRIP_RENDER_PROOF_V1, commit c22afa9):**
  - FNV-1a hash over rows 0..50 (50×w pixels) after first live render
  - Hash printed atomically (single pdx_call) to avoid scheduler interleave
  - Baseline hash: `0x3c8d391f6e312fca` (QEMU virtio-gpu, 1280 wide, default bar state)
  - Verify: `grep "silk.render_proof" /tmp/silk-render-proof.log`
- **SilkBar contract locked (SILK_DE_CONTRACT_LOCK_V1, commit 17cbbe7):**
  - `validate_silkbar_contract() -> u32` added to silkbar-model (reason code: 0=ok, 1=contract, 2=vectors)
  - Both silkbar and sexdisplay emit `[silk.contract.validate.start/ok/fail]` markers at `_start`
  - Stale digest constant removed (was blocking validation since initial commit)
  - Both servers print `[silk.contract.validate.ok] version=1` on every boot
- **Scheduler stall is FIXED.** All PDX domains spawn and schedule correctly.
- **USB HID boot-class mouse pipeline is code-complete** (committed through `proof-xhci-intr-ring-advance-20260502`).
- **QEMU usb-tablet HID support (04566ab) — PROVEN:**
  - Tablet HID interface detection via config walk (`hid_tablet.found`)
  - HID report descriptor shape scan recognizes tablet/pointer (`tablet_shape.ok`)
  - SHORT_PACKET (cc=13) accepted in interrupt-IN event handler
  - Absolute position reports decoded: `[sexusb.hid.tablet.raw] b0=0x0 b1=0x0 b2=0x0 b3=0x0 b4=0x0 actual=6`
  - **Nonzero position reports captured** in SDL X11 session:
    ```
    [sexusb.hid.tablet.report] i=1 buttons=0x0 x=32741 y=9625 dx=127 dy=127
    [sexusb.hid.tablet.nonzero.ok] i=1 buttons=0x0 x=32741 y=9625 dx=127 dy=127
    [sexusb.hid.tablet.report] i=2 buttons=0x0 x=32741 y=9379 dx=0 dy=-128
    ```
  - Shell pointer state received nonzero reports:
    ```
    [shell.pointer.usb_state.nonzero.ok] x=767 y=487 buttons=0x0 wheel=0 dx=127 dy=127
    [shell.pointer.usb_state.nonzero.ok] x=894 y=486 buttons=0x0 wheel=0 dx=0 dy=-128
    ```
- **Workspace switch through silkbar PROVEN (SILKBAR_WORKSPACE_SWITCH_V1, commit e675df7+):**
  - Workspace clicks now update real active workspace state: shell → silkbar → sexdisplay
  - `[silkbar.workspace.recv/active.set/active.send.start/active.send.ok]` markers
  - No renderer or ABI changes; uses existing OP_SILKBAR_UPDATE transport
- **SilkBar clickable controls PROVEN (SILKBAR_CLICKABLE_CONTROLS_V1, commit c5c24d8+):**
  - Shell hit-test for panel regions using silkbar-model geometry + DEFAULT_SILK_BAR layout
  - `[shell.silkbar.click] target=launcher/workspace/status/clock` markers
  - Synthetic proof clicks all four target types via HID_EVENT path (ticks 2-17)
  - Workspace clicks dispatch OP_SILKBAR_WORKSPACE_ACTIVE; others classified and logged
  - Drag and click-focus preserved (bar at y<50, surfaces at y>=50 — no overlap)
  - Verify: `grep "shell.silkbar.click" /tmp/silkbar-click.log`
- **Drag-window proof PROVEN (DRAG_WINDOW_PROOF_V1, commit 04566ab+):**
  - Synthetic drag proof via HID_EVENT path (sexinput -> silk-shell)
  - `[shell.drag.start] id=100 x=200 y=200`
  - `[shell.drag.move] id=100 x=206 y=204 dx=6 dy=4`
  - `[shell.drag.send.ok] id=100`
  - `[shell.drag.end] id=100 x=206 y=204`
  - USB path also supports drag (start/end/movement with proof markers)
  - No new ABI, no kernel edits, no sexdisplay edits
  - Verify: `grep -E "shell.drag.start|shell.drag.move|shell.drag.end" /tmp/drag-proof.log`
- **Click-focus chain PROVEN (SYNTHETIC_CLICK_FOCUS_PROOF_V1, commit 72753aa):**
  - Sexinput synthetic one-shot routes via `OP_USB_MOUSE_REPORT` → silk-shell
  - `[shell.click_focus.down] x=940 y=520 buttons=0x1`
  - `[shell.click_focus.hit] id=200` (SURFACE_ID_LINEN)
  - `[shell.click_focus.send.ok] id=200`
- **Not yet proven via physical USB tablet:**
  - **Button events from USB tablet** — blocked by SDL2/XTest filter + QEMU 11.0 routing
  - Tablet decode code `buf[0]&0x07` is correct (code inspection)
  - Physical mouse or `/dev/uinput` needed for full USB button proof
- **Critical blocker (QEMU 11.0):** QMP/HMP input injection does NOT route to USB HID devices. Events consumed by PS/2 display layer only. Confirmed: `input-send-event` returns `{"return": {}}` but usb-mouse/tablet sees nothing.
- **Workaround discovered:** `SDL_VIDEO_DRIVER=x11` + `-display sdl` produces a visible X11 window (confirmed via `xdotool`). Mouse events from the host X11 desktop forwarded through SDL do reach the usb-tablet device. This enables proof in headless environments with Xvfb or similar.

**Button injection blocked (confirmed 2026-05-03):**
- SDL2 filters XTest synthetic events (`send_event=True`); `xdotool` uses XTest → zero USB reports
- VNC RFB `PointerEvent` also does NOT reach USB HID in QEMU 11.0
- QMP/HMP: already confirmed blocked (prior session)
- USB tablet decode code is correct; silk-shell click-focus code is correct
- Only real physical mouse over SDL window can deliver button events

**Completed (2026-05-03):**
- **Synthetic drag proof** (DRAG_WINDOW_PROOF_V1): `USB_PROOF_DISABLE_SYNTH_DRAG = false` enables the sexinput→shell→drag chain. Verified via `grep -E "shell.drag" /tmp/drag-proof.log`.
- **Input replay storm fix** (INPUT_REPLAY_STORM_FIX_V1): Synthetic drag proof no longer wraps forever via `% 3`. One-shot gate `SYNTHETIC_DRAG_PROOF_DONE` prevents replay after stage 2. `shell.drag.start` count reduced from 50 to 1 per boot. Only `servers/sexinput/src/main.rs` changed. See `docs/handoff/INPUT_REPLAY_STORM_FIX_V1.md`.

**Next action — choose one:**
1. **Physical mouse proof**: move real mouse into `SDL_VIDEO_DRIVER=x11 SEXUSB_QEMU_DEVICE=tablet ./dev.sh run` window, click twice (first click grabbed by SDL, second reaches USB tablet). Check for `buttons=0x01` and `[shell.click_focus.down/hit/send.ok]`.
2. **uinput virtual mouse**: create Linux virtual input device via `/dev/uinput` — events appear as real device events, bypass SDL XTest filter. Full-chain proof.
3. **Re-enable synthetic click-focus proof**: set `USB_PROOF_DISABLE_SYNTH_CLICK = false` — proves click-focus chain alongside drag proof.

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

---

## xHCI Interrupt-IN Transfer Ring (sexusb)

**Critical invariant (FIXED 2026-05-02):** Never write all Normal TRBs to ring slot 0.

After the xHCI processes slot 0 and the software re-writes slot 0 again, the controller
dequeue pointer is at slot 1. Ringing the doorbell makes the controller re-read slot 1
(not slot 0). If slot 1 has cycle=0, controller stops — all polls after the first stall.

**Fix in `servers/sexusb/src/main.rs`:**
- Ring layout: `INTR_TR_RING_SIZE = 16`. Slots 0–14 = Normal TRBs. Slot 15 = Link TRB.
- Link TRB: `d0/d1 = intr_ring_phys`, `d3 = (TRB_TYPE_LINK<<10) | TC | intr_pcs`.
  TC=1 causes xHCI to toggle its Consumer Cycle State on wrap.
- Poll loop state: `intr_prod: u64 = 0`, `intr_pcs: u32 = 1`.
- Each iteration: write Normal TRB at `intr_prod` with `intr_pcs`, ring doorbell, wait event.
- After event consumed: `intr_prod += 1`. If `intr_prod >= 15`: toggle `intr_pcs`,
  update Link TRB cycle bit to new `intr_pcs`, `intr_prod = 0`.
- Endpoint dequeue: `ep_deq = intr_ring_phys | 1` (DCS=1 matches initial `intr_pcs=1`).

**QEMU SDL/tablet grab:**
- SDL requires a left-click inside the window to grab host mouse. First click consumed by SDL (not forwarded to USB). Second click = first real USB button event.
- `dx`/`dy` events only arrive after grab in boot-mouse mode. For usb-tablet, absolute position reports arrive even without grab (QEMU SDL forwards absolute motion directly).
- **Key finding:** `SDL_VIDEO_DRIVER=x11` is required when DISPLAY is available but Wayland is default. Without this, SDL uses Wayland backend and creates no visible X11 window.
- Do NOT use `-display gtk,grab-on-hover=on` — GTK steals keyboard focus, stray keypresses open Limine config editor and prevent boot.
- Proof sequence: `SDL_VIDEO_DRIVER=x11 SEXUSB_QEMU_DEVICE=tablet ./dev.sh run`, wait for desktop, find window via `xdotool search --name "QEMU"`, inject mouse via `xdotool mousemove --window $WID X Y` and `xdotool click 1`.

---

## USB Input Pipeline Architecture

```
QEMU usb-mouse (boot HID, relative, 4-byte reports)
  OR QEMU usb-tablet (absolute, 6-byte reports via interrupt-IN, decoded to relative deltas)
  → sexusb (PD7 @ 0x46000000): xHCI interrupt-IN polling, circular ring,
                                SHORT_PACKET acceptance, tablet absolute→relative delta
  → sexinput (PD4 @ 0x43000000): normalize, clamp, send OP_USB_MOUSE_REPORT to silk-shell
  → silk-shell (PD3 @ 0x42000000): update POINTER_X/Y/buttons, move cursor surface (0xEB),
                                    click-focus hit-test (0xED)
  → sexdisplay (PD1 @ 0x40000000): render surfaces, cursor z-top pass, arrow bitmap
```

**Tablet decode path (sexusb):**
- `decode_tablet_report(buf, len) -> Option<TabletReport>`: parses 5 bytes (buttons, abs_x u16 LE, abs_y u16 LE)
- Static mut state: `PREV_ABS_X`, `PREV_ABS_Y`, `FIRST_TABLET_REPORT`
- Delta computation: `dx = clamp( abs_x - prev_x, -128, 127 )` (same for dy)
- First report: sets prev to current, sends zero delta (prevents initial position jump)
- Same PDX message format as boot mouse (OP_USB_MOUSE_REPORT = 0x260, packed_axes)
- **Key invariant:** tablet absolute positions (0..32767) are converted to relative deltas before reaching sexinput. sexinput and silk-shell see no difference from boot mouse reports.

**Surface opcodes (shell→sexdisplay):**
- `0xEC` create surface: arg0=id, arg1=(y<<32)|x, arg2=(h<<32)|w
- `0xEB` move surface: arg0=id, arg1=x, arg2=y
- `0xED` focus surface: arg0=id
- `0xEE` destroy surface: arg0=id
- `0xEF` fill rect: arg0=id, arg1=(sy<<32)|sx, arg2=(color<<32)|(sh<<16)|sw

**Cursor surface:** `SURFACE_ID_CURSOR = 0x90` (144). Created first at boot (slot 0 in SURFACES array). `draw_cursor_z_top()` renders arrow bitmap unconditionally after all other passes.

**Click-focus guard:** `CLICK_ACTIVE` bool prevents repeat focus on held button. Rising edge only (button down, not held).

---

## Stable Baseline Reference (2026-05-03)

Read `docs/handoff/STABLE_BASELINE_20260503.md` before any new feature work.
It documents:
- All proven features and their last verification date
- Locked invariants (what must not be violated)
- Known limitations (USB button proof, panel visuals)
- Standard verification command
- **NEXT_BOUNDARY_HARDENING_PLAN_V1** (section 5): ordered phases, boundary rules, anti-scope-creep rules
- Recurring bug handoff format
- Next recommended feature order
- Surface ID registry (0x90 cursor, 0x92-0x94 panels, 0x95 reserved, 100+ apps)

**Hard rule: Every feature proves exactly one boundary.**
**Anti-scope-creep: Reject patches touching USB + shell + display + kernel + sex-pdx together.**

Phases (exact order from baseline):
1. USB_BUTTON_CLICK_PROOF_V1
2. SHELL_FOCUS_CONTRACT_V1
3. SURFACE_OWNERSHIP_CONTRACT_V1
4. DOCK_OVERLAYBAR_MODEL_V1
5. BELL_CAPABILITY_ATTENTION_V1
6. LINEN_STATIC_SURFACE_V1

## Shell Global Interaction Contract (2026-05-03)

Read `docs/handoff/STABLE_BASELINE_20260503.md` section 6 (`SHELL_GLOBAL_INTERACTION_CONTRACT_V1`).

**Core insight:** Local phase proofs are not sufficient. Global UI behavior can fail from event-order bugs, focus conflicts, chrome conflicts, surface ID ambiguity, or dead-PD dangling state.

7 subcontracts govern interaction integrity:
- A. SHELL_INTERACTION_STATE_V1 — unified state table (no scattered `*_ACTIVE` booleans)
- B. HIT_TEST_PRIORITY_V1 — strict z-order
- C. EVENT_ORDERING_CONTRACT_V1 — deterministic pipeline
- D. SURFACE_ID_LIFETIME_V1 — monotonic IDs, tombstoning
- E. CHROME_MODE_ARBITRATION_V1 — exclusive chrome, no focus steal
- F. DEAD_PD_SURFACE_CLEANUP_V1 — safe teardown
- G. INTEGRATED_SCENARIO_PROOF_V1 — combined scenario verification

**Every feature must prove:** boundary proof, negative proof, integration proof, handoff proof.

---

## SilkBar Action Slot Expansion (2026-05-03)

ABI v1→v2 expansion adds Bell module slot (index 10, between Battery and Clock, x=1020).
Read `docs/handoff/SILKBAR_ACTION_SLOT_EXPANSION_V1.md` before adding Bell panel behavior.

**Key invariants:**
- LAYOUT_COUNT = 11 (was 10)
- MAX_CHIPS = 4 (unchanged — Bell is a ModuleSlot, not a chip)
- Bell hit-test → `Action::OpenBell` (no panel toggle yet)
- Bell rendering: gold 0x00FFD700 at (1020, 18, 18, 22)
- After this expansion is proven, BELL_PANEL_TOGGLE_V1 wires toggle_os_panel()
