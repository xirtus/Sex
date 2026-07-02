# SEXFILES_DISK_OBJECT_ALLOCATOR_PLAN_V1

**Date:** 2026-05-07 (revised 2026-05-07)
**Status:** PLAN — **BLOCKED on V2 fixed-slot proofs**
**Gate:** `SEXOS_SEXFILES_DISK_OBJECT_ALLOCATOR_PROOF=1` (deferred to V3)

---

## 1. Shape

Plan for the smallest safe fixed-region extent allocator for DiskFS objects,
wiring the existing first-fit block bitmap into object creation. This plan is
**docs-only** — no implementation until V2 fixed-slot proofs complete.

The allocator already exists in `diskfs.rs` (first-fit bitmap, 1024 blocks ×
4096 bytes = 4 MiB = 8192 LBAs). This plan covers wiring it into object lifecycle
as a **V3 manifest extension**, not a competing metadata format.

## 2. Current Storage Layer Audit

### 2.1 V1 → V2 Evolution

| Version | What                                          | Status          |
|---------|-----------------------------------------------|-----------------|
| V1      | Single-entry manifest, 1 fixed object         | PROVEN          |
| V2      | Multi-entry manifest, 3 fixed object slots    | PLANNED (docs/handoff/SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1.md) |
| V3      | Dynamic allocator extending V2 manifest        | THIS PLAN       |

**V2 is the current stable product layer.** V3 must extend V2, not replace it.

### 2.2 V2 Fixed Object Slots (Authoritative)

| Slot | path_id | Object              | LBA Range    | Sectors | Size  | Flags |
|------|---------|---------------------|--------------|---------|-------|-------|
| 0    | 0       | SexFiles proof      | 2038–2045    | 8       | 4096  | 0x3   |
| 1    | 1       | Linen object        | 2030–2037    | 8       | 4096  | 0x3   |
| 2    | 2       | Quil object         | 2022–2029    | 8       | 4096  | 0x3   |

### 2.3 Reserved High LBAs (512-byte sectors)

```
LBA    Purpose
────── ──────────────────────────────────────────────
2047   WRITE_PROOF_LBA — never in manifest
2046   MANIFEST_LBA — V2 manifest sector
2045   ┐
  ⋮    │ SexFiles proof object (path_id=0)
2038   ┘
2037   ┐
  ⋮    │ Linen object (path_id=1)
2030   ┘
2029   ┐
  ⋮    │ Quil object (path_id=2)
2022   ┘
```

**Total reserved high span**: LBAs 2022–2047 = 26 sectors.

### 2.4 Reserved Low LBAs

```
LBA    Purpose
────── ──────────────────────────────────────────────
0–7    Superblock (block 0, 4096 bytes)
8–15   Object table (block 1, 4096 bytes)
```

### 2.5 Journal / Checkpoint / Bitmap — In-Memory Only

The append-only journal (`DISKFS_JOURNAL_CAPACITY = 64`), checkpoint array
(`DISKFS_MAX_CHECKPOINTS = 4`), and extent bitmap (`DISKFS_EXTENT_BITMAP_WORDS = 16`)
are **in-memory scaffold structures**. They are not mapped to fixed on-disk LBAs.
In a future real-block backend, they would occupy blocks 2..N, but that allocation
is part of the real-block-wiring handoff, not this allocator plan.

## 3. Block-to-LBA Mapping (Critical Correction)

### 3.1 Definition

```
Block size  = DISKFS_BLOCK_SIZE   = 4096 bytes
Sector size = BLOCK_SECTOR_SIZE   = 512 bytes
1 block     = 8 sectors (LBAs)

Block N covers LBAs: [N × 8, N × 8 + 7]
```

### 3.2 Reserved Block Map (Recalculated)

| Block | LBAs         | Reserved By                                    |
|-------|-------------|-------------------------------------------------|
| 0     | 0–7         | Superblock                                      |
| 1     | 8–15        | Object table (16 entries × ~128 bytes)          |
| 252   | 2016–2023   | Quil object (LBAs 2022–2023 at end of block)    |
| 253   | 2024–2031   | Quil (2024–2029) + Linen (2030–2031)            |
| 254   | 2032–2039   | Linen (2032–2037) + SexFiles proof (2038–2039)  |
| 255   | 2040–2047   | SexFiles proof (2040–2045) + manifest (2046) + write proof (2047) |

**Blocks 252–255 are ALL reserved** — each contains at least one reserved sector.
With 4096-byte block granularity, partial sector usage makes the entire block unavailable for allocation.

### 3.3 Allocatable Region (Corrected)

```
Blocks 2..251 = 250 blocks × 4096 bytes = 1,024,000 bytes (~1000 KiB)
```

**Verification chain:**

- Allocatable start: block 2, LBA 16 (immediately after object table)
- Allocatable end: block 251, LBA 251×8+7 = 2015
- First reserved high LBA: 2022 (Quil object start)
- Last allocatable LBA (2015) < first reserved high LBA (2022) ✓
- **No collision.** The 6-LBA gap (2016–2021) is unused headroom; these are
  the first 6 sectors of block 252 which is reserved wholesale.

### 3.4 What the Original Plan Got Wrong

| Claim in original plan          | Reality                                        |
|---------------------------------|-------------------------------------------------|
| "Allocatable blocks 2..253"     | Block 252 collides with Quil LBA 2022–2023      |
| Block 253 in allocatable region | Block 253 contains Quil + Linen reserved sectors |
| 252 allocatable blocks          | Actual safe count is 250 blocks                  |
| "SAFE TO PROCEED"               | Must wait for V2 fixed-slot proofs to land first |

### 3.5 Why Not Use Blocks 256+?

Blocks 256..1023 (768 blocks, 3 MiB) are above the proof region. They are
freely allocatable in principle. However:

- V2 fixed slots define a tight reserved zone at LBAs 2022–2047. Placing
  dynamic objects at LBA 2048+ (block 256+) would create a fragmented layout:
  reserved zone sandwiched between two allocatable regions.
- A split-region allocator requires zone-aware first-fit scan logic, which
  increases complexity.
- V1 (this plan) keeps it simple: single contiguous region 2..251.
- V3+ can add a second zone (256..1023) or relocate the reserved zone to the
  very end of the address space, simplifying to a single contiguous region.
  Out of scope for V3.

## 4. V3 Allocator Design (Extends V2 Manifest)

### 4.1 Metadata Path — One Canonical Format

The allocator does **NOT** introduce a second object table or competing metadata
struct. It extends the V2 manifest:

- **Manifest sector (LBA 2046)**: Contains `DiskManifestEntryV1` entries.
  Fixed V2 entries (path_id 0,1,2) remain unchanged with their hardcoded LBA
  ranges. Dynamic V3 entries get `start_lba` and `len_bytes` assigned by the
  allocator.
- **`SexfilesObjectEntry.first_block`**: Used as an in-memory cache of the
  allocated block number, derived from the manifest entry's `start_lba`.
  Rebuilt on mount by reading the manifest.
- **`SexfilesObjectEntry.object_size_bytes`**: Mirrors the manifest entry's
  `len_bytes`.

The manifest is the **single source of truth** for LBA assignments. The object
table's `first_block` is a derived cache, rebuilt on mount.

### 4.2 Entry Lifecycle

```
V2 (fixed, path_id 0/1/2):    start_lba = hardcoded constant, never changes
V3 (dynamic, path_id 3..14): start_lba = allocated by extent allocator
                               len_bytes = object_size_bytes
                               flags    = READ|WRITE
                               name_hash = FNV-1a of object path
```

Existing V2 entries (path_id 0/1/2) are **never modified** by the allocator.
The allocator only operates on path_id ≥ 3 (up to `DISKFS_MANIFEST_ENTRY_MAX - 1`).

### 4.3 Allocation Workflow

```
create_object_entry(kind, owner_pd, path_id)  [path_id >= 3]
  → Object created with first_block=0, object_size_bytes=0
  → No manifest entry yet (deferred until first write)
  → Journaled (existing path, unchanged)

allocate_for_object(object_id, size_bytes)
  → find_contiguous_free_blocks(ceil(size_bytes / 4096))
    in region [2, 251]
  → check_overlap() against all manifest entries
  → check_region_bounds()
  → Mark blocks in extent_bitmap
  → Set first_block, object_size_bytes in object table entry
  → Write manifest entry (name_hash, start_lba, len_bytes, flags)
    into next free manifest entry slot
  → Journal metadata update

mount() — bitmap rebuild from manifest:
  → Clear extent_bitmap
  → Mark blocks 0, 1, 252, 253, 254, 255 as reserved
  → For each manifest entry (V2 fixed + V3 dynamic):
      → Convert LBA → block: block = start_lba / 8
      → blocks = ceil(len_bytes / 4096)
      → Mark blocks [block .. block+blocks] as allocated
```

### 4.4 Append-Only Guarantee

| Property              | V3 Behavior                                         |
|-----------------------|-----------------------------------------------------|
| Allocation mode       | Append-only (no free, no reuse)                     |
| Free/reuse            | Deferred — `free_blocks()` exists but NOT called    |
| Max dynamic objects   | `DISKFS_MANIFEST_ENTRY_MAX - 3 = 12`                |
| Max total objects     | `DISKFS_MAX_OBJECTS = 16` (object table capacity)   |
| Allocation unit       | Block (4096 bytes). Objects span 1..N blocks.       |
| Fragmentation         | Append-only → zero fragmentation by construction    |
| Crash safety          | Bitmap rebuilt from manifest entries on mount       |
| Contiguous guarantee  | Always contiguous within 2..251 (append-only)       |

**Why append-only is safe without journaling the allocator:**
- Blocks are marked in bitmap, manifest entry is written. If crash occurs:
  - After bitmap mark, before manifest write: blocks are leaked but harmless.
    On next mount, bitmap is rebuilt from manifest — leaked blocks are freed.
  - After manifest write, before bitmap mark: bitmap is wrong. On next mount,
    bitmap is rebuilt from manifest — blocks correctly marked.
- The manifest is the authoritative source. Bitmap is a derived performance
  cache, never trusted across mounts.

## 5. Corruption / Collision Checks

### 5.1 Overlapping Extent Rejection

Scan all manifest entries and reject if new extent `[start_lba, start_lba+len_sectors)`
intersects any existing entry:

```rust
fn check_overlap_manifest(
    manifest: &[DiskManifestEntryV1; DISKFS_MANIFEST_ENTRY_MAX as usize],
    entry_count: u16,
    new_start_lba: u64,
    new_len_bytes: u32,
) -> Result<(), i64> {
    let new_sectors = (new_len_bytes as u64 + 511) / 512;
    let new_end_lba = new_start_lba + new_sectors;
    for i in 0..entry_count as usize {
        let e = manifest[i];
        let e_sectors = (e.len_bytes as u64 + 511) / 512;
        let e_end = e.start_lba + e_sectors;
        if new_start_lba < e_end && new_end_lba > e.start_lba {
            return Err(messages::ERR_OVERFLOW);
        }
    }
    Ok(())
}
```

### 5.2 Out-of-Region Rejection

```rust
fn check_region_bounds(start_block: u64, num_blocks: u64) -> Result<(), i64> {
    if start_block < 2 {
        return Err(messages::ERR_OVERFLOW); // reserved low (blocks 0-1)
    }
    if start_block + num_blocks > 252 {
        // Block 252 = first reserved high block
        return Err(messages::ERR_OVERFLOW);
    }
    Ok(())
}
```

Region bounds: `2 ≤ start_block < start_block + num_blocks ≤ 252` (exclusive end).

Note: ceiling is 252 (exclusive), meaning max `start_block + num_blocks == 252`,
i.e., blocks 2..251 inclusive. Block 252 is the first reserved block.

### 5.3 Zero-Length Rejection

```rust
if len_bytes == 0 {
    return Err(messages::ERR_BAD_LEN); // or ERR_INVALID_HANDLE — document exact choice
}
```

Map to existing error codes only:
- `BLOCK_ERR_BAD_LEN` (from sex-pdx) or `messages::ERR_INVALID_HANDLE`.
  Do NOT introduce new error codes.

### 5.4 Max Objects Check

```rust
if used_manifest_entries >= DISKFS_MANIFEST_ENTRY_MAX {
    return Err(messages::ERR_FULL);
}
// Also check object table capacity:
if in_use_object_count >= DISKFS_MAX_OBJECTS {
    return Err(messages::ERR_FULL);
}
```

### 5.5 Reserved Collision Rejection

Before any V3 allocation, verify the new extent does not collide with:
- LBA 2046 (manifest sector itself)
- LBA 2047 (write proof slot)
- Any V2 fixed-slot LBA range (2022–2045)

These are caught by both `check_overlap_manifest()` (since V2 entries are in the
manifest) and `check_region_bounds()` (since V2 slots are above block 251).

### 5.6 Error Code Mapping (Use Existing Only)

| Condition                  | Error Code                  | Existing? |
|----------------------------|-----------------------------|-----------|
| Overlapping extent         | `messages::ERR_OVERFLOW`    | Yes       |
| Out of region (low)        | `messages::ERR_OVERFLOW`    | Yes       |
| Out of region (high)       | `messages::ERR_OVERFLOW`    | Yes       |
| Zero length                | `BLOCK_ERR_BAD_LEN`         | Yes       |
| Manifest full              | `messages::ERR_FULL`        | Yes       |
| Object table full          | `messages::ERR_FULL`        | Yes       |
| Invalid path_id            | Existing not-found code     | Yes       |

**No new error codes.** All conditions map to existing `messages::ERR_*` or
`sex_pdx::BLOCK_ERR_*` constants.

## 6. Proof Plan (Deferred — Requires V2 Slots First)

### 6.1 Prerequisites Before Proofs Can Run

1. `LINEN_DISKFS_SLOT_OBJECT_PROOF_V1` passes for path_id=1
2. `QUIL_DISKFS_SLOT_OBJECT_PROOF_V1` passes or documents blocker
3. V2 multi-entry manifest is implemented and verified
4. Manifest constants (`DISKFS_OBJECT_SLOT_QUIL`, `DISKFS_OBJECT_SLOT_LINEN`) exist

### 6.2 Proof Gate

`SEXOS_SEXFILES_DISK_OBJECT_ALLOCATOR_PROOF=1`

### 6.3 Proof Scenarios (All In-Memory Scaffold, Bounded)

#### Proof A: Allocate Object A, Write, Read Back

1. `format_init_empty()` + `mount()` (bitmap rebuilt, V2 entries present)
2. `create_object_entry(kind=0xA0, owner_pd=1, path_id=3)` → `oid_a`
3. `allocate_for_object(oid_a, 4096)` → `first_block ∈ [2, 250]`
4. Verify `first_block != 0` and `first_block + 1 ≤ 252`
5. Verify V2 manifest entries (path_id 0,1,2) unchanged
6. Write 4096 bytes of known pattern to object at offset 0
7. Read back 4096 bytes → exact match
8. Verify superblock magic intact (no reserved-low corruption)

**Marker**: `[sexfiles.alloc.v3.proof.obj_a]`

#### Proof B: Allocate Object B, No Overlap

1. Continue from Proof A
2. `create_object_entry(kind=0xB0, owner_pd=1, path_id=4)` → `oid_b`
3. `allocate_for_object(oid_b, 4096)` → `first_block_b != first_block_a`
4. Verify `first_block_b >= first_block_a + 1` (append-only, contiguous)
5. Write pattern B, read both A and B → both intact, no cross-corruption

**Marker**: `[sexfiles.alloc.v3.proof.obj_b]`

#### Proof C: Simulated Reboot

1. Export manifest entries + object table
2. Re-format + re-mount
3. Rebuild bitmap from manifest (mount-time)
4. Verify V2 entries (path_id 0,1,2) restored correctly
5. Verify V3 entries (path_id 3,4) restored with correct start_lba/len
6. Read both objects → patterns match

**Marker**: `[sexfiles.alloc.v3.proof.reboot]`

#### Proof D: Overlap Rejection

1. Attempt `allocate_for_object()` with start_block that overlaps existing V2
   or V3 entry → `check_overlap_manifest()` returns `ERR_OVERFLOW`
2. Verify allocation rejected, existing objects unchanged

**Marker**: `[sexfiles.alloc.v3.proof.overlap_reject]`

#### Proof E: Bounds Rejection

1. Attempt allocation at block 0 → ERR_OVERFLOW (superblock)
2. Attempt allocation at block 1 → ERR_OVERFLOW (object table)
3. Attempt allocation that crosses block 252 → ERR_OVERFLOW (reserved high)
4. Attempt zero-length → `ERR_BAD_LEN`

**Marker**: `[sexfiles.alloc.v3.proof.bounds_reject]`

#### Proof F: Manifest Full (Metadata-Only)

1. Fill manifest to `DISKFS_MANIFEST_ENTRY_MAX` (15 entries):
   V2 fixed 3 + 12 V3 dynamic entries **by metadata insertion only** —
   do NOT write 4096 bytes to each. Insert entries with `len_bytes = 0`
   or minimal size to verify entry-count rejection.
2. Verify 16th manifest entry attempt returns `ERR_FULL`
3. Verify no actual block I/O beyond what's needed for metadata writes

**Marker**: `[sexfiles.alloc.v3.proof.manifest_full]`

**Note**: Original Proof F "fill all 252 blocks with real writes" is removed
as too large/noisy for startup gate. Manifest entry exhaustion is sufficient
to prove the bounded-max-objects contract.

### 6.4 Proof Markers Summary

| Marker                                 | Meaning                                    |
|----------------------------------------|--------------------------------------------|
| `[sexfiles.alloc.v3.proof.start]`      | Gate active                                |
| `[sexfiles.alloc.v3.proof.obj_a]`      | Object A allocated, written, read-back ok  |
| `[sexfiles.alloc.v3.proof.obj_b]`      | Object B allocated, no overlap              |
| `[sexfiles.alloc.v3.proof.reboot]`     | Reboot rebuild, V2+V3 entries intact       |
| `[sexfiles.alloc.v3.proof.overlap_reject]` | Overlap rejected                       |
| `[sexfiles.alloc.v3.proof.bounds_reject]`  | OOB + zero-length rejected             |
| `[sexfiles.alloc.v3.proof.manifest_full]`  | Manifest entry exhaustion → ERR_FULL    |
| `[sexfiles.alloc.v3.proof.done]`       | All checks passed                          |

## 7. STOP FIRST Conditions

| # | Condition                                                    | Status     |
|---|--------------------------------------------------------------|------------|
| 1 | **V2 fixed-slot proofs incomplete**                          | **BLOCKER** |
| 2 | `LINEN_DISKFS_SLOT_OBJECT_PROOF_V1` not yet passing          | **BLOCKER** |
| 3 | `QUIL_DISKFS_SLOT_OBJECT_PROOF_V1` not yet passing (or blocker documented) | **BLOCKER** |
| 4 | Allocator would introduce competing metadata format          | AVOIDED — extends V2 manifest |
| 5 | Allocator requires new on-disk structures                    | AVOIDED — uses existing manifest entry format |
| 6 | Allocator requires ABI/kernel changes                        | No         |
| 7 | Allocator requires block device persistence                  | No (scaffold only) |
| 8 | Allocator requires delete/reuse semantics                    | Deferred   |
| 9 | Block/LBA math not verified against V2 layout                | **FIXED in this revision** |

**Verdict: BLOCKED — do not implement until V2 fixed-slot proofs complete.**

## 8. Implementation Sequence (When Unblocked)

### Phase A: Complete V2 Fixed Slots (Now)

1. `SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1` — multi-entry manifest
2. `LINEN_DISKFS_SLOT_OBJECT_PROOF_V1` — Linen uses path_id=1
3. `QUIL_DISKFS_SLOT_OBJECT_PROOF_V1` — Quil uses path_id=2 (or blocker documented)
4. `FINAL_V2_FIXED_OBJECT_STORAGE_AUDIT_V1` — full V2 audit

### Phase B: Allocator V3 (After V2 Proven)

5. `SEXFILES_DISK_OBJECT_ALLOCATOR_IMPL_V1` — this plan's implementation
6. `LINEN_DISKFS_ALLOCATOR_BRIDGE_V1` — Linen uses dynamic path_id ≥ 3

## 9. Files to Change (Phase B, Deferred)

| File                                      | Changes                                               |
|-------------------------------------------|-------------------------------------------------------|
| `servers/sexfiles/src/backends/diskfs.rs` | +`reserve_fixed_blocks()` for blocks 1, 252–255        |
|                                           | +`rebuild_bitmap_from_manifest()` in mount()            |
|                                           | +`allocate_for_object()` using manifest as authority    |
|                                           | +`check_overlap_manifest()` + `check_region_bounds()`   |
|                                           | +Proof methods A–F                                      |
| `servers/sexfiles/src/proof.rs`           | +`run_disk_object_allocator_proofs()`                   |
| `servers/sexfiles/src/trampoline.rs`      | +gate under SEXOS_SEXFILES_DISK_OBJECT_ALLOCATOR_PROOF  |
| **No changes to:**                        |                                                         |
| `crates/sex-pdx/`                         | No ABI changes                                          |
| `kernel/`                                 | No kernel changes                                       |
| `servers/linen/`                          | Linen bridge unchanged until Phase B step 6             |
| `apps/sexdrive/`                          | Write guard already extended for V2 (LBAs 2022-2037)    |
|                                           | No allocator-specific sexdrive changes needed           |

## 10. Non-Goals (Explicitly Out of Scope)

- **No competing metadata format**: Allocator entries go into the V2 manifest,
  not a separate table.
- **No V2 entry modification**: path_id 0/1/2 are immutable fixed slots.
- **No block-device persistence**: In-memory scaffold only. Real NVMe persistence
  requires `SEXFILES_REAL_BLOCK_BACKEND_V1`.
- **No delete/reuse/rename**: Append-only.
- **No dynamic resizing**: Fixed constants (1024 blocks, 250 allocatable).
- **No sub-block allocation**: Block granularity only.
- **No directory tree / POSIX paths**: Flat object namespace, path_id addressing.
- **No free_blocks() wiring**: Function exists but is never called in V3.

## 11. Risks and Mitigations

| Risk                                          | Impact | Mitigation                                     |
|-----------------------------------------------|--------|------------------------------------------------|
| V2 slot layout changes before V3 implemented   | Medium | Wait for V2 audit. Re-verify block math.        |
| Block 252 overlap (Quil LBA 2022–2023)         | HIGH   | **Fixed**: ceiling lowered to 251               |
| Manifest and object table desync after crash   | Low    | Manifest is authority; bitmap rebuilt on mount   |
| 250 allocatable blocks insufficient for 12     | Low    | 250/12 ≈ 20.8 blocks/obj ≈ 83 KiB avg.          |
| dynamic objects (DISKFS_MANIFEST_ENTRY_MAX-3)  |        | Sufficient for V3.                               |
| Proof F too slow if doing real block writes    | Fixed  | Metadata-only exhaustion test.                   |
| V3 implementation breaks V2 fixed-slot proofs  | Medium | V2 entries immutable; overlap check prevents     |
|                                                |        | collision with V2 slots.                          |

## 12. Allocation Region Diagram (Corrected)

```
Block   LBAs          Purpose
──────  ────────────  ──────────────────────────────────────────
0       0–7           ████████ Superblock (reserved)
1       8–15          ████████ Object Table (reserved)
2       16–23         ........ Allocatable Region
  ⋮        ⋮            ........ (250 blocks, blocks 2..251)
251     2008–2015     ........ Last allocatable block
252     2016–2023     ████████ Quil LBA 2022–2023 (reserved)
253     2024–2031     ████████ Quil + Linen (reserved)
254     2032–2039     ████████ Linen + SexFiles proof (reserved)
255     2040–2047     ████████ SexFiles proof + manifest + write proof (reserved)
256..1023 2048–8191   ──────── Unused in V3 (3 MiB, deferred to V4)
```

**Verification**: Last allocatable LBA = 251×8+7 = 2015.
First reserved high LBA = 2022 (Quil slot).
2015 < 2022 ✓ — no collision.

## 13. Implementation Prompt (Deferred)

```
MISSION: SEXFILES_DISK_OBJECT_ALLOCATOR_IMPL_V1

PREREQUISITES (must be complete before starting):
- [ ] SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1 (V2 multi-entry manifest)
- [ ] LINEN_DISKFS_SLOT_OBJECT_PROOF_V1 passing (path_id=1)
- [ ] QUIL_DISKFS_SLOT_OBJECT_PROOF_V1 passing or blocker documented (path_id=2)
- [ ] FINAL_V2_FIXED_OBJECT_STORAGE_AUDIT_V1 complete

Goal:
Wire the existing first-fit extent bitmap allocator into the DiskFS object
creation path as a V3 manifest extension. Dynamic objects (path_id ≥ 3)
get allocated block ranges, written into the manifest as new entries alongside
the three V2 fixed-slot entries. Append-only, bitmap rebuilt from manifest on mount.

BACKUP BEFORE CHANGES.
READ docs/handoff/SEXFILES_DISK_OBJECT_ALLOCATOR_PLAN_V1.md first.
rg first, small snippets only.
Save recurring fixes/issues in docs/handoff.

NO Linux assumptions. NO POSIX.
Strict no_std Rust.
No kernel/ABI edits.
No new error codes.
No new metadata tables — extend V2 manifest only.

SCOPE:
DiskFS object allocator implementation in in-memory scaffold.
Extends V2 manifest; does not replace it.

TASK:
1. In format_init_empty(), set extent_bitmap bits for blocks 1, 252, 253, 254, 255.
   (Block 0 is already reserved.)

2. Add rebuild_bitmap_from_manifest() — on mount:
   a. Clear extent_bitmap.
   b. Mark blocks 0, 1, 252, 253, 254, 255 as reserved.
   c. For each manifest entry (V2 fixed + V3 dynamic):
      - block = entry.start_lba / 8
      - blocks = ceil(entry.len_bytes / 4096)
      - Mark blocks [block .. block+blocks] in bitmap.
   d. Call this in mount() instead of trusting stale bitmap.

3. Add check_overlap_manifest(manifest, entry_count, new_start_lba, new_len_bytes):
   - Scan all existing manifest entries, reject interval intersection.
   - Returns Ok(()) or messages::ERR_OVERFLOW.

4. Add check_region_bounds(start_block, num_blocks):
   - Reject start_block < 2 → ERR_OVERFLOW.
   - Reject start_block + num_blocks > 252 → ERR_OVERFLOW.
   - Reject num_blocks == 0 → BLOCK_ERR_BAD_LEN.

5. Add allocate_for_object(object_id, size_bytes):
   a. blocks_needed = ceil(size_bytes / 4096).
   b. find_contiguous_free(blocks_needed) in bitmap (existing first-fit scan).
   c. Convert block → LBA: lba = block * 8.
   d. check_overlap_manifest() against all manifest entries.
   e. check_region_bounds().
   f. Mark blocks in extent_bitmap.
   g. Update object table entry: first_block = block, object_size_bytes = size_bytes.
   h. Write new DiskManifestEntryV1 into next free manifest slot.
   i. Journal metadata update.
   j. Return first_block.

6. Proof methods (wired under SEXOS_SEXFILES_DISK_OBJECT_ALLOCATOR_PROOF=1):
   a. proof_alloc_obj_a_b — two objects, write/read, verify no overlap.
   b. proof_alloc_reboot — export manifest+table, re-mount, rebuild bitmap, verify.
   c. proof_alloc_overlap_reject — overlap attempt rejected.
   d. proof_alloc_bounds_reject — block 0, 1, 252+, zero-length all rejected.
   e. proof_alloc_manifest_full — fill 15 manifest entries (metadata-only),
      verify 16th returns ERR_FULL.

7. Do NOT:
   - Modify V2 fixed-slot entries (path_id 0/1/2).
   - Wire into Linen bridge.
   - Call free_blocks().
   - Add new error codes.
   - Introduce a second object table or competing metadata format.
   - Use block granularity different from 4096 bytes.

8. Verify:
   cargo check -p sexfiles --target x86_64-unknown-none → clean, zero warnings
   SEXOS_SEXFILES_DISK_OBJECT_ALLOCATOR_PROOF=1 ./scripts/master_runtime_gate.sh
   → all markers present, GREEN_MASTER, no #PF/#GP/panic.

STOP FIRST if:
- V2 fixed-slot proofs are not yet complete
- Implementation would modify immutable V2 manifest entries
- Implementation requires new PDX opcodes or kernel syscalls
- Implementation introduces new error codes not in existing conventions
- Block/LBA math does not match the corrected map in this plan
```
