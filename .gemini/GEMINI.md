# Sex Project Mandates (GEMINI.md)

Canonical cross-agent policy is now in [CREW.md](/home/xirtus_arch/Documents/microkernel/CREW.md).
Use this file as Gemini bootstrap/context, but keep shared execution rules aligned to CREW.
Build path is sealed to `scripts/entrypoint_build.sh` with spec authority in `sexos_build_spec.toml`.

## 🌌 Project Vision

Sex is a **Single Address Space Operating System (Sex)** microkernel designed for maximum performance and security on modern hardware.

### Core Tenets:
1.  **Tiny TCB:** Keep the kernel privileged code as small as possible (~5-8 kLOC).
2.  **Hardware-Backed Isolation:** Leverage Intel PKU/MPK and CHERI for domain protection.
3.  **SAS Efficiency:** No TLB flushes on domain context switches.
4.  **Zero-Copy IPC:** PDX and shared memory regions are the primary communication channels.
5.  **Capability-First:** Every resource access is backed by an unforgeable capability.

---

## 🛠 Coding Standards

### 🦀 Idiomatic Rust
- Use safe Rust wherever possible. `unsafe` is a last resort and must be documented with a `// SAFETY: ...` comment explaining the invariant.
- Leverage the type system to enforce security and resource boundaries.
- Follow standard Rust naming conventions (`snake_case`, `PascalCase`).
- Prefer `Option` and `Result` over panics. Kernel panics should be reserved for unrecoverable hardware failures or critical internal inconsistencies.

### 🚀 Performance-First
- Zero-copy is the goal for all data transfers.
- Minimize memory allocations in the kernel. Prefer static allocation or pool-based management where possible.
- Avoid unnecessary synchronization. Prefer lock-free structures or fine-grained locking.

### 💂 Unsafe Invariants
- Every `unsafe` block MUST have a clear, documented invariant.
- For pointer arithmetic and memory manipulation, ensure that boundaries are checked and validated before use.

### 🧩 Modular Architecture
- Keep components decoupled. The kernel should provide generic primitives (PDX, PD management, capabilities) that can be composed in user-space.

---

## 🤝 Project Rules

1.  **Incremental Progress:** Small, verifiable commits. Each change should move us closer to the Phase goals.
2.  **Documentation First:** Update `ARCHITECTURE.md` as new features are designed and implemented.
3.  **Strict Review:** All code must be reviewed for safety and adherence to the Sex vision.
4.  **Minimal Dependencies:** In the kernel, use only essential `no_std` crates. Avoid heavy libraries that increase TCB size.

---

## 🔗 References

- [README.md](../README.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)

## 🤖 Agents & Skills

### Sub-Agents
- **asm-sniper:** cargo-show-asm sniper for `wrpkru` / `switch_to` verification.
- **ast-unsafe-tracker:** Structural AST sniper for unsafe Rust + MPK/PKRU patterns.
- **build-noise-nuker:** RTK-wrapped build commander. Returns ONLY first fatal error.
- **elf-surgeon:** LLVM JSON ELF surgeon for linen binary mapping verification.
- **qemu-qmp-interrogator:** Live QMP socket interrogator for CR4/PKRU/CR3 state.
- **rg-pkey-sweeper:** Lightning-fast PKEY/SLOT sweep across crates/sex-pdx and servers/.
- **stack-demangler:** addr2line + rustfilt stack trace compressor.
- **symbol-sniper:** TokToken-powered symbol-level extractor for GlobalVas, cap_table, pdx_call.
- **codebase_investigator:** Architectural mapping and system-wide dependencies.
- **generalist:** Broad, data-heavy, or turn-intensive batch refactoring tasks.

### Skills
- **caveman:** Ultra-compressed communication mode to save tokens.
- **caveman-review:** Ultra-compressed PR code review comments.
- **caveman-commit:** Ultra-compressed Conventional Commits message generator.
- **compress:** Compress memory files into caveman format.
- **skill-creator:** Guide for creating effective skills.

**Caveman Rules:**
- Always activate Caveman `/caveman ultra` mode at session start. Default intensity: ultra.
- Follow the Caveman SKILL.md rules for all coding, reviewing, commits, compression, and planning unless explicitly told otherwise.

---

---

## 🛠️ Environment Rules

### 🏰 Domain Separation
- **`kernel/`**: Bare-metal `x86_64-sex`. Pure `no_std`. Build via `./build_payload.sh`.
- **`apps/` & `servers/`**: Ring-3 `x86_64-sex`. SASOS-native. Build via `./build_payload.sh`.
- **`tools/sex-debug`**: Host Linux `std`. Ring-3 Linux. Build via `cargo build`.

### 🚀 Build & Run Workflow
1.  **Rebuild All**: `./build_payload.sh` (compiles kernel and modules, stages `iso_root`).
2.  **Package ISO**: `make iso` (creates `sexos-v1.0.0.iso`).
3.  **Execute**: `make run-sasos` (QEMU with PKU enabled).
-   **Kernel Path**: `iso_root/sexos-kernel` (ELF).
-   **Target Spec**: `x86_64-sex.json`.

### 🏰 GDT/TSS Standards (Phase 32)
-   **Layout**: Index 1: KCode (0x08), Index 2: KData (0x10), Index 3: SysretBase (0x18), Index 4: UData (0x20), Index 5: UCode (0x28), Index 6: TSS (0x30).
-   **TSS Requirement**: Must be 16-byte aligned. Limit must be exactly `0x67`. Initial Type must be `0x09` (Available).
-   **KStack Management**: Every `Task` must have a private `kstack_top` written to `TSS.rsp0` during `switch_to`.

---

## 🛠️ Tooling: `sex-debug`
The `sex-debug` orchestrator is a unified single-command analysis tool.
- **`trace`**: Visualize traces, parsing and resolving symbol mappings.
- **`live`**: QMP live socket polling for domain states (RIP, PKRU, CR3).
- **`analyze`**: Anomaly detection logic (domain mismatch, bad RIP, etc) with JSON/text reporting.
- **`panic`**: Unified pass. Runs analysis, outputs root cause, confidence, location, and actionable fix suggestions, followed by TUI visualization with failure highlighting.

---

## 📝 Upcoming Plan: Linen 2Ring Problem
**Objective:** Resolve the 2-ring (Ring 2/Ring 3) boundary enforcement and transition issues within the `linen` domain.
1. **Trace Analysis:** Capture and feed QEMU/Trace logs into `sex-debug panic` to identify exact instruction pointers failing during ring transitions.
2. **State Verification:** Use `qemu-qmp-interrogator` to dump PKRU and CR4 states precisely at the Ring 2 boundary.
3. **Binary Mapping Review:** Use `elf-surgeon` to ensure the `linen` binary is mapped with the correct read/write/execute permissions for Ring 2.
4. **Code Patching:** Employ `ast-unsafe-tracker` to pinpoint the `switch_to` or `wrpkru` missing/misplaced calls and patch the boundary entry point.
