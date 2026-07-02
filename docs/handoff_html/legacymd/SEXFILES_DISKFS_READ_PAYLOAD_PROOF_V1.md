# SEXFILES_DISKFS_READ_PAYLOAD_PROOF_V1

## Mission
Prove a real DiskFS read path can return payload bytes from SexDrive via MemLend, beyond the earlier synthetic-only flow.

## Outcome
PASS (DiskFS block-level payload path)

## 1) DiskFS read reality
- Proven scope is **block-level DiskFS payload read**, not file-level semantics.
- Implemented as a narrow helper in `DiskFs`:
  - `DiskFs::proof_diskfs_read_payload_block()`
- No manifest/path/file mapping claims were added.

## 2) Did payload path reach real DiskFS API?
Yes.
- `proof.rs` now calls `DiskFs::proof_diskfs_read_payload_block()`.
- That helper performs:
  - MemLend grant (`sys_grant_mem_lend`)
  - sentinel fill (`0xA5`)
  - typed `DiskFs::diskfs_block_read(0, 512, SLOT_BUF_LEND)`
  - reply check
  - payload verify from granted producer VA

## 3) Proof markers observed
From `.gate_master/serial.log`:
- `[sexfiles.diskfs.payload.read.begin] offset=0x0 size=512`
- `[sexfiles.diskfs.payload.bufcap.grant.ok] ...`
- `[sexfiles.diskfs.payload.block.call] cmd=BLOCK_READ ...`
- `[sexdrive.block.read.handoff.nvme.begin] ...`
- `[sexdrive.block.read.handoff.nvme.cqe] ...`
- `[sexdrive.block.read.handoff.copy.ok] phase=B len=512`
- `[sexfiles.diskfs.payload.reply.ok] status=0`
- `[sexfiles.diskfs.payload.verify.ok] overwritten=1 first_byte=0x0`
- `[sexblock.bufcap.phase_b.ok]`

## 4) First byte/checksum result
- Sentinel overwrite verified.
- First byte changed from `0xA5` to `0x00` (valid for zero-filled QEMU NVMe image).

## 5) Negative test status
- Preserved and passing (`typed_summary honest=1`):
  - bad cmd -> `ERR_BAD_CMD`
  - bad len -> `ERR_BAD_LEN`
  - unaligned offset -> `ERR_BAD_LEN`
  - `BLOCK_WRITE`/`BLOCK_SYNC` unchanged -> `ERR_NO_DEVICE`

## 6) Files changed
- `servers/sexfiles/src/backends/diskfs.rs`
- `servers/sexfiles/src/proof.rs`
- `docs/handoff/SEXFILES_DISKFS_READ_PAYLOAD_PROOF_V1.md`

## Build/Runtime
- `build_payload.sh`: PASS
- Runtime gate build: PASS
- No `#PF/#GP/panic` for this mission path
- `FINAL_SCORE` still `RED_MASTER` due unrelated `CLOCK_GATE` miss

## 7) Final grep command
```bash
grep -E "sexfiles\.diskfs\.payload|sexblock\.bufcap\.phase_b|sexdrive\.block\.read\.handoff\.nvme|sexdrive\.block\.read\.handoff\.copy|#PF|#GP|panic" .gate_master/serial.log
```

## 8) Next prompt
`SEXDRIVE_NVME_WRITE_GUARD_V1`
