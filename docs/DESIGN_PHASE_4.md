# Phase 4 Design: Distribution

## 🎯 Objective
Transition the Sex Microkernel from a single-node Single Address Space Operating System (SASOS) to a distributed sexnode. The goal is to make networked nodes act as a single logical system via transparent remote IPC, distributed capability management, and sexnode node discovery.

## 🏛 Architectural Vision

In a distributed environment, the microkernel relies on three core pillars:
1.  **Transparent Networked IPC:** An IPC call to a Protection Domain (PD) on another physical node should be syntactically identical to a local `safe_pdx_call`. The kernel intercepts remote calls and forwards them via the `sexnet`.
2.  **Distributed Capability Management:** Capabilities must remain unforgeable across the sexnet. A capability granted by Node A to Node B must be verifiable and cryptographic or authenticated by a central/distributed Capability Authority.
3.  **sexnode Discovery:** Nodes must be able to discover each other, form a sexnode, and establish secure channels for IPC routing.

---

## 🗺 Implementation Roadmap

### 1. Transparent Networked IPC
- **Extended IPC Capabilities:** Modify `IpcCapData` (and potentially other capabilities) to include a `node_id`.
- **IPC Router:** If an IPC call targets a remote node, `safe_pdx_call` will seamlessly route the payload to the local `sexnet` instead of performing a hardware context switch.
- **Remote Invocation:** The receiving node's sexnet translates the incoming sexnet packet back into a local PDX call.

### 2. Distributed Capability Management
- **Global Capability IDs:** Capabilities are extended to include the originating node's ID, ensuring global uniqueness.
- **Capability Export/Import:** When a capability is sent over the sexnet, it is serialised. The receiving node registers an "Imported Capability" that points back to the remote resource.

### 3. sexnode Management & Node Discovery
- **sexnode Server:** A dedicated user-space PD (`sexnode.rs`) responsible for broadcasting node presence (e.g., via UDP multicast) and maintaining a registry of active nodes in the sexnode.
- **Node ID Assignment:** Each node receives a unique identifier upon joining the sexnode.

---

## 🧪 Phase 4 Verification
- **Remote PDX Benchmark:** Measure the latency of a `safe_pdx_call` executed over the loopback/sexnet interface compared to local PDX.
- **Capability Integrity:** Verify that a node cannot spoof a capability belonging to another node.
- **Dynamic Discovery:** Start a simulated second node and verify that the sexnode Server automatically discovers it and establishes an IPC route.
