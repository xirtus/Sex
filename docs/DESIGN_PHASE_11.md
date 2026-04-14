# Phase 11 Design: GNU Pipeline & Filesystem Parity

## 🎯 Objective
Achieve rapid ecosystem growth by enabling unmodified Linux binaries to run on the Sex Microkernel. This phase also focuses on **Filesystem Parity**, ensuring the system can read and write to all major storage formats (Ext4, Btrfs, FAT32, exFAT, NTFS).

## 🏛 Architectural Vision: "The Linux Mirror"

1.  **Lin-Sex Loader:** A specialized Protection Domain that can load standard Linux ELF binaries. It intercepts Linux system calls and translates them into Sex PDX calls targeting our native servers (sexvfs, sexnet, sext).
2.  **MultivFS Lifting:** Instead of writing complex filesystems from scratch, we will use **DDE-Sex** to lift the Linux filesystem sexdrives.
    *   **Ext4 & Btrfs:** Lifted from the Linux kernel source.
    *   **FAT/exFAT:** Lifted from Linux or BSD.
    *   **NTFS:** Lifted via `ntfs-3g` or the modern Linux `ntfs3` sexdrive.
3.  **GNU Userland:** Use `sex-src` to build and package the standard GNU utilities (`coreutils`, `bash`, `gcc`) using the `sexc` POSIX layer.

---

## 🗺 Implementation Roadmap

### 1. Lin-Sex (Linux Binary Compatibility)
- [ ] **ELF Loader:** Implement a loader that handles Linux-specific ELF headers and dynamic linking (ld-linux.so).
- [ ] **Syscall Translator:** Map Linux x86_64/ARM64 syscall numbers to `sexc` / PDX primitives.
- [ ] **Procfs/Sysfs Emulation:** Implement a virtual sexvfs provider that mimics `/proc` and `/sys` for Linux binary compatibility.

### 2. Filesystem Parity (sexvfs Lifting)
- [ ] **`sexdrives/fs/ext4`**: DDE-Sex template for Linux Ext4.
- [ ] **`sexdrives/fs/btrfs`**: DDE-Sex template for Linux Btrfs.
- [ ] **`sexdrives/fs/fat-exfat`**: DDE-Sex template for FAT/exFAT support.
- [ ] **`sexdrives/fs/ntfs3`**: DDE-Sex template for the modern Linux NTFS sexdrive.

### 3. GNU Toolchain & Repository
- [ ] **`sex-packages` Repo:** Create a public registry for GNU tool templates.
- [ ] **Self-Hosting:** Achieve a state where `gcc` or `clang` running on Sex can compile the Sex Microkernel.

---

## 🧪 Phase 11 Verification
- **Binary Execution:** A standard `ls` or `grep` binary from a Linux distribution runs natively on Sex via `Lin-Sex`.
- **Filesystem Mount:** The system successfully mounts and browses an existing **Btrfs** or **NTFS** partition from the X17R1's internal NVMe.
- **Compiler Test:** The system compiles a "Hello World" C program using a native `gcc` PD.
