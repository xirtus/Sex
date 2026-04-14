# Phase 13 Design: Native Self-Hosting

## 🎯 Objective
Achieve the ultimate milestone for any operating system: **Self-Hosting**. This phase focuses on porting the Rust toolchain (`rustc`, `cargo`, `llvm`) to run natively on the Sex Microkernel using the `sexc` POSIX layer. Success means building SexOS *from within* SexOS.

## 🏛 Architectural Vision: The "Developer PD"

1.  **The Developer Cage:** A high-performance, isolated Protection Domain (PD) that contains the entire compiler toolchain.
2.  **PD Spawning (Process Management):** We will implement a `spawn_pd()` system call in `sexc`. In SexOS, "spawning a process" means creating a new isolated PD, granting it the necessary capabilities (memory, VFS, net), and jumping to its entry point.
3.  **Unified Build VAS:** Since we are a SASOS, the compiler, linker, and the code being built all exist in the same 64-bit address space. This allows the linker to perform "Zero-Copy Linking" by simply updating capability pointers instead of copying massive binary blobs.

---

## 🗺 Implementation Roadmap

### 1. `sexc` Extension (The Compiler Bridge)
- [ ] **Advanced File I/O:** Support for `mmap`, `stat`, and `unlink` targeting `sexvfs`.
- [ ] **PD Lifecycle Management:** Implement `fork()` and `exec()` equivalents that map to PD creation and capability inheritance.
- [ ] **Signal Handling:** Basic signal emulation for compiler error reporting.

### 2. Rust Toolchain Porting
- [ ] **Target Definition:** Create an `x86_64-sexos-unknown` and `aarch64-sexos-unknown` target for LLVM/Rust.
- [ ] **Linker Integration:** Adapt the linker to produce Sex-compatible `.spd` (Sex Protection Domain) images.
- [ ] **Cargo Porting:** Ensure Cargo can fetch dependencies via `sexnet` using our URL resolver (`sexnet://github.com`).

### 3. Build-System Bootstrapping
- [ ] **Stage 1:** Cross-compile `rustc` from Linux to SexOS.
- [ ] **Stage 2:** Run `rustc` on SexOS to build a "Hello World" app.
- [ ] **Stage 3 (Self-Hosting):** Use the native `rustc` to compile the Sex Microkernel source code.

---

## 🧪 Phase 13 Verification
- **Native Compile:** Running `rustc hello.rs` inside a SexOS terminal produces a working binary.
- **Cargo Build:** Running `cargo build` on a SexOS app correctly resolves dependencies and compiles.
- **Full Bootstrap:** The system successfully rebuilds its own `sexvfs` or `sexnet` PD images from source while running.
