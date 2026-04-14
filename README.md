# Sex (Single Environment eXecutive)
Single Address Space Microkernel System

Sex is a from-scratch, high-performance microkernel written in Rust. It is designed to be a tiny, safe, and lightning-fast alternative to traditional monolithic kernels, leveraging modern hardware features like Intel PKU and CHERI to provide memory safety in a single global address space.

## 🚀 Key Features

- **Tiny Privileged Kernel:** Target size of ~5-8 kLOC for a minimal Trusted Computing Base (TCB).
- **Single Global Address Space (SAS):** Eliminates traditional address space switching overhead.
- **Hardware-Accelerated Protection Domains:** Uses Intel PKU (Memory Protection Keys) and MPK for zero-cost domain switching.
- **Zero-Copy IPC (PDX):** Protected Procedure Calls (PDX) via shared memory regions for maximum throughput.
- **Capability-Based Security:** Sparse, unforgeable capabilities for granular access control, inspired by seL4 and Mungi.
- **Distributed by Design:** Transparent networked IPC allowing a cluster to behave as a single machine.
- **User-Space Everything:** Pagers, VFS, Drivers, and Network stack all run in isolated user-space domains.
- **Rust First:** Written entirely in safe Rust where possible, minimizing `unsafe` blocks and ensuring memory safety at the language level.

## 📁 Project Structure

- `kernel/`: The core `no_std` microkernel.
- `servers/`: User-space system servers (Pager, VFS, Drivers, etc.).
- `docs/`: In-depth documentation and specifications.
- `tests/`: Integration and unit tests.
- `scripts/`: Build and run scripts (QEMU, etc.).

## 🛠 Getting Started

### Prerequisites

- **Rust Nightly:** Required for various `no_std` and embedded features.
- **QEMU:** For running and testing the kernel.
- **Cargo-bootimage:** (Optional, depending on bootloader choice).

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
cargo install bootimage
```

### Build and Run

To build and run the kernel in QEMU:

```bash
make run
# or
./scripts/run.sh
```

## 📚 Documentation

- [**ARCHITECTURE.md**](docs/ARCHITECTURE.md): Full living specification of the kernel and its components.
- [**GEMINI.md**](.gemini/GEMINI.md): Project context, coding style, and rules.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
