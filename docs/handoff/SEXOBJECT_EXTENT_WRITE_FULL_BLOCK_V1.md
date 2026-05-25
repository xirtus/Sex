# SEXOBJECT_EXTENT_WRITE_FULL_BLOCK_V1 Handoff

## Status: PASS

Gate `sexobject_extent_write_full_block` — PASS.
All required markers confirmed present. Pre-existing `linen_diskfs_direct` FAIL is unrelated and unchanged.

---

## What Was Proved

One SexObject can write and read back a full 4KiB object extent through SexFS v0 on real NVMe-backed DiskFS.

- Allocated one data block (LBA 128) from freemap (object data region LBAs 128–2019)
- Wrote 4096 bytes across 8 contiguous 512-byte sectors via DiskFS bridge
- Read back and verified byte-for-byte match
- Persisted object table entry: slot 0, size=4096, extent_count=1, content_hash=FNV-1a(4KiB payload)
- Remount: re-read table, freemap, and content hash — all match
- Negative: corrupt sector 3 → hash mismatch correctly detected
- Negative: extent_count=1 but size>4096 → `ERR_OVERFLOW` correctly rejected

---

## Markers Confirmed

```
[sexobject.full_block.gate] begin
[sexobject.full_block.begin]
[sexobject.full_block.payload.ready] len=4096 has_test=1
[sexobject.full_block.write.ok] lba=128 sectors=8 len=4096
[sexobject.full_block.read.ok] lba=128 sectors=8 len=4096
[sexobject.full_block.match] ok=1
[sexobject.full_block.entry.persist.ok] slot=0 size=4096 extent_count=1
[sexobject.full_block.remount.entry.match] ok=1
[sexobject.full_block.remount.freemap.used.ok] lba=128
[sexobject.full_block.remount.content.match] ok=1
[sexobject.full_block.neg.hash_mismatch.reject] ok=1
[sexobject.full_block.neg.oversize_single_extent.reject] ok=1
[sexobject.full_block.done] ok=1
[sexobject.full_block.gate] ok=1
[sexobject.full_block.gate] done
```

---

## Files Changed

- `servers/sexfiles/src/backends/diskfs.rs` — payload helpers, `proof_sexobject_extent_write_full_block()`
- `servers/sexfiles/src/proof.rs` — `run_sexobject_extent_write_full_block_proofs()`
- `servers/sexfiles/src/trampoline.rs` — dispatch for `SEXOBJECT_EXTENT_WRITE_FULL_BLOCK_PROOF`
- `servers/sexfiles/build.rs` — `rerun-if-env-changed=SEXOBJECT_EXTENT_WRITE_FULL_BLOCK_PROOF`
- `scripts/run_daily_driver_proof.sh` — export env var, bump `PROBE_SECONDS` to 120 for full-block profile
- `scripts/daily_driver_master_gate.sh` — gate init, check block, ALL_GATES entry

---

## Preserved Gates

- `sexfs_v0_superblock_format_mount` — PASS (unchanged)
- `sexobject_table_persist` — PASS (unchanged)
- `sexobject_table_extent_alloc` — PASS (unchanged)
- `linen_sexfiles_100_current_tier_release` — PASS (unchanged)

---

## Pre-existing FAIL

- `linen_diskfs_direct` — FAIL, pre-existing, unrelated to this work, unchanged since before Mission 1.

---

## Invariants Preserved

- No kernel edits
- No sex-pdx ABI edits
- Fixed-object proof region LBAs 2022–2047 untouched
- No POSIX semantics, no directories, no rename/delete
- No shared-memory/backing-buffer redesign
- No Linen direct SexDrive access
- Payload computed on-the-fly (`sexfs_v0_full_block_payload_byte(i)`) — no 4KiB stack array
