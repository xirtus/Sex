# SEXDRIVE_STORAGE_100_AP3_SINGLE_BLOCK_RW

## Scope
Advance SexDrive/storage 100 from AP2 IOQ-ready to AP3 real single-block write/read/match proof, with no kernel edits and no sex-pdx ABI edits.

## Files changed
- `apps/sexdrive/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Backups created
- `scripts/daily_driver_master_gate.sh.sexdrive_storage_100_ap3.bak`
- `apps/sexdrive/src/main.rs.sexdrive_storage_100_ap3.bak`
- `servers/sexfiles/src/backends/diskfs.rs.sexdrive_storage_100_ap3.bak` (if present)

## Proof path used
A) Existing SexDrive self-test/proof hook (chosen)
- Reused existing real NVMe IOQ path and CQE status checks in `sexdrive`.
- Added AP3 self-test trigger immediately after `[sexdrive.nvme.ioq.ready]` when NVMe setup succeeds.
- No Linen involvement.

## AP3 payload
- LBA: `2047` (existing safe proof LBA already used by current storage proof guard)
- Block size: `512`
- Pattern: `byte[i] = (0xA5 ^ i ^ 0x3C) & 0xFF`
- Writes one block, reads same block, compares all 512 bytes.

## Required markers emitted
- `[sexdrive.storage100.rw.begin] lba=2047 bytes=512`
- `[sexdrive.storage100.write.submit] lba=2047 bytes=512`
- `[sexdrive.storage100.write.complete] status=0 bytes=512`
- `[sexdrive.storage100.read.submit] lba=2047 bytes=512`
- `[sexdrive.storage100.read.complete] status=0 bytes=512`
- `[sexdrive.storage100.read.match] lba=2047 bytes=512 ok=1`
- `[sexdrive.storage100.rw.done] ok=1`

Failure marker path is wired:
- `[sexdrive.storage100.rw.fail] reason=...`

## Gate added
- `sexdrive_storage_single_block_rw` in `scripts/daily_driver_master_gate.sh`
- Behavior:
  - SKIP if rw.begin absent
  - FAIL if rw.begin present but IOQ ready absent
  - FAIL on `no_ioq_ready`
  - FAIL on `rw.fail`
  - FAIL if write/read completion status markers are missing or nonzero
  - FAIL if read.match `ok=1` missing
  - PASS only with IOQ ready + write.complete status=0 + read.complete status=0 + read.match ok=1 + rw.done ok=1

## Runtime proof evidence (SEXOS_STORAGE_100_PROOF=1)
From `/tmp/sexos_daily_driver_proof.log`:
- `1571:[sexdrive.nvme.ioq.ready] qid=1 depth=16 ...`
- `1572:[sexdrive.storage100.rw.begin] lba=2047 bytes=512`
- `1575:[sexdrive.storage100.write.submit] lba=2047 bytes=512`
- `1582:[sexdrive.storage100.write.complete] status=0 bytes=512`
- `1583:[sexdrive.storage100.read.submit] lba=2047 bytes=512`
- `1586:[sexdrive.storage100.read.complete] status=0 bytes=512`
- `1587:[sexdrive.storage100.read.match] lba=2047 bytes=512 ok=1`
- `1588:[sexdrive.storage100.rw.done] ok=1`

Gate result on proof log with proof env enabled:
- `sexdrive_storage_ioq_ready PASS`
- `sexdrive_storage_single_block_rw PASS`
- `faults_zero PASS`
- `FINAL: PASS`

## Default daily-driver check
Default run (without `SEXOS_STORAGE_100_PROOF=1`) keeps storage gates SKIP as expected:
- `sexdrive_storage_ioq_ready SKIP`
- `sexdrive_storage_single_block_rw SKIP`

Observed unrelated blocker in this run:
- `atlas_phase_e2_keyboard_scene_cycle FAIL`
- Therefore global default final was `FINAL: FAIL (1 gate(s) failed)` in that run.

## Updated ladder
- AP1: BAR-cap resolve path exists (prior)
- AP2: IOQ-ready gate PASS (prior)
- AP3: single-block write/read/match gate PASS (this change)
- AP4+: durable persistence/reboot claim remains out-of-scope and not claimed.

## STOP FIRST blockers
None for AP3 implementation.
- No kernel edits needed.
- No sex-pdx ABI edits needed.
- Existing capability/buffer path was sufficient.
- Real NVMe I/O CQE path used (not RAM-only fake).
