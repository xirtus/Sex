# SexOS Architecture Overview

SexOS is a capability-based microkernel designed for performance and security.

## Build Authority
- Single source of build truth: `sexos_build_spec.toml`
- Single build root: `scripts/entrypoint_build.sh`
- Trace interpreter: `scripts/sexos_build_trace.sh`
- Any direct build path outside entrypoint is invalid by policy.

## The Stack
- **SexOS**: The microkernel. Responsible for:
    - Capability-based isolation (UCGM).
    - IPC (PDX).
    - Hardware Abstraction (HAL).
    - Resource Management (PDs).

- **Silk DE**: The Desktop Environment. Built on top of SexOS.
    - Compositor (Surface management, rendering pipeline).
    - Frame Pump (VBlank synchronization).
    - Visual projection layer.

- **Linen**: The Desktop Shell and Interaction layer.
    - Session management.
    - Filesystem navigation.
    - Input focus authority.
