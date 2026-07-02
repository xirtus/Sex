# SEXDRIVE_BAR_CAP_RESOLVE_PROOF_V1

- date: 2026-05-07
- scope: gate NVMe detect fix + sexdrive BAR cap resolve proof markers
- no kernel/ABI/storage protocol changes

## Gate NVMe detection fix/result

Fixed false-negative NVMe support detection in `scripts/master_runtime_gate.sh`.

Before:
```bash
grep -qE '(^|[[:space:],])nvme([[:space:],]|$)'
```

After:
```bash
grep -qE 'name[[:space:]]+"nvme"[[:space:]]*,|(^|[[:space:],])nvme([[:space:],]|$)'
```

Result: with `SEXOS_GATE_NVME=1`, gate enables NVMe args and runs QEMU successfully (no early `nvme unsupported` fail).

## BAR cap resolve path (sexdrive)

In `apps/sexdrive/src/main.rs` (`nvme_probe_bar`):
1. syscall 43 `MAP_PCI_BAR(SLOT_NVME_HOST=16, BAR0, 0x4000)`
2. emit proof markers:
   - `[sexdrive.nvme.bar.resolve.begin]`
   - `[sexdrive.nvme.bar.resolve.ok]` on nonzero/non-`u64::MAX`
   - `[sexdrive.nvme.bar.resolve.err]` on failure
3. preserve existing marker:
   - `[sexdrive.device.nvme_cap.present]` on success
   - `[sexdrive.device.no_nvme_cap]` on failure

No queue init, doorbells, DMA, storage reads/writes, or fake block success.
Typed block path remains honest `ERR_NO_DEVICE` pre-init.

## Proof markers observed
From `.gate_master/serial.log`:
- `[kernel.pci.nvme.found]`
- `[kernel.pci.nvme.bar0]`
- `[kernel.cap.nvme_bar.grant]`
- `[sexdrive.nvme.bar.resolve.begin]`
- `[sexdrive.nvme.bar.resolve.ok]`
- `[sexdrive.device.nvme_cap.present]`

No `#PF`, `#GP`, or `panic` found in proof grep.

## Notes
- `MASTER_RUNTIME_GATE_V1` overall remains `RED_MASTER` due existing clock gate (`silkbar.clock.send`) threshold miss in this lane; unrelated to NVMe BAR resolve.

## Next prompt
`SEXDRIVE_NVME_REG_IDENTITY_PROOF_V1`
(only CAP/VS/CC/CSTS identity proof, no queue init)
