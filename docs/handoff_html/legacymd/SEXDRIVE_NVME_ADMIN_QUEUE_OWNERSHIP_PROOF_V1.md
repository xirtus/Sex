# SEXDRIVE_NVME_ADMIN_QUEUE_OWNERSHIP_PROOF_V1

- date: 2026-05-07
- scope: admin queue ownership/coordination proof only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no Identify submission in this mission
- no kernel/syscall/ABI/storage changes

## Result

Ownership classification: **B** (preconfigured/active queue not coordinated by sexdrive).

Reason:
- Controller enabled and ready (`CC.EN=1`, `CSTS.RDY=1`).
- `AQA/ASQ/ACQ` preconfigured and nonzero.
- SQ/CQ contents are already populated with existing command/completion patterns.
- CQ phase bits are mixed and active before any new sexdrive-admin submit in this mission.

## Decoded Queue State

From runtime markers:
- `AQA = 0x00ff003f`
  - `ASQS = (AQA[11:0] + 1) = 64`
  - `ACQS = (AQA[27:16] + 1) = 256`
- `ASQ = 0x1ffdd000`
- `ACQ = 0x1ffde000`
- `CC = 0x00460001` (`CC.EN=1`)
- `CSTS = 0x00000001` (`CSTS.RDY=1`)

## Doorbell Layout Verification

- `CAP.DSTRD = 0`
- `SQ0TDBL offset = 0x1000`
- `CQ0HDBL offset = 0x1004`
- Observed DB values in this run: `sq_tail_db=0`, `cq_head_db=0`

## Bounded Queue Peeks

ASQ first 4 entries (`[sexdrive.nvme.adminq.asq.peek]`):
- `idx=0 d0=0x6 d6=0x1efe8000 d7=0x0 d10=0x0`
- `idx=1 d0=0x10006 d6=0x1efe8000 d7=0x0 d10=0x0`
- `idx=2 d0=0x20006 d6=0x1efe8000 d7=0x0 d10=0x0`
- `idx=3 d0=0x30006 d6=0x1efe8000 d7=0x0 d10=0x0`

ACQ first 8 entries (`[sexdrive.nvme.adminq.acq.peek]`):
- `idx=0 dw2=0x1 dw3=0x0 phase=0`
- `idx=1 dw2=0x2 dw3=0x1 phase=1`
- `idx=2 dw2=0x3 dw3=0x2 phase=0`
- `idx=3 dw2=0x4 dw3=0x10003 phase=1`
- `idx=4 dw2=0x5 dw3=0x10004 phase=0`
- `idx=5 dw2=0x6 dw3=0x10005 phase=1`
- `idx=6 dw2=0x7 dw3=0x10006 phase=0`
- `idx=7 dw2=0x8 dw3=0x10007 phase=1`

## Phase Inference

- `phase0=4`, `phase1=4`, `nonzero_cqe=8`
- CQ appears actively populated with an existing producer sequence and mixed phase state.
- This is incompatible with assuming sexdrive owns queue epoch/head/tail/phase without an explicit handoff or reprovision.

## Safety Decision

Recommended safe path: **Option 2**
- Disable controller cleanly and wait `CSTS.RDY=0`.
- Provision sexdrive-owned admin SQ/CQ.
- Re-enable controller and wait `CSTS.RDY=1`.
- Then retry Identify on owned queue.

This mission intentionally does **not** submit Identify.

## Proof Markers

Observed:
- `[sexdrive.nvme.adminq.inspect]`
- `[sexdrive.nvme.adminq.aqa]`
- `[sexdrive.nvme.adminq.doorbell.layout]`
- `[sexdrive.nvme.adminq.asq.peek]`
- `[sexdrive.nvme.adminq.acq.peek]`
- `[sexdrive.nvme.adminq.phase.infer]`
- `[sexdrive.nvme.adminq.ownership.result] class=B ...`

No `#PF`, `#GP`, or `panic` seen in this run.

## Next Prompt

`SEXDRIVE_NVME_ADMIN_QUEUE_REPROVISION_V1`
