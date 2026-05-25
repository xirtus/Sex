# SEXFS_V0_ONDISK_CONTRACT_SPEC_V1

**Date**: 2026-05-25
**Mission**: Freeze native SexObject/SexFS v0 on-disk contract before persistence implementation.
**Status**: SPEC ONLY — no implementation changes.

---

## A) Outcome: PASS

All struct layouts, LBA assignments, serialization rules, proof gates, and
STOP FIRST items are defined. Ready for Phase 1 implementation.

---

## B) Native Model

### B1. SexFS v0 Identity

SexFS v0 is a **native bounded object-store for SexObjects**. Not a POSIX filesystem.

| Concept | Authority | Lookup |
|---------|-----------|--------|
| **object_id** (u64) | Canonical identity | Primary key |
| **SexObjectRef** {id, gen} | Cross-PD capability | Collar gate |
| **kind** (u16) | SexObjectKind discriminant | Filter/classify |
| **name_hash** (u64) | Optional search key | Future name→id lookup |

### B2. Current-Tier Scope

**IN scope**:
- format (write superblock + empty table + empty freemap)
- mount (read superblock from LBA 0, validate magic/version/checksum)
- object table persist (16 entries at LBAs 2-5)
- freemap persist (bitmap at LBA 6)
- object data write/read via allocated blocks
- controlled reboot restore (format→create→reboot→mount→verify)
- bad magic/version/checksum rejection

**OUT of scope (honest non-claims)**:
- POSIX, directories, rename, delete, symlinks
- Concurrent multi-writer
- Powerloss durability
- Full journaling guarantees (journal scaffold may be used for reboot consistency only)
- Variable-length names on disk (name_hash only)
- Compression, encryption, dedup

---

## C) LBA Map

### C1. Baseline: Current LBA Usage

nvme.img = 2048 sectors × 512 bytes = 1 MB = 256 blocks × 4096 bytes

| LBAs | Current Use |
|------|-------------|
| 0-127 | **UNUSED** |
| 128-131 | AP4 multi-block proof (self-test) |
| 132-2021 | **UNUSED** |
| 2022-2029 | quil-object-v1 (proof) |
| 2030-2037 | linen-object-v1 (proof) |
| 2038-2045 | sexfiles-proof-v1 (proof) |
| 2046 | Manifest |
| 2047 | Write proof |

### C2. SexFS v0 Layout

```
SECTOR   LBA      SIZE        CONTENTS
------   ---      ----        --------
   0       0      1 sector    PRIMARY SUPERBLOCK
   1       1      1 sector    BACKUP SUPERBLOCK
 2-5     2-5      4 sectors   OBJECT TABLE (16 entries × 128 bytes)
   6       6      1 sector    FREEMAP (extent bitmap)
   7       7      1 sector    RESERVED
 8-15    8-15     8 sectors   JOURNAL (64 records × 64 bytes)
16-47   16-47    32 sectors   CHECKPOINTS (4 × 8 sectors = 4 × 4096 bytes)
48-127  48-127   80 sectors   RESERVED (metadata expansion)
128-2019         1892 sectors  OBJECT DATA REGION
128-131             4 sectors  |-- AP4 proof (reclaimable post-AP4)
132-255           124 sectors  |-- Object data (low)
256-259             4 sectors  |-- AP5A proof (reclaimable)
260-383           124 sectors  |-- Object data (mid)
384-385             2 sectors  |-- AP6 proof (reclaimable)
386-2019         1634 sectors  |-- Object data (main)
2020-2021          2 sectors   RESERVED
2022-2045  2022-2045  24 sectors  PROOF OBJECT REGION (unchanged)
2046       2046       1 sector   MANIFEST (unchanged)
2047       2047       1 sector   WRITE PROOF (unchanged)
```

### C3. Extent Allocator (Block-Level View)

1 block = 4096 bytes = 8 sectors. 2048 sectors = 256 blocks.

| Block | Sectors | Status |
|-------|---------|--------|
| 0 | 0-7 | Reserved (metadata: superblock, table, freemap, journal) |
| 1 | 8-15 | Reserved (journal tail) |
| 2-5 | 16-47 | Reserved (checkpoints) |
| 6-15 | 48-127 | Reserved (metadata expansion) |
| 16-252 | 128-2019 | **Allocatable object data** (237 blocks) |
| 253-255 | 2020-2047 | Reserved (proof objects + manifest + write proof) |

### C4. Write Guard Impact (STOP FIRST)

Current `write_guard_allows()` permits writes only to:
- LBA 2046 (manifest): `buf_cap == SLOT_BUF_LEND && size == 512 && offset == manifest_offset`
- LBAs 2022-2045 (objects): `proof_mode && aligned 512`
- LBA 2047 (proof): `proof_mode && offset == expected`

**SexFS v0 metadata writes (LBAs 0-23) and object data writes (LBAs 128-2019) will be REJECTED** by the current write guard. The guard MUST be extended before any SexFS v0 format/mount/object write can succeed.

---

## D) On-Disk Struct Layouts

All multi-byte fields: **little-endian**. All structs: **fixed-size, repr(C)-compatible**.
All padding: **zero-filled**.

### D1. SexfsSuperblockV0 — 512 bytes (1 sector)

```
Offset  Size  Field                   Contents
------  ----  -----                   --------
0       8     magic                   u64 LE: 0x307631_335346_5853
                                      ("SEXFSv01" → 53 45 58 46 53 30 76 31)
8       2     version_major           u16 LE: 1
10      2     version_minor           u16 LE: 0
12      4     block_size              u32 LE: 4096
16      8     fs_generation           u64 LE: monotonic counter (starts at 1)
24      8     object_table_sector     u64 LE: LBA of object table (= 2)
32      4     object_table_entries    u32 LE: max entries (= 16)
36      4     object_entry_bytes      u32 LE: bytes per entry (= 128)
40      8     freemap_sector          u64 LE: LBA of freemap (= 6)
48      8     freemap_blocks          u64 LE: total blocks tracked (= 1024)
56      8     journal_sector          u64 LE: LBA of journal start (= 8)
64      8     journal_records_max     u64 LE: max records (= 64)
72      8     checkpoint_sector       u64 LE: LBA of first checkpoint (= 16)
80      8     checkpoint_count        u64 LE: max checkpoints (= 4)
88      8     feature_flags           u64 LE: bit 0=journal_enabled, others=0
96      4     checksum                u32 LE: XOR of bytes [0..96)
100     412   reserved                zero-filled
```

Checksum rule: XOR all u32 words in bytes 0..96 (24 u32 values), EXCLUDING the checksum field itself. Verification re-computes and compares.

### D2. SexfsObjectEntryV0 — 128 bytes per record

```
Offset  Size  Field                   Contents
------  ----  -----                   --------
0       8     object_id               u64 LE: 0 = free slot, ≥1 = valid
8       2     kind                    u16 LE: SexObjectKind discriminant
10      2     flags                   u16 LE: bit0=IN_USE, bit1=SEALED, bit2=REDACTED, bit3=MIGRATING
12      4     owner_pd                u32 LE: owning PD
16      8     rights_generation       u64 LE: capability revocation epoch
24      8     content_generation      u64 LE: content write epoch (new real field for V0)
32      8     metadata_generation     u64 LE: metadata write epoch
40      8     object_size_bytes       u64 LE: logical payload size
48      8     first_block             u64 LE: first 4KB block number (0=none)
56      8     extent_count            u64 LE: contiguous blocks from first_block
64      8     name_hash               u64 LE: FNV-1a 64-bit of name label (0=none)
72      8     content_hash            u64 LE: FNV-1a 64-bit of content payload (0=not hashed)
80      8     created_at_gen          u64 LE: fs_generation at create time
88      8     modified_at_gen         u64 LE: fs_generation at last modification
96      4     checksum                u32 LE: XOR of bytes [0..96)
100     28    reserved                zero-filled
```

Key changes from scaffold SexfilesObjectEntry:
- `in_use: bool` → `flags: u16` (bit 0 = IN_USE, bit 1 = SEALED)
- Added: `content_generation`, `name_hash`, `content_hash`, `created_at_gen`, `modified_at_gen`
- `first_block` + `extent_count` replace old `first_block` alone (V0 supports extents)
- Record size fixed at 128 bytes (was ~51 bytes in scaffold)

### D3. FreemapBlockV0 — 512 bytes (1 sector)

```
Offset  Size  Field                   Contents
------  ----  -----                   --------
0       8     magic                   u64 LE: 0x30564D50_45455246 ("FREEMAPV0")
8       2     version                 u16 LE: 1
10      2     reserved0               u16 LE: 0
12      4     block_count             u32 LE: 1024
16      4     checksum                u32 LE: XOR of bytes [0..20) + bitmap bytes
20      12    reserved1               zero-filled
32      128   bitmap                  [u64; 16] LE: bit i = block i status (1=used, 0=free)
160     352   reserved2               zero-filled
```

Block 0 is always marked in-use (metadata region). Blocks 1-5 also marked in-use (journal + checkpoints). Blocks 6-15 marked in-use (reserved metadata). Blocks 16-252 are allocatable. Blocks 253-255 marked in-use (proof + manifest).

### D4. JournalRecordV0 — 64 bytes per record

```
Offset  Size  Field                   Contents
------  ----  -----                   --------
0       2     kind                    u16 LE: 1=TxBegin, 2=ObjectMetaUpdate, 3=TxCommit
2       2     reserved0               u16 LE: 0
4       4     payload_len             u32 LE: metadata payload bytes
8       8     tx_id                   u64 LE: transaction ID
16      8     generation              u64 LE: fs_generation at record write
24      8     object_id               u64 LE: affected object (0 for TxBegin/TxCommit)
32      8     metadata_generation     u64 LE: metadata epoch for this update
40      8     reserved1               u64 LE: 0
48      8     reserved2               u64 LE: 0
56      4     checksum                u32 LE: XOR of bytes [0..56)
60      4     reserved3               u32 LE: 0
```

### D5. CheckpointV0 — 4096 bytes (8 sectors)

```
Offset  Size  Field                   Contents
------  ----  -----                   --------
0       8     magic                   u64 LE: 0x31564B4E_484B5043 ("CPCHKNV1")
8       8     checkpoint_generation   u64 LE: monotonic checkpoint epoch
16      8     fs_generation           u64 LE: fs_generation at snapshot time
24      4     entry_size              u32 LE: 128
28      4     entry_count             u32 LE: 16
32      4     checksum                u32 LE: XOR of header + all table bytes
36      28    reserved_hdr            zero-filled (header padded to 64 bytes)
64      2048  table                   [SexfsObjectEntryV0; 16] — full table snapshot
2112    1984  reserved                zero-filled to 4096 bytes
```

4 checkpoints occupy LBAs 16-47 (32 sectors, 16384 bytes).

---

## E) Serialization Rules

### E1. Encode (write to disk)

1. Build struct in memory
2. Compute checksum over all fields EXCEPT the checksum field itself
3. Set checksum field
4. Serialize as LE byte array
5. Copy into MemLend buffer
6. Call `diskfs_block_write(sector * 512, 512, SLOT_BUF_LEND)`

### E2. Decode (read from disk)

1. Call `diskfs_block_read(sector * 512, 512, SLOT_BUF_LEND)`
2. Read data from MemLend buffer into byte array
3. Deserialize each field as LE
4. Compute checksum over all fields EXCEPT checksum field
5. Compare with stored checksum → reject on mismatch
6. Validate magic → reject on mismatch
7. Validate version → reject if unsupported
8. Validate block_size == 4096 → reject on mismatch

### E3. Validation Gates (every decode)

| Check | Rejection Code | When |
|-------|---------------|------|
| magic != expected | ERR_NOT_FOUND (-3) | Superblock, freemap, checkpoint reads |
| version > supported | ERR_OVERFLOW (-4) | Superblock, freemap reads |
| block_size != 4096 | ERR_OVERFLOW (-4) | Superblock read |
| checksum mismatch | ERR_OVERFLOW (-4) | All struct reads |
| object_id == 0 in table entry with IN_USE flag | ERR_INVALID_HANDLE (-1) | Object table scan |
| object_id not found | ERR_INVALID_HANDLE (-1) | Object lookup |
| freemap double-alloc | ERR_FULL (-5) | Block allocation |
| freemap out-of-space | ERR_FULL (-5) | Block allocation |
| extent_count == 0 with IN_USE flag | ERR_OVERFLOW (-4) | Object stat |

### E4. Bounded Limits

- Max 16 objects (DISKFS_MAX_OBJECTS)
- Max 1024 blocks tracked in freemap
- Max 64 journal records
- Max 4 checkpoints
- Max 237 allocatable blocks (~971 KB object data)
- Single contiguous extent per object (V0 limitation)
- 512-byte sector alignment for all reads/writes

---

## F) Proof Gates for Implementation

### F1. Format Proof (Phase 1)

```
[sexfs.v0.format.ok] superblock_written=1 backup_written=1 table_zeroed=1 freemap_init=1
```

- Writes primary superblock to LBA 0
- Writes backup superblock to LBA 1
- Writes zeroed object table to LBAs 2-5
- Writes initialized freemap to LBA 6 (blocks 0-15 and 253-255 marked in-use)
- Reads back and validates all four structs

### F2. Mount Proof (Phase 1)

```
[sexfs.v0.mount.ok] magic_ok=1 version_ok=1 checksum_ok=1 generation=1
```

- Reads superblock from LBA 0
- Validates magic, version, block_size, checksum
- Falls back to backup superblock at LBA 1 if primary invalid
- Reads object table from LBAs 2-5
- Reads freemap from LBA 6
- Validates all struct checksums

### F3. Bad Magic/Version/Checksum Rejection (Phase 1)

```
[sexfs.v0.bad_magic.reject] ok=1
[sexfs.v0.bad_version.reject] ok=1
[sexfs.v0.bad_checksum.reject] ok=1
```

- Corrupt superblock magic → mount fails with ERR_NOT_FOUND
- Set unsupported version → mount fails with ERR_OVERFLOW
- Corrupt superblock checksum → mount fails with ERR_OVERFLOW
- Catastrophic: both primary AND backup corrupt → mount fails

### F4. Object Create/Write/Read Proof (Phase 2)

```
[sexfs.v0.object.create.ok] object_id=1 kind=1 owner_pd=11 size=128 blocks_alloc=1 first_block=16
[sexfs.v0.object.write.ok] object_id=1 bytes=128 blocks=1
[sexfs.v0.object.read.ok] object_id=1 bytes=128 match=1
```

- Creates object entry in first free table slot
- Allocates contiguous blocks from freemap
- Writes updated object table to LBAs 2-5
- Writes updated freemap to LBA 6
- Writes object data to allocated blocks
- Reads back object data and verifies match

### F5. Freemap Rejection Proofs (Phase 2)

```
[sexfs.v0.freemap.double_alloc.reject] ok=1
[sexfs.v0.freemap.out_of_space.reject] ok=1
```

- Attempt double allocation → ERR_FULL
- Fill all 237 allocatable blocks → next alloc returns ERR_FULL

### F6. Reboot Restore Proof (Phase 3 — TWO-BOOT)

```
BOOT 1 (write):
  [sexfs.v0.format.ok] ...
  [sexfs.v0.object.create.ok] object_id=1 ...
  [sexfs.v0.object.write.ok] object_id=1 ...
  [sexfs.v0.flush.ok] (if NVMe FLUSH available)
  [sexfs.v0.reboot.write.done] ok=1

--- QEMU STOP / START ---

BOOT 2 (read):
  [sexfs.v0.mount.ok] ...
  [sexfs.v0.reboot.mount.ok] objects_found=1
  [sexfs.v0.reboot.read.ok] object_id=1 match=1
  [sexfs.v0.reboot.restore.done] ok=1
```

---

## G) STOP FIRST Items

1. **Write guard extension** — The current `write_guard_allows()` MUST be extended to permit writes to LBAs 0-47 (metadata) and 128-2019 (object data), with the same `proof_mode && size==512` constraints. This is the single gate between "block I/O works" and "SexFS v0 can persist." DO NOT change the guard without reviewing the impact on existing proof LBAs (2046-2047, 2022-2045).

2. **AP4/AP5A/AP6 self-tests** — These use LBAs 128-131, 256-259, 384-385 which overlap the object data region. These self-tests must be gated behind an env var (e.g., `SEXDRIVE_STORAGE_SELFTEST=1`) so they don't corrupt SexFS v0 data. Alternatively, relocate them to LBAs 2020-2021 (reserved region).

3. **Superblock sector size** — The superblock is 512 bytes (1 sector). If future expansion adds fields, the sector count must increase. The `object_table_sector` field already accounts for this by storing an LBA, not a hardcoded constant.

4. **Content_generation field** — This is a NEW field in SexfsObjectEntryV0. The in-memory scaffold used `metadata_generation` as a V1 proxy. The sexobject.rs adapter (`sexobject_header_from_entry`) must be updated to read the real `content_generation` field. The SexObjectHeader ABI (80 bytes, cross-PD) does NOT change.

5. **Object table entry size** — 128 bytes is a V0 contract. Future versions may increase this. The `object_entry_bytes` field in the superblock allows version detection.

6. **Backup superblock** — LBA 1 is the backup. If the primary superblock is corrupt, mount falls back to the backup. If BOTH are corrupt, mount fails irrecoverably. The format function writes both.

7. **No kernel edits** — MAP_PCI_BAR, SYS_ALLOC_PHYS, SYS_MAP_PHYS, MemLend are already wired. No kernel changes needed.

8. **No sex-pdx edits** — SLOT_BLOCK=15, SLOT_BUF_LEND=17, BLOCK_READ/WRITE/SYNC opcodes are already defined. No ABI changes needed.

---

## H) Next Prompt

**Recommended**: `SEXFS_V0_SUPERBLOCK_FORMAT_AND_MOUNT_V1`

Exact target:
1. Extend `write_guard_allows()` in `apps/sexdrive/src/main.rs` to whitelist LBAs 0-47 and 128-2019 (metadata + object data regions) with same `proof_mode && size==512` constraints
2. Implement `DiskFs::format_init_empty()` to write superblock, backup superblock, zeroed object table, and initialized freemap to disk using `diskfs_block_write()`
3. Implement `DiskFs::mount()` to read superblock from LBA 0, validate magic/version/checksum, fall back to LBA 1 backup, then read object table and freemap
4. Add proof markers for format + mount validation
5. Gate AP4/AP5A/AP6 self-tests behind `SEXDRIVE_STORAGE_SELFTEST=1`
6. Do NOT implement object create/write/read or reboot restore yet

---

*End of spec. No files changed. Commit: this spec doc only.*
