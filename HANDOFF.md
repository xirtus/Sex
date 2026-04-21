
# SexOS SASOS Handoff: Silk Desktop Environment - Phase 19
## ℹ IMPORTANT: Working Directory                                               │
│ 43 + All work is strictly constrained to the directory:                               │
│    `/home/xirtus_arch/Documents/microkernel`. 

## 🛑 THE MANDATE: LEGACY IS DEAD (Continuing from Phase 18)
SexCompositor now replaces PdxCompositor. We are building a pure-PDX zero-copy path `SexCompositor` as a Pdx system that will solve issues from Orbital, Wayland, Smithay, or Cosmic. Those are legacy compositors that speak their own protocols (DRM-like messages, `wl_*` over Unix sockets) and cause allocator loops and `std` leakage. 
**The entire point of the pure-PDX zero-copy path (Phase 18) is to permanently excise every trace of them.**

## 🏗️ THE NEW ARCHITECTURE: SexCompositor & Silk Shell
The `SexCompositor` stub currently in the `sexdisplay` full `lib.rs` replacement is the final native object. 
* It is a minimal, `no_std`, zero-copy PDX-native object.
* It lives entirely inside the SEX microkernel’s display server (`sexdisplay`).
* It **does not** wrap or inherit from any external compositor — **it is the compositor.**
* **Mechanism:** Zero-copy frame commit happens via direct PDX syscalls (`pdx_call`) that hand off PFN (physical frame number) lists straight to the kernel’s MMIO/scanout path.
* **Constraints:** No protocol translation. No shared-memory shims. No allocator loops. No legacy crates.

### Key File Locations:
*   **`sexdisplay` Crate Location:** `servers/sexdisplay/`
*   **`SexCompositor` Definition (and `handle_pdx_call` implementation):** `servers/sexdisplay/src/lib.rs`
*   **`sex-pdx` Crate Location:** `crates/sex-pdx/`

### Completed ###
1. KERNEL & PDX IPC
Memory/Syscalls: PdxMapMemory, PdxAllocateMemory, PDX_SEX_WINDOW_CREATE, PDX_GET_TIME, PDX_SWITCH_VIEW, PDX_GET_ALL_VIEWS, PDX_SILKBAR_*.
Structs/Fixes: Added SexWindowCreateParams, SilkbarRegisterParams. Fixed pdx_commit_window_frame 5-arg ABI.
2. SEXCOMPOSITOR & GRAPHICS
Core: WindowState, dynamic advanced tiling (Master/Stack/V/H, ratios, gaps), window decorations/borders.
sex-graphics: no_std shared crate, WindowBuffer, draw primitives, 8x8 CP437 font engine.
Input Pipeline: sexdisplay forwards sexinput HID events to focused PD.
3. SILK SHELL (BARE-METAL PDX CLIENT)
Capabilities: Real HIDEvent PDX dequeuing, dynamic FB layout queries, workspace switching, window move/resize, app launching (via sex-ld).
4. USERLAND FLEET (APPS & DAEMONS)
silkbar: 40px Catppuccin panel. 3-zone layout (launcher, task, tray). Handles HID clicks & applet registration.
silknet: Tray applet + 360x480 GUI. WireGuard/WiFi toggles via sexnet PDX calls.
tatami: 900x640 Settings daemon (Display, Network, Sound, Input, Capabilities).
toys: Desktop widget framework (Clock, CPU sparkline, RAM, Calendar).
5. INFRASTRUCTURE
Restored .cargo/config.toml. Re-established workspace build rules. Planned gesture & modern UI (blur/animations) architecture.
6. Wire `PDX_GET_TIME` / `PDX_GET_CPU_USAGE` / `PDX_GET_MEM_USAGE` in kernel
7. Add sexnet PDX server (SEXNET_GET_STATUS, SEXNET_SCAN_WIFI, etc.)
8. silkbar workspace switching (PDX_SWITCH_VIEW integration)
9. Real HID event routing from sexinput → silkbar
11. USERLAND FLEET: sexsh v2, linen, silkbar, tatami completion.

### COMPLETED (Phase 19) ###
1. KERNEL & PDX IPC
Memory/Syscalls: PdxMapMemory, PdxAllocateMemory, PDX_SEX_WINDOW_CREATE, PDX_GET_TIME, PDX_SWITCH_VIEW, PDX_GET_ALL_VIEWS, PDX_SILKBAR_*.
Structs/Fixes: Added SexWindowCreateParams, SilkbarRegisterParams. Fixed pdx_commit_window_frame 5-arg ABI.
2. SEXCOMPOSITOR & GRAPHICS
Core: WindowState, dynamic advanced tiling (Master/Stack/V/H, ratios, gaps), window decorations/borders.
sex-graphics: no_std shared crate, WindowBuffer, draw primitives, 8x8 CP437 font engine.
Input Pipeline: sexdisplay forwards sexinput HID events to focused PD.
3. SILK SHELL (BARE-METAL PDX CLIENT)
Capabilities: Real HIDEvent PDX dequeuing, dynamic FB layout queries, workspace switching, window move/resize, app launching (via sex-ld).
4. USERLAND FLEET (APPS & DAEMONS)
silkbar: 40px Catppuccin panel. 3-zone layout (launcher, task, tray). Handles HID clicks & applet registration.
silknet: Tray applet + 360x480 GUI. WireGuard/WiFi toggles via sexnet PDX calls.
tatami: 900x640 Settings daemon (Display, Network, Sound, Input, Capabilities).
toys: Desktop widget framework (Clock, CPU sparkline, RAM, Calendar).
sexsh v2: GPU-accelerated terminal.
linen: Dual-pane file manager.
5. INFRASTRUCTURE
Restored .cargo/config.toml. Re-established workspace build rules. Planned gesture & modern UI (blur/animations) architecture.
6. Wire `PDX_GET_TIME` / `PDX_GET_CPU_USAGE` / `PDX_GET_MEM_USAGE` in kernel
7. Add sexnet PDX server (SEXNET_GET_STATUS, SEXNET_SCAN_WIFI, etc.)
8. silkbar workspace switching (PDX_SWITCH_VIEW integration)
9. Real HID event routing from sexinput → silkbar
10. linen file manager completion
11. sexsh v2 completion
12. silkbar completion
13. tatami completion

### TODO ###
0. Phase 20: `kaleidoscope` browser (servo port)
1. Phase 21: `qupid` media player (rust-media fork)
2. Phase 22: Media & Automation (`rasta`, `celluloid`, `rosebud`, `snowseed`)
3. Phase 23: `sex-hub` + `sexshop`



### 2. Important Apps (make it actually usable)

- `kaleidoscope` HTML5/JS/Node capable 
    Native Rust browser (servo + custom WebRender fork). Full adblock (uBlock Origin rules baked in + AI classifier), YouTube 4K/60 HDR, Google Docs/Sheets real-time collab, X.com full feature parity, hardware video decode via SexOS DRM. No Electron. Multi-process sandbox using microkernel capabilities.


- `qupid` (pure-Rust VLC clone, all codecs)
    Best foundations we can base qupid on (ranked for SexOS):
    rust-media (pcwalton/rust-media) — Closest thing to a “libvlc for Rust”.
    Designed exactly as a portable media player framework for Servo. Supports containers (MP4/MKV/WebM), codecs (VP8, H.264 via safe paths, Vorbis, AAC), and was built for embedding. Zero-copy friendly. We fork + extend with SexCompositor surfaces + wgpu hardware decode.
    rust-av + oxideav — Pure-Rust multimedia toolkit (demuxers, muxers, primitives). Active 2026 crates for AV1/VP9/HEVC parsing. We build the player UI on top using our sex-graphics crate and Silk zero-copy frames.
    Symphonia (pure-Rust audio decoders + container support) + minimal video layer from video-rs or tarang. Excellent for audio-first, then layer video.

    What we will NOT use:
    GStreamer or ffmpeg-next (mature but C/FFI-heavy — violates pure-Rust SexOS mandate).
    Tauri/Electron wrappers (bloat).
    mpv/libVLC bindings.

    qupid plan (pure-Rust, SexOS-native):
    Backend: rust-media fork + rust-av/oxideav codecs.
    Frontend: Native silk-client + SexCompositor zero-copy surfaces + wgpu GPU decode/encode.
    Goal: All major codecs (AV1, HEVC, VP9, Opus, etc.) via safe Rust paths or minimal approved FFI only where hardware demands it.
    sex-forge new qupid --base rust-media

    This keeps us 100% aligned with the microkernel + Silk purity.




### 3. Media & Automation Kickoff

- `rasta` (Photoshop/GIMP pure-Rust clone — start stub)  
- `celluloid` (Kdenlive pure-Rust clone — start stub)  
- `rosebud` (right now tuxedo is our DDE linux driver tool, we will create rosebud as auto translation / i18n pipeline)  
- `snowseed` (Proton-style Linux ELF / Steam compatibility layer)

- `sexhub`sex-hub` the **client** (GUI app store front-end).  
  Native Silk app that users open → browse, one-click install, “native Rust” badge system, integrates with `sex-pkg`. Ships as part of the ISO. - **`sexshop`** = the **server** (backend service).  
  Dedicated PDX service that runs the actual package index, metadata, signing, mirror sync, and secure download daemon.  
  `sex-hub` talks to `sexshop` over PDX (zero-copy).  
  Enables community repos, verified builds, and future “sexshop publish” for devs.


### begin import of First wave of Redox + COSMIC ports ###
(Files, Editor, Calculator) live in **sex-hub**. Full dogfooding of Silk as daily driver.




## ℹ️ IMPORTANT: Working Directory
All work is strictly constrained to the directory: `/home/xirtus_arch/Documents/microkernel`.
