# SEXFILES_EXTENT_ALLOCATOR_V1 — Bounded Block-Level Allocator

**Date:** 2026-05-06
**Status:** Implemented, proof-gated
**Gate:** `SEXOS_SEXFILES_EXTENT_PROOF=1`

---

## Shape

A bounded first-fit contiguous block allocator built into the DiskFS in-memory scaffold.
Operates on a fixed-size block bitmap:

| Constant                   | Value | Meaning                          |
|----------------------------|-------|----------------------------------|
| DISKFS_EXTENT_BLOCK_COUNT  | 1024  | Total addressable blocks         |
| DISKFS_EXTENT_BITMAP_WORDS | 16    | u64 words in bitmap (1024/64)    |
| DISKFS_BLOCK_SIZE          | 4096  | Bytes per block                  |
| **Total addressable**      | 4 MiB | 1024 × 4096 bytes                |

Block 0 is reserved (superblock). Blocks 1..1023 are allocatable.

## Allocator Algorithm

- **First-fit**: scans linearly from block 1 upward, finds the first contiguous
  run of N free blocks.
- **No fragmentation solver**: fragmentation is only resolved by freeing used
  extents and reusing the holes naturally on the next first-fit scan.
- **Deterministic ERR_FULL**: when no span of the requested size exists, returns
  `ERR_FULL` deterministically (no retry, no compaction).

## Journal

A separate small extent journal (`DISKFS_EXTENT_JOURNAL_CAPACITY = 32` records)
mirrors the main object-table journal pattern. Every allocate and free operation
generates a journal record (kind: `ObjectMetadataUpdate` with `payload_len = 10`
as the extent discriminator). On free, the high bit of `object_id` is set to
distinguish free records from alloc records.

Journal replay of extent records is reserved for a future handoff; the current
implementation validates that records are written and their count increases with
operations.

## Files Changed

| File                                    | Changes                                    |
|-----------------------------------------|--------------------------------------------|
| `servers/sexfiles/src/backends/diskfs.rs` | +bitmap struct fields, +alloc/free/bounds/full/journaled methods |
| `servers/sexfiles/src/proof.rs`         | +run_sexfiles_extent_proofs, +6 proof fns  |
| `servers/sexfiles/src/trampoline.rs`    | +SEXOS_SEXFILES_EXTENT_PROOF gate          |

## Proof Markers

All markers emitted at startup when `SEXOS_SEXFILES_EXTENT_PROOF=1`:

| Marker                                | Meaning                              |
|---------------------------------------|--------------------------------------|
| `[sexfiles.extent.proof.start]`       | Proof gate active                    |
| `[sexfiles.extent.proof.alloc]`       | Basic contiguous allocation works    |
| `[sexfiles.extent.proof.free]`        | Free returns blocks to bitmap        |
| `[sexfiles.extent.proof.reuse]`       | Alloc→free→realloc reuses same hole  |
| `[sexfiles.extent.proof.full]`        | Deterministic out-of-space (ERR_FULL)|
| `[sexfiles.extent.proof.bounds]`      | OOB/zero alloc rejected              |
| `[sexfiles.extent.proof.journaled]`   | Alloc+free produce journal records   |
| `[sexfiles.extent.proof.done]`        | All extent checks passed             |

## Build/Runtime Result

```
$ cargo build -p sexfiles --target x86_64-unknown-none
   Compiling sexfiles v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] in 0.22s
```

Clean build, zero warnings.

At runtime with `SEXOS_SEXFILES_EXTENT_PROOF=1`:
- All 6 proof checks pass
- Allocator rejects zero-length, overflow, and full-buffer conditions
- First-fit reuse is verified at the bitmap level
- Journal produces records for both alloc and free operations

## Fragmentation / Scale Limits

| Limit                     | Value | Notes                                       |
|---------------------------|-------|---------------------------------------------|
| Max single allocation     | 1023 blocks (blocks 1..1023)                |
| Contiguous guarantee      | First-fit only; no compaction               |
| Fragmentation worst case  | Alternating 1-block alloc/free may prevent  |
|                           | large contiguous allocations even with      |
|                           | ample total free space                       |
| Fragmentation mitigation  | None beyond free→reuse natural recycling    |
| Journal capacity          | 32 extent records before journal full       |
| Scale ceiling             | 4 MiB total; increase DISKFS_EXTENT_BLOCK_COUNT for more |

## STOP Conditions (none triggered)

- No broad DiskFS rewrite: bitmap added as additional fields, existing paths unchanged
- No kernel edit required
- No sex-pdx ABI edit required
- No unbounded allocation: bitmap is fixed-size, all loops are bounded
- No POSIX path/file model introduced
- No shared-memory/backing-buffer redesign

## Future Work (not in scope)

1. Extent journal replay on mount (recover bitmap state from journal after crash)
2. Best-fit or buddy allocator for reduced fragmentation
3. Growing the bitmap size (requires on-disk format change)
4. Real block I/O path to persist bitmap to actual media
