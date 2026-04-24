# Sex-Debug Standards (Orchestrator v2.1)

## Architectural Standards
1. **Host-Only Environment**: `sex-debug` is a diagnostic tool designed for the host OS (Linux/macOS). It **must** depend on the standard library (`std`).
2. **Zero Kernel Dependencies**: To avoid circular configuration and `build-std` conflicts, `sex-debug` must not depend on `no_std` kernel crates. It interacts with the kernel strictly via external traces (`trace.log`) or live socket interrogation (QEMU QMP).
3. **Symbol Resolution**: Must use ELF parsers (like `object` crate) to map raw RIPs from traces to source symbols.

## Capability Requirements
- **Live Mode**: Interrogate QEMU QMP for real-time `PKRU`, `CR3`, and `RIP` states.
- **Panic Analysis**: Support `--json` output for direct consumption by Gemini CLI sub-agents (`sex-debug-driver`).
- **TUI Verification**: Provide a human-readable TUI (via `ratatui`) for visual correlation of domain violations.

## Build Standards
- **Standard Cargo**: Must build with a simple `cargo build --release` when executed from its own directory.
- **Isolated Workspace**: Managed as an independent workspace to prevent inheriting the microkernel's custom JSON target and unstable `build-std` settings.

## Usage in Pipeline
- **Primary Analyzer**: The first tool called when a kernel panic or domain violation occurs.
- **Confidence Rating**: Must output a confidence score for its diagnosis (e.g., `confidence: 0.95`).
