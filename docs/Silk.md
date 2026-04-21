# Silk Desktop Environment

## Overview
Silk is the official SexOS Desktop Environment, built as a pure PDX-native userspace on top of the SEX microkernel ecosystem. It focuses on a luxurious, smooth, and modern user experience, inspired by concepts from tiled window managers like Cosmic DE but implemented entirely natively without legacy dependencies or external protocols.

## Core Principles
- **100% SEX microkernel ecosystem:** All components are designed to run within the SEX microkernel's Protection Domain (PD) system.
- **SexCompositor as Foundation:** Silk extends the `sexdisplay` server's `SexCompositor` via direct PDX syscalls. It is the sole compositor.
- **Zero-Copy Everywhere:** All graphical operations, including window management, animations, and buffer handling, utilize PFN (Page Frame Number) lists for direct memory access, eliminating data copies.
- **No Legacy or Shims:** Avoids Unix sockets, IPC protocols, and other traditional OS abstractions in favor of direct `pdx_call` syscalls for inter-process communication.
- **Minimal Userspace:** Aims for `no_std` and bare-metal efficiency where possible, with minimal userspace logic solely for UI and interaction management.

## Components
Silk is composed of several key components, all running as distinct PDs or libraries:

### SilkCompositor
- **Role:** Extends `SexCompositor` (within `sexdisplay`) to provide advanced window management features.
- **Functionality:** Handles window creation, movement, resizing, layering, and damage tracking. Incorporates a built-in tiling engine inspired by modern WMs.
- **Interface:** Utilizes a new PDX syscall: `pdx_call(0, 0xDE, ...)`.

### SilkShell
- **Role:** The main user-facing process, running as a dedicated PD.
- **Functionality:** Manages the user interface, including:
    - **Panel:** Top bar with clock, system tray, and workspace indicators.
    - **Launcher:** Fuzzy search and application grid, driven by keyboard and gestures.
    - **Notifications / OSD:** System notifications and on-screen displays.
    - **Gesture Recognizer:** Processes raw input PDX events for gestures (swipes, pinches).

### SilkClient Library
- **Role:** A minimal Rust crate for applications to interface with the Silk DE.
- **Functionality:** Provides simple functions for creating and managing windows, e.g., `silk_window_create(title, width, height)`.
- **Usage:** Allows applications to draw directly into their PFN-backed buffers, committing updates via `SexCompositor`.

### SilkTheme System
- **Role:** Manages the visual appearance of the desktop.
- **Functionality:** Supports system-wide dark/light modes and accent colors. Enables smooth, high-frame-rate animations (60–240 fps) due to the zero-copy foundation. Features include blurred backgrounds, rounded corners, and fluid scaling inspired by modern design trends.

### SilkInput & SilkOutput
- **Role:** Manages all input and output devices.
- **Functionality:** Implements a pure PDX input stack, routing mouse, keyboard, and touch events to the SilkShell. Supports multi-monitor configurations through `SexCompositor`'s scanout capabilities.

## Development Phase 19 Plan
The initial implementation focuses on establishing the core Silk architecture:
1.  Extend `servers/sexdisplay/src/lib.rs` to integrate the SilkCompositor layer, including new window structs and the `pdx_call 0xDE` syscall.
2.  Create the `servers/silk-shell/` directory as a new bare-metal PD (no_std environment).
3.  Develop the `silk-client` crate for application integration.
4.  Update `kernel/src/init.rs` to automatically start the `silk-shell` PD after `sexdisplay`.
5.  Perform initial `cargo check` and `cargo build` for `x86_64-unknown-none`.
6.  Mint an ISO, boot in QEMU, and verify the visibility of the panel and launcher.
7.  Proceed to mainline push upon achieving stability.

---
