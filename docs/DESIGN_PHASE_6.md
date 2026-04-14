# Phase 6 Design: SexSD (Sex Software Distribution)

## 🎯 Objective
Create a convenient, source-based distribution system called **SexSD**. Inspired by `xbps-src` (Void Linux) and BSD Ports, this system will automate the process of fetching, patching, and "lifting" sexdrives from existing Linux/BSD repositories into Sex Microkernel Protection Domains (PDs).

## 🏛 Architectural Vision: `sex-src`

The `sex-src` tool will be the primary interface for building the operating system from source. It uses a **Template-based Build System** hosted on GitHub.

1.  **The Template:** A simple script (like `xbps-src` templates) that defines:
    *   `source`: URL to the Linux/BSD sexdrive source.
    *   `patches`: Sex-specific patches to wire the sexdrive to `DDE-Sex`.
    *   `pd_id`: The Protection Domain ID and PKU key assigned to the sexdrive.
2.  **The Registry:** A mapping of **PCI IDs** (x86) and **Device Tree Nodes** (ARM64) to specific `sex-src` templates.
3.  **Binary Caching:** While everything is built from source, successful builds can be "caged" into signed `.spd` (Sex Protection Domain) images for fast deployment.

---

## 🗺 Implementation Roadmap

### 1. `sex-src` Build Infrastructure
- [ ] Create the `sex-src` command-line tool (Python or Go).
- [ ] Implement the `fetch` and `patch` logic to pull from Linux/NetBSD GitHub mirrors.
- [ ] Support **Cross-Compilation** (Targeting both `x86_64-unknown-none` and `aarch64-unknown-none`).

### 2. The sexdrive Registry (`sexdrives.json`)
- [ ] Build a database mapping hardware IDs to templates:
  - `0x10DE:0x2484` $\rightarrow$ `sexdrives/gpu/nvidia-3070`
  - `brcm,bcm2712-gpio` $\rightarrow$ `sexdrives/gpio/rpi5-gpio`
- [ ] Implement a `sex-probe` tool that identifies local hardware and recommends builds.

### 3. Initial "Lifting" Templates
- [ ] **`sexdrives/net/netbsd-stack`**: Lifts the NetBSD TCP/IP stack via Rump.
* [ ] **`sexdrives/gpu/nouveau-rtx`**: Lifts the Linux Nouveau sexdrive for the 3070.
* [ ] **`sexdrives/storage/nvme-generic`**: Lifts the standard Linux NVMe sexdrive.
* [ ] **`sexdrives/rpi5/vc7-gpu`**: Lifts the Raspberry Pi VideoCore VII sexdrive.

### 4. The Sex Installer (`sex-provision`)
- [ ] A tool to generate a bootable UEFI/Pi image containing the kernel + the selected set of `.spd` sexdrive images.
- [ ] Support for "Live Build": Fetch and build the entire OS from a single GitHub clone.

---

## 🧪 Phase 6 Verification
- **Automated Build:** `sex-src pkg nvidia-3070` successfully fetches Linux source, patches it for DDE-Sex, and produces a valid PD image.
- **Hardware Probing:** `sex-probe` correctly identifies the RTX 3070 and the Pi 5's BCM2712 SoC.
- **End-to-End Boot:** A "provisioned" image successfully boots on physical hardware and loads the required lifted sexdrives.
