# SEXFILES_ON_DISK_FORMAT_LOCK_V1

Date: 2026-05-06  
Status: **Format lock (design), no implementation claims**

## Scope
This document locks the **SexFiles V1 on-disk format contract** before backend implementation.

- No kernel changes
- No sex-pdx ABI changes
- No POSIX path/permission model
- No app raw disk access
- No snapshot implementation in V1

SexFiles is a capability-native object store served over PDX (`SLOT_STORAGE`), not a Unix filesystem.

## V1 Design Targets
- Bounded metadata structures
- Deterministic decode/validation
- Crash recovery with replay-only committed records
- Capability persistence fields (`owner_pd`, rights, revocation generation)
- Upgrade-safe version and feature flags

---

## 1. Superblock (Primary)

### Layout (fixed-size, little-endian)
Suggested size: **256 bytes** (room for future fields, deterministic parse).

| Field | Type | Description |
|------|------|-------------|
| `magic` | `u64` | `SEXFILESV1\0` magic (`0x31565346584553`-style constant; exact byte lock in code constants phase) |
| `version_major` | `u16` | `1` |
| `version_minor` | `u16` | `0` |
| `block_size` | `u32` | V1 fixed allowed set: `{4096}` |
| `fs_generation` | `u64` | Monotonic filesystem checkpoint generation |
| `object_table_start_block` | `u64` | First block of object table region |
| `object_table_block_count` | `u32` | Block span for object table |
| `object_table_entry_count` | `u32` | Max object entries in table |
| `free_map_start_block` | `u64` | First block of allocation bitmap/table |
| `free_map_block_count` | `u32` | Block span for free map |
| `journal_start_block` | `u64` | First block of journal ring |
| `journal_block_count` | `u32` | Journal size in blocks |
| `feature_flags_compat` | `u64` | Forward-compatible features |
| `feature_flags_incompat` | `u64` | Must-understand flags |
| `superblock_crc32c` | `u32` | CRC over superblock with this field zeroed |
| `reserved` | bytes | Zeroed, must validate as zero in V1 strict mode |

### Superblock invariants
1. `magic` and `version_major==1` required.
2. `block_size==4096` in V1.
3. Regions must be non-overlapping and in-range.
4. `fs_generation` monotonic (never decreases on valid mount replay).
5. CRC must validate before any region trust.

---

## 2. Object Table Entry (OTE)

### Layout (fixed-size, little-endian)
Suggested size: **96 bytes** per entry.

| Field | Type | Description |
|------|------|-------------|
| `object_id` | `u64` | Stable object identifier (`0` = free entry) |
| `object_kind` | `u16` | `SexfsObjectKind` enum |
| `entry_state` | `u16` | `0=free,1=live,2=tombstoned,reserved` |
| `owner_pd` | `u32` | Owning PD identity |
| `rights_bits` | `u32` | Capability-native rights mask |
| `rights_generation` | `u64` | Revocation/cap-generation reference |
| `object_size_bytes` | `u64` | Logical object size |
| `first_extent_block` | `u64` | First extent or head block pointer (`0` none) |
| `extent_block_count` | `u32` | Extent span (`0` for metadata-only/tombstone) |
| `metadata_generation` | `u64` | Monotonic per-object metadata generation |
| `data_checksum` | `u32` | Optional fast data checksum root/summary (V1 policy field) |
| `entry_crc32c` | `u32` | CRC over entry with this field zeroed |
| `reserved` | bytes | Zeroed for V1 |

### OTE invariants
1. `object_id==0` only when `entry_state==free`.
2. Live entry requires valid `owner_pd`, non-reserved kind, valid CRC.
3. `metadata_generation` must increase on metadata mutation.
4. Tombstoned entries preserve `rights_generation` and generation history.

---

## 3. Object Kinds (V1)

`SexfsObjectKind`:
- `0`: Unknown/Reserved
- `1`: RawBlob
- `2`: QuilDocument
- `3`: LinenObject
- `4`: SceneSnapshot
- `5`: AppState
- `6`: BellEventLog

Rules:
- Unknown kinds are non-mount-fatal if not referenced by active routes, but must not be opened by V1 clients.
- Kind-specific semantics are route-layer contracts, not POSIX files.

---

## 4. Free/Block Map

### V1 choice
- **Bitmap-based allocator**, fixed region.
- One bit per data block (`1=allocated`, `0=free`).

### Format
- Block map is contiguous region from superblock pointers.
- Bits beyond total data blocks must be zero.

### Allocation invariants
1. Deterministic out-of-space: return explicit `ERR_NO_SPACE` (existing storage status mapping to be bound in implementation).
2. No implicit block reuse without journaling commit.
3. Free-map update must be journaled before checkpoint publish.

---

## 5. Journal Record Format

### V1 model
Append-only journal records with explicit commit marker record.

### Record header (fixed 32 bytes)
| Field | Type | Description |
|------|------|-------------|
| `record_type` | `u16` | Begin/Update/Alloc/Free/Commit/Abort/Checkpoint |
| `record_flags` | `u16` | Reserved bits (V1 zero) |
| `tx_id` | `u64` | Transaction identifier |
| `object_id` | `u64` | Target object (`0` for global allocator ops) |
| `payload_len` | `u32` | Payload bytes following header |
| `record_generation` | `u64` | Monotonic record generation |
| `header_crc32c` | `u32` | CRC over header with crc field zeroed |

Payload:
- Type-specific bounded payload.
- Followed by `payload_crc32c: u32`.

Commit marker:
- Dedicated `record_type=Commit` with same `tx_id`.
- Replay considers a tx committed only if Commit record exists and validates.

---

## 6. Checksums (V1)

Chosen checksum: **CRC-32C (Castagnoli)**

Reason:
- no_std-friendly
- simple software implementation
- better error detection than XOR
- already fits existing bounded storage patterns

Use in V1:
- superblock CRC
- object table entry CRC
- journal header + payload CRC

Upgrade path:
- future `feature_flags_compat` may add stronger checksums (e.g., xxHash64/BLAKE3 summary trees) while preserving CRC-32C baseline compatibility.

---

## 7. Crash Recovery Rules

1. Scan journal from last known checkpoint position.
2. Validate each record CRC before use.
3. Group by `tx_id`.
4. **Replay only transactions with valid Commit marker**.
5. Ignore incomplete/uncommitted/CRC-invalid transactions.
6. After replay, publish new checkpoint with strictly higher `fs_generation`.
7. Superblock checkpoint pointer/generation must be monotonic; never roll back generation on clean mount.

---

## 8. Capability Persistence Model

Per-object persisted authority fields:
- `owner_pd: u32`
- `rights_bits: u32`
- `rights_generation: u64`

Interpretation:
- `owner_pd` is the default authority root for object operations.
- `rights_bits` encode capability-native rights classes (read/write/link/admin-style object rights; exact bit assignments locked in route-layer capability spec).
- `rights_generation` supports revocation/rotation semantics: stale handles must fail generation checks.

Explicitly excluded:
- Unix UID/GID
- rwx triplets
- ACL text/path models

---

## 9. Explicit Non-Goals (V1)

1. No POSIX path model.
2. No ext4/btrfs/zfs feature cloning.
3. No snapshots implementation (only `SceneSnapshot` object kind reserved).
4. No persistence reliability claim until DiskFS proof gates pass.
5. No direct app raw disk access.
6. No kernel/ABI redesign in this format-lock phase.

---

## 10. Deterministic Error Classes (for implementation bind)

V1 implementation must provide deterministic mapping for at least:
- invalid superblock/version/checksum
- invalid object entry checksum/state
- object not found
- permission denied (owner/rights/generation mismatch)
- out of space
- journal corruption (record invalid)
- transaction incomplete (ignored, non-fatal)

---

## 11. Implementation Sequence (Recommended)

1. **Constants/types lock** in `sexfiles` (superblock/OTE/journal structs + enums, no behavior).
2. **Serializer/validator** for superblock + OTE + journal record CRC checks.
3. **In-memory mount parser** for image region validation.
4. **Allocator/free-map operations** with deterministic bounds + errors.
5. **Journal append + commit** (no replay yet).
6. **Replay engine** (commit-only replay, incomplete tx ignored).
7. **Checkpoint write** with monotonic generation.
8. **Route bind** to existing storage operations behind proof gates.
9. **DiskFS durability proof phase** before any persistence claims.

---

## 12. V1 Lock Invariants Summary

- Region layout is explicit and checksum-verified.
- All metadata writes are generation-aware.
- Journal replay is commit-gated and CRC-validated.
- Capability data is first-class object metadata.
- No POSIX semantics leak into model.
- No app raw disk authority.

