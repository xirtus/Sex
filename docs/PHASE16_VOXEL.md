# Phase 16: The Voxel & Multimedia Milestone

## Objective
Enable high-performance multimedia and 3D voxel rendering on SexOS by porting **Classic DOOM** and **ClassiCube**.

## Target Applications
1.  **DOOM (Chocolate DOOM or doom-fbf):**
    *   Requirement: Simple framebuffer or SDL1.2/2.0.
    *   Strategy: Use `sexdrm` directly for the framebuffer or port SDL2 to talk to `srv_wayland`.
2.  **ClassiCube (C Minecraft Classic Clone):**
    *   Requirement: C99, POSIX CRT, SDL2, OpenGL/GLES, OpenAL.
    *   Strategy: Utilize the 100% complete `sexc` POSIX layer. Integrate Mesa (already in `upstream-mesa`) for software/hardware GLES rendering.

## Dependencies
*   **SDL2 SexOS Backend:** A new backend for SDL2 that utilizes `safe_pdx_call` to communicate with `srv_wayland` for input and surface management.
*   **srv_audio:** A minimal audio server utilizing the `e1000` (or future HDA) driver for DMA-based playback.
*   **srv_font:** A font rendering server (see FONTS.md).

## Lessons from vib OS
*   **Vibe Coding:** Utilize integrated Gemini AI (`sex-gemini`) to assist in porting legacy C code to the SexOS environment.
*   **GUI Integration:** Adopt a macOS-inspired clean UI for the system menu, but maintain the high-performance Wayland compositor model for 3D apps.

---
*Status: Future Phase (Queued)*
