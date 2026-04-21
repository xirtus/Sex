# SexOS / SASOS Build Pipeline v1.0.0
## Phase 19 - Pure PDX Desktop Environment (Silk)

**Status:** Plan sketched — Phase 19 unlocked.

Silk is the official SexOS Desktop Environment, built as pure PDX-native userspace on SexCompositor.

---

## Silk DESIGN & ARCHITECTURE PLAN (pure PDX, zero-copy, no legacy)

### Core Principles (non-negotiable)
- 100% inside SEX microkernel ecosystem
- SexCompositor is the only compositor — Silk extends it via PDX syscalls
- Zero-copy everywhere: windows, panels, animations all hand PFN lists directly
- No Unix sockets, no protocols, no shims — everything is direct pdx_call
- no_std / bare-metal where possible; minimal userspace only for UI logic

### High-Level Components (all new crates under servers/ or new/velvet-*)
1.  **SilkCompositor** (extension of SexCompositor in sexdisplay)
    *   Adds window management, layering, damage tracking
    *   New syscall: `pdx_call(0, 0xDE, ...)` for window create/move/resize/flip
    *   Built-in tiling engine (inspired by Cosmic but native)

2.  **SilkShell** (main DE process — runs as PDX domain)
    *   Panel (top bar with clock, tray, workspaces)
    *   Launcher (fuzzy search, app grid — keyboard + gesture driven)
    *   Notifications / OSD
    *   Gesture recognizer (swipe, pinch — using raw input PDX events)

3.  **SilkClient Lib** (for apps)
    *   Minimal Rust crate for any app to talk to Silk
    *   Example: `velvet_window_create(title, width, height)` → returns window ID
    *   Apps draw directly into their PFN-backed buffer and commit via SexCompositor

4.  **SilkTheme System**
    *   System-wide dark/light + accent colors
    *   Smooth animations (60–240 fps possible because zero-copy)
    *   Cosmic-inspired: blurred backgrounds, rounded corners, fluid scaling

5.  **SilkInput & SilkOutput**
    *   Pure PDX input stack (mouse/keyboard/touch → events to shell)
    *   Multi-monitor support via SexCompositor scanout paths

---

### Phase 19 Execution Steps
1.  Extend `servers/sexdisplay/src/lib.rs` with SilkCompositor layer (window structs + new `pdx_call 0xDE`)
2.  Create `servers/velvet-shell/` as new bare-metal PD (`no_std` main that spawns panels/launcher)
3.  Add `velvet-client` crate for future apps
4.  Update `kernel/src/init.rs` to auto-start `velvet-shell` PD after `sexdisplay`
5.  `cargo check`/`build` everything as pure `x86_64-unknown-none`
6.  Mint ISO → QEMU boot → verify Silk boots over gradients with panel + launcher visible
7.  Mainline push once stable
