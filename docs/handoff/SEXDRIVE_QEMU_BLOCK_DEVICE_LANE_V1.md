# SEXDRIVE_QEMU_BLOCK_DEVICE_LANE_V1

- date: 2026-05-07
- scope: host QEMU block-device lane audit + SexOS support reality only
- code behavior changes: none

## 1. Host QEMU Supported Block Devices (exact audit)
From:
```bash
qemu-system-x86_64 -device help | rg -n "nvme|ich9-ahci|\\bahci\\b|ide-hd|virtio-blk-pci"
```
Observed:
- `ich9-ahci` (alias `ahci`)
- `ide-hd`
- `nvme` (+ `nvme-ns`, `nvme-subsys`)
- `virtio-blk-pci` (+ transitional variants)

## 2. SexOS Existing Block-Lane Support (current reality)

### Kernel discovery (`kernel/src/devmgr.rs`)
- Explicit NVMe discovery path exists:
  - class/subclass match: `(0x01, 0x08)`
  - markers: `[kernel.pci.nvme.found]`, `[kernel.pci.nvme.bar0]`, `[kernel.cap.nvme_bar.grant]`
  - absent marker: `[kernel.pci.nvme.absent]`
- No AHCI (`0x01/0x06`) or IDE/virtio discovery/grant path present.

### BAR mapping gate (`kernel/src/syscalls/mod.rs`, syscall 43)
- Allows MMIO BAR mapping only for:
  - XHCI `(0x0c,0x03,0x30)`
  - NVMe `(0x01,0x08)`
- AHCI/IDE/virtio currently rejected by class gate.

### SexDrive app (`apps/sexdrive/src/main.rs`)
- Probes NVMe BAR capability only (`SLOT_NVME_HOST=16`)
- Emits:
  - present: `[sexdrive.device.nvme_cap.present]`
  - absent: `[sexdrive.device.no_nvme_cap]`
- Typed block API returns honest `ERR_NO_DEVICE` until real backend exists.
- No AHCI/IDE/virtio backend path implemented.

## 3. Lane Decision
Decision: **A) NVMe available** on this host, and SexOS already has the smallest existing discovery lane for it.

Reason:
- Host QEMU exposes `-device nvme`.
- SexOS already has NVMe-only kernel discovery + grant + MAP_PCI_BAR class allowlist.
- Switching to AHCI now would require kernel PCI class expansion (`0x01/0x06`) and new lane wiring, which hits STOP FIRST constraints.

## 4. Important discrepancy found
Earlier gate failure (`[FAIL] QEMU nvme device not supported`) came from match logic in `scripts/master_runtime_gate.sh`, not host capability absence.

Current match:
```bash
grep -qE '(^|[[:space:],])nvme([[:space:],]|$)'
```
This can miss `name "nvme", ...` because of quotes around `nvme`.

No fix applied in this mission (audit-only scope).

## 5. Exact next prompt
`SEXDRIVE_BAR_CAP_RESOLVE_PROOF_V1`

Follow-on after that (if BAR CAP resolves cleanly):
`SEXDRIVE_NVME_REG_IDENTITY_PROOF_V1`

## 6. Blockers / STOP FIRST conditions
- **AHCI lane now**: blocked unless approved kernel class expansion for `0x01/0x06` + grant/map path.
- **virtio-blk lane now**: blocked (requires new virtio transport/protocol path).
- **IDE lane now**: blocked (legacy PIO path not present).
- **Host blocker status**: none for NVMe device model itself based on `-device help` + parse smoke lane.
