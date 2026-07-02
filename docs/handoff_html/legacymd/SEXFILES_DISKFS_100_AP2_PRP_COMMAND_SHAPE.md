# SEXFILES_DISKFS_100_AP2_6 PRP/Command Shape Diagnostic

## 1) Files changed
- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXFILES_DISKFS_100_AP2_PRP_COMMAND_SHAPE.md`

## 2) Markers added
- `[sexdrive.nvme.cmd.shape]` for:
  - `path=selftest op=WRITE`
  - `path=selftest op=READ`
  - `path=typed op=WRITE`
  - `path=typed op=READ`
- `[sexdrive.nvme.cmd.range] path=typed ...`
- `[sexdrive.nvme.cmd.shape.err] path=typed reason=memlend_map_failed buffer_cap=...`

## 3) Self-test command shape (from `/tmp/sexfiles_diskfs_ap26_prp_shape.log`)
- WRITE (`cid=1280`):
  - `op=WRITE nsid=1 slba=2047 nlb=0 bytes=512`
  - `prp1=0x1f804000 prp2=0x0`
  - `buf_va=0x40000000b000 buf_phys=0x1f804000 buffer_kind=internal`
  - completion: `found=1 status=0 polls=1`
- READ (`cid=1281`):
  - `op=READ nsid=1 slba=2047 nlb=0 bytes=512`
  - `prp1=0x1f805000 prp2=0x0`
  - `buf_va=0x40000000c000 buf_phys=0x1f805000 buffer_kind=internal`
  - completion: `found=1 status=0 polls=1`

## 4) Typed DiskFS command shape (failing AP2 commands)
- READ (`cid=1291` from `sexfiles.diskfs.block.call op=READ lba=2046 bytes=512 buffer_cap=0x11`):
  - `op=READ nsid=1 slba=2046 nlb=0 bytes=512`
  - `prp1=0x102ea000 prp2=0x0`
  - `buf_va=0x400000369000 buf_phys=0x102ea000 buffer_cap=0x11 buffer_kind=memlend`
  - range marker: `max_lba=2047 ok=1`
  - completion: `found=0 status=0 polls=1000000` timeout; `seen_cid=0 seen_phase=1 seen_status=16395`
- WRITE (`cid=1292` from `sexfiles.diskfs.block.call op=WRITE lba=2046 bytes=512 buffer_cap=0x11`):
  - `op=WRITE nsid=1 slba=2046 nlb=0 bytes=512`
  - `prp1=0x102eb000 prp2=0x0`
  - `buf_va=0x40000036b000 buf_phys=0x102eb000 buffer_cap=0x11 buffer_kind=memlend`
  - range marker: `max_lba=2047 ok=1`
  - completion: `found=0 status=0 polls=1000000` timeout; `seen_cid=0 seen_phase=1 seen_status=16395`

## 5) Difference table
| Field | Self-test | Typed (failing) | Result |
|---|---|---|---|
| opcode | READ/WRITE | READ/WRITE | same class |
| nsid | 1 | 1 | same |
| slba/nlb | valid (`2047/0`) | valid (`2046/0`) | both in-range |
| bytes | 512 | 512 | same |
| PRP2 | 0 | 0 | same |
| PRP1 | non-zero page-aligned phys | non-zero page-aligned phys | same shape |
| buffer_kind | internal | memlend (source/dest VA), but PRP uses local allocated phys | data-source differs, PRP programming shape same |
| completion | immediate CQE status=0 | no CQE for typed CID | divergence at device completion |

## 6) Root cause classification (A-G)
- **F**: range is valid and PRP looks valid; deeper NVMe queue/device diagnostic required.
- Evidence:
  - Typed PRP/NSID/SLBA/NLB/bytes are valid and command-shaped like successful self-test commands.
  - Typed commands still do not receive CQE at `cq_head=10` with `phase=1` and stale observed entry (`cid=0 status=16395`).

## 7) AP2 status
- **AP2 remains blocked**.
- DiskFS typed READ/WRITE at `lba=2046` still time out (`cid=1291/1292`).

## 8) Next patch recommendation
- Keep command/PRP fields unchanged.
- Instrument IOQ/CQ slot state around `cq_head=10` and SQ slot ownership transitions specifically for post-selftest typed submissions (queue state continuity and doorbell/CQ consumption ordering), since command-shape mismatch was not observed.
