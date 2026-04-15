# Sex (Single Address Space Operating System) Architecture

## 🏛 Kernel Core

Sex is a **microkernel**, meaning it implements only the most fundamental primitives required to build a complete operating system. The goal is to keep the privileged code within a **Tiny Trusted Computing Base (TCB)** of roughly 5-8 kLOC.

### 🌟 Design Principles
- **Global Single Virtual Address Space (SAS):** Every thread, process, and server shares the same 64-bit virtual address space.
- **Hardware-Enforced Protection:** Isolation is achieved through hardware features like Intel PKU/MPK and CHERI (where available), rather than page table switching.
- **Zero-Copy IPC:** High-performance messaging through shared memory regions and Protected Procedure Calls (PDX).
- **Capability-Based Security:** All system resources (memory, IPC ports, interrupts) are managed via unforgeable capabilities.
- **Asynchronous I/O (Zero Mediation):** Hardware signals are pushed into lockless ring buffers, eliminating context switch jitter.

---

## 🔒 Memory & Protection Domains

### 🌏 Global Single Virtual Address Space (SAS)
In Sex, all data is mapped into a single 64-bit address space. This removes the need for page table switching (and the resulting TLB flushes) when moving between protection domains. Addresses are globally unique across the entire system.

### 🛡 Protection Domains (PDs)
Isolation is enforced by **Protection Domains (PDs)**. Each domain represents a logical set of permissions (Read, Write, Execute) over specific address ranges.
- **Intel PKU / MPK:** Sex uses Intel Memory Protection Keys (PKU) to assign a 4-bit "key" to each page. A hardware register (PKRU) defines the current thread's access rights for each of the 16 available keys.
- **Atomic PKRU Management:** Domain masks are managed via atomic operations to ensure thread-safe switches in multicore environments.

---

## ⚡ IPC Primitives: PDX & Ring Buffers

### 🚀 Protected Procedure Calls (PDX)
The core IPC mechanism is a synchronous **Protected Procedure Call (PDX)**.
1. **Safe PDX:** Validates a capability before performing a hardware-accelerated domain switch.
2. **Fused PDX:** Bypasses capability checks for high-frequency hot-paths between "fused" domains.

### 📦 Asynchronous Ring Buffers
For interrupts and high-volume data transfer, Sex uses lockless, SPSC (Single Producer, Single Consumer) ring buffers. These are cache-line aligned to prevent false sharing on 128-core systems.

---

## 🗝 Capability System

Sex uses a **Sparse Capability** system. A capability is an unforgeable token that proves its holder has the right to access a specific resource.

### 🛠 Capability Types
- **Memory Capabilities:** Define access (R/W/X) to a range of virtual memory via "Memory Lending".
- **IPC Capabilities:** Grant permission to call or listen on a specific PDX port.
- **Interrupt Capabilities:** Allow a user-space sexdrive to respond to hardware interrupts via ring buffers.
- **Domain Capabilities:** Allow management and configuration of Protection Domains.

---

## 🖥 User-space Servers

All traditional OS services run in isolated user-space Protection Domains.

| Server | Responsibility |
| --- | --- |
| **sext** | Manages the global VAS and handles asynchronous page faults via large pages. |
| **Capability Server** | Central authority for capability policy, distributed for local core scaling. |
| **sexvfs** | Unified file system interface (Phase 3). |
| **sexnet** | User-space TCP/IP stack (Phase 3). |
| **sexinput** | Isolated hardware management (Mouse, Keyboard, etc.). |

---

## 🗺 Roadmap

### Phase 0: Bootstrap (Complete ✅)
- [x] Bootable kernel skeleton (`no_std`, `no_main`).
- [x] Basic HAL for x86_64.
- [x] VGA/Serial console output.
- [x] Bootloader integration (UEFI).

### Phase 1: Core Primitives (Stable 🚀)
- [x] Physical memory manager (Bitmap Frame allocator).
- [x] Sexting and Global VAS setup.
- [x] Protection Domain management (PKU/MPK).
- [x] Basic PDX implementation (synchronous).
- [x] **Page Fault Forwarder:** Asynchronous fault handling via `sext`.
- [x] **Robust SMP Boot:** 16-bit to 64-bit Trampoline and INIT-SIPI sequence.

### Phase 2: Capabilities & Servers (Stable 🚀)
- [x] Formal Capability Engine implementation.
- [x] User-Space sext Server (Asynchronous demand paging).
- [x] SMP Boot (128-core discovery & signaling).
- [x] Asynchronous Interrupt management (Ring Buffers).
- [x] **Hardware Drivers:** Functional NVMe (SQ/CQ) and e1000 (RX/TX DMA).
- [x] Domain Fusion & Revocation.

### Phase 3: Services & sexvfs (Stable 🚀)
- [x] **Multi-FS sexvfs:** Unified path resolution for FAT32, Ext4, and Btrfs.
- [x] IPC-based storage sexdrives (NVMe/IDE).
- [x] **Networking:** Functional e1000 driver and `sexnet` protocol stack.
- [x] **User-Land SDK:** `libsys` (sexos.h) and `malloc` runtime.
- [x] **Cross-Toolchain:** `sexos-cc` wrapper for porting complex apps.

### Phase 4: Distribution (Complete ✅)
- [x] Transparent networked IPC.
- [x] sexnode management and node discovery.
- [x] Distributed Capability management.

### Phase 5: Hardware & sexdrives (Complete ✅)
- [x] ARM64 Design (Raspberry Pi 5).
- [x] DDE-Sex Shim (Linux/BSD sexdrive lifting).
- [x] NVIDIA 3070 GPU PD (Nouveau-lifted skeleton).
- [x] Pi 5 Peripheral support design.

### Phase 6: Asynchronous POSIX Signal Trampoline (Complete ✅ (polished on lock-free core))
- [x] Dedicated trampoline stack per-PD (no kernel stack touch).
- [x] Lock-free atomic signal handlers in `sexc` (RCU-style).
- [x] Asynchronous signal routing via `safe_pdx_call`.
- [x] Full `sigaction` ABI support (SA_RESTART, etc.).

### Phase 7: Real Memory Subsystem & Async Page-Fault Forwarding (Complete ✅ (hardened on new lock-free core))
- [x] Lock-free Buddy Allocator (4 KiB / 2 MiB / 1 GiB sharding).
- [x] Hardware-enforced PKU domain management.
- [x] Asynchronous #PF forwarding to standalone `sext` server.
- [x] Demand paging via lent-memory capabilities.

### Phase 8: Full ELF Loader + PD Spawn (Complete ✅ (hardened on lock-free memory))
- [x] Kernel ELF loader: Segment parsing and buddy-allocator mapping (Lock-Free).
- [x] PD creation: Pure PDX-based ELF loading and initial capability grants via RCU Table.
- [x] sys_spawn_pd: Fully asynchronous spawn path.

### Phase 9: Driver Enablement (Storage + Input) (COMPLETE ✅)
- [x] Production-grade NVMe/AHCI driver in standalone PD.
- [x] Zero-copy DMA via Lent-Memory capabilities.
- [x] MSI-X interrupt routing to driver SPSC rings.
- [x] Polished Input stack (PS/2 + USB HID) with TTY routing.
- [x] sexvfs Block I/O dispatch integration.

### Phase 10: Refactor: Decoupling & Lock-Free Core (COMPLETE ✅)
- [x] All monolithic servers decoupled into standalone `no_std` ELFs in `/servers/`.
- [x] Eradicated Mutex/RwLock from core kernel (`scheduler.rs`, `capability.rs`, `memory.rs`).
- [x] Wait-free FLSCHED runqueues and capability tables.
- [x] Pure PDX and lent-memory IPC routing established.

### Phase 11: Signal Delivery (Complete ✅)
- [x] Signal Trampoline: COMPLETE
- [x] Lock-free message-based signal routing over per-PD rings.
- [x] sexc-owned POSIX signal dispatch without kernel stack hijacks.

### Phase 11: GNU Pipeline & Filesystem Parity (Complete ✅)
- [x] Lin-Sex (Linux Binary Compatibility).
- [x] Multi-Filesystem support (Ext4, Btrfs, FAT, NTFS).
- [x] GNU Toolchain integration (GCC/Bash).
- [x] `sex-packages` repository design.

### Phase 12: Dynamic Translators & URL-Based DSAS (Complete ✅)
- [x] Hurd-style VFS Translators (PD-to-Node attachment).
- [x] Redox-style URL Schemes (sexnet://, sexdrm://).
- [x] Dynamic On-Demand Translation.
- [x] Global URL-driven routing.

### Phase 13: Native Self-Hosting (Complete ✅)
- [x] `sexc` Self-Hosting extensions (spawn_pd, mmap, stat).
- [x] Developer PD & Spawn Capability.
- [x] Rust toolchain porting design.
- [x] Native build demonstration.

### Phase 14: The Sex-Store (Next 🚀)
- [ ] Graphical Sex-Store Application.
- [ ] Package browsing & one-click sexting.
- [ ] Binary caching & SPD image management.
- [ ] User-contributed sexdrives and apps.

---

## 🏆 The Vision Realized
The Sex Microkernel project has successfully evolved from a single-core bootloader into a high-performance, distributed Single Address Space Operating System. By leveraging Intel PKU for zero-cost isolation and a global 64-bit VAS, we have created a platform that treats a sexnode of machines as one unified, secure, and lightning-fast computer.
