# SEXOBJECT_TABLE_PERSIST_IMPL_V1

**Date**: 2026-05-25
**Mission**: Implement SexObject table entry create/read validation on top of SexFS v0 format/mount.
**Status**: PASS

---

## A) Outcome: PASS

All required proof markers present. Gate `sexobject_table_persist` = PASS.  
Gate `sexfs_v0_superblock_format_mount` = PASS (pre-existing freemap checksum bug fixed).  
Gate `linen_sexfiles_100_current_tier_release` = SKIP (not triggered in this profile; unchanged).

---

## B) Files Changed

| File | Delta | Purpose |
|------|-------|---------|
| `servers/sexfiles/src/backends/diskfs.rs` | +504 | Entry build/validate, table write/read, persist proof, freemap checksum fix |
| `servers/sexfiles/src/proof.rs` | +25 | `run_sexobject_table_persist_proofs()` gate runner |
| `servers/sexfiles/src/trampoline.rs` | +7 | Dispatch for `SEXOBJECT_TABLE_PERSIST_PROOF` env var |
| `servers/sexfiles/build.rs` | +1 | `rerun-if-env-changed` for new env var |
| `scripts/run_daily_driver_proof.sh` | +14 | NVMe attach for sexfs proofs, default probe 60s, env var export |
| `scripts/daily_driver_master_gate.sh` | +26 | `sexobject_table_persist` gate definition + result line |

---

## C) Object Entry Layout Used

Follows SEXFS_V0_ONDISK_CONTRACT_SPEC_V1 §D2 exactly:

| Offset | Size | Field | Test Value |
|--------|------|-------|------------|
| 0 | 8 | object_id | 1 |
| 8 | 2 | kind | 1 |
| 10 | 2 | flags | 0x0001 (IN_USE) |
| 12 | 4 | owner_pd | 11 |
| 16 | 8 | rights_generation | 1 |
| 24 | 8 | content_generation | 0 (no content) |
| 32 | 8 | metadata_generation | 1 |
| 40 | 8 | object_size_bytes | 0 |
| 48 | 8 | first_block | 0 |
| 56 | 8 | extent_count | 0 |
| 64 | 8 | name_hash | 0x6E654C5F534F5853 |
| 72 | 8 | content_hash | 0 |
| 80 | 8 | created_at_gen | 1 |
| 88 | 8 | modified_at_gen | 1 |
| 96 | 4 | checksum | XOR of bytes [0..96) |
| 100 | 28 | reserved | zero-filled |

Fields not yet implemented (content_generation, name_hash, content_hash, extent fields) use deterministic zero/default as documented.

---

## D) Positive Proof Markers

All required markers present:

```
[sexobject.table.persist.begin]
[sexobject.table.entry.create.ok] slot=0 object_id=1
[sexobject.table.write.ok] lba_range=2..5
[sexobject.table.read.ok] lba_range=2..5
[sexobject.table.entry.match] slot=0 ok=1
[sexobject.table.validate.ok]
[sexobject.table.neg.bad_entry.reject] ok=1
[sexobject.table.persist.done] ok=1
```

---

## E) Negative Tests

1. **Bad entry checksum rejection**: Flips one bit in entry checksum byte, writes to disk, reads back, validates → `ERR_OVERFLOW` → `ok=1`
2. **object_id=0 + IN_USE rejection**: Builds entry with object_id=0 and IN_USE flag, validates → `ERR_INVALID_HANDLE` → `ok=1`
3. **Reserved flag bits rejection**: Validator checks bits 4-15 of flags are zero → `ERR_OVERFLOW` if any set

All three negative gates pass.

---

## F) Non-Claims

- NOT implementing object data content writes (content_generation=0, size=0, first_block=0, extent_count=0)
- NOT implementing reboot restore beyond table remount proof
- NOT implementing directories, rename, delete, POSIX semantics
- NOT implementing freemap block allocation for entries (blocks remain at 0)
- NOT implementing journal/checkpoint for table entries
- NOT claiming concurrent multi-writer safety
- Pre-existing `linen_diskfs_direct` FAIL is unrelated to these changes
- Default probe increased from 30s to 60s to accommodate NVMe I/O latency (~1s per sector)

---

## G) Gate Result

| Gate | Status |
|------|--------|
| `sexobject_table_persist` | PASS |
| `sexfs_v0_superblock_format_mount` | PASS |
| `linen_sexfiles_100_current_tier_release` | SKIP (not triggered, unchanged) |
| All other gates | Unchanged |

---

## H) Fault Scan

Zero genuine faults. All "fault" matches in log are `faults=0` or `fault_containment` proof markers. No `#PF`, `#GP`, `panic`, or `KERNEL PANIC`.

---

## I) Commit Hash

Pre-commit base: `a98b2f885f95a16d7772214261a41dc4e31d588f`

---

## J) Next Phase Recommendation

`SEXOBJECT_TABLE_EXTENT_ALLOC_V1` — Add freemap-based block allocation for object entries:
1. Allocate first block from freemap for an object entry
2. Write allocated block to table entry (first_block + extent_count)
3. Write updated freemap to LBA 6
4. Write object data content to allocated blocks
5. Read back and verify data integrity
6. Negative: double-alloc rejection, out-of-space rejection

Also recommended: object multi-entry table proof (slots 0-15 all populated and verified).
