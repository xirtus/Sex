# SexOS: Roadmap to "Daily Driver" Status

This document outlines the strategic progression from a microkernel architectural prototype to a production-ready, AI-native, high-performance operating system.

## 🏁 Completed Milestones

### Step 1: The "Hello World" Userland (Foundation) - **COMPLETE ✅**
*   **PMM:** Robust `BitmapFrameAllocator` for physical memory recycling.
*   **Paging:** Dynamic 64-bit Single Address Space (SASOS) management.
*   **Binary Execution:** Real ELF64 loader mapping segments into memory.
*   **Privilege Levels:** Ring 3 support via GDT expansion and `iretq` context switching.
*   **Scheduling:** Preemptive APIC-timer-driven round-robin scheduler with Wait Queues.

### Step 2: Basic I/O and Storage (Physical Control) - **COMPLETE ✅**
*   **Hardware Discovery:** Functional PCI/PCIe enumeration engine.
*   **Storage Drivers:** Real PIO-based IDE/ATA driver.
*   **Filesystems:** Functional FAT32 and Ext2 parsing (BPB and Inode logic).
*   **VFS:** Client-Side Path Resolution (CSPR) and Hurd-style active translators.

---

## 🚀 Current & Future Milestones

### Step 3: Interactive Shell & Libc (Userland Alpha) - **COMPLETE ✅**
*   **Drivers:** Real PS/2 keyboard/mouse scancode decoding in `sexinput` wired to TTY.
*   **Libc:** `relibc` (Rust-native) ported to SexOS `sexc` PDX interface with real file ops.
*   **CoreUtils:** Mature `sexc` bridge unblocks GNU `bash`, `ls`, `cat`, and `grep`.
*   **Milestone:** Boot into a blinking cursor, type `ls`, and see files on a real physical disk.

### Step 4: Networking & Display (High-Throughput I/O) - **IN PROGRESS 🏗️**
*   **Driver:** Real Intel e1000 Gigabit driver with DMA ring initialization.
*   **Network Stack:** Asynchronous Zero-Copy `sexnet` stack (sDDF model) with ARP and IPv4 logic.
*   **Graphics:** `sexdrm` with real VESA/GOP framebuffer mapping and GEM buffer management.
*   **Milestone:** The system can `ping` an IP address and render high-resolution 32bpp graphics.

### Step 5: The Desktop Environment (The Modern OS) - **PLANNED 📅**
*   **Inter-Domain SHM:** Implement `mmap` shared memory for zero-copy pixel transfer between App and Compositor.
*   **Wayland Lifting:** Use `sex-lift-ai` to port a Wayland compositor (e.g., `river` or `sway`) and the `Mesa` library.
*   **Input Translation:** Connect `sexinput` event rings to the Wayland server.
*   **Milestone:** A graphical desktop with a functional terminal emulator (e.g., Alacritty) and window management.

### Step 6: Self-Hosting & Ecosystem (The Pinnacle) - **PLANNED 📅**
*   **Toolchain:** Port `rustc` and `gcc` to the `sexc` layer.
*   **Package Management:** Full-scale deployment of the **Sex-Store** graphical manager.
*   **Mass Driver Lifting:** Achieved hardware parity by lifting Nouveau (NVIDIA), AMDGPU, and Intel Wifi using the `sex-lift-ai` pipeline.
*   **Apps:** Native ports of modern browser engines (Webkit/Blink), text editors (Vim/VSCode), and Git.
*   **Milestone: DAILY DRIVER.** Use SexOS to write, compile, and commit the next version of SexOS.

---

## 💎 The "SexOS Advantage" (Unique Selling Points)
1.  **AI-Native Bootstrapping:** Gemini AI is embedded in the early-boot shell to help users discover hardware and lift drivers on the fly.
2.  **SASOS Performance:** Single Address Space eliminates the overhead of page table swapping during IPC.
3.  **sDDF Zero-Copy:** High-throughput storage and networking via lock-free descriptor rings.
4.  **Hardware-Enforced Isolation:** Intel PKU ensures zero-cost context switching between drivers and apps.
5.  **Runit-Style Stability:** Minimalist, dependency-aware service management with automated reincarnation.
