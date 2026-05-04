# SexOS Microkernel — Claude Session Memory

> Canonical team policy lives in [CREW.md](/home/xirtus_arch/Documents/microkernel/CREW.md).
> Reference material offloaded to `claude-references/*.md` — read on demand.
> Build authority: `scripts/entrypoint_build.sh` + `sexos_build_spec.toml`.

This file is read automatically by Claude Code at session start.
It encodes **critical session invariants and standing orders only**.
**Never delete or contradict entries here without updating the date.**

---

## Standing Orders

### Anti-Scope-Creep Rule
**STOP FIRST:** Reject patches touching USB + shell + display + kernel + sex-pdx together.
Any patch spanning more than two major domains must STOP FIRST before implementation.
Ensure every feature proves exactly **ONE** boundary.

### Token Discipline
- **Read files before searching.** Do not web search for things defined in this codebase.
- **No speculative reads.** Only open files directly relevant to the current task.
- **No redundant builds.** Do not run `cargo build` more than once per fix unless the first build produced an unexpected error.
- **Prefer targeted edits.** Use `str_replace` on exact lines. Do not rewrite whole files for a 2-line bug.
- **State assumptions explicitly.** If unsure, say so and ask rather than reading multiple files to guess.

### Self-Update Rule (CRITICAL)
When you discover something **blocking progress** — a wrong assumption, missing invariant, bug root cause, correct ABI detail — update the appropriate file before ending the session:
- Add new invariants/ABI facts to `claude-references/ABI_LAYOUT.md`
- Add confirmed bug fixes to `claude-references/BUG_HISTORY.md`
- Update `claude-references/PROJECT_STATUS.md` with what changed
- Update `CLAUDE.md` only if the change affects standing orders or core invariants
- Remove resolved items from debugging checklists

These files are the only persistent memory across sessions.

---

## Core Invariants (MUST NOT BE VIOLATED)

### 1. GDT Order (8-slot limit, 16-byte TSS rule)
| Slot | Content       |
|------|---------------|
| 0    | Null          |
| 1    | Kernel Code   |
| 2    | Kernel Data   |
| 3 & 4| TSS           |
| 5    | User Data (SS)|
| 6    | User Code (CS)|

- SYSRET math: Kernel SS = Kernel CS + 1 (index 2 = 1+1). User CS = User SS + 1 (index 5 = 4+1).
- **User CS = 0x2B** (index 5 | RPL3). **NOT 0x33** — that's TSS at index 6 → `#GP(0x30)`.
- **User SS = 0x23** (index 4 | RPL3).
- See `claude-references/ABI_LAYOUT.md` for full details.

### 2. Syscall Return Value Trap
**`pop rax` after `call syscall_handler` restores the ORIGINAL rax (syscall number), NOT the handler return value.** To return a value to userland, you MUST write `regs.rax = value` before returning from dispatch. Simply returning from `dispatch()` does NOT set userland rax.

### 3. PKU
- Kernel entry: `xor eax,eax; xor edx,edx; xor ecx,ecx; wrpkru` (opens ALL keys).
- **Never use** `core::arch::x86_64::_wrpkru` directly — use `crate::pku::wrpkru`.
- Only PKU file: `kernel/src/pku.rs` (was `kernel/src/memory/pku.rs` — deleted).
- `serial_println!` in sex-pdx uses direct asm `syscall` rax=69 — NOT a null deref.
- Use `const` not `static` for cross-crate shared data (no GOT relocation in ELF loader).

### 4. PDX Calling Convention
ALWAYS use **5-argument arity**: `pdx_call(slot, syscall, arg0, arg1, arg2)`. Never 4-argument — causes stack misalignment on `sysretq`/`iretq`.

### 5. Tiny TCB Policy
- Minimize unsafe blocks. Remove if flagged as unnecessary.
- Exception: raw pointer dereferences on `cap_table` in `init.rs` genuinely require unsafe.
- Keep the kernel small. Don't add abstractions not needed for the current phase.

---

## Quick Reference Index

| Topic | File |
|-------|------|
| GDT, Memory Layout, PKU, Syscall ABI, PDX ABI, Surface Opcodes | `claude-references/ABI_LAYOUT.md` |
| Fixed bugs table, scheduler bug history | `claude-references/BUG_HISTORY.md` |
| Build pipeline, QEMU flags, env vars, proof verification | `claude-references/BUILD_AND_DEPLOY.md` |
| USB input pipeline, xHCI ring, QEMU SDL/tablet, keyboard cursor | `claude-references/INPUT_PIPELINE.md` |
| **USB continuation status — where we left off** | **`claude-references/USB_STATUS.md`** |
| SilkBar ABI, action slot expansion, interaction contracts | `claude-references/SILKBAR_ABI.md` |
| ELF loader details, GOT relocation gap | `claude-references/ELF_LOADER.md` |
| Display bring-up checklist, interrupts quickmap, diagnostics | `claude-references/DEBUGGING.md` |
| Current status, completed features, next actions | `claude-references/PROJECT_STATUS.md` |

---

## Debugging Shortcuts

- **Black screen?** See `claude-references/DEBUGGING.md` §Display Bring-up Checklist.
- **Interrupts.rs too large?** Use `rg` for specific symbols, then `sed -n '±80'`. See `claude-references/DEBUGGING.md` §Interrupts Reading Discipline.
- **Null RIP panic?** iretq with RIP=0, sysretq with rcx=0, or null fn pointer call in userland.
- **Scheduler not switching?** See BUG 5 in `claude-references/BUG_HISTORY.md`.
- **Continue USB input work?** Start at `claude-references/USB_STATUS.md` — single document with blockers, audits, workarounds, and next steps.
- **Stable baseline:** `docs/handoff/STABLE_BASELINE_20260503.md` before any new feature work.

---

## Important File Paths for rg

| File | Purpose |
|------|---------|
| `kernel/src/interrupts.rs` | syscall_entry (131-293), page_fault_stub (295-336), timer handler (361-456) |
| `kernel/src/syscalls/mod.rs` | Syscall dispatch table |
| `kernel/src/gdt.rs` | GDT setup (8-slot limit) |
| `kernel/src/elf.rs` | `load_elf_for_pd` — no GOT relocation |
| `kernel/src/pku.rs` | PKU wrpkru/rdpkru functions |
| `kernel/src/init.rs` | PD spawning, ELF loading, scheduler start |
| `kernel/src/scheduler.rs` | WorkStealingQueue, switch_to |
| `kernel/src/memory/manager.rs` | Heap init, GLOBAL_VAS |
| `crates/sex-pdx/src/lib.rs` | PDX calling convention, serial_println macro |
| `servers/sexinput/src/main.rs` | HID event handling, synthetic proofs |
| `servers/silk-shell/src/main.rs` | Shell state, focus, click-focus |
| `servers/sexdisplay/src/main.rs` | Compositor, surface rendering |
| `servers/sexusb/src/main.rs` | xHCI driver, interrupt-IN ring |
| `servers/silkbar/src/main.rs` | Top bar clock, workspace updates |
| `crates/silkbar-model/src/lib.rs` | SilkBar types, DEFAULT_SILK_BAR, apply_update |

