# SEX Microkernel: Phase 23 Self-Hosting Build Optimization Handoff

Do not run an unconstrained 'find' command from my macOS home directory. Restrict ALL operations strictly to: '/Users/xirtus/sites/microkernel/'.
In large repositories, consider narrowing your search scope by specifying a 'dir_path' or an 'include_pattern'.                                                                             │

## 1. ARCHITECTURE SUMMARY
- **Project:** SEX (Single Environment XIPC) Microkernel.
- **Tenets:** Tiny TCB, 100% `no_std` Rust (2024 Edition).
- **Isolation:** Intel PKU/MPK + CHERI (Capability-first).
- **SASOS:** Single Address Space OS. All PDs share one 64-bit VAS.
- **PDX:** synchronous/asynchronous Protected Procedure Calls via ring buffers.

## 2. COMPLETED IN PHASES 20, 21, 22
- **Phase 20 (Sexshop):** Replaced legacy `sex-store` with pure-Rust `sexshop`.
  - Implemented `storage`, `pdx` handler, `cache` (PFN LRU), and `transactions` (WAL-style).
  - Integrated with `sexfiles` (Slot 1) for zero-copy persistent object/KV storage.
- **Phase 21 (Dynamic Linker & GPU):**
  - **sex-ld:** Implemented dynamic linker PD with `LdProtocol` for shared object resolution.
  - **GPU DMA:** Added `DmaBufferSubmit` path in `sexdisplay` → `sexdrive` (Slot 11) for zero-copy command buffers.
  - **Cluster Fabric:** Extended `sexnet` to proxy object fetches across cluster nodes.
- **Phase 22 (Cluster Maturity):**
  - **Distributed Registry:** `sexnode` now handles `CapabilityResolve` via `sexnet` (XIPC).
  - **Object Migration:** Added `ObjectMove` to `StoreProtocol` and `ClusterObjectMigrate` to `NodeProtocol`.
  - **Node Tracking:** Cluster `Heartbeat` and `NodeRegister` implemented in `sexnode`.

## 3. CURRENT STATE
- **Branch:** `feature/sexshop-redox-integration` (Phases 20-22 logic implemented).
- **Servers:** `sexshop`, `sex-ld`, `sexnode`, `sexnet` all updated with Phase 21/22 protocols.
- **Kernel:** `init.rs` grants IPC slots for `sexdisplay` → `sexdrive` (11) and `sexnode` → `sexshop` (1).

## 4. PHASE 28: BUILD OPTIMIZATION


### Implementation Plan:
1. Define a `no_std` compatible PDX client for `sexbuild`.
2. Map `sexbuild`'s build directory to `sexshop`'s object store.
3. Implement artifact deduplication using SHA-256 hashes as PDX object keys.

## 5. STRICT REQUIREMENTS
- **Rust Only:** 100% `no_std` Rust for all system servers.
- **No Mutexes:** Hot paths must be lock-free.
- **Zero-Copy:** Maintain zero-copy PFN handovers for all object migrations.
- **Asynchronous:** PDX messages must not block the main polling loops.
