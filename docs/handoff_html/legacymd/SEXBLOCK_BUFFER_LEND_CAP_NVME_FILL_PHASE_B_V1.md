# SEXBLOCK_BUFFER_LEND_CAP_NVME_FILL_PHASE_B_V1

## Mission
Replace Phase A pattern-fill with real NVMe-backed payload handoff through MemLend buffer cap.

## Result
PASS (Phase B)

## Implementation Path
- Used **bounce-buffer copy** path (safe path):
  1. SexDrive maps MemLend VA via `sys_map_mem_lend(SLOT_BUF_LEND)`.
  2. SexDrive performs real NVMe READ via existing IOQ path into SexDrive-owned DMA bounce page.
  3. After CQE success, SexDrive copies exactly 512 bytes from bounce to MemLend VA.
  4. Reply `OK` only after CQE success and copy success.
- Did **not** pass MemLend VA directly as PRP (no unsafe VA→phys translation).

## Validation Rules
In `BLOCK_READ` when `buf_cap == SLOT_BUF_LEND`:
- `size` must equal `512`, else `ERR_BAD_LEN`.
- `offset` still must be sector-aligned (`offset % 512 == 0`).
- `sys_map_mem_lend(SLOT_BUF_LEND)` must return valid VA (`!= 0`, `!= u64::MAX`), else `ERR_NO_DEVICE`.
- Real NVMe CQE must indicate success before payload copy and `OK` reply.

## Proof Markers (Observed)
From `.gate_master/serial.log`:
- `[sexblock.bufcap.phase_b.begin]`
- `[kernel.memlend.grant.ok] va=0x400000356000 phys=0x1f90c000 len=4096`
- `[kernel.memlend.map.ok] va=0x400000357000 len=4096`
- `[sexdrive.bufcap.map.ok] fill_va=0x400000357000`
- `[sexdrive.block.read.handoff.nvme.begin] offset=0x0 size=512 dst_va=0x400000357000`
- `[sexdrive.block.read.handoff.nvme.cqe] cid=1281 phase=1 dw2=0x10003 dw3=0x10501`
- `[sexdrive.block.read.handoff.copy.ok] phase=B len=512`
- `[sexfiles.bufcap.verify.ok] phase=B overwritten=1 first_byte=0x0 reply=0`
- `[sexblock.bufcap.phase_b.ok]`

## Payload Verification
- SexFiles wrote sentinel `0xA5` into first 512 bytes before call.
- After reply, first byte changed to `0x00` (`overwritten=1`), consistent with zero-filled QEMU NVMe image and valid success.

## Negative Typed Tests
Still passing (`typed_summary honest=1`):
- bad cmd -> `ERR_BAD_CMD`
- bad len -> `ERR_BAD_LEN`
- unaligned offset -> `ERR_BAD_LEN`
- `BLOCK_WRITE`/`BLOCK_SYNC` unchanged -> `ERR_NO_DEVICE`

## Files Changed
- `apps/sexdrive/src/main.rs`
- `servers/sexfiles/src/proof.rs`
- `docs/handoff/SEXBLOCK_BUFFER_LEND_CAP_NVME_FILL_PHASE_B_V1.md`

## Build/Gate
- `build_payload.sh`: PASS
- Runtime gate build: PASS
- `FINAL_SCORE` remained `RED_MASTER` due unrelated `CLOCK_GATE` miss (`silkbar.clock.send`), not storage path failure.

## Final Grep Command
```bash
grep -E "memlend|bufcap|phase_b|handoff\.nvme|handoff\.copy|handoff\.err|#PF|#GP|panic" .gate_master/serial.log
```

## Next Prompt
`SEXFILES_DISKFS_READ_PAYLOAD_PROOF_V1`
