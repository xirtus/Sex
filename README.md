# Sex (Single Environment eXecutive)
Single Address Space Microkernel System

Sex is a from-scratch, high-performance microkernel written in Rust. It is designed to be a tiny, safe, and lightning-fast alternative to traditional monolithic kernels, leveraging modern hardware features like Intel PKU and CHERI to provide memory safety in a single global address space.

## 🚀 Key Features (Phase 4 Status)

- **Tiny Privileged Kernel:** Target size of ~5-8 kLOC for a minimal Trusted Computing Base (TCB).
- **Single Global Address Space (SASOS):** Eliminates traditional address space switching overhead.
- **Hardware-Accelerated Protection Domains:** Uses Intel PKU (Memory Protection Keys) for zero-cost domain switching.
- **Zero-Copy IPC (PDX):** Protected Procedure Calls (PDX) with hardware-enforced isolation.
- **Asynchronous I/O (Zero Mediation):** Lockless, cache-aligned ring buffers for interrupts and IPC events.
- **Capability-Based Security:** Sparse, unforgeable capabilities for granular access control.
- **Distributed & Transparent IPC:** Lockless multikernel architecture scaling across 128 cores and multiple physical nodes.
- **User-Space Everything:** Pagers, VFS, NetStack, Drivers, and Cluster Managers all run in isolated user-space domains.

## 📁 Project Structure

- `kernel/src/`: The core `no_std` microkernel.
  - `ipc.rs`: Hardware-accelerated and transparent remote IPC routing.
  - `ipc_ring.rs`: Lockless SPSC ring buffers for asynchronous events.
  - `capability.rs`: Distributed capability engine and protection domain management.
  - `scheduler.rs`: Per-core, lockless task schedulers.
  - `apic.rs` / `smp.rs`: Multicore discovery and bootstrap (up to 128 cores).
- `kernel/src/servers/`: User-space system servers.
  - `vfs.rs`: Unified Virtual File System with Node capabilities.
  - `storage.rs`: High-throughput, zero-copy storage drivers.
  - `network.rs`: User-space TCP/IP stack with zero-copy sockets.
  - `cluster.rs`: Node discovery and distributed capability management.
  - `pager.rs`: Asynchronous demand paging and large page management.
  - `serial.rs`: Isolated serial driver.
  - `input.rs`: Asynchronous keyboard/input driver.
- `docs/`: In-depth documentation and Phase plans.

## 🛠 Getting Started

### Prerequisites

- **Rust Nightly:** Required for various `no_std` and assembly features.
- **QEMU:** For running and testing the kernel.

### Build and Run

To build and run the kernel in QEMU:

```bash
make run
```

The kernel currently performs a suite of Phase 4 validation tests on boot, including:
1. Formal Capability authorization (Memory, IPC, Node).
2. Cross-domain and **Remote** PDX calls.
3. Unified VFS path resolution and Node capability granting.
4. Asynchronous Page Fault forwarding to the Pager.
5. "Domain Fusion" hot-path optimization.
6. Per-core context switching.
7. Cluster-wide node discovery and capability importing.

## 📚 Documentation

- [**ARCHITECTURE.md**](docs/ARCHITECTURE.md): Full living specification of the kernel.
- [**DESIGN_PHASE_3.md**](docs/DESIGN_PHASE_3.md): Implementation plan for VFS and Services.
- [**DESIGN_PHASE_4.md**](docs/DESIGN_PHASE_4.md): Architecture for distributed clusters.
- [**IPCtax.txt**](IPCtax.txt): Original technical mandates for high-performance IPC.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
