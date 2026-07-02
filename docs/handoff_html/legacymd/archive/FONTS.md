# SexOS Font Strategy

## Objective
Provide high-quality font rendering for the terminal, shell, and GUI applications (like ClassiCube) within the Single Address Space.

## Strategy
1.  **Stage 1: PSF (PC Screen Font):**
    *   Implement a minimal PSF2 parser in the kernel `vga` and `srv_drm` modules for basic terminal output.
    *   Status: Functional for boot logging.
2.  **Stage 2: srv_font (User-Space Font Server):**
    *   Create an isolated PD (`srv_font`) that embeds a TrueType rendering engine (e.g., `freetype` port or Rust-native `fontdue`/`rusttype`).
    *   Apps request glyph bitmaps via `safe_pdx_call`.
    *   Utilize SAS zero-copy: The font server renders to a shared memory region (SHM) which the compositor (`srv_wayland`) or client apps can access directly.
3.  **Stage 3: Hardware Acceleration:**
    *   Use Mesa/Gallium to perform GPU-accelerated font rendering for high-DPI displays.

## Font Assets
*   Import standard open-source fonts:
    *   *Fixed-width:* Terminus, JetBrains Mono (for the developer shell).
    *   *Proportional:* Inter (for GUI elements).

---
*Status: Implementing Stage 1, Stage 2 in design.*
