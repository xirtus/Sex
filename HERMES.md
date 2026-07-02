# Hermes Agent — SexOS Project Knowledge

> Auto-loaded at session start when working in `/home/xirtus_arch/Projects/Sex`.
> Derived from CLAUDE.md, HANDOFF.md, ARCHITECTURE.md, STABLE_BASELINE_20260503.md,
> GEMINI.md, CREW.md, and the docs/handoff/ ecosystem.

---

## 0. What This Is

SexOS is a **Single Address Space OS (SASOS) microkernel** written in Rust (`no_std`),
booted via Limine 7.x on bare-metal x86_64. All kernel + userland share one virtual
address space. Isolation is via Intel PKU (Memory Protection Keys), not per-process
page tables. Access is capability-mediated through `sex-pdx`.

**One-line**: A closed, deterministic SMP state lattice where the kernel is pure
enforcement + routing and userland servers consume/produce ordered frame graphs.

---

## 1. Build & Run (SEALED)

```bash
# Build (the ONLY valid build path)
./scripts/entrypoint_build.sh

# Run with GUI
./dev.sh run

# Run headless + capture serial log
./dev.sh run-nographic 2>/tmp/serial.log | tee /tmp/stdout.log

# USB device: tablet-display-sdl (default, mouse via SDL)
SEXUSB_QEMU_DEVICE=mouse ./dev.sh run
SEXUSB_QEMU_DEVICE=kbd+tablet ./dev.sh run
```

**Build authority is sealed** to `sexos_build_spec.toml` + `scripts/entrypoint_build.sh`.
Legacy paths (`build_payload.sh`, `make iso`, `make run-sasos`) are INVALID and must fail.
Do NOT use them.

**Rust toolchain**: nightly, `rust-src` component, target `x86_64-unknown-none`.
Custom target spec: `x86_64-sex.json` (in repo root).

---

## 2. Architecture (Must Not Violate)

### Memory Model Hierarchy
```
GLOBAL_VAS  →  defines ADDRESSING only (what can be named)
sex-pdx     →  defines AUTHORITY only  (what can be accessed)
PKU         →  defines ENFORCEMENT     (hardware acceleration only)
```

- GLOBAL_VAS does NOT imply visibility or authority
- sex-pdx is the sole source of access authority — PKU never is
- PKU is an optional hardware accelerator for sex-pdx enforcement
- No layer may substitute for or override the layer above it

### Precedence Chain
```
BOOT_DAG  →  capability genesis   (static, boot-time only)
sex-pdx   →  runtime authority    (all access decisions)
PKU       →  enforcement accel    (hardware acceleration only)
```

### Determinism Invariant
All system state = `f(epoch_id, input_stream, capability_graph)`.
No hidden state. Replay from epoch 0 must produce identical output.

### Protection Keys
```
PKEY 0  — kernel + default (Ring-0, unrestricted)
PKEY 1  — sexdisplay / framebuffer
PKEY 2+ — userland PDs
```

Userland is Ring-3. Kernel is Ring-0. Transition via syscall / iretq.
**Critical**: `wrpkru(0)` ("God Mode") in all interrupt stubs before Ring-0 Rust code.
Never use `core::arch::x86_64::_wrpkru` directly — use `crate::pku::wrpkru`.

---

## 3. Critical Invariants (Will Crash If Violated)

### GDT Order (8-slot limit, 16-byte TSS)
| Slot | Content       |
|------|---------------|
| 0    | Null          |
| 1    | Kernel Code   |
| 2    | Kernel Data   |
| 3 & 4| TSS (must be 16-byte aligned, limit=0x67) |
| 5    | User Data (SS)|
| 6    | User Code (CS)|

- User CS = **0x2B** (index 5 | RPL3). NOT 0x33 — that's TSS → #GP(0x30).
- User SS = **0x23** (index 4 | RPL3).
- SYSRET math: Kernel SS = Kernel CS + 1. User CS = User SS + 1.

### PDX Calling Convention
ALWAYS 5-argument arity: `pdx_call(slot, syscall, arg0, arg1, arg2)`.
Never 4-argument — causes stack misalignment on sysretq/iretq.

### Syscall Return Value Trap
`pop rax` after `call syscall_handler` restores the ORIGINAL rax (syscall number),
NOT the handler return value. To return a value to userland, MUST write
`regs.rax = value` before returning from dispatch. Simply returning from
`dispatch()` does NOT set userland rax.

### User Fault Containment (USER_FAULT_CONTAINMENT_V1)
- Never return to a poisoned user IRET frame
- Fatal user fault → Exited state → redirect to `faulted_task_halt` (kernel mode, IF=1)
- Timer interrupt fires in halt loop, scheduler picks remaining PDs, system continues

### Framebuffer Write Guards
Every framebuffer write path MUST validate `idx < total_pixels` before `write_volatile`.
No exceptions. Missing this = sexdisplay PD1 page fault → clock freeze → black screen.

### PKU Rules
- `serial_println!` in sex-pdx uses direct asm `syscall` rax=69 — NOT a null deref
- Use `const` not `static` for cross-crate shared data (no GOT relocation in ELF loader)
- Kernel entry MUST: `xor eax,eax; xor edx,edx; xor ecx,ecx; wrpkru` (opens ALL keys)

### Interrupts Reading Discipline
Do NOT read `kernel/src/interrupts.rs` whole. Use:
```bash
rg "pattern" kernel/src/interrupts.rs -n
sed -n 'N,Mp' kernel/src/interrupts.rs  # N..M from rg output
```
See `docs/INTERRUPTS_QUICKMAP.md` for full section index with line ranges.

---

## 4. Repository Layout

```
kernel/              — Ring-0 transition arbiter only (trap + forward)
  src/
    interrupts.rs    — IDT, syscall_entry, page_fault_stub, timer handler
    scheduler.rs     — WorkStealingQueue, switch_to
    gdt.rs           — GDT setup (8-slot limit)
    pku.rs           — wrpkru/rdpkru functions
    elf.rs           — load_elf_for_pd (no GOT relocation)
    init.rs          — PD spawning, ELF loading, scheduler start
    syscalls/mod.rs  — Syscall dispatch table
    memory/manager.rs— Heap init, GLOBAL_VAS

crates/
  sex-pdx/           — Global capability substrate (sole authority layer)
  silkbar-model/     — Shared SilkBar model, contract, update queue, ABI types
  sex-graphics/      — Graphics primitives
  sex-object-model/  — Object persistence model
  silk-client/       — Client library for Silk DE

servers/             — All Ring-3 protection-domain servers
  sexdisplay/        — Compositor + scanout (sole FB writer, PKEY 1)
  silk-shell/        — Execution orchestration + desktop shell/window manager (PKEY 6)
  sexinput/          — Input normalization + event routing
  sexusb/            — xHCI driver, HID report parsing
  silkbar/           — Top bar producer (workspace, clock, chips)
  linen/             — File manager
  quil/              — Text editor
  sexfiles/          — VFS / filesystem server
  sexnet/            — Network stack
  sexstore/          — Persistent object store
  sext/              — Demand pager (fault-resolution memory authority)
  sexbell/           — Notification service

apps/
  sexdrive/          — Block-device driver
  kaleidoscope/      — Demo app
  spindle/           — App framework
  purple-scanout/    — Minimal scanout test
  cosmic-*/          — COSMIC DE port apps

docs/
  HANDOFF.md         — Runtime status, regressions, operational notes
  ARCHITECTURE.md    — Canonical SASOS reference
  ROADMAP.md         — Current Silk DE milestone sequence
  INTERRUPTS_QUICKMAP.md — Interrupts.rs section index
  manual_sex.md      — System developer manual (901 lines)
  manual_servers.md  — Server developer manual
  SILK_DE_EXECUTION_PLAN.md — Phase plan + agent prompts
  CREW.md            — Cross-agent collaboration policy
  handoff/           — 200+ per-feature handoff proof documents
```

---

## 5. Current Runtime State (2026-06-06)

### Proven & Stable (from STABLE_BASELINE_20260503.md)
- 33 features verified passing as of 2026-05-03
- All 6 PDs spawn and run (sexdisplay, sexdrive, silk-shell, sexinput, silkbar, linen)
- GUI visible, SilkBar clock counting continuously
- PD3 silk-shell null-jump is CONTAINED (kernel kills PD3, others continue)
- USB xHCI HID report route working (keyboard + mouse/tablet)
- Click-focus, drag-window, workspace switching, panel toggles all working
- SilkBar contract validates at startup in both producer and renderer
- Top-strip render proof (FNV-1a hash) passes every boot

### Known Limitations
- PD3 root cause (null-jump) is contained, not fixed — real loop fix still needed
- Physical USB button proof is environment-dependent
- Panel visuals are solid-color rects only, no content yet
- Claude-references/ directory has been consolidated into docs/handoff/

### Active Work (git status)
- Modified but uncommitted: dev.sh, sexdisplay, sexinput, sexusb
- Latest commits: USB pointer smoothing, input route proofing, cursor interaction
- Working on: cursor input interactivity, USB pointer delta handling, keyboard cursor

---

## 6. Roadmap (from docs/ROADMAP.md)

| Milestone | Status |
|-----------|--------|
| M1: Contract Lock | DONE |
| M2: Renderer Conformance | IN PROGRESS |
| M3: Deterministic Verification | NEXT |
| M4: Visual Polish + Interaction Stability | PLANNED |

### M3 Goal
Deterministic top-strip render harness (headless, no GUI/QEMU dependency).
Fixed update vectors → golden hash compare → fail fast on contract/render drift.

### Agent Ownership (from SILK_DE_EXECUTION_PLAN.md)
- **Codex**: Contract integration, conformance patches, build-gate wiring
- **Claude**: ABI correctness/invariant audit, assertion recommendations
- **DeepSeekClaude**: Deterministic verification harness architecture + vectors
- **Gemini**: Cross-file slot/index drift audit, mismatch patch list

---

## 7. Debug Quick Reference

### Common Issues
| Symptom | Check |
|---------|-------|
| Black screen | `rg "fault.kill" serial.log` — PD1/PD5 killed? |
| | Missing `validate_deterministic_vectors` pass → sexdisplay hung |
| | Try `-display sdl` not `-display gtk` |
| Clock freeze after 2s | Missing `idx < total_pixels` bounds guard in sexdisplay |
| Null RIP panic | iretq with RIP=0, sysretq with rcx=0, or null fn pointer call |
| Scheduler stall | BUG 5 in BUG_HISTORY.md |
| Clock stops at 19 | STALE_CLOCK_SOURCE_FIX |

### Serial Log Patterns
```bash
# Null-jump kills
rg "fault.kill user_null_jump" serial.log

# Which PDs alive
rg "task.running id=" serial.log | tail -20

# Timer ticks
rg "timer.tick.enter" serial.log | wc -l

# Sexdisplay lifecycle
rg "sexdisplay|pd=1|PD 1" serial.log

# Baseline pass markers
grep -aE "silk.contract|silk.render_proof|click_focus|shell.drag|fault|panic|GP|PF" serial.log | head -200

# Expected pass markers (must ALL appear):
# [silk.contract.validate.ok] version=1
# [silk.render_proof.top_strip.ok]
# [shell.silkbar.click] target=launcher/status/clock/workspace
# NO [fault], [panic], [GP], [PF]
```

### Kernel Source (interrupts.rs)
| Range | What |
|-------|------|
| 131-293 | syscall_entry (naked asm) |
| 295-336 | page_fault_stub (naked asm) |
| 337-360 | general_protection_fault_stub |
| 361-456 | timer_interrupt_stub + handler |
| 458-465 | faulted_task_halt |
| 466-618 | page_fault_handler |
| 620-725 | general_protection_fault_handler |

---

## 8. Boundary Rules (Anti-Scope-Creep)

From NEXT_BOUNDARY_HARDENING_PLAN_V1 and CLAUDE.md:

- **STOP FIRST**: Patches touching USB + shell + display + kernel + sex-pdx together
- **Max 2 major domains** per patch
- Every feature proves exactly ONE boundary
- Every feature must prove: boundary proof, negative proof, integration proof, handoff proof

### Domain Boundaries
| Domain | Allowed | Forbidden |
|--------|---------|-----------|
| USB | Normalized input events only | Policy, FB access |
| sexinput | Normalize + deliver over PDX | Shell/display policy |
| silk-shell | Pointer/focus/placement/panel policy | FB writes, app internals, kernel ABI |
| sexdisplay | Render pixels only | Input policy, app lifecycle |
| sex-pdx | Capability routing | Policy decisions |

---

## 9. Agent Ecosystem

### Other Agents' Files
| Agent | Bootstrap File |
|-------|---------------|
| Claude | `CLAUDE.md` (project root) |
| Gemini | `.gemini/GEMINI.md` |
| Codex/ChatGPT | `CHATGPT.md` (project root) |
| Crew (all) | `docs/CREW.md` |

### Gemini Sub-Agents (`.gemini/agents/`)
- `asm-sniper` — cargo-show-asm for wrpkru/switch_to verification
- `ast-unsafe-tracker` — AST sniper for unsafe Rust + MPK/PKRU patterns
- `elf-surgeon` — LLVM JSON ELF surgeon for binary mapping verification
- `qemu-qmp-interrogator` — Live QMP socket interrogator for CR4/PKRU/CR3
- `symbol-sniper` — Symbol-level extractor for GlobalVas, cap_table, pdx_call
- `sex-debug-driver` — Unified analysis tool (trace, live, analyze, panic)
- `panic-correlator` — Panic analysis + root cause correlation

### Shared Skills (`.agents/skills/` = `.claude/skills/`)
- `caveman` — Ultra-compressed communication mode (lite/full/ultra/wenyan)
- `cavecrew` — Caveman-style subagent delegation (investigator/builder/reviewer)
- `caveman-commit` — Ultra-compressed Conventional Commits
- `caveman-review` — Ultra-compressed PR code review
- `caveman-compress` — Compress memory files into caveman format

---

## 10. Hermes-Specific Workflows

### Before Any Code Change
```bash
# 1. Read the stable baseline
read_file docs/handoff/STABLE_BASELINE_20260503.md

# 2. Check what's modified
git status --short
git diff --stat

# 3. Verify boundary compliance
# No patch touches more than 2 major domains
git diff --stat | grep -cE "kernel/|sex-pdx/|sexdisplay|silk-shell|sexinput|sexusb|apps/"
```

### Working on a Feature
1. Read `CLAUDE.md` — standing orders and invariants
2. Read `HANDOFF.md` — current runtime state and regressions
3. Read relevant handoff docs in `docs/handoff/`
4. Make smallest safe change
5. Build: `./scripts/entrypoint_build.sh`
6. If build passes: `./dev.sh run-nographic 2>/tmp/test.log` + verify markers
7. Document in handoff format: symptom, root cause, invariant, proof command, fix pattern

### Hermes Tool Advantages
- `search_files` > `rg` in terminal — ripgrep-backed, respects .gitignore
- `read_file` > `cat` — paginated with line numbers, won't flood context
- `patch` > `sed` — fuzzy matching, 9 strategies, auto syntax checks
- `terminal` — use for builds, git, QEMU, cargo. Reserve for shell-native operations.
- `delegate_task` — for parallel subagent work (code audit, build verification)

### Creating New Handoff Docs
Pattern: `docs/handoff/FEATURE_NAME_V1.md`
Format:
```markdown
# FEATURE_NAME_V1
- Symptom: ...
- Root cause: ...
- Invariant violated: ...
- Fix: ...
- Proof command: ...
- Files changed: ...
```

---

## 11. OS Surface ID Registry (DO NOT REASSIGN)

| ID  | Surface        | Owner      |
|-----|----------------|------------|
| 0x90| Cursor         | OS (shell) |
| 0x92| Launcher panel | OS (shell) |
| 0x93| Status panel   | OS (shell) |
| 0x94| Clock panel    | OS (shell) |
| 0x95| Bell panel     | OS (shell) |
| 100 | SURFACE_ID_APP | App        |
| 101 | SURFACE_ID_STATIC | App    |
| 200 | SURFACE_ID_LINEN | App     |

---

## 12. Build Spec Key Details

- **abi_version_hash**: `a8545feed4f4a7474be5f631da4118d93c6ef893d33eaa6b2850bc536fe92623`
- **Forbidden env vars**: SEXOS_BUILD_MODE, SEXOS_BUILD_PROFILE, SEXOS_DISABLE_ABI_GUARD, SEXOS_SKIP_CONTRACT, CARGO_FEATURES, CARGO_BUILD_TARGET, RUSTC_BOOTSTRAP
- **Allowed crates**: 16 crates explicitly listed in sexos_build_spec.toml
- **Contract validation**: silkbar and sexdisplay both validate contract at startup

---

## 13. Hermes Memory Notes

Key environment facts for Hermes memory:

- **Project root**: `/home/xirtus_arch/Projects/Sex`
- **User**: xirtus_arch on Linux (7.0.9-zen1-1-zen)
- **Hermes profile**: default
- **This doc**: Read at session start when working in this directory
- **Build**: `./scripts/entrypoint_build.sh` (sealed, no alternatives)
- **Run**: `./dev.sh run` (SDL GUI) or `./dev.sh run-nographic` (headless + serial)
- **Tone**: Caveman-capable. Gemini uses `/caveman ultra` as default. Hermes can use `skill_view('caveman')` if needed.
- **Interrupts**: Never read full file — use rg + targeted reads. See `docs/INTERRUPTS_QUICKMAP.md`.
- **Safety**: `trash` > `rm`. No `rm -rf` on project. No force pushes.
- **No kernel edits without STOP FIRST** — kernel is foundation, changes risk all PDs.
- **No sex-pdx ABI edits without STOP FIRST** — all inter-domain communication depends on it.
- **sexdisplay is sole framebuffer writer** — no other PD may write to FB.
