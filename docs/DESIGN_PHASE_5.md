# Phase 5 Design: Hardware Enablement & sexdrives

## 🎯 Objective
Port the Sex Microkernel to physical hardware: **Raspberry Pi 5 (ARM64)** and **Intel x86_64 (X17R1)**. The goal is to move from QEMU to raw silicon while maintaining high-performance SASOS primitives and enabling complex sexdrives (NVIDIA GPU, Pi 5 Peripherals).

## 🏛 Architectural Vision: "DDE-Sex" (Device sexdrive Environment)

To avoid writing sexdrives from scratch, we will implement **DDE-Sex**, a compatibility layer that allows us to "lift" existing sexdrives from Linux and BSD into isolated Protection Domains (PDs).

1.  **Isolated sexdrive Domains:** Each sexdrive (NVMe, WiFi, GPU) runs in its own PD, gated by capabilities.
2.  **Shim Layer:** A "DDE-Sex" shim emulates the minimum required Linux/BSD kernel APIs (kmalloc, request_irq, pci_register) and maps them to Sex PDX calls and Ring Buffers.
3.  **Zero-Copy Hardware Paths:** sexdrives use "Memory Lending" to grant the hardware (DMA) direct access to application buffers, bypassing kernel mediation.

---

## 🗺 Implementation Roadmap

### 1. Hardware Abstraction Layer (HAL) Expansion
- **ARM64 Port (Pi 5):**
  - [ ] Implement ARM64 bootloader integration (U-Boot/RPi Firmware).
  - [ ] Port Memory Management to ARM64 (LPAE Sexting).
  - [ ] Implement ARM GIC (Generic Interrupt Controller) sexdrive.
- **x86_64 (X17R1) Refinement:**
  - [ ] Map NVIDIA 3070 PCI BARs into the Global VAS.
  - [ ] Refine APIC/MSI-X handling for modern Intel chipsets.

### 2. DDE-Sex (The sexdrive Shim)
- **Linux DDE (for NVIDIA/Complex HW):**
  - [ ] Port Genode's `dde_linux` concepts to Sex.
  - [ ] Implement the `create_dummies` tool for Sex-specific symbol resolution.
- **BSD Rump (for Filesystems/Net):**
  - [ ] Implement the `rumpuser` hypercall layer for the Sex Microkernel.
  - [ ] Link NetBSD's TCP/IP and Ext4/ZFS components into user-space PDs.

### 3. Specific sexdrive Targets
- **Raspberry Pi 5:**
  - [ ] GPIO/UART sexdrive (Native).
  - [ ] SDHCI/SD Card sexdrive (Lifted via Rump).
  - [ ] VideoCore VII GPU sexdrive (Lifted via DDE).
- **Intel x86_64 (X17R1):**
  - [ ] NVMe Storage (Native/Lifted).
  - [ ] **NVIDIA 3070 (Nouveau/Lifted):** Use DDE to lift the Nouveau or proprietary-shim sexdrive into a GPU PD.
  - [ ] Intel WiFi/Ethernet (Lifted).

---

## 🧪 Phase 5 Verification
- **Pi 5 Boot:** Kernel boots to serial console on physical Raspberry Pi 5.
- **NVIDIA Initialization:** The NVIDIA 3070 is recognized, and basic framebuffer/GPGPU initialization is performed via the GPU PD.
- **Hardware Throughput:** Measure NVMe I/O performance on the X17R1, targeting >90% of raw hardware bandwidth.
- **Cross-Architecture IPC:** Verify that a PDX call on ARM64 performs with similar relative efficiency to x86_64.
