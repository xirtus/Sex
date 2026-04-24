# PHASE 31 STATUS: Hardware-Backed PKU Warden

**Phase 28/29 Accomplishments:**
- Syscall 27 (`pdx_call`) dispatcher integrated.
- Ring-3 preemption stability verified.

## Phase 31: The Warden (ACTIVE)
The current objective is the enforcement of the **2Ring model** using Intel MPK/PKU.

### 1. The PKU Warden (`kernel/src/pku.rs`)
- **Status**: IMPLEMENTED
- **Logic**: Performs manual page table walks (PML4->PDPT->PD->PT) to extract the **Color (PKEY)** of faulting virtual addresses.
- **Diagnostics**: Analyzes Bit 4 (Execute) and Bit 1 (Write) to detect cross-domain capability violations.

### 2. Interrupt Integration (`kernel/src/interrupts.rs`)
- **Trigger**: `page_fault_handler` now intercepts Bit 5 (PK bit) of error codes.
- **Enforcement**: Invokes Warden on violation. Currently panics to provide forensic log; production will terminate domain.

### 3. Debugger Orchestration
- **Tool**: `sex-debug` (Standardized v2.1)
- **Status**: Workspace isolated, build reliability restored.
- **Standards**: `tools/sex-debug/STANDARDS.md` defines host-tool behaviors.

## Verification Protocol
1. **Build Tools**: `make build-tools`
2. **Build Kernel**: `make iso`
3. **Live Analysis**: `sex-debug live` or `sex-debug panic` against `qemu_serial.log`.

**Linen Monitoring**: PKEY 2 (linen) under active monitoring for Ring 2 boundary regressions.
