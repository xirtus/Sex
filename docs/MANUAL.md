# SexOS User Manual

Welcome to **SexOS**, the high-performance, Distributed Single Address Space Operating System (DSAS). This manual provides an overview of the core system commands and services. 

Type `help` in your SexOS terminal at any time to display this reference.

---

## 🛠 `sex-src` (Software & sexdrive Management)

`sex-src` is the primary build and package management system for SexOS. It fetches source code, applies DDE-Sex patches, and compiles packages into isolated Protection Domains (PDs).

*   `sex-src pkg <name>`: Fetch and compile a package or sexdrive (e.g., `sex-src pkg nouveau-rtx`).
*   `sex-src probe`: Analyze your local PCI and Device Tree hardware to recommend sexdrives.
*   `sex-src install <script>`: Launch an interactive installation script (e.g., `sex-src install sex-install`).
*   `sex-provision <target>`: Bundle the compiled kernel and `.spd` (Sex Protection Domain) images into a bootable EFI/Pi disk image.

## 🕹 System Services & Supervisors

SexOS runs traditional monolithic kernel components as isolated, user-space servers.

*   **`sexit`**: The minimalist service supervisor (replaces `systemd`). It ensures that critical PDs stay alive.
    *   *Usage in code:* `sexit::start_service("name", pd_id)`
*   **`sexvfs`**: The Virtual File System registry. Manages mount points for ext4, btrfs, FAT32, and NTFS.
*   **`sext`**: The Global Memory Manager (Pager). Handles demand sexting and Distributed Shared Memory (DSM) across the cluster.
*   **`sexnet`**: The Zero-Copy TCP/IP Network Stack.
*   **`sexnode`**: The Cluster Manager. Discovers other SexOS machines on the network for transparent remote IPC.
*   **`sexdrm`**: The Direct Rendering Manager. Interfaces with your GPU (e.g., NVIDIA 3070) for Wayland compositors.
*   **`sexsound`**: The ALSA-lifted audio server.
*   **`sexinput`**: The libinput-lifted server for mouse and keyboard events.
*   **`sexwifi`**: The wireless networking server.

## 🐧 `Lin-Sex` (Linux Compatibility)

SexOS can run unmodified Linux binaries (like `bash`, `gcc`, `grep`).
*   The `Lin-Sex` loader automatically intercepts Linux system calls and translates them into native high-speed **PDX** calls routed through **`sexc`** (our POSIX emulation layer).
*   Just execute Linux ELF binaries directly from the shell; the kernel handles the rest.

## 🖥 Desktop Environments

SexOS natively supports modern Wayland compositors by providing zero-copy Shared Memory (SHM) and `AF_UNIX` socket emulation.
*   **Supported DEs:** KDE Plasma, Hyprland, River.
*   **Terminal:** Kitty (GPU accelerated via `sexdrm`).
*   *Note:* These can be installed during the `sex-install` setup or compiled later via `sex-src pkg <de-name>`.

---
*SexOS: The Pinnacle of Microkernel Architecture.*
