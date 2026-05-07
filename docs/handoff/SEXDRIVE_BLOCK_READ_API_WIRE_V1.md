# SEXDRIVE_BLOCK_READ_API_WIRE_V1

- date: 2026-05-07
- scope: wire typed `BLOCK_READ` to real NVMe IO completion path
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no ABI/kernel/storage protocol changes

## What Changed

Added a minimal runtime NVMe IO state and a reusable read helper:
- `NVME_IO_STATE` stores IO queue pointers/state (`map_va`, `io_sq_va`, `io_cq_va`, `sq1tdbl`, `cq1hdbl`, `sq_tail`, `cq_head`, `cq_phase`).
- State is initialized after `ioq.ready` and updated after successful direct IO read proof.
- New helper `nvme_read_into_bounce(offset, size)`:
  - validates request
  - alloc/map zeroed bounce buffer
  - submits real NVMe READ on IOQ1
  - polls CQE with corrected decode
  - consumes CQ (`CQ1HDBL`)
  - returns `0` only on real IO success

Typed dispatch wiring:
- `BLOCK_READ` now routes to `nvme_read_into_bounce(...)` after validation.
- `BLOCK_WRITE` and `BLOCK_SYNC` remain honest `ERR_NO_DEVICE` in this mission.
- unknown cmd remains `ERR_BAD_CMD`.

## BLOCK_READ Validation Rules

For `BLOCK_READ`:
- `size > 0`
- `size <= BLOCK_MAX_XFER`
- `size <= 4096` (current bounded bounce path)
- `offset % 512 == 0`
- `size % 512 == 0`

Mapping:
- `SLBA = offset / 512`
- `NLB = (size / 512) - 1`

## Data Handoff Status

- Current wiring is **bounce-buffer-only** in SexDrive.
- `buffer_cap` handoff to caller memory is **not wired** in this mission.
- `BLOCK_READ` now returns `OK` only after real NVMe CQE success, but payload copy-back via caller capability is a follow-up step.

## Runtime Evidence

Hardware path remains green in gate logs:
- `admin.identify.v2.ok`
- `ioq.ready`
- `io.read.ok`
- no `#PF/#GP/panic`

Typed block request markers were not observed in this probe window:
- no `sexdrive.block.read.api.*`
- no `sexdrive.block.typed.*`

So this mission proves wiring by code path integration; a dedicated SexFiles-trigger run is needed to prove live typed traffic.

## Status Matrix (Current Behavior)

- `BLOCK_READ`:
  - `ERR_BAD_LEN` on invalid size/alignment
  - `ERR_NO_DEVICE` on queue/device/timeout/failure path
  - `OK (0)` only after real NVMe read CQE success in helper
- `BLOCK_WRITE`: `ERR_NO_DEVICE`
- `BLOCK_SYNC`: `ERR_NO_DEVICE`
- unknown cmd: `ERR_BAD_CMD`

## Final Grep Used

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 60 --keep-log \
| grep -E 'sexdrive\.block\.read\.api|sexdrive\.block\.typed|sexblock\.abi|#PF|#GP|panic'
```

## Next Prompt

`SEXFILES_SEXDRIVE_REAL_READ_PROOF_V1`

(If caller-buffer return is required immediately, use `SEXDRIVE_BLOCK_READ_DATA_HANDOFF_V1`.)
