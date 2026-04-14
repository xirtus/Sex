# Phase 7 Design: POSIX & Desktop Foundation

## 🎯 Objective
Transform the Sex Microkernel from a hardware-aware foundation into a usable operating system. This phase focuses on the **User-Space Ecosystem**: providing a POSIX-compliant environment for applications and a high-performance graphical desktop leveraging the NVIDIA 3070 and Pi 5 GPUs.

## 🏛 Architectural Vision: "Lifting the Desktop"

1.  **Sex-Libc (POSIX Emulation):** A lightweight C library (mapping to Rust's `no_std` primitives) that translates standard POSIX calls (`open`, `read`, `write`, `socket`) into Sex **PDX calls** targeting the VFS, NetStack, and Pager.
2.  **Graphical Compositor (Wayland-Lifted):** Instead of writing a display server from scratch, we will "lift" a Wayland-based compositor (like `sway` or `weston`) into a dedicated **Graphics PD**. It will use DDE-Sex to interact with the NVIDIA/VideoCore drivers.
3.  **Global SAS Shell:** A command-line interface that runs in its own PD, allowing users to spawn tasks, manage files via VFS, and monitor the 128-core cluster.

---

## 🗺 Implementation Roadmap

### 1. Sex-Libc (The POSIX Layer)
- [ ] Implement the `syscall` mapping for `sex-libc`.
- [ ] Map `malloc` to the Global SAS Pager.
- [ ] Map `printf/write` to the Serial/VGA PDs via PDX.
- [ ] Port a minimal shell (e.g., `dash` or a custom Rust shell).

### 2. High-Performance Graphics (Wayland PD)
- [ ] Implement the **Graphics PD** (ID 2000, Key 12).
- [ ] Lift the Wayland compositor core using DDE-Sex.
- [ ] **Zero-Copy Framebuffers:** The Graphics PD "lends" VRAM pages directly to applications for zero-copy window updates.

### 3. Application Framework
- [ ] Implement `sex-app`, a high-level Rust framework for building native Sex applications using asynchronous PDX and Ring Buffers.
- [ ] Port a basic Terminal Emulator.

### 4. Self-Hosting (The Ultimate Goal)
- [ ] Port the Rust compiler (`rustc`) and `cargo` to run natively on the Sex Microkernel.
- [ ] Use `sex-src` to build the kernel *from within the kernel*.

---

## 🧪 Phase 7 Verification
- **POSIX Hello World:** A standard C "Hello World" compiled with `sex-libc` runs and prints to the serial console.
- **Graphical Boot:** The system boots into a Wayland-based graphical interface on the NVIDIA 3070 / Pi 5.
- **Interactive Shell:** Users can navigate the VFS and spawn new Protection Domains from an interactive terminal.
