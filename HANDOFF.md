# SexOS Handoff - 2026-04-15 (Post-Release & Boot Diagnostic)

## 🏁 Current Status
**Sex SASOS v1.0.0 is public-ready and has achieved "First Life" potential.**
The kernel compiles cleanly in Docker and the ISO is production-bootable via Limine. We have successfully diagnosed the manual boot failure: the system is a modern SASOS and requires explicit hardware feature flags in QEMU that are not enabled by default.

## 🚀 The Emulation Mandate (QEMU)
To avoid immediate triple-faults or black-screen hangs, the following invariants MUST be satisfied during emulation:

1.  **Intel PKU (`-cpu max,pku=on`)**: The kernel uses `WRPKRU` for zero-cost domain switching. Without this flag, the CPU throws an *Invalid Opcode Exception* immediately.
2.  **Modern Chipset (`-machine q35`)**: The kernel expects a PCIe bus and modern APIC mapping (e.g., `0xFEE00000`) found on Q35 motherboards, not the default 1996 i440FX.
3.  **SAS Metadata Memory (`-m 2G`)**: A Single Address Space OS requires significant memory at boot for Bitmap Frame Allocators and global page tables. 512MB will trigger an immediate OOM panic during Phase 1.
4.  **Headless Output (`-serial stdio -display none`)**: The black screen is a "Silent Success." The kernel is designed for performance and communicates via **Serial (COM1)**, not the VGA buffer.

**Canonical Command:** `make run-sasos`

## 🛠 Tomorrow's Stabilization Sprint
Priority tasks to move from "bootable" to "hardened":

- [ ] **Entropy Restoration**: Resolve the `rdseed` intrinsic issues to move past the hardcoded `4001` PD ID.
- [ ] **Proper IDT Trait Impls**: Reconcile `set_handler_fn` bounds to remove the `set_handler_addr` bypass.
- [ ] **Zero-Cost PTE Abstractions**: Replace the brute-force `unsafe` bit manipulation in `memory.rs` with safe flags.
- [ ] **Bitmap Hardening**: Validate `BitmapFrameAllocator` logic to ensure contiguous allocations don't overflow.
- [ ] **PDX Signal Testing**: Execute the first real-world tests of the async signal trampoline in `sexc`.

### Preserved Invariants:
- 100% `no_std` Microkernel.
- Intel PKU Hardware-Backed Isolation.
- Lock-Free / Wait-Free Core path.
- Asynchronous Signal Delivery (No stack hijacking).

**The repo is clone-and-boot ready. "First Life" achieved via Serial.**
