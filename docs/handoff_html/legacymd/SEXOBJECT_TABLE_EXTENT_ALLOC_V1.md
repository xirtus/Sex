# SEXOBJECT_TABLE_EXTENT_ALLOC_V1

**Date**: 2026-05-25
**Mission**: Implement freemap-backed extent allocation for a SexObject table entry and prove object data content write/read through real SexDrive/NVMe.
**Status**: PASS

---

## A) Outcome

All required proof markers present. Gate `sexobject_table_extent_alloc` = PASS.
Gate `sexfs_v0_superblock_format_mount` = PASS (pre-existing, unchanged).
Gate `sexobject_table_persist` = PASS (pre-existing, unchanged).
Gate `linen_sexfiles_100_current_tier_release` = SKIP (not triggered, unchanged).
Pre-existing `linen_diskfs_direct` FAIL is unrelated and unchanged.

---

## B) Files Changed

| File | Delta | Purpose |
|------|-------|---------|
| `servers/sexfiles/src/backends/diskfs.rs` | +~280 | Freemap helpers, data sector I/O, extent bounds validator, extent alloc proof |
| `servers/sexfiles/src/proof.rs` | +25 | `run_sexobject_table_extent_alloc_proofs()` gate runner |
| `servers/sexfiles/src/trampoline.rs` | +6 | Dispatch for `SEXOBJECT_TABLE_EXTENT_ALLOC_PROOF` env var |
| `servers/sexfiles/build.rs` | +1 | `rerun-if-env-changed` for new env var |
| `scripts/run_daily_driver_proof.sh` | +7 | Env var export + NVMe trigger for extent alloc proof |
| `scripts/daily_driver_master_gate.sh` | +40 | `sexobject_table_extent_alloc` gate init, check logic, ALL_GATES entry |
| `docs/handoff/SEXOBJECT_TABLE_EXTENT_ALLOC_V1.md` | new | This file |

---

## C) Allocation/Data Proof Design

**Freemap block mapping**:
- Block N = sector LBA N×8 (each block = 4KiB = 8 sectors)
- Block 16 = LBA 128 (first block in data region, first free after metadata reservation)
- Block 252 = LBA 2016 (last block in data region)
- Data region guard: sector LBAs [128, 2019]

**Helpers added** (`diskfs.rs` module level):
- `sexfs_v0_recompute_freemap_checksum` — XOR bytes [0..16)+[20..32)+[32..160), store at [16..20)
- `sexfs_v0_read_freemap_sector` — read LBA 6 → `[u8; 512]`
- `sexfs_v0_write_freemap_sector` — write LBA 6 ← `[u8; 512]`
- `sexfs_v0_freemap_alloc_block_in_range` — first-fit alloc in [min, max], marks bit, recomputes checksum
- `sexfs_v0_freemap_alloc_specific` — try to mark specific block used; ERR_OVERFLOW if already used
- `sexfs_v0_freemap_is_block_used` — read-only bit check
- `sexfs_v0_write_data_sector` — write 512 bytes to sector LBA
- `sexfs_v0_read_data_sector` — read 512 bytes from sector LBA
- `sexfs_v0_fnv1a` — FNV-1a 64-bit hash of byte slice
- `sexfs_v0_validate_extent_bounds` — extent consistency validator (see §D)

**Payload**:
- `b"sexobject extent alloc proof: testtest"` = 38 bytes
  - bytes [0..34): main proof string
  - bytes [34..38): 4-byte "test" continuity tag
- Stored at offset 0 of 512-byte sector, rest zeroed
- `content_hash` = FNV-1a of first 38 bytes

**Object entry for proof** (slot 0):
| Field | Value |
|-------|-------|
| object_id | 1 |
| kind | 1 |
| flags | 0x0001 (IN_USE) |
| owner_pd | 11 |
| rights_generation | 1 |
| content_generation | 1 |
| metadata_generation | 2 |
| object_size_bytes | 38 |
| first_block | 128 (= alloc_lba) |
| extent_count | 1 |
| name_hash | 0x6E654C5F534F5853 |
| content_hash | FNV-1a(payload) |
| created_at_gen | 1 |
| modified_at_gen | 2 |

**Required proof markers**:
```
[sexobject.extent_alloc.begin]
[sexobject.freemap.read.ok] lba=6
[sexobject.extent.alloc.ok] lba=128
[sexobject.freemap.persist.ok] lba=6
[sexobject.entry.extent.update.ok] slot=0 lba=128 extent_count=1
[sexobject.table.write.ok] lba_range=2..5
[sexobject.data.write.ok] lba=128 len=38
[sexobject.data.read.ok] lba=128 len=38
[sexobject.data.match] ok=1
[sexobject.remount.entry.match] ok=1
[sexobject.remount.freemap.used.ok] lba=128
[sexobject.neg.double_alloc.reject] ok=1
[sexobject.neg.bad_extent_lba.reject] ok=1
[sexobject.neg.zero_extent_nonzero_size.reject] ok=1
[sexobject.extent_alloc.done] ok=1
```

---

## D) Negative Tests

### 1. Double alloc reject
After allocating block 16 and persisting freemap, read back freemap from disk and attempt `sexfs_v0_freemap_alloc_specific(fm, 16)`. Block 16 bit is set → `ERR_OVERFLOW` → `ok=1`.

### 2. Bad extent LBA reject
Build entry with `first_block=2` (object table region, below LBA 128), `extent_count=1`, `size=10`, IN_USE. `sexfs_v0_validate_extent_bounds` checks: `extent_count > 0 && first_block < 128` → `ERR_OVERFLOW` → `ok=1`.

### 3. Zero extent nonzero size reject
Build entry with `extent_count=0`, `object_size_bytes=38`, IN_USE. `sexfs_v0_validate_extent_bounds` checks: `size > 0 && extent_count == 0` → `ERR_OVERFLOW` → `ok=1`.

---

## E) Non-Claims

- NOT implementing multi-extent objects
- NOT implementing object delete/rename/directory
- NOT claiming POSIX semantics
- NOT claiming power-loss durability or journaling
- NOT claiming concurrent multi-writer safety
- NOT implementing multiple allocated blocks (extent_count > 1)
- NOT changing sex-pdx ABI
- Proof LBAs 2022-2047 (fixed object bridge) preserved — not touched
- Existing `sexfs_v0_superblock_format_mount` gate not modified
- Existing `sexobject_table_persist` gate not modified
- `linen_sexfiles_100_current_tier_release` not touched

---

## F) Gate Result

| Gate | Status |
|------|--------|
| `sexobject_table_extent_alloc` | PASS |
| `sexfs_v0_superblock_format_mount` | PASS (pre-existing, unchanged) |
| `sexobject_table_persist` | PASS (pre-existing, unchanged) |
| `linen_sexfiles_100_current_tier_release` | SKIP (not triggered, unchanged) |

Gate activation: `SEXOBJECT_TABLE_EXTENT_ALLOC_PROOF=1` (default in run_daily_driver_proof.sh).

---

## G) Fault Scan

Expected: zero faults. All `fault_containment` or `faults=0` markers are proof markers, not kernel faults.

---

## H) Commit Hash

`dc58b0b0` — sexfs: prove SexObject extent allocation and data readback

---

## I) Next Phase Recommendation

`SEXOBJECT_MULTI_ENTRY_TABLE_V1` — Populate all 16 object table slots (each with a distinct `object_id`, deterministic `name_hash`, allocated extents) and verify all slots read back correctly after remount. Also candidate:

`SEXOBJECT_EXTENT_WRITE_FULL_BLOCK_V1` — Write a full 4KiB block (8 sectors × 512 bytes) and verify all 8 sectors. Proves full-block I/O path.

`SEXOBJECT_FREEMAP_MULTI_ALLOC_V1` — Allocate N blocks (N=4 or 8), verify all marked used, verify freemap consistency across each cycle.
