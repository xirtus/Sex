# SASOS Project Mandates (GEMINI.md)

## 🌌 Project Vision

SASOS is a **Single Address Space Operating System (SASOS)** microkernel designed for maximum performance and security on modern hardware.

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
3.  **Strict Review:** All code must be reviewed for safety and adherence to the SASOS vision.
4.  **Minimal Dependencies:** In the kernel, use only essential `no_std` crates. Avoid heavy libraries that increase TCB size.

---

## 🔗 References

- [README.md](../README.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)
