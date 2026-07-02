# SEXDRIVE_NVME_IO_QUEUE_CREATE_PROOF_V1

- date: 2026-05-07
- scope: create one IO CQ and one IO SQ only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no IO read/write in this mission

## Result

PASS. IO CQ and IO SQ creation both completed successfully through the proven admin path.

## IO Queue Allocation

- `io_cq_phys=0x1f919000`, `io_cq_va=0x400000008000`
- `io_sq_phys=0x102ac000`, `io_sq_va=0x400000009000`
- both pages page-aligned and zeroed

## Admin Command Fields Used

### Create IO Completion Queue
- opcode `0x05`
- CID `67`
- QID `1`
- QSIZE `15` (depth 16)
- PRP1 = `io_cq_phys`
- CDW10 = `qid | (qsize<<16)`
- CDW11 = `0x1` (`PC=1`, `IEN=0`, `IV=0`)

### Create IO Submission Queue
- opcode `0x01`
- CID `68`
- QID `1`
- QSIZE `15`
- PRP1 = `io_sq_phys`
- CDW10 = `qid | (qsize<<16)`
- CDW11 = `(CQID=1)<<16 | PC=1 | QPRIO=0`

## CQE Results

- Create CQ completion:
  - marker: `[sexdrive.nvme.ioq.create_cq.cqe] cid=67 dw2=0x2 dw3=0x10043 phase=1`
  - status: `sc=0 sct=0` (`[sexdrive.nvme.ioq.create_cq.ok]`)
- Create SQ completion:
  - marker: `[sexdrive.nvme.ioq.create_sq.cqe] cid=68 dw3=0x10044 phase=1`
  - status: `sc=0 sct=0` (`[sexdrive.nvme.ioq.create_sq.ok]`)

## IO Doorbell Offsets (QID 1, DSTRD=0)

- `SQ1TDBL = 0x1008`
- `CQ1HDBL = 0x100C`

Ready marker:
- `[sexdrive.nvme.ioq.ready] qid=1 depth=16 sq_tail=0 cq_head=0 cq_phase=1 sq1tdbl=0x1008 cq1hdbl=0x100c`

## Safety

- no `#PF`, `#GP`, `panic`
- block path still honest pre-IO (no `BLOCK_READ` OK)

## Files Changed

- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXDRIVE_NVME_IO_QUEUE_CREATE_PROOF_V1.md`

## Final Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(admin\.identify\.v2|ioq)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_NVME_IO_READ_ONE_BLOCK_PROOF_V1`
