# SEXFILES_DISKFS_100_AP2_FIXED_OBJECT_BRIDGE_RW

## 1) Files changed
- servers/sexfiles/src/vfs.rs
- servers/sexfiles/src/trampoline.rs
- scripts/run_daily_driver_proof.sh
- scripts/daily_driver_master_gate.sh

## 2) Exact env vars
- `SEXFILES_DISKFS_100_PROOF=1`
- Runner coupling: when `SEXFILES_DISKFS_100_PROOF=1`, runner forces `SEXOS_STORAGE_100_PROOF=1`.

## 3) Object identity
- Fixed object: `sexfiles-proof-v1`
- Path: `/disk/sexfiles-proof-v1`
- SELECT path_id: `0`

## 4) Methods used
- `DiskFs::diskfs_write_object(path, offset, data, buf_va)`
- `DiskFs::diskfs_read_object(path, offset, out, buf_va)`

## 5) Payload/chunking
- Payload size: 128 bytes
- Pattern: `byte[i] = (0xC7 ^ i ^ 0x55) & 0xFF`
- Chunking: 16-byte writes, 16-byte reads (8 chunks each)

## 6) AP2.1 blocker root cause (manifest_ensure_v2_failed code=4)
- Status: **STOP FIRST (not fixed in this mission)**.
- Root cause classification: **A)** manifest ensure hits real block path and receives `BLOCK_ERR_NO_DEVICE(4)` from SexDrive, not a VFS/order bug.
- Exact call chain:
  - `OP_DISKFS_SELECT` → `handle_diskfs_select()` calls `DiskFs::diskfs_ensure_manifest_v2(buf_va)` before select success.
  - `diskfs_ensure_manifest_v2()` performs real `diskfs_block_read/write` on `DISKFS_MANIFEST_LBA=2046`.
  - block status is propagated unchanged; nonzero returns from select as `[sexfiles.bridge.diskfs.select.err] reason=manifest_ensure_v2_failed code=4`.
- Source anchors:
  - `servers/sexfiles/src/vfs.rs`: select path manifest ensure + error return.
  - `servers/sexfiles/src/backends/diskfs.rs`: `diskfs_ensure_manifest_v2` read/write/verify block calls and `Err(write_status|verify_status)`.
- Runtime markers from AP2 run:
  - `[sexfiles.disk.manifest.v2.ensure.begin]`
  - `[sexdrive.block.read.handoff.err] reason=no_ioq_ready`
  - `[sexfiles.disk.manifest.v2.err] reason=read_failed status=4`
  - `[sexfiles.disk.manifest.v2.bootstrap] entries=3`
  - `[sexdrive.nvme.write.err] reason=no_ioq_ready`
  - `[sexfiles.disk.manifest.v2.err] reason=write_failed status=4`

## 7) Marker evidence
- `[sexfiles.diskfs100.ap2.begin] object=sexfiles-proof-v1 bytes=128`
- `[sexfiles.diskfs100.ap2.select.ok] object=sexfiles-proof-v1`
- `[sexfiles.diskfs100.ap2.write.chunk] off=O len=L ok=1`
- `[sexfiles.diskfs100.ap2.read.chunk] off=O len=L ok=1`
- `[sexfiles.diskfs100.ap2.read.match] bytes=128 ok=1`
- `[sexfiles.diskfs100.ap2.done] ok=1`
- Failure markers:
  - `[sexfiles.diskfs100.ap2.fail] reason=...`
  - `[sexfiles.diskfs100.ap2.read.match] ok=0 first_bad=I expected=E got=G`

## 8) Gate result
- New gate: `sexfiles_diskfs_bridge_fixed_object_rw`
- PASS only if:
  - `sexdrive.nvme.ioq.ready` exists
  - `sexfiles.diskfs100.ap2.select.ok` exists
  - `sexfiles.diskfs100.ap2.read.match bytes=128 ok=1` exists
  - `sexfiles.diskfs100.ap2.done ok=1` exists
- FAIL if:
  - `no_ioq_ready` exists
  - `sexfiles.diskfs100.ap2.fail` exists
  - any required success marker is missing
- SKIP if AP2 begin marker is absent

## 9) AP2/default run results (this mission)
- AP2 profile (`SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_PROOF=1`):
  - `sexdrive_storage_ioq_ready`: `PASS`
  - `sexfiles_diskfs_bridge_fixed_object_rw`: `FAIL`
  - master gate: `FAIL gates: 2`, `FINAL: FAIL`
- Default profile (no AP2 env):
  - `sexdrive_storage_ioq_ready`: `SKIP`
  - `sexfiles_diskfs_bridge_fixed_object_rw`: `SKIP`
  - master gate: `FAIL gates: 0`, `FINAL: PASS`

## 10) Default boot result
- Default profile keeps this gate SKIP unless `SEXFILES_DISKFS_100_PROOF=1` is set.

## 11) Non-claims
- no Linen
- no generic VFS path claims
- no directory claims
- no fsync durability claims
- no power-loss durability claims

## 12) Updated ladder
- AP1: bridge dispatch and honest blocker classification
- AP2: fixed-object 128B write/read/match proof lane exists, but blocked by AP2.1 manifest ensure `code=4`
- AP3 (next): durability semantics and persistence-cycle checks
