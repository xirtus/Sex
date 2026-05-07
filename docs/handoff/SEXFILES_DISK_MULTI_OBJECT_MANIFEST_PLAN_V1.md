# SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1

## Date
2026-05-07

## Status
PLAN COMPLETE — No implementation. Next: `SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1`.

## 1. Current Manifest Audit

### Manifest Sector (LBA 2046, 512 bytes)

```
Offset  Size  Field
0       8     magic        = 0x3156_4D4B_5349_4453 ("SDISKMV1" LE)
8       2     version      = 1
10      2     entry_count  = 1
12      4     flags        = 0
16      8     reserved0    = 0
24      4     header_crc32 = 0 (deferred)
28      4     reserved1    = 0
32      32    entry[0]     = /disk/sexfiles-proof-v1
               - name_hash: 0xdb0809f591d496d6
               - start_lba: 2038
               - len_bytes: 4096
               - flags:     0x3 (READ|WRITE)
               - reserved:  0
               - entry_crc: 0 (deferred)
32+32*N  ...   entry[N]     = not used (14 slots free)
```

**Capacity**: (512 - 32) / 32 = 15 entries max. V1 uses 1.

### Current Object Map

| Object                  | Path                      | Hash                 | LBA      | Size  |
|-------------------------|---------------------------|----------------------|----------|-------|
| SexFiles proof          | /disk/sexfiles-proof-v1   | 0xdb0809f591d496d6   | 2038-2045| 4096  |
| (none)                  | —                         | —                    | —        | —     |
| (none)                  | —                         | —                    | —        | —     |

### Reserved (never in manifest)

| Purpose           | LBA   |
|-------------------|-------|
| Write proof slot  | 2047  |
| Manifest sector   | 2046  |

## 2. Proposed V2 Layout

### Reserved Object Slots (3 total, 15 max capacity)

| Slot | Object               | Path                    | Hash                 | LBA range   | Size | Flags |
|------|----------------------|-------------------------|----------------------|-------------|------|-------|
| 0    | SexFiles proof       | /disk/sexfiles-proof-v1 | 0xdb0809f591d496d6   | 2038-2045   | 4096 | 0x3   |
| 1    | Linen object         | /disk/linen-object-v1   | 0x6a271e295a85a332   | 2030-2037   | 4096 | 0x3   |
| 2    | Quil object          | /disk/quil-object-v1    | 0xaaf5c55ad6c063b5   | 2022-2029   | 4096 | 0x3   |

### LBA Map (NVMe sectors, 512B each)

```
Sector  Manifest/Objects
──────  ──────────────────────────────────────────────
2047    WRITE_PROOF_LBA — reserved, never in manifest
2046    MANIFEST_LBA — 3-entry manifest sector
2038    SexFiles proof object (8 sectors, 2038-2045)
2030    Linen object          (8 sectors, 2030-2037)
2022    Quil object           (8 sectors, 2022-2029)
  ↓
 48     Free area (~247 sectors, ~123.5KB per region)
  ↓      (available for future objects / allocator)
  0     Superblock + DiskFS metadata (reserved)
```

### Collision Avoidance

Each reserved object occupies exactly 8 consecutive sectors (4096 bytes).
All objects are 8-sector aligned. No overlaps.

```
Proof:         check `end_lba < DISKFS_WRITE_PROOF_LBA` (i.e., 2045 < 2047)
Manifest:      entries must not intersect each other
               entries must not intersect MANIFEST_LBA (2046)
               entries must not intersect WRITE_PROOF_LBA (2047)
               entry.start_lba + entry.len_bytes/512 <= entry.start_lba + 8
```

**Overlap check** (for N entries):
```
For each i in 0..N:
  For each j in i+1..N:
    If entry[i].start_lba <= entry[j].end_lba AND
       entry[j].start_lba <= entry[i].end_lba:
         → REJECT (overlap)
```

## 3. Design Changes (Implementation Plan)

### 3A. New Constants in diskfs.rs

```rust
// Reserved object slots (pre-allocated LBA ranges).
// Each slot = 8 sectors = 4096 bytes.
pub const DISKFS_OBJECT_SLOT_QUIL:  u64 = 2022;  // /disk/quil-object-v1
pub const DISKFS_OBJECT_SLOT_LINEN: u64 = 2030;  // /disk/linen-object-v1
// DISKFS_PROOF_OBJECT_START_LBA = 2038 remains for /disk/sexfiles-proof-v1

// V2 manifest entry count
pub const DISKFS_MANIFEST_V2_ENTRY_COUNT: u16 = 3;

// Reserved paths
pub const DISKFS_OBJECT_PATH_SEXFILES: &[u8] = b"/disk/sexfiles-proof-v1";
pub const DISKFS_OBJECT_PATH_LINEN:    &[u8] = b"/disk/linen-object-v1";
pub const DISKFS_OBJECT_PATH_QUIL:     &[u8] = b"/disk/quil-object-v1";

// Manifest format version bump (backward compatible)
pub const DISKFS_MANIFEST_VERSION_V2: u16 = 2;
```

### 3B. Replace `proof_manifest_build_single_entry_sector` with V2

New function: `proof_manifest_build_sector_v2() -> [u8; 512]`
- Builds 3 entries from the reserved object slots
- Header: version=2, entry_count=3
- Overlap validation before serialization

New function: `proof_manifest_parse_v2(sector: &[u8; 512]) -> Result<[DiskManifestEntryV1; 3], u64>`
- Parses version=2 header
- Validates entry_count is within bounds (1..15)
- Validates no overlapping LBA ranges
- Validates all entries do not intersect LBA 2046 or 2047
- Returns array of valid entries

### 3C. Update `diskfs_lookup_path` for V2

Currently `diskfs_lookup_path` only accepts `/disk/sexfiles-proof-v1`:

```rust
let arg_hash = Self::proof_manifest_name_hash(path);
let expected_hash = Self::proof_manifest_name_hash(DISKFS_MANIFEST_OBJECT_PATH);
if arg_hash != expected_hash {
    return Err(messages::ERR_NOT_FOUND as u64);
}
```

V2 change: parse all entries, find matching name_hash:

```rust
let entries = Self::proof_manifest_parse_v2(&sector)?;
let arg_hash = Self::proof_manifest_name_hash(path);
for entry in &entries {
    if entry.name_hash == arg_hash {
        return Ok(*entry);
    }
}
Err(messages::ERR_NOT_FOUND as u64)
```

### 3D. Update `diskfs_ensure_manifest` for V2

Currently writes V1 single-entry manifest. V2 should:
- Try to parse as V2 (3 entries)
- If V1 (1 entry), upgrade to V2 by adding Linen + Quil entries
- If invalid, bootstrap V2 from scratch
- Always idempotent: valid V2 manifest → no-op

### 3E. Bridge Opcode Updates

New opcode: `OP_DISKFS_SELECT = 0x3D`
- arg0 = name_hash (u64, FNV-1a of object path)
- Selects which object subsequent WRITE/READ/FLUSH operate on
- Default (no SELECT): operates on /disk/sexfiles-proof-v1 (backward compatible)
- Returns 0 on success, ERR_NOT_FOUND if hash unknown

OR keep it simpler: add a `path_id` selector:
- arg0 = 0 → /disk/sexfiles-proof-v1 (default)
- arg0 = 1 → /disk/linen-object-v1
- arg0 = 2 → /disk/quil-object-v1

Linen-side: add `pdx_storage_sync(OP_DISKFS_SELECT, 1, 0, 0)` before
WRITE/READ to operate on the Linen object.

### 3F. Backward Compatibility

- V1 manifests (version=1, 1 entry) still parse and work
- V2 manifests (version=2, N entries) extend V1
- `proof_manifest_parse_v2` handles both versions
- `diskfs_ensure_manifest` upgrades V1→V2 on first bridge op
- Existing proofs (file ops, persistence) unchanged — they use V1 path

## 4. Boot Validation Rules

1. Read manifest sector (LBA 2046)
2. If magic != DISKFS_MANIFEST_MAGIC → bootstrap V2 manifest
3. If version == 1 → parse V1 entry, upgrade to V2 (add Linen + Quil entries)
4. If version == 2 → parse V2 entries, validate:
   a. entry_count in [1, 15]
   b. No overlapping LBA ranges between entries
   c. No entry intersects LBA 2046 or 2047
   d. All required reserved objects present (optional: warn if missing)
5. If any validation fails → rebuild manifest from scratch (V2, 3 entries)
6. Cache as valid, set DISKFS_MANIFEST_READY

## 5. Proof Sequence (for implementation gate)

### Gate: SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF=1

**Phase 1: Bootstrap**
1. Read LBA 2046 → detect invalid → write V2 manifest (3 entries)
2. Marker: `sexfiles.disk.manifest.v2.bootstrap ok=1 entries=3`

**Phase 2: Write object A (Linen)**
3. SELECT path_id=1 (Linen)
4. Write 128-byte deterministic payload (matching Linen proof)
5. Marker: `sexfiles.disk.multi.object.write.ok path=/disk/linen-object-v1 size=128`

**Phase 3: Write object B (Quil)**
6. SELECT path_id=2 (Quil)
7. Write 128-byte deterministic payload (different pattern)
8. Marker: `sexfiles.disk.multi.object.write.ok path=/disk/quil-object-v1 size=128`

**Phase 4: Read back A + verify**
9. SELECT path_id=1 → read 128 bytes → verify match
10. Marker: `sexfiles.disk.multi.object.match path=/disk/linen-object-v1`

**Phase 5: Read back B + verify**
11. SELECT path_id=2 → read 128 bytes → verify match
12. Marker: `sexfiles.disk.multi.object.match path=/disk/quil-object-v1`

**Phase 6: Verify no collision**
13. Read object A again → must match original, not B's data
14. Marker: `sexfiles.disk.multi.object.no_collision ok=1`

**Phase 7: Verify SexFiles proof object still intact**
15. SELECT path_id=0 → read original 4096 bytes → verify pattern match
16. Marker: `sexfiles.disk.multi.object.proof_still_ok ok=1`

**Phase 8: Manifest integrity**
17. Re-read LBA 2046 → parse V2 → verify all 3 entries correct
18. Marker: `sexfiles.disk.multi.object.manifest_still_ok ok=1`

**Phase 9: Limits**
19. SELECT path_id=99 → ERR_NOT_FOUND
20. Collision injection: try to write entry with overlapping LBA → ERR_OVERFLOW
21. Marker: `sexfiles.disk.multi.object.bounds_negative ok=1`

## 6. Files to Change (Implementation)

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | V2 constants, manifest_build_v2, manifest_parse_v2, update lookup + ensure |
| `servers/sexfiles/src/messages.rs` | Add OP_DISKFS_SELECT (0x3D) |
| `servers/sexfiles/src/vfs.rs` | Add SELECT handler, per-object state |
| `servers/sexfiles/src/proof.rs` | Add run_disk_multi_object_proof() |
| `servers/sexfiles/src/trampoline.rs` | Wire SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF |

**NOT changed**:
- `crates/sex-pdx/` — no ABI edits
- `kernel/` — no kernel changes
- `apps/sexdrive/` — no sexdrive changes (same LBA range)
- `servers/linen/` — Linen uses existing bridge; SELECT is optional

## 7. STOP FIRST Conditions

| Condition | Met? | Resolution |
|-----------|------|------------|
| Requires allocator | NO | Fixed reserved slots |
| Requires journaling | NO | Idempotent write, no crash consistency needed |
| Conflicts with LBA 2047 | NO | All objects below 2046 |
| Requires kernel changes | NO | All within SexFiles |
| Requires sex-pdx changes | NO | No new slots |
| Requires general filesystem design | NO | 3 fixed objects, bounded |

## 8. Exact Next Prompt

```
SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1

Implement the V2 manifest with 3 reserved object slots per
SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1.

1. Add V2 constants and reserved object slots to diskfs.rs.
2. Add manifest_build_v2() and manifest_parse_v2().
3. Update diskfs_lookup_path() for multi-entry lookup.
4. Update diskfs_ensure_manifest() for V1→V2 upgrade.
5. Add OP_DISKFS_SELECT (0x3D) to messages.rs.
6. Add SELECT handler in vfs.rs with per-object path state.
7. Add multi-object proof in proof.rs.
8. Wire SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF in trampoline.rs.
9. Build and run:
   SEXOS_GATE_NVME=1 SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF=1
   ./scripts/master_runtime_gate.sh --probe 45 --keep-log
10. Verify all 9 proof phases pass.
11. Write docs/handoff/SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1.md.
```
