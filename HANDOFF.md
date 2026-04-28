# HANDOFF.md — Phase 35/80 (Deterministic SMP Lattice)

## Current Runtime Handoff: Syscall Entry Probe

This supersedes the older phase guidance for the current debug thread.

### Verified So Far
- The `iretq` blocker is fixed.
- Final user iret frame is correct:
  - `rip=0x40001640`
  - `cs=0x2b`
  - `rsp=0x700000100000`
  - `ss=0x23`
- The user entrypoints in `sexdisplay` and `purple-scanout` are real `.text` code.
- Both user `_start` paths now begin with a minimal syscall probe that avoids user pointers.

### Current Probe State
- `kernel/src/interrupts.rs` now contains a raw, no-stack COM1 marker at the top of `syscall_entry` before `swapgs`.
- The intended boundary markers are:
  - `syscall.stub.enter.raw`
  - `syscall.stub.after.kstack.switch`
  - `syscall.stub.before.dispatch`
  - `syscall.enter`
  - `syscall.magic.hit`
- `./scripts/entrypoint_build.sh` succeeds.
- Boot output still does not show any of the syscall boundary markers in the serial grep path.

### What Gemini Should Check Next
1. Inspect `kernel/src/interrupts.rs` `syscall_entry` for any GS/stack dependency that can prevent the raw serial loop from reaching COM1.
2. Verify whether the `syscall` instruction is being reached but failing before the first kernel-side log.
3. If the raw pre-`swapgs` marker is present but hidden from grep, inspect the serial byte stream directly.
4. If none of the syscall markers appear, the userland transition is still failing before the `syscall` instruction despite the correct iret frame and valid entrypoint bytes.

## Current Status
We have permanently abandoned legacy scheduler stall debugging (Phase 28). The system is now a **closed deterministic SMP execution lattice**.

Our immediate goal is **Milestone 1 (Minimal Silicon Bootstrap)**: Achieving a stable purple framebuffer at 60Hz across multiple cores with zero scheduling, zero locks, and perfect determinism.

## Execution Protocol: The Formalized Emergence Loop
We are executing a strict, manual emergence loop to prevent silent drift and ensure zero Undefined Behavior (UB):

**ARCH → (Gemini prove) → CODE → (Gemini verify) → RUN → REPEAT**

### Roles:
- **Gemini CLI (Local Agent):** Verifier, invariant checker, and architecture validator. Not the primary code generator.
- **Claude (External Agent):** Code generator and workspace scaffold creator. Not the architect.

### The Loop:
1. **ARCH (Completed):** `ARCHITECTURE.md` is the single source of truth.
2. **Gemini Prove (Step C):** Gemini validates the spec sanity (acyclic DAG, single-writer IPC, no shared mutable state, valid PKU).
3. **CODE (Step D):** Claude generates the Rust code against strict constraints (no Vec, no Box, no Mutex, no threads, no async).
4. **Gemini Verify (Step E):** Gemini analyzes the Claude-generated crates for hardware limits, memory violations, and UB using specific agents (`ast-unsafe-tracker`, `sasos-memory-violator`, `pkru-timeline-reconstructor`).
5. **RUN:** `make iso && make run-smp`.

## Immediate Next Step (Handoff)
- The current task is not the old Milestone 1 pre-pass. It is the syscall-entry boundary probe after the iretq fix.
- Continue from `kernel/src/interrupts.rs`, `kernel/src/syscalls/mod.rs`, `servers/sexdisplay/src/main.rs`, and `purple-scanout/src/main.rs`.
- The next concrete question is whether the user reaches `syscall_entry` at all, or whether the failure happens before the syscall instruction.
