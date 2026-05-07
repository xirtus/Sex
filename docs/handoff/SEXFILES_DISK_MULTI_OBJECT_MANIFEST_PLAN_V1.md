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

### 3E. SELECT Opcode (0x3D) — Tightened Semantics

**Decision**: Option A — single-client proof-only global SELECT, explicitly
not concurrent-safe. Future V3 should use caller-scoped session state.

```rust
OP_DISKFS_SELECT = 0x3D
```

**arg0 = path_id** (u64):
- `0` → `/disk/sexfiles-proof-v1` (default, backward compatible)
- `1` → `/disk/linen-object-v1`
- `2` → `/disk/quil-object-v1`
- Any other value → `ERR_BAD_CMD` (-1)
- SELECT does NOT accept name hashes, raw paths, or LBA addresses from the client.
- SELECT does NOT expose LBA addresses to Linen or Quil.

**Server state**: One global `AtomicU64 DISKFS_SELECTED_PATH_ID` (defaults to 0).
A SELECT call sets it; subsequent WRITE/READ/FLUSH/STAT/HASH operate on the
selected object. Single-client V1 only — concurrent clients would race.

**Marker on first use**: `[sexfiles.bridge.diskfs.select.v1_single_client]`

**STAT after SELECT**: Returns size and flags of the SELECTED object
(e.g., `path_id=1` → size=4096, flags=0x3 for /disk/linen-object-v1).

**HASH after SELECT**: Returns the FNV-1a hash of the SELECTED object's path.
This allows the client to confirm which object is selected.

**Default**: path_id=0 at boot and after manifest bootstrap. Backward compatible
— existing Linen bridge proof operates on path_id=0 without calling SELECT.

### 3F. Write Guard Extension

Extend sexdrive `write_guard_allows()` to accept the new LBA ranges
in addition to existing allowed ranges:

```
Existing:  2038..2045 (SexFiles proof), 2046 (manifest), 2047 (write proof)
New:       2022..2029 (Quil object), 2030..2037 (Linen object)
```

All writes outside these ranges → `ERR_NO_DEVICE honest=write_not_implemented_guard_only`.
No generic writes. No arbitrary LBA writes.

### 3G. V1→V2 Upgrade Safety

`diskfs_ensure_manifest` upgrade path:

1. Read LBA 2046
2. If magic != DISKFS_MANIFEST_MAGIC → **bootstrap V2** from scratch
   - Marker: `[sexfiles.disk.manifest.v2.bootstrap] entries=3`
3. If version == 1 (V1, 1 entry) → **upgrade to V2**
   - Preserve existing V1 entry (SexFiles proof object)
   - Add Linen + Quil entries at their reserved LBAs
   - Write new 3-entry V2 manifest to LBA 2046
   - Must NOT rewrite any object data ranges (LBAs 2022-2045)
   - Must NOT touch LBA 2047
   - Marker: `[sexfiles.disk.manifest.v2.upgrade] from_version=1 entries=3`
4. If version == 2 → validate all entries
   - Check no overlaps, no LBA 2046/2047 intersection
   - If valid: marker `[sexfiles.disk.manifest.v2.valid] entries=N`
   - If invalid and proof mode: **bootstrap** V2 from scratch
     Marker: `[sexfiles.disk.manifest.v2.err] reason=corrupt action=bootstrap`
   - If invalid and NOT proof mode: return `ERR_OVERFLOW`
5. Cache as ready.

### 3H. Backward Compatibility

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
   d. All entries have valid reserved start_lba values
5. If any validation fails and proof mode → rebuild manifest from scratch (V2, 3 entries)
6. If any validation fails and NOT proof mode → return ERR_OVERFLOW
7. Cache as valid, set DISKFS_MANIFEST_READY

## 5. Proof Sequence (for implementation gate)

### Gate: SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF=1

**Phase 1: Bootstrap or upgrade**
1. Read LBA 2046 → detect V1 or invalid → upgrade/bootstrap V2 manifest (3 entries)
2. Marker: `sexfiles.disk.manifest.v2.upgrade` or `.v2.bootstrap`

**Phase 2: SELECT V1 single-client marker**
3. Marker: `sexfiles.bridge.diskfs.select.v1_single_client`

**Phase 3: Write Linen object**
4. SELECT path_id=1
5. Write 128-byte deterministic payload to Linen object
6. Marker: `sexfiles.disk.multi.object.write.ok path_id=1 size=128`

**Phase 4: Write Quil object**
7. SELECT path_id=2
8. Write 128-byte deterministic payload (different pattern) to Quil object
9. Marker: `sexfiles.disk.multi.object.write.ok path_id=2 size=128`

**Phase 5: Read Linen + match**
10. SELECT path_id=1 → read 128 bytes → verify match
11. Marker: `sexfiles.disk.multi.object.match path_id=1 ok=1`

**Phase 6: Read Quil + match**
12. SELECT path_id=2 → read 128 bytes → verify match
13. Marker: `sexfiles.disk.multi.object.match path_id=2 ok=1`

**Phase 7: No collision**
14. Read Linen again → must match Linen data, not Quil data
15. Marker: `sexfiles.disk.multi.object.no_collision ok=1`

**Phase 8: Proof object intact**
16. SELECT path_id=0 → read original 4096 bytes → verify pattern match
17. Marker: `sexfiles.disk.multi.object.proof_still_ok ok=1`

**Phase 9: Manifest integrity**
18. Re-read LBA 2046 → parse V2 → verify all 3 entries correct
19. Marker: `sexfiles.disk.multi.object.manifest_still_ok ok=1`

**Phase 10: Limits**
20. SELECT path_id=99 → ERR_BAD_CMD
21. SELECT path_id=3 → ERR_BAD_CMD
22. Marker: `sexfiles.disk.multi.object.bounds_negative ok=1`

**Phase 11: Regression**
23. Existing persistence proof still passes
24. Storage negatives still pass
25. No #PF/#GP/panic

## 6. Files to Change (Implementation)

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | V2 constants, manifest_build_v2, manifest_parse_v2, update lookup + ensure with V1→V2 upgrade |
| `servers/sexfiles/src/messages.rs` | Add OP_DISKFS_SELECT (0x3D), ERR_BAD_CMD |
| `servers/sexfiles/src/vfs.rs` | Add SELECT handler with global path_id state, update STAT/HASH for per-object, SELECT marker |
| `apps/sexdrive/src/main.rs` | Extend write_guard_allows() for LBAs 2022-2037 |
| `servers/sexfiles/src/proof.rs` | Add run_disk_multi_object_proof() (11 phases) |
| `servers/sexfiles/src/trampoline.rs` | Wire SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF |

**NOT changed**:
- `crates/sex-pdx/` — no ABI edits
- `kernel/` — no kernel changes
- `servers/linen/` — avoid touching Linen if OpenIntent branch is active; proof runs from SexFiles side

## 7. STOP FIRST Conditions

| Condition | Met? | Resolution |
|-----------|------|------------|
| Requires allocator | NO | Fixed reserved slots |
| Requires journaling | NO | Idempotent write, no crash consistency needed |
| Conflicts with LBA 2047 | NO | All objects below 2046 |
| Requires kernel changes | NO | All within SexFiles |
| Requires sex-pdx changes | NO | No new slots |
| Requires general filesystem design | NO | 3 fixed objects, bounded |
| SELECT is concurrent-unsafe | YES (V1) | Documented single-client only; marker emitted |
| V1→V2 upgrade touches object data | NO | Manifest sector only; object LBAs preserved |
| OpenIntent collision | AVOIDED | Proof runs in SexFiles; Linen untouched |

## 8. Limitations (Must Be Stated in Implementation Handoff)

- **Fixed-slot multi-object storage, NOT a general allocator.**
- **No delete/rename. No directory tree. No dynamic object creation.**
- **SELECT is single-client proof-only V1.** Concurrent clients would race on
  the global `DISKFS_SELECTED_PATH_ID`. Future V3 should use caller-scoped
  session state keyed by caller_pd.
- **No dynamic path IPC.** Paths are mapped to fixed path_id values.
  Clients never send path strings or LBA addresses.
- **V1→V2 upgrade only from proof mode or first bridge use.** Accidental
  upgrade of a valid V1 manifest by a non-proof client is prevented.
- **Write guard must be extended in sexdrive** for LBAs 2022-2037.
  This is a minor allowed change (same guard pattern, new ranges).

## 9. Exact Next Prompt

```
SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1

Implement the V2 manifest with 3 reserved object slots per the refined
SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1 (with tightened SELECT semantics).

STOP FIRST: confirm no OpenIntent branch is active touching servers/linen/.

1. Add V2 constants and reserved object slots to diskfs.rs.
2. Add manifest_build_v2() and manifest_parse_v2().
3. Update diskfs_lookup_path() for multi-entry hash lookup.
4. Update diskfs_ensure_manifest() for V1→V2 upgrade with safety markers:
   .v2.bootstrap, .v2.upgrade, .v2.valid, .v2.err.
5. Add OP_DISKFS_SELECT (0x3D) to messages.rs — path_id only, no hashes.
6. Add ERR_BAD_CMD to messages.rs for invalid path_id.
7. Add SELECT handler in vfs.rs:
   - global DISKFS_SELECTED_PATH_ID (AtomicU64, default 0)
   - valid values: 0/1/2 only
   - emit [sexfiles.bridge.diskfs.select.v1_single_client] on first SELECT
8. Update STAT/HASH handlers to return per-selected-object data.
9. Extend write_guard_allows() in sexdrive for LBAs 2022-2037.
10. Add run_disk_multi_object_proof() in proof.rs (11 phases).
11. Wire SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF in trampoline.rs.
12. Build and run:
    SEXOS_GATE_NVME=1 SEXOS_DISK_MULTI_OBJECT_MANIFEST_PROOF=1
    ./scripts/master_runtime_gate.sh --probe 45 --keep-log
13. Verify all 11 proof phases pass.
14. Write docs/handoff/SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1.md
    with limitations clearly stated.
```
