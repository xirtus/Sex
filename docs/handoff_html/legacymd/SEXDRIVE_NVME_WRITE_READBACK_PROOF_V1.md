# SEXDRIVE_NVME_WRITE_READBACK_PROOF_V1

## Mission
Execute one guarded NVMe WRITE to reserved proof LBA and verify readback marker.

## Result
PASS

## Implementation
- Guard preserved:
  - write allowed only when `buf_cap == SLOT_BUF_LEND`, `offset == 0xffe00`, `size == 512`
  - all other writes remain denied (`ERR_NO_DEVICE`)
- For guarded write only:
  - map MemLend buffer (`sys_map_mem_lend`)
  - build write DMA buffer (page-aligned) from mapped content
  - write marker:
    - bytes 0..8 = `WRITE_PROOF_MAGIC` (`0x3156455449525753`)
    - bytes 8..16 = `WRITE_PROOF_LBA` (`2047`)
  - submit NVMe IO WRITE (`opcode=0x01`) to `SLBA=2047`, `NLB=0`
  - poll CQE success
  - submit NVMe IO READ (`opcode=0x02`) from same LBA
  - poll CQE success and verify readback magic match

## 1) Write buffer phys/virt
From runtime marker:
- write `PRP1 phys = 0x1f907000`
- write CQ submit marker includes this address

## 2) Write command fields
From marker:
- `opcode=0x01` (WRITE)
- `cid=1282`
- `nsid=1`
- `slba=2047`
- `nlb=0`
- `prp1=0x1f907000`
- `sq_tail=3`

## 3) Write CQE decode
From marker:
- `[sexdrive.nvme.write.cqe] cid=1282 phase=1 dw2=0x0 dw3=0x10502`
- status interpreted as success in helper path (continued to readback)

## 4) Readback result
From markers:
- `[sexdrive.nvme.write.readback.cqe] cid=1283 phase=1 dw2=0x10005 dw3=0x10503`
- `[sexdrive.nvme.write.readback.match] magic=0x3156455449525753 slba=2047`
- Guarded BLOCK_WRITE probe returned `status=0`

## 5) Guard behavior
From markers:
- denied path:
  - `[sexdrive.write.guard.begin] offset=0x0 size=512 buf_cap=0x0 proof_mode=0`
  - `[sexdrive.write.guard.deny] ...`
- allowed path:
  - `[sexdrive.write.guard.begin] offset=0xffe00 size=512 buf_cap=0x11 proof_mode=1`
  - `[sexdrive.write.guard.allow] ...`
- LBA 0 remains denied.

## 6) Files changed
- `apps/sexdrive/src/main.rs`
- `servers/sexfiles/src/proof.rs` (probe expectation text only)
- `docs/handoff/SEXDRIVE_NVME_WRITE_READBACK_PROOF_V1.md`

## Build / Runtime
- `build_payload.sh`: PASS
- runtime gate build: PASS
- no `#PF/#GP/panic` in this proof path
- negative typed tests still pass (`typed_summary honest=1`)
- `FINAL_SCORE` remains `RED_MASTER` due unrelated `CLOCK_GATE` miss

## 7) Final grep command
```bash
grep -E "sexdrive\\.nvme\\.write\\.|sexdrive\\.write\\.guard\\.|sexfiles\\.block\\.proof\\.write_guard\\.probe|#PF|#GP|panic" .gate_master/serial.log
```

## 8) Next prompt
`SEXFILES_SEXDRIVE_REAL_WRITE_READBACK_V1`
