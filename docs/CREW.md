# SexOS Crew Operating Document

Canonical collaboration document for Claude, Gemini, and ChatGPT/Codex sessions.
This file is the shared source of truth for execution policy, phase progress, and handoff quality.

## 1) Mission

Build SexOS as a SASOS microkernel with:
- small trusted kernel core
- capability-first authority model
- PKU-backed domain isolation
- deterministic build -> boot -> verify loop

## 2) Non-Negotiables

- Do not bypass build root: `./scripts/entrypoint_build.sh` is the only valid build path.
- Keep kernel security boundaries explicit; document every `unsafe` invariant with `// SAFETY:`.
- Keep syscall/PDX ABI consistent across kernel, servers, and apps.
- Prefer narrow edits over broad rewrites.
- Preserve existing session knowledge; append corrections rather than deleting context.

## 3) Shared Technical Invariants

- GDT/TSS and ring transition logic must remain hardware-valid for x86_64 long mode.
- PKRU writes must go through kernel PKU helpers and preserve domain policy.
- Capability slots and PDX call arity/register mapping must match kernel dispatch expectations.
- Build tooling for host utilities (`tools/sex-debug`) stays separate from bare-metal crates.

If any invariant changes, update this file and the architecture docs in the same change.

## 4) Execution Protocol (All Agents)

1. Read this file first, then read only files needed for the current task.
2. Make the smallest safe change that advances the current phase objective.
3. Run targeted verification for touched scope; run full build/boot loop when kernel/runtime paths change.
4. Capture outcomes in the session log section below.
5. Leave explicit next actions so the next agent can continue without rediscovery.

## 5) Session Log Format (Append-Only)

Use this block format at the end of each work session:

```md
### Session YYYY-MM-DD HH:MM (Agent: Claude|Gemini|Codex)
- Objective:
- Files changed:
- Commands run:
- Result:
- Regressions/risks:
- Next step:
```

## 6) Phase Tracking Rules

- Use one active phase at a time with a concrete exit criterion.
- Store implementation detail in phase docs (`phase*.md`, `docs/*.md`) and keep this file as the operator index.
- If phase intent changes, record old intent and reason for change (do not silently overwrite).

## 7) Agent Adapters

### Claude
- Keep deep invariant memory in `CLAUDE.md`, but treat this file as canonical policy.
- Mirror any newly discovered hard ABI facts into both `CREW.md` and `CLAUDE.md`.

### Gemini
- Keep `.gemini/GEMINI.md` as Gemini bootstrap instructions.
- Do not introduce alternative ABI rules there; reference this file for shared policy.
- **Always activate Caveman `/caveman ultra` mode at session start.** Default intensity: ultra.
- Follow the Caveman SKILL.md rules for all coding, reviewing, commits, compression, and planning unless explicitly told otherwise.

### ChatGPT/Codex
- Use `CHATGPT.md` as bootstrap pointer to this file.
- Apply the same session log and phase tracking rules as other agents.

## 8) Immediate Next Milestone

- ABI drift closure is complete for core kernel/`sex-pdx` IPC surfaces:
  - `PdxListenResult` removed from live IPC paths
  - `r9` listen/call side-channel removed from live IPC paths
  - register-only IPC return contract enforced in code and guard script
  - sealed single entrypoint + linear trace build path enabled
- Current milestone: enforce and maintain deterministic spec-driven execution from:
  - `sexos_build_spec.toml` (single build specification of truth)
  - `scripts/entrypoint_build.sh` (validator/snapshot initializer)
  - `scripts/sexos_build_trace.sh` (spec interpreter)

### Session 2026-04-28 (Agent: Codex)
- Objective: collapse hybrid IPC ABI, remove legacy return paths, and seal build flow.
- Files changed:
  - `kernel/src/syscalls/mod.rs`
  - `crates/sex-pdx/src/lib.rs`
  - `scripts/sexos_pipeline.sh`
  - `sexos_contract.toml`
  - `scripts/entrypoint_build.sh`
  - `scripts/sexos_build_trace.sh`
  - `sexos_build_spec.toml`
  - `Makefile`
  - `.github/workflows/ci.yml`
  - `build_payload.sh`
- Commands run:
  - repo sweeps (`rg`) for ABI drift markers
  - guard execution (`./scripts/sexos_pipeline.sh`, `make abi-guard`)
  - entrypoint/trace execution validation
- Result:
  - core ABI drift closed in kernel + `sex-pdx`
  - sealed single build root established
  - build graph externalized into declarative spec and interpreted linearly
- Regressions/risks:
  - host toolchain differences may still affect optional compile checks (`E0463` in some environments)
  - legacy docs may still contain historical instructions; treat `CREW.md` + build spec as authoritative
- Next step:
  - keep docs aligned with sealed spec and continue runtime validation (`qemu`/`sex-debug`) per commit.

### Session 2026-04-28 17:34 (Agent: Codex)
- Objective: validate the userland syscall boundary after the iret frame fix.
- Files changed:
  - `sexos_build_spec.toml`
  - `kernel/src/interrupts.rs`
  - `servers/sexdisplay/src/main.rs`
  - `purple-scanout/src/main.rs`
- Commands run:
  - `./scripts/entrypoint_build.sh`
  - `qemu-system-x86_64 -m 4G -cdrom sexos-v1.0.0.iso -serial stdio -display none -no-reboot`
  - `objdump -d -Mintel target/x86_64-sex/release/sexdisplay`
  - `objdump -d -Mintel target/x86_64-sex/release/purple-scanout`
- Result:
  - build succeeds
  - both user `_start` functions begin with a syscall probe
  - `syscall_entry` now emits a raw pre-`swapgs` COM1 marker
  - no `syscall.stub.enter.raw`, `syscall.enter`, or `syscall.magic.hit` appeared in the serial grep path yet
- Regressions/risks:
  - the syscall path may still be failing before the first kernel-side log
  - the raw serial marker must be interpreted carefully; an earlier version accidentally printed slice metadata bytes instead of the literal marker
- Next step:
  - have Gemini inspect `kernel/src/interrupts.rs` and decide whether the issue is a bad syscall-entry path, a GS/kernel-stack dependency, or a userland transition that never reaches the `syscall` instruction.
