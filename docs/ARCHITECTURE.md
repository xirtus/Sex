# SASOS (Single Address Space Operating System) Architecture

## 🏛 Kernel Core

SASOS is a **microkernel**, meaning it implements only the most fundamental primitives required to build a complete operating system. The goal is to keep the privileged code within a **Tiny Trusted Computing Base (TCB)** of roughly 5-8 kLOC.

### 🌟 Design Principles
- **Global Single Virtual Address Space (SAS):** Every thread, process, and server shares the same 64-bit virtual address space.
- **Hardware-Enforced Protection:** Isolation is achieved through hardware features like Intel PKU/MPK and CHERI (where available), rather than page table switching.
- **Zero-Copy IPC:** High-performance messaging through shared memory regions and Protected Procedure Calls (PDX).
- **Capability-Based Security:** All system resources (memory, IPC ports, interrupts) are managed via unforgeable capabilities.

---

## 🔒 Memory & Protection Domains

### 🌏 Global Single Virtual Address Space (SAS)
In SASOS, all data is mapped into a single 64-bit address space. This removes the need for page table switching (and the resulting TLB flushes) when moving between protection domains. Addresses are globally unique across the entire system.

### 🛡 Protection Domains (PDs)
Isolation is enforced by **Protection Domains (PDs)**. Each domain represents a logical set of permissions (Read, Write, Execute) over specific address ranges.
- **Intel PKU / MPK:** SASOS uses Intel Memory Protection Keys (PKU) to assign a 4-bit "key" to each page. A hardware register (PKRU) defines the current thread's access rights for each of the 16 available keys.
- **CHERI Capabilities:** Where hardware support exists, CHERI (Capability Hardware Enhanced RISC Instructions) provides fine-grained, spatial and temporal memory safety for every pointer.

---

## ⚡ IPC Primitives: PDX & Shared Regions

### 🚀 Protected Procedure Calls (PDX)
The core IPC mechanism is a synchronous **Protected Procedure Call (PDX)**. This allows a thread to "call" a function in another protection domain as if it were a local function.
1. **Transfer:** The kernel switches the thread's Protection Key (PKRU) to the target domain's key.
2. **Execute:** The thread executes code in the target domain.
3. **Return:** The kernel restores the original PKRU.
This mechanism provides zero-copy messaging and near-instantaneous context switching.

### 📦 Shared Memory Regions
For high-volume data transfer, servers can negotiate **Shared Memory Regions**. These are ranges of the global address space that are mapped into multiple domains with appropriate permissions.

---

## 🗝 Capability System

SASOS uses a **Sparse Capability** system, similar to seL4 and Mungi. A capability is an unforgeable token that proves its holder has the right to access a specific resource.

### 🛠 Capability Types
- **Memory Capabilities:** Define access (R/W/X) to a range of virtual memory.
- **IPC Capabilities:** Grant permission to call or listen on a specific PDX port.
- **Interrupt Capabilities:** Allow a user-space driver to respond to hardware interrupts.
- **Domain Capabilities:** Allow management and configuration of Protection Domains.

---

## 🖥 User-space Servers

All traditional OS services run in isolated user-space Protection Domains.

| Server | Responsibility |
| --- | --- |
| **Pager** | Manages the global virtual address space and physical memory allocation. |
| **Capability Server** | Central authority for capability creation, derivation, and revocation. |
| **VFS** | Provides a unified file system interface, delegating to specific file system drivers. |
| **Network** | Implements TCP/IP and other protocols in user-space. |
| **Drivers** | Isolated hardware management (Graphics, Disk, Input). |
| **Init** | The system bootstrap and supervisor server. |

---

## 🛰 Security Model

- **Minimal Kernel:** Reduces the surface area for vulnerabilities.
- **Hardware-Enforced Isolation:** Intel PKU and CHERI provide strong, hardware-backed boundaries between domains.
- **Principle of Least Privilege:** Every domain starts with zero capabilities and must be explicitly granted the minimum set of permissions required.
- **Network Transparency:** Distributed IPC capabilities are managed by the Capability Server, ensuring that security policies are consistent across the entire cluster.

---

## 🗺 Roadmap

### Phase 0: Bootstrap (Current)
- [ ] Bootable kernel skeleton (`no_std`, `no_main`).
- [ ] Basic HAL for x86_64.
- [ ] Simple VGA/Serial console output.
- [ ] Bootloader integration (UEFI).

### Phase 1: Core Primitives
- [ ] Physical memory manager (Frame allocator).
- [ ] Paging and Global VAS setup.
- [ ] Protection Domain management (PKU/MPK).
- [ ] Basic PDX implementation (synchronous).

### Phase 2: Capabilities & Servers
- [ ] Capability engine implementation.
- [ ] Pager Server (demand paging).
- [ ] Interrupt management.
- [ ] First user-space driver (Serial/Input).

### Phase 3: Services & VFS
- [ ] VFS implementation.
- [ ] IPC-based storage drivers.
- [ ] Network stack (user-space).

### Phase 4: Distribution
- [ ] Transparent networked IPC.
- [ ] Cluster management and node discovery.
- [ ] Distributed Capability management.
