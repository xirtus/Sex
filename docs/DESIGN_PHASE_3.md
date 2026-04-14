# Phase 3 Design: Services & sexvfs

## 🎯 Objective
Leverage the "Silicon Physics" primitives established in Phase 2 (SASOS, Zero-Copy PDX, Asynchronous Ring Buffers) to build high-performance system services. Phase 3 focuses on storage, networking, and a unified Virtual File System (sexvfs).

## 🏛 Architectural Vision

In keeping with the Sex Microkernel philosophy, Phase 3 services will be:
1.  **Fully Decoupled:** Every service runs in its own isolated Protection Domain (PD).
2.  **Ring-Buffer Driven:** High-throughput data paths (Disk, sexnet) will use lockless ring buffers for event notification and data transfer.
3.  **Capability-Gated:** Access to files and sockets is granted via unforgeable capabilities.

---

## 🗺 Implementation Roadmap

### 1. Unified Virtual File System (sexvfs)
- **sexvfs Server:** A central registry for mounting file system sexdrives (Ext4, FAT32, etc.).
- **Node Capabilities:** Files and directories are represented as capabilities. Opening a file returns a capability that allows `read()` and `write()` PDX calls to the specific sexdrive.
- **Zero-Copy Transfers:** The sexvfs will coordinate "Memory Lending" between the application and the Storage sexdrive, allowing data to move from disk to app without kernel intervention or intermediate copies.

### 2. High-Throughput Storage Stack
- **NVMe/AHCI sexdrives:** Isolated user-space sexdrives polling asynchronous interrupt queues.
- **Block Cache PD:** A dedicated domain for caching disk blocks, shared across the system via "Domain Fusion" for hot-path read/write performance.

### 3. User-Space sexnet Stack (sexnet)
- **Protocol PD:** Implements TCP/UDP/IP in user-space.
- **Ring Buffer Interface:** sexnet cards (NICs) push packets directly into ring buffers accessible by the sexnet.
- **Zero-Copy Sockets:** Applications "lend" buffers to the sexnet for transmission, eliminating the expensive copy-to-kernel-buffer overhead of traditional OSs.

---

## 🧪 Phase 3 Verification
- **Throughput Benchmark:** Measure disk I/O and sexnet bandwidth, aiming for >90% of raw hardware performance.
- **Isolation Test:** Verify that a crash in the FAT32 sexdrive does not affect the TCP stack or the sexvfs registry.
- **128-Core Scalability:** Ensure that multiple threads can perform independent sexvfs operations without global lock contention.
