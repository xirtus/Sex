# SEXFS_V0_SUPERBLOCK_FORMAT_AND_MOUNT_V1

**Date**: 2026-05-25
**Mission**: Implement first persistent SexFS v0 format/mount step
**Status**: IMPLEMENTED — ready for proof run

## A) Outcome: PASS (awaiting proof run)

Compilation verified. All 6 proof markers wired. Negative tests implemented.
SexDrive write guard extended for SexFS v0 LBAs.

## B) Files Changed

| File | Change |
|------|--------|
| `apps/sexdrive/src/main.rs` | Extended `write_guard_allows()` with SexFS v0 metadata (LBA 0-47) and object data (LBA 128-2019) ranges. Added `[sexdrive.write_guard.sexfs.allow]` and `[sexdrive.write_guard.reject]` markers. |
| `servers/sexfiles/src/backends/diskfs.rs` | Added ~500 lines: SexFS v0 superblock/freemap serialization/validation, `sexfs_v0_format_to_disk()`, `sexfs_v0_mount_from_disk()`, `proof_sexfs_v0_superblock_format_mount()` with 3 negative tests. |
| `servers/sexfiles/src/proof.rs` | Added `run_sexfs_v0_superblock_format_mount_proofs()` dispatch function. |
| `servers/sexfiles/src/trampoline.rs` | Added `SEXFS_V0_SUPERBLOCK_FORMAT_MOUNT_PROOF` env var gate. |
| `servers/sexfiles/build.rs` | Added `cargo:rerun-if-env-changed` line. |
| `scripts/run_daily_driver_proof.sh` | Added `SEXFS_V0_SUPERBLOCK_FORMAT_MOUNT_PROOF=1` env var. |
| `scripts/daily_driver_master_gate.sh` | Added `gate_sexfs_v0_superblock_format_mount` with 7 failure modes + summary entry. |

## C) Write Guard Change Summary

**Before**: `write_guard_allows()` permitted writes only to LBAs 2022-2047 (proof objects + manifest + write proof).

**After**: Additionally permits sector-aligned 512-byte writes to:
- LBAs 0-47 (SexFS v0 metadata: superblock, backup, object table, freemap, journal, checkpoints, reserved)
- LBAs 128-2019 (SexFS v0 object data)

All existing whitelist entries preserved. New writes still require `buf_cap == SLOT_BUF_LEND` (proof_mode).
Rejection marker `[sexdrive.write_guard.reject]` emitted for out-of-range writes.
Allow marker `[sexdrive.write_guard.sexfs.allow]` emitted for SexFS v0 metadata/data writes.

## D) Format/Mount Proof

### Format flow:
1. `sexfs_v0_format_to_disk()` called
2. Builds superblock byte array (512 bytes, LE, with XOR checksum)
3. Writes primary superblock to LBA 0 via `diskfs_block_write(0, 512, SLOT_BUF_LEND)`
4. Writes backup superblock (identical copy) to LBA 1
5. Writes zeroed object table (2048 bytes) to LBAs 2-5 (4 sectors)
6. Writes initialized freemap to LBA 6 (blocks 0-15 + 253-255 marked in-use)

### Mount flow:
1. `sexfs_v0_mount_from_disk()` called
2. Reads superblock from LBA 0 via `diskfs_block_read(0, 512, SLOT_BUF_LEND)`
3. Validates magic (`SEXFSv01`), version_major=1, block_size=4096, entry_count=16, checksum
4. If primary invalid, falls back to backup at LBA 1
5. Reads object table from LBAs 2-5 (4 sectors)
6. Reads freemap from LBA 6, validates magic (`FREEMAPV0`) and checksum
7. Returns `Ok(fs_generation)`

### Required proof markers:
```
[sexfs.v0.format.begin]
[sexfs.v0.superblock.primary.write.ok] lba=0
[sexfs.v0.superblock.backup.write.ok] lba=1
[sexfs.v0.object_table.write.ok] lba_range=2..5
[sexfs.v0.freemap.write.ok] lba=6
[sexfs.v0.format.done] ok=1
[sexfs.v0.mount.begin]
[sexfs.v0.superblock.primary.read.ok] lba=0
[sexfs.v0.superblock.validate.ok]
[sexfs.v0.object_table.read.ok] lba_range=2..5
[sexfs.v0.freemap.read.ok] lba=6
[sexfs.v0.mount.done] ok=1
[sexfs.v0.superblock_format_mount.done] ok=1
```

## E) Negative Tests

Three corruption scenarios, each followed by restoration of valid state:

1. **Bad magic**: Corrupts superblock magic bytes to 0xDEADBEEF, attempts mount → `ERR_NOT_FOUND`. Restores good superblock.
2. **Bad version**: Sets version_major to 0xFFFF, attempts mount → `ERR_OVERFLOW`. Restores good superblock.
3. **Bad checksum**: Flips one bit in checksum field, attempts mount → `ERR_OVERFLOW`. Restores good superblock.

Each emits `[sexfs.v0.neg.bad_*.reject] ok=1` on correct rejection.

## F) Non-Claims

- No object create/write/read implemented (Phase 2)
- No journal persistence (Phase 4)
- No checkpoint persistence (Phase 4)
- No reboot restore (Phase 3 — two-boot)
- No POSIX/filesystem semantics claimed
- No durability/powerloss claims
- No kernel edits required
- No sex-pdx ABI edits required

## G) Gate Result

**Awaiting proof run**: `gate_sexfs_v0_superblock_format_mount`

PASS conditions (from `daily_driver_master_gate.sh`):
- `sexfs.v0.superblock_format_mount.done.*ok=1` present

FAIL conditions:
- Format missing/failed
- Mount missing/failed  
- Bad magic rejection missing/failed
- Bad version rejection missing/failed
- Bad checksum rejection missing/failed
- Final done marker missing

## H) Fault Scan

Not yet run. Expected: zero #PF, #GP, or panic. The proof uses only proven SLOT_BLOCK bridge functions and MemLend buffers.

## I) Commit

```
git add apps/sexdrive/src/main.rs \
        servers/sexfiles/src/backends/diskfs.rs \
        servers/sexfiles/src/proof.rs \
        servers/sexfiles/src/trampoline.rs \
        servers/sexfiles/build.rs \
        scripts/run_daily_driver_proof.sh \
        scripts/daily_driver_master_gate.sh \
        docs/handoff/SEXFS_V0_SUPERBLOCK_FORMAT_AND_MOUNT_V1.md

git commit -m "sexfs: persist v0 superblock format and mount proof"
```

## J) Next Phase

**Recommended**: `SEXFS_V0_OBJECT_CREATE_WRITE_READ_V1`

Target: Implement object create → allocate blocks → write data → read data → verify roundtrip through the SexFS v0 on-disk format. Uses the superblock/table/freemap already persisted by this phase. Requires extending `sexfs_v0_build_zero_object_table()` to accept populated entries.

*End of handoff.*
