# SEXDRIVE_NVME_QUEUE_INIT_PROOF_V1

- date: 2026-05-07
- scope: minimal NVMe admin-queue readiness proof only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no kernel/syscall/ABI/storage changes

## Summary

Queue readiness proved by register inspection. In this QEMU lane, admin queue registers were already configured while controller was enabled/ready, so SexDrive reports `queue.ready` in `preconfigured` mode and does not reprogram queues.

## Code Changes

In `apps/sexdrive/src/main.rs` (`nvme_probe_bar`):
- Added helper syscalls for physical memory primitives (no ABI changes):
  - `sys_alloc_phys` (syscall 31)
  - `sys_map_phys` (syscall 30)
- Added queue-register inspection reads:
  - `AQA @ 0x0024`
  - `ASQ @ 0x0028`
  - `ACQ @ 0x0030`
  - plus `CC @ 0x0014`, `CSTS @ 0x001C`
- Added proof markers:
  - `[sexdrive.nvme.queue.inspect]`
  - `[sexdrive.nvme.queue.aqa]`
  - `[sexdrive.nvme.queue.asq]`
  - `[sexdrive.nvme.queue.acq]`
  - `[sexdrive.nvme.queue.alloc.ok]`
  - `[sexdrive.nvme.queue.program.ok]`
  - `[sexdrive.nvme.queue.ready]`
  - `[sexdrive.nvme.queue.err]`
- Safety behavior:
  - If queues are already configured (`AQA/ASQ/ACQ != 0`): emit `queue.ready mode=preconfigured`, return.
  - If queues are unset but `CC.EN=1` or `CSTS.RDY=1`: emit `queue.err` STOP-FIRST reason and return (no disable/reset in this mission).
  - Programming path exists only for disabled/not-ready state.

Block API behavior remains unchanged and honest: `ERR_NO_DEVICE` for pre-IO requests.

## Runtime Proof

Build:
```bash
./scripts/entrypoint_build.sh
```

Gate run:
```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(bar\.resolve|reg|identity|queue)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

Serial evidence (`.gate_master/serial.log`):
- `[kernel.pci.nvme.found] 00:04.0 vendor=1b36 device=0010`
- `[kernel.pci.nvme.bar0] pa=0xfebd8000`
- `[kernel.cap.nvme_bar.grant] pd=2 slot=16`
- `[sexdrive.nvme.identity.ok] cap=0x4008200f0107ff vs=0x10400 ...`
- `[sexdrive.nvme.queue.inspect] aqa=0xff003f asq=0x1ffdd000 acq=0x1ffde000 cc=0x460001 csts=0x1`
- `[sexdrive.nvme.queue.aqa] aqa=0xff003f`
- `[sexdrive.nvme.queue.asq] asq=0x1ffdd000`
- `[sexdrive.nvme.queue.acq] acq=0x1ffde000`
- `[sexdrive.nvme.queue.ready] mode=preconfigured cc_en=1 csts_rdy=1 aqa=0xff003f asq=0x1ffdd000 acq=0x1ffde000`

No `#PF`, `#GP`, or `panic` lines found in `.gate_master/serial.log` for this run.

## Initial Register Values

- `AQA = 0x00ff003f`
- `ASQ = 0x000000001ffdd000`
- `ACQ = 0x000000001ffde000`
- `CC  = 0x00460001` (`CC.EN=1`)
- `CSTS = 0x00000001` (`CSTS.RDY=1`)

## Controller/Queue State Decision

- Controller already enabled: **yes** (`CC.EN=1`, `CSTS.RDY=1`).
- Admin queues already configured: **yes** (`AQA/ASQ/ACQ all nonzero`).
- Queue programming performed by SexDrive in this run: **no** (not required; preserved safe preconfigured state).

## Next Prompt

`SEXDRIVE_NVME_ADMIN_IDENTIFY_PROOF_V1`

(If a future lane has `AQA/ASQ/ACQ == 0` while `CC.EN=1`, use STOP FIRST and explicitly justify disable/reset sequence before any queue reprogram attempt.)
