# QEMU_NVME_DEVICE_ENABLE_V1

## Scope
- Updated only runtime gate QEMU lane in `scripts/master_runtime_gate.sh`.
- No kernel/ABI/storage protocol/filesystem changes.

## What Changed
- Added optional NVMe lane controls:
  - `SEXOS_GATE_NVME=1` (default enabled)
  - `--nvme` / `--no-nvme`
- Added deterministic NVMe image creation in gate dir:
  - `NVME_IMG="${GATE_DIR}/nvme.img"`
  - `dd if=/dev/zero of="$NVME_IMG" bs=512 count=2048 2>/dev/null || true`
- Added QEMU NVMe args when enabled:
  - `-drive if=none,id=nvm,file="${NVME_IMG}",format=raw`
  - `-device nvme,serial=sexos01,drive=nvm`
- Added fail-fast support check:
  - if `qemu-system-x86_64 -device help` does not expose `nvme`, gate exits with `[FAIL]` and hint to run `--no-nvme`.

## Present vs Absent Paths
- Present path (default): run gate with NVMe enabled and expect markers:
  - `[kernel.pci.nvme.found]`
  - `[kernel.pci.nvme.bar0]`
  - `[kernel.cap.nvme_bar.grant]`
  - `[sexdrive.device.nvme_cap.present]`
- Absent path: disable NVMe with `--no-nvme` or `SEXOS_GATE_NVME=0`; expect absent markers (`kernel.pci.nvme.absent`, `sexdrive.device.no_nvme_cap`) to remain valid.

## Proof Grep
```bash
rg -n "kernel\.pci\.nvme\.(found|bar0|absent)|kernel\.cap\.nvme_bar\.grant|sexdrive\.device\.(nvme_cap\.present|no_nvme_cap)|#PF|#GP|panic" .gate_master/serial.log
```
