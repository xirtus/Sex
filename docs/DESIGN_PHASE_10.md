# Phase 10 Design: Wayland Ecosystem & Input Lifting

## 🎯 Objective
Enable high-performance, GPU-accelerated Wayland compositors (**KDE Plasma**, **River**, **Hyprland**) to run natively on the Sex Microkernel. This phase builds the "plumbing" required for Wayland IPC and handles human interface devices (HID).

## 🏛 Architectural Vision: Wayland on SASOS

Wayland relies on three main components that we must implement or "lift":
1.  **Wayland IPC (`AF_UNIX` + FD Passing):** Wayland uses Unix Domain Sockets for communication. We will emulate these in `sexc` using our **PDX** primitives. File descriptor (FD) passing will be mapped directly to **Capability Passing** in the SASOS.
2.  **Shared Memory (Wayland-SHM):** Zero-copy buffer sharing between apps and the compositor. We will implement a **SHM sext** that uses "Memory Lending" to grant the compositor access to application-allocated pixels.
3.  **Input Handling (`libinput`):** We will "lift" `libinput` via DDE-Sex into an **Input PD**. It will process raw events from the Pi 5 GPIO/USB and x86 USB HID, pushing them into a ring buffer accessible by the compositor.

---

## 🗺 Implementation Roadmap

### 1. Wayland Plumbing (sexc & sext)
- [ ] **`AF_UNIX` Emulation:** Map `socket()`, `bind()`, and `sendmsg()` to PDX calls.
- [ ] **Capability-to-FD Mapping:** Ensure `sexc` can translate unforgeable capabilities into integer "File Descriptors" for standard C apps.
- [ ] **SHM Provider:** Implement a "Shared Memory" capability type in the sext for zero-copy Wayland buffers.

### 2. Input Parity (libinput PD)
- [ ] **The Input PD:** A dedicated domain (ID 2400, Key 16) running a lifted `libinput`.
- [ ] **USB HID Lifting:** Use DDE-Sex to lift the Linux USB stack and HID sexdrives for mice/keyboards.
- [ ] **Ring Buffer Events:** Push mouse/keyboard events into a cache-aligned SPSC ring buffer for the compositor.

### 3. Graphics & Mesa (The GPU Path)
- [ ] **Mesa Lifting:** Create `sex-src` templates to build Mesa (Nouveau/V3D) for the Sex target.
- [ ] **EGL/GBM Support:** Map Mesa's memory allocation (GBM) to our **sexdrm** PD.

### 4. Compositor Templates (The Big Three)
- [ ] **`sex-src pkg wlroots`**: Build the foundation for River and Hyprland.
- [ ] **`sex-src pkg hyprland`**: The dynamic tiling compositor.
- [ ] **`sex-src pkg river`**: The stack-based tiling compositor.
- [ ] **`sex-src pkg kde-plasma`**: The full KWin-based desktop environment.

---

## 🧪 Phase 10 Verification
- **Wayland Socket Test:** A simple Wayland client successfully connects to a "Hello World" compositor PD via `AF_UNIX` emulation.
- **Input Delivery:** Mouse movements on the x17r1 / Pi 5 are correctly reflected in the Graphics PD's logs via the Input PD.
- **GPU Rendering:** **Hyprland** or **River** launches with full OpenGL acceleration on the **NVIDIA 3070**, rendering its first frame via the sexdrm PD.
