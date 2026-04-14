# Phase 8 Design: Distributed SAS & Global Resource Fusion

## 🎯 Objective
Realize the final vision of the Sex Microkernel: transforming a sexnode of independent nodes into a single, logical, and massive **Distributed Single Address Space (DSAS)** machine. This phase focuses on **Global Shared Memory** and **sexnode-Wide Resource Virtualization**, making the sexnet transparent to both the developer and the hardware.

## 🏛 Architectural Vision: "The Global Computer"

1.  **Software Distributed Shared Memory (DSM):** The Global VAS is extended across the sexnet. If a thread on the Raspberry Pi 5 accesses an address owned by the x86_64 workstation, the **Global sext** transparently fetches the page via the sexnet (using RDMA where available).
2.  **Global Domain Fusion:** We will extend the "Domain Fusion" primitive across physical nodes. This allows a client on Node A to "fuse" with a service on Node B, enabling zero-copy-like performance for remote IPC by pre-mapping shared memory regions across the sexnet.
3.  **sexnode-Wide Task Migration:** The **Global Scheduler** can migrate a Protection Domain (PD) from the Pi 5 to the x86_64 node mid-execution based on resource demand, without the PD losing its state or memory context.

---

## 🗺 Implementation Roadmap

### 1. Global sext (sexnode-Wide Memory)
- [ ] Implement the **DSAS Sexting Protocol**: Handling remote page faults via sexnet packets.
- [ ] Implement **Page Consistency Models** (Sequential vs. Release consistency) for distributed memory.
- [ ] Support for **RDMA (Remote Direct Memory Access)** to minimize latency between nodes.

### 2. Global Resource Fusion
- [ ] Implement **Remote Capability Export**: Allowing a node to export its hardware (e.g., the NVIDIA 3070) as a capability to the entire sexnode.
- [ ] **Transparent Device Access:** A thread on the Pi 5 can open a Node Capability for the NVIDIA GPU on the x86_64 node and perform direct PDX calls to it.

### 3. Distributed SMP Scaling
- [ ] Extend the **128-Core SMP** logic to the sexnode level.
- [ ] Implement **Inter-Node Interrupts (INI)**: Allowing one node to signal another node's LAPIC/GIC via the sexnet stack.

### 4. sexnode Virtualization
- [ ] Implement a **Global System Monitor**: A graphical tool (using the Wayland Graphics PD) to visualize memory and CPU usage across the entire 64-bit DSAS.

---

## 🧪 Phase 8 Verification
- **Distributed Page Fault:** Accessing a memory address on the Pi 5 that is physically located on the x86_64 node triggers a sexnet fetch and completes successfully.
- **Remote GPU Acceleration:** The Pi 5 successfully offloads a GPGPU task to the NVIDIA 3070 on the workstation via a remote PDX call.
- **Seamless Migration:** A running POSIX application is migrated from ARM64 to x86_64 without crashing or losing its sexvfs state.
