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
- **Interrupt Capabilities:** Allow a user-space driver to respond to hardware interrupts via ring buffers.
- **Domain Capabilities:** Allow management and configuration of Protection Domains.

---

## 🖥 User-space Servers

All traditional OS services run in isolated user-space Protection Domains.

| Server | Responsibility |
| --- | --- |
| **Pager** | Manages the global VAS and handles asynchronous page faults via large pages. |
| **Capability Server** | Central authority for capability policy, distributed for local core scaling. |
| **VFS** | Unified file system interface (Phase 3). |
| **Network** | User-space TCP/IP stack (Phase 3). |
| **Drivers** | Isolated hardware management (Serial, Input, etc.). |

---

## 🗺 Roadmap

### Phase 0: Bootstrap (Complete ✅)
- [x] Bootable kernel skeleton (`no_std`, `no_main`).
- [x] Basic HAL for x86_64.
- [x] VGA/Serial console output.
- [x] Bootloader integration (UEFI).

### Phase 1: Core Primitives (Complete ✅)
- [x] Physical memory manager (Frame allocator).
- [x] Paging and Global VAS setup.
- [x] Protection Domain management (PKU/MPK).
- [x] Basic PDX implementation (synchronous).
- [x] Page Fault Forwarder (Prestep).

### Phase 2: Capabilities & Servers (Complete ✅)
- [x] Formal Capability Engine implementation.
- [x] User-Space Pager Server (Asynchronous demand paging).
- [x] SMP Boot (128-core discovery & signaling).
- [x] Asynchronous Interrupt management (Ring Buffers).
- [x] First user-space driver (Serial/Input).
- [x] Domain Fusion & Revocation.

### Phase 3: Services & VFS (Complete ✅)
- [x] VFS implementation.
- [x] IPC-based storage drivers (NVMe).
- [x] Network stack (user-space).

### Phase 4: Distribution (Complete ✅)
- [x] Transparent networked IPC.
- [x] Cluster management and node discovery.
- [x] Distributed Capability management.

### Phase 5: Hardware & Drivers (Complete ✅)
- [x] ARM64 Design (Raspberry Pi 5).
- [x] DDE-Sex Shim (Linux/BSD driver lifting).
- [x] NVIDIA 3070 GPU PD (Nouveau-lifted skeleton).
- [x] Pi 5 Peripheral support design.

### Phase 6: SexSD (Distribution & Builds) (Next 🚀)
- [ ] `sex-src` build system (xbps-src style).
- [ ] Central Hardware-to-Driver Registry.
- [ ] Provisioning tools (Flashable UEFI/Pi images).
- [ ] Source-to-PD lifting pipeline.
