# SEX Microkernel: 100% Rust Boot Migration Handoff

## 1. ARCHITECTURE SUMMARY
- **Project:** SEX (Single Environment XIPC) Microkernel.
- **Tenets:** Tiny TCB, 100% `no_std` Rust (2024 Edition).
- **Isolation:** Intel PKU/MPK + CHERI (Capability-first).
- **Efficiency:** Single Address Space (SAS), Zero-copy PDX ring-buffer IPC.
- **Networking/Signals:** Message-based; signals use `MessageType::Signal`.

## 2. CURRENT TASK
**Migration to 100% Rust Boot Path via Limine v0.6+ Protocol.**
Eliminate all remaining C glue and external assembly (`trampoline.asm`) to achieve a pure-Rust trusted computing base.

## 3. BLOCKER / PENDING ACTIONS
The roadmap is approved, but the following code-level implementations are required to complete the migration:

### A. `kernel/Cargo.toml`
- Swap `bootloader_api` for `limine = "0.6.0"`.

### B. `kernel/src/main.rs`
- Replace `entry_point!` with `#[no_mangle] pub extern "C" fn _start() -> !`.
- Implement Limine request markers (`BaseRevision`, `HhdmRequest`, `MemoryMapRequest`, `RsdpRequest`).
- Align requests in the `.requests` section using `#[link_section = ".requests"]`.

### C. `kernel/src/smp.rs`
- **Critical:** Provide the `global_asm!` block for the AP trampoline (16-bit real mode -> 32-bit protected mode -> 64-bit long mode transition).
- Must load the BSP's P4 page table (passed via `0x508`) to ensure SAS consistency across cores.
- Must jump to the higher-half Rust `ap_kernel_entry` using the address stored at `0x500`.

### D. `kernel/linker.ld`
- Define the `.requests` section with **8-byte alignment** (mandatory for Limine protocol).
- Adjust VMA to Higher Half (`0xffffffff80000000`).

## 4. STRICT REQUIREMENTS
- **NO C Code:** Zero manual C compilation steps.
- **NO External .asm:** All assembly must be `global_asm!` or `asm!`.
- **Platform:** Must support both BIOS and UEFI via Limine's unified protocol.
- **TCB:** Minimal and memory-safe.

## 5. NEXT STEPS (FOR NEXT SESSION)
Generate the full, production-ready code blocks for the files listed above based on the approved migration strategy.
