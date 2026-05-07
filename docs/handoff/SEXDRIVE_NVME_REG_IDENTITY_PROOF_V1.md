# SEXDRIVE_NVME_REG_IDENTITY_PROOF_V1

- date: 2026-05-07
- scope: NVMe BAR0 register identity read only (CAP, VS, CC, CSTS)
- files allowed: `apps/sexdrive/src/main.rs`, this handoff
- no kernel/syscall/ABI/storage changes

## What Changed

In `apps/sexdrive/src/main.rs` (`nvme_probe_bar()`), after BAR0 resolve success:
- Added volatile MMIO reads:
  - `CAP` at `0x0000` (`u64`)
  - `VS` at `0x0008` (`u32`)
  - `CC` at `0x0014` (`u32`)
  - `CSTS` at `0x001C` (`u32`)
- Added minimal decode and markers:
  - `CAP.MQES`
  - `CAP.DSTRD`
  - `VS major/minor`
  - `CSTS.RDY`
- Added markers:
  - `[sexdrive.nvme.reg.cap]`
  - `[sexdrive.nvme.reg.vs]`
  - `[sexdrive.nvme.reg.cc]`
  - `[sexdrive.nvme.reg.csts]`
  - `[sexdrive.nvme.identity.ok]` when `CAP != 0 && VS != 0`
  - `[sexdrive.nvme.identity.err]` otherwise

Block path remains honest pre-queue: `ERR_NO_DEVICE`.

## Runtime Proof (Gate)

Command used:
```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(bar\.resolve|reg|identity)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

Serial evidence (`.gate_master/serial.log`):
- `[kernel.pci.nvme.found] 00:04.0 vendor=1b36 device=0010`
- `[kernel.pci.nvme.bar0] pa=0xfebd8000`
- `[kernel.cap.nvme_bar.grant] pd=2 slot=16`
- `[sexdrive.nvme.bar.resolve.begin] ... [sexdrive.nvme.bar.resolve.ok] ... cap=0x4008200f0107ff`
- `[sexdrive.nvme.reg.cap] cap=0x4008200f0107ff mqes=2047 dstrd=0`
- `[sexdrive.nvme.reg.vs] vs=0x10400 major=1 minor=4`
- `[sexdrive.nvme.reg.cc] cc=0x460001`
- `[sexdrive.nvme.reg.csts] csts=0x1 rdy=1`
- `[sexdrive.nvme.identity.ok] cap=0x4008200f0107ff vs=0x10400 mqes=2047 dstrd=0 major=1 minor=4 csts_rdy=1`

No `#PF`, `#GP`, or `panic` lines found in `.gate_master/serial.log` for this run.

## Register Offsets and Values

- `CAP @ 0x0000 (u64) = 0x4008200f0107ff`
- `VS @ 0x0008 (u32) = 0x00010400`
- `CC @ 0x0014 (u32) = 0x00460001`
- `CSTS @ 0x001C (u32) = 0x00000001`

## Minimal Decode

- `CAP.MQES = 2047`
- `CAP.DSTRD = 0`
- `VS.major = 1`
- `VS.minor = 4`
- `CSTS.RDY = 1`

## Next Prompt

`SEXDRIVE_NVME_QUEUE_INIT_PROOF_V1`

(If a future run shows zero/invalid identity registers or MMIO faults, STOP FIRST and re-check BAR mapping validity before any queue work.)
