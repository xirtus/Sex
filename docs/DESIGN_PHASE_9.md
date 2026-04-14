# Phase 9 Design: Desktop Ecosystem & Hardware Parity

## 🎯 Objective
Elevate the Sex Microkernel from a functional distributed system to a fully capable daily driver. This phase focuses on achieving **Hardware Parity** (Networking, Sound) and building a modern **Desktop Ecosystem** supporting Wayland compositors (KDE Plasma, Hyprland, River) and modern applications (Kitty Terminal).

## 🏛 Architectural Vision: The "Lifting" Philosophy Applied

Instead of reinventing the wheel, we will aggressively leverage our **DDE-Sex** (Device Driver Environment) and **Sex-Libc** (POSIX Emulation) layers to bring existing, high-quality software to our high-performance SASOS.

1.  **The Graphics Stack (Wayland/DRM):**
    *   Compositors like **Hyprland**, **River**, and **KWin (KDE)** rely on the Linux DRM/KMS subsystem and Mesa.
    *   We will create a specialized **DRM-Sex PD** that uses DDE to lift the necessary Linux DRM core and hardware-specific drivers (NVIDIA Nouveau, Pi 5 VC4/VC7).
    *   Wayland compositors will run unmodified atop `Sex-Libc` and `DRM-Sex`.
2.  **The Audio Stack (Sound):**
    *   Lift the Linux **ALSA (Advanced Linux Sound Architecture)** core via DDE-Sex into a dedicated **Audio PD**.
    *   Run **PipeWire** or **PulseAudio** as a standard user-space service managed by our `sex-runit` supervisor.
3.  **Connectivity (Ethernet & WiFi):**
    *   Ethernet (e.g., Realtek, Intel IGB) is straightforward via DDE.
    *   WiFi (e.g., Intel `iwlwifi`, Broadcom `brcmfmac` for Pi 5) requires lifting the `mac80211` subsystem and `wpa_supplicant`.

---

## 🗺 Implementation Roadmap

### 1. Hardware Parity: Networking & Sound
- [ ] **Ethernet/WiFi PDs:**
  - Create `sex-src` templates for `iwlwifi` (x86_64) and `brcmfmac` (Pi 5).
  - Lift the `mac80211` wireless stack via DDE-Sex.
  - Integrate with the existing `NetStack` PD.
- [ ] **Audio PD:**
  - Lift the ALSA core and Intel HDA / Broadcom audio drivers.
  - Implement a `sex-runit` service for PipeWire.

### 2. The Graphics Stack (DRM/KMS)
- [ ] **DRM-Sex PD:** Implement the compatibility layer for Linux Direct Rendering Manager.
- [ ] **Mesa Integration:** Ensure Mesa's user-space drivers (Nouveau/V3D) can allocate and map graphics memory (GEM/TTM) via Sex PDX calls.
- [ ] **Wayland Support:** Implement the necessary `AF_UNIX` socket emulation in `Sex-Libc` for Wayland client-server communication.

### 3. The Desktop Experience
- [ ] **Compositors:**
  - Build `sex-src` templates for **River** (dynamic tiling) and **Hyprland** (wlroots-based).
  - Build `sex-src` templates for **KDE Plasma** (KWin).
- [ ] **Applications:**
  - Build `sex-src` templates for the **Kitty** terminal emulator (requires OpenGL/Mesa support).
  - Ensure font rendering (FreeType/Fontconfig) functions correctly over `Sex-Libc`.

---

## 🧪 Phase 9 Verification
- **Connectivity:** The system successfully connects to a WPA2/WPA3 WiFi network using an Intel or Broadcom chipset.
- **Audio:** A test WAV file plays through the physical audio output via the Audio PD.
- **Graphical Desktop:** The system boots directly into **Hyprland** or **River**, and the **Kitty** terminal launches with full GPU acceleration on the NVIDIA 3070.
