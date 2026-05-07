# SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V2

- date: 2026-05-07
- scope: Identify retry with corrected CQE decode/phase interpretation
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no IO queue creation, no block read success claim

## Result

PASS. Identify completion is decoded on owned admin queue after correcting CQE field extraction.

## Corrected CQE Decode

For the observed QEMU path, decode used in `sexdrive`:
- `DW2`:
  - `SQHD = DW2[15:0]`
  - `SQID = DW2[31:16]`
- `DW3`:
  - `CID = DW3[15:0]`  (low 16 bits)
  - `Phase = DW3[16]`
  - `StatusField = DW3[31:17]`
  - `SC = StatusField[7:0]`
  - `SCT = StatusField[10:8]`

This matches observed raw completion `DW3=0x00010042` for `CID=0x0042`.

## Raw CQE

Observed markers:
- before submit:
  - `DW0=0x0 DW1=0x0 DW2=0x0 DW3=0x0`
- after submit/doorbell (poll 0):
  - `DW0=0x0 DW1=0x0 DW2=0x1 DW3=0x10042`
- decoded match:
  - `cid=66`, `phase=1`, `SQHD=0`, `SQID=0`
  - `SC=0`, `SCT=0`

## Identify Result Summary

- completion status: success (`SC=0`, `SCT=0`)
- CQ consume update performed:
  - `CQ0HDBL` written with `head+1` (`cqh=1`)
- bounded identify summary from buffer:
  - `sn0=0x203130736f786573`
  - `mn0=0x4d564e20554d4551`
  - `nn=256`

## Proof Markers

- `[sexdrive.nvme.admin.identify.v2.begin]`
- `[sexdrive.nvme.admin.identify.v2.submit]`
- `[sexdrive.nvme.admin.identify.v2.cqe.raw]`
- `[sexdrive.nvme.admin.identify.v2.cqe.decode]`
- `[sexdrive.nvme.admin.identify.v2.ok]`

No `#PF`, `#GP`, or `panic` in this run.

## Files Changed

- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V2.md`

## Final Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(reprovision|admin\.cqphase|admin\.identify\.v2)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_NVME_IO_QUEUE_CREATE_PROOF_V1`
