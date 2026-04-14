# Phase 9 Design: Desktop Ecosystem & Hardware Parity

## 🎯 Objective
Elevate the Sex Microkernel from a functional distributed system to a fully capable daily sexdrive. This phase focuses on achieving **Hardware Parity** (Networking, Sound) and building a modern **Desktop Ecosystem** supporting Wayland compositors (KDE Plasma, Hyprland, River) and modern applications (Kitty Terminal).

## 🏛 Architectural Vision: CAPIO & The "Slicer"

Following the **CAPIO (Capability-based I/O)** architecture, sexdrives in Sex are not loaded into the kernel. Instead, they run as isolated compartments in the SASOS.

1.  **Device Manifests:** The kernel stub handles initial device discovery (e.g., PCIe probing for the NVIDIA 3070). It defines a **Device Manifest** that partitions MMIO registers into "Safe Data Plane" and "Privileged Control Plane" (e.g., DMA config).
2.  **The Slicer:** When a userspace sexdrive (like the NVIDIA PD) starts, the **Slicer** intercepts mapping requests and issues unforgeable hardware capabilities for the specific **byte-slices** of the device MMIO defined in the manifest.
3.  **Kernel-Bypass Data Plane:** Once capabilities are issued, the sexdrive communicates with the hardware natively. The core microkernel is never involved in the data transfer, achieving raw hardware performance.
4.  **IOMMU-Enforced Zero-Copy DMA:** For massive transfers (textures, packets), the system uses an IOMMU to restrict device access to specific "lent" memory capabilities, ensuring safety without intermediate copies.

---

## 🗺 Implementation Roadmap

### 1. CAPIO Infrastructure
- [ ] **Kernel Discovery Stub:** Implement PCIe/DT probing and Manifest generation.
- [ ] **The Slicer:** Implement the capability slicer that issues granular MMIO access tokens.
- [ ] **IOMMU Manager:** Integrate IOMMU protection for zero-copy DMA lending.

### 2. Hardware Parity: Networking & Sound
- [ ] **Ethernet/sexwifi PDs:**
  - Create `sex-src` templates for `iwlwifi` (x86_64) and `brcmfmac` (Pi 5).
  - Lift the `mac80211` wireless stack via DDE-Sex.
  - Integrate with the existing `sexnet` PD.
- [ ] **sexsound PD:**
  - Lift the ALSA core and Intel HDA / Broadcom audio sexdrives.
  - Implement a `sexit` service for PipeWire.

### 2. The Graphics Stack (sexdrm/KMS)
- [ ] **sexdrm PD:** Implement the compatibility layer for Linux Direct Rendering Manager.
- [ ] **Mesa Integration:** Ensure Mesa's user-space sexdrives (Nouveau/V3D) can allocate and map graphics memory (GEM/TTM) via Sex PDX calls.
- [ ] **Wayland Support:** Implement the necessary `AF_UNIX` socket emulation in `sexc` for Wayland client-server communication.

### 3. The Desktop Experience
- [ ] **Compositors:**
  - Build `sex-src` templates for **River** (dynamic tiling) and **Hyprland** (wlroots-based).
  - Build `sex-src` templates for **KDE Plasma** (KWin).
- [ ] **Applications:**
  - Build `sex-src` templates for the **Kitty** terminal emulator (requires OpenGL/Mesa support).
  - Ensure font rendering (FreeType/Fontconfig) functions correctly over `sexc`.

---

## 🧪 Phase 9 Verification
- **Connectivity:** The system successfully connects to a WPA2/WPA3 sexwifi sexnet using an Intel or Broadcom chipset.
- **sexsound:** A test WAV file plays through the physical audio output via the sexsound PD.
- **Graphical Desktop:** The system boots directly into **Hyprland** or **River**, and the **Kitty** terminal launches with full GPU acceleration on the NVIDIA 3070.
