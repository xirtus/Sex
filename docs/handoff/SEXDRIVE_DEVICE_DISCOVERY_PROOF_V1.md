# SEXDRIVE_DEVICE_DISCOVERY_PROOF_V1

- date: 2026-05-07
- proves: sexdrive NVMe device discovery path (absent case)

## Summary

Wired the full NVMe discovery path from kernel PCI scan → BAR lease → sexdrive probe.
QEMU gate has no NVMe device, so absent path fires honestly. Found path ready when
QEMU NVMe device added.

## Two Blockers Fixed

### 1. syscall 43 XHCI-only gate lifted

`kernel/src/syscalls/mod.rs` line 344: was gated to XHCI-only:
```rust
// Before:
if class_id != 0x0c || subclass_id != 0x03 || prog_if != 0x30 {
    u64::MAX  // rejected NVMe
}

// After:
let is_xhci = class_id == 0x0c && subclass_id == 0x03 && prog_if == 0x30;
let is_nvme = class_id == 0x01 && subclass_id == 0x08;
if !is_xhci && !is_nvme {
    u64::MAX
}
```

`sexos_build_spec.toml` `abi_version_hash` updated to match.

### 2. devmgr NVMe grant: slotless → slotted at SLOT_NVME_HOST = 16

`kernel/src/devmgr.rs`: NVMe branch changed from `pd.grant(...)` (no slot) to
`pd.grant_capability(16, ...)`. Without a named slot, sexdrive couldn't reference
the cap via `MAP_PCI_BAR`. `SLOT_NVME_HOST = 16` defined locally (not in sex-pdx —
no ABI hash churn).

Also added:
- `[kernel.pci.nvme.found] bus:dev.func vendor device` — when NVMe found
- `[kernel.pci.nvme.bar0] pa=0x...` — physical BAR0 address when found
- `[kernel.cap.nvme_bar.grant] pd= slot=` — cap grant confirmation
- `[kernel.pci.nvme.absent]` — after full PCI scan, no NVMe found

## sexdrive Changes

`apps/sexdrive/src/main.rs`: Added `nvme_probe_bar()`, called before `xhci_probe_mmio()`:

```rust
fn nvme_probe_bar() {
    let map_va: u64;
    // syscall 43 = MAP_PCI_BAR(SLOT_NVME_HOST=16, BAR0, 0x4000)
    ...
    if map_va == u64::MAX || map_va == 0 {
        serial_println!("[sexdrive.device.no_nvme_cap]");
        return;
    }
    // Read NVMe CAP register (64-bit at offset 0)
    let nvme_cap = ...;
    serial_println!("[sexdrive.device.nvme_cap.present] va={:#x} cap={:#x}", ...);
}
```

## Observed Markers (no QEMU NVMe device)

```
[kernel.pci.nvme.absent]
[sexdrive.device.no_nvme_cap]
```

All gates pass except pre-existing CLOCK_GATE. No panic/fault.

## Files Changed

| File | Change |
|------|--------|
| `kernel/src/syscalls/mod.rs` | Extend MAP_PCI_BAR gate to NVMe class |
| `kernel/src/devmgr.rs` | Slotted NVMe cap grant + proof markers |
| `apps/sexdrive/src/main.rs` | `nvme_probe_bar()` + SLOT_NVME_HOST=16 |
| `sexos_build_spec.toml` | Updated `abi_version_hash` |

## Build/Verify Commands

```bash
./scripts/entrypoint_build.sh
./scripts/master_runtime_gate.sh --skip-build --probe 25 --keep-log
grep -E "nvme|NVMe|nvme_cap|kernel\.pci" .gate_master/serial.log
```

## Next: QEMU_NVME_DEVICE_ENABLE_V1

QEMU gate currently launches with no NVMe device. To prove the found+BAR-mapped path:

### Required QEMU args (add to scripts/master_runtime_gate.sh)

```bash
# Create a small NVMe backing image once
NVME_IMG="${GATE_DIR}/nvme.img"
if [ ! -f "$NVME_IMG" ]; then
    dd if=/dev/zero of="$NVME_IMG" bs=512 count=2048 2>/dev/null
fi

# Add to QEMU launch:
-drive if=none,id=nvm,file="${NVME_IMG}",format=raw
-device nvme,serial=sexos01,drive=nvm
```

With these args:
- `devmgr::init` finds NVMe at some bus:dev.func
- Emits `[kernel.pci.nvme.found]`, `[kernel.pci.nvme.bar0]`, `[kernel.cap.nvme_bar.grant]`
- sexdrive: `MAP_PCI_BAR(16, 0, 0x4000)` succeeds
- Emits `[sexdrive.device.nvme_cap.present] va=0x... cap=0x...`
- NVMe CAP register readable (QEMU emulates standard NVMe CAP layout)

### After QEMU NVMe enabled, next prompt

**SEXDRIVE_QUEUE_INIT_PROOF_V1** — initialize NVMe Admin Queue (ASQ/ACQ) and
send Identify Controller command. Proves device is responsive.

### Full backend milestone path after found case

| Step | Prompt |
|------|--------|
| BAR0 mapped, CAP register read | DONE after QEMU_NVME_DEVICE_ENABLE_V1 |
| Admin Queue init + Identify | SEXDRIVE_QUEUE_INIT_PROOF_V1 |
| I/O Queue init | SEXDRIVE_IO_QUEUE_INIT_V1 |
| Real BLOCK_READ (one sector, DMA) | SEXDRIVE_REAL_BLOCK_READ_V1 |
| BLOCK_OK returned to sexfiles | SEXFILES_REAL_READ_E2E_V1 |
