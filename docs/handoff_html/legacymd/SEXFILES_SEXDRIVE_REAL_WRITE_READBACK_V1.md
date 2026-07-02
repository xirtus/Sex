# SEXFILES_SEXDRIVE_REAL_WRITE_READBACK_V1

## Mission
Prove SexFiles/DiskFS can perform guarded BLOCK_WRITE via SLOT_BLOCK and read back the same reserved LBA through MemLend payload path.

## Result
PASS (block-level write/readback proof)

## 1) Write payload pattern
SexFiles writes deterministic payload into MemLend buffer before BLOCK_WRITE:
- bytes `0..8`: `0x3156455449525753` (`SWRITEV1` LE)
- bytes `8..16`: `2047` (reserved proof LBA)
- bytes `16..24`: `0xA5A5A5A5A5A5A5A5`
- remaining bytes: deterministic pattern (`(i ^ 0x5A)` for `i in 0..512`)

## 2) Write command chain
Observed markers:
- `[sexfiles.realwrite.begin]`
- `[sexfiles.realwrite.bufcap.grant.ok] ok=1 ...`
- `[sexfiles.diskfs.typed.write.call] offset=0xffe00 size=512 buf_cap=0x11`
- `[sexdrive.block.write.api.recv] offset=0xffe00 size=512 buf_cap=0x11`
- `[sexdrive.write.guard.allow] ...`
- `[sexdrive.block.write.api.nvme.submit] ...`
- `[sexdrive.block.write.api.cqe] ...`
- `[sexdrive.block.write.api.ok] ...`
- `[sexfiles.realwrite.write.reply.ok] status=0`

## 3) Readback command chain
Observed markers:
- `[sexfiles.realwrite.readback.begin] offset=0xffe00 size=512`
- `[sexfiles.diskfs.typed.read.call] offset=0xffe00 size=512 buf_cap=0x11`
- `[sexdrive.nvme.write.readback.begin] slba=2047`
- `[sexdrive.nvme.write.readback.cqe] ...`
- `[sexfiles.realwrite.readback.reply.ok] status=0`

## 4) Match result
Observed:
- `[sexdrive.nvme.write.readback.match] magic=0x3156455449525753 slba=2047`
- `[sexfiles.realwrite.readback.match] magic=0x3156455449525753 lba=2047 tag=0xa5a5a5a5a5a5a5a5`

## 5) Guard behavior
- LBA0 write denied in same run:
  - `[sexdrive.write.guard.begin] offset=0x0 ... proof_mode=0`
  - `[sexdrive.write.guard.deny] ...`
- Reserved proof write allowed only for:
  - `buf_cap == SLOT_BUF_LEND`
  - `offset == 0xffe00`
  - `size == 512`

## 6) Negative tests
Still passing (`typed_summary honest=1`):
- bad cmd -> `ERR_BAD_CMD`
- bad len -> `ERR_BAD_LEN`
- unaligned -> `ERR_BAD_LEN`
- sync -> `ERR_NO_DEVICE`
- LBA0 write denied (guard marker + `ERR_NO_DEVICE`)

## 7) Files changed
- `servers/sexfiles/src/proof.rs`
- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXFILES_SEXDRIVE_REAL_WRITE_READBACK_V1.md`

## Build/Runtime
- `build_payload.sh`: PASS
- runtime gate build: PASS
- no `#PF/#GP/panic`
- `FINAL_SCORE=RED_MASTER` remains due unrelated `CLOCK_GATE` miss

## 8) Final grep command
```bash
grep -E "sexfiles\.realwrite|sexfiles\.diskfs\.typed\.(write|read)\.call|sexdrive\.block\.write\.api|sexdrive\.write\.guard|sexdrive\.nvme\.write\.|#PF|#GP|panic" .gate_master/serial.log
```

## 9) Next prompt
`SEXFILES_PERSISTENCE_REBOOT_PROOF_V1`
