# SEXDRIVE_NVME_ADMIN_CQ_PHASE_FIX_V1

- date: 2026-05-07
- scope: admin CQ phase/head/tail/config proof only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no IO queue work

## Key Result

`ACQ` memory **does change immediately** after SQ doorbell.

This proves controller acceptance/completion activity exists on the owned queue.
The remaining blocker is CQE interpretation (phase/CID bitfield extraction), not “no DMA writes”.

## Evidence

From `.gate_master/serial.log`:
- `[sexdrive.nvme.admin.cqphase.acq.before] dw0=0x0 dw1=0x0 dw2=0x0 dw3=0x0 phase=0`
- `[sexdrive.nvme.admin.cqphase.submit] cid=66 opc=0x6 cns=1 prp1=0x1f919000 sq_tail=0`
- `[sexdrive.nvme.admin.cqphase.submit] sq0tdbl=0x1000 new_tail=1 dstrd=0`
- `[sexdrive.nvme.admin.cqphase.acq.change] poll=0 dw0=0x0 dw1=0x0 dw2=0x1 dw3=0x10042 phase=0`
- `[sexdrive.nvme.admin.cqphase.err] reason=cqe_timeout ... acq_changed=1 first_change_poll=0 ...`

So the “most important thing to learn” is proven: **ACQ changed at poll 0**.

## CC and Doorbell Validation

- `CC=0x460001`
- decoded:
  - `EN=1`
  - `CSS=0`
  - `MPS=0`
  - `AMS=0`
  - `IOSQES=6`
  - `IOCQES=4`
- doorbells:
  - `SQ0TDBL=0x1000`
  - `CQ0HDBL=0x1004`
  - `DSTRD=0`

These are logged by `[sexdrive.nvme.admin.cqphase.cc]`.

## CQE Layout/Extraction Finding

Observed CQE after submit:
- `DW2=0x00000001`
- `DW3=0x00010042`

Current parser in code used:
- `phase = DW3 bit0`
- `cid = DW3[31:16]`

That mapping is likely wrong for this lane/spec interpretation. The value `0x00010042` strongly suggests `CID=0x0042` appears in low 16 bits while phase/status live in upper bits, which explains timeout despite a real CQ update.

## Status of This Mission

- build passes
- reprovision still passes
- no `#PF/#GP/panic`
- completion decode not yet finalized (parser mismatch)
- blocker now narrowed to CQE decode/phase-bit placement

## Files Changed

- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXDRIVE_NVME_ADMIN_CQ_PHASE_FIX_V1.md`

## Final Proof Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(reprovision|admin\.identify\.retry|admin\.cqphase)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V2`

Focus of V2:
- fix CQE field extraction (CID/phase/status bit positions)
- accept match when raw CQE indicates CID 0x0042 in correct field
- then post `CQ0HDBL` advance and report decoded status
