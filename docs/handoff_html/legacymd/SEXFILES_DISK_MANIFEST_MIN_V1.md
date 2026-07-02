# SEXFILES_DISK_MANIFEST_MIN_V1

## Mission Result
Plan-only completed. No code changes.

## Current Reality (Inspected)
- Disk transport path is proven (`DiskFs::diskfs_block_read/write` -> `SLOT_BLOCK` -> `sexdrive`).
- MemLend handoff is proven for payload transfer.
- Guarded persistence proof slot is in use:
  - `WRITE_PROOF_LBA = 2047` (`offset=0xffe00`, `size=512`).
- Final audit already states filesystem generalization is not yet claimed.

## Design Goal
Add the smallest safe on-disk manifest to map one named `/disk/...` proof object to a fixed reserved block range, with strict fail-safe parsing and no allocator/journaling.

## Minimal Manifest Layout (Fixed, 512 bytes)
Single-sector manifest record at a fixed LBA.

### Header (first 32 bytes)
- `magic: u64` = `0x31564D4B53494453` (`"SDISKMV1"` LE)
- `version: u16` = `1`
- `entry_count: u16` = `N` (bounded)
- `flags: u32` = 0 for V1
- `reserved0: u64` = 0
- `header_crc32: u32` (optional in V1-plan; can be 0 if CRC deferred)
- `reserved1: u32` = 0

### Entry format (32 bytes each, fixed)
- `name_hash: u64` (fixed hash of canonical path string)
- `start_lba: u64`
- `len_bytes: u32`
- `flags: u16` (bit0=READ, bit1=WRITE)
- `reserved: u16`
- `entry_crc32: u32` (optional V1-plan; may be 0 initially)
- `reserved2: u32`

### Capacity
- `(512 - 32) / 32 = 15` max entries
- V1 uses **entry_count <= 1** (single proof object)

## Reserved LBA Map (No Collision)
Image tail policy for this proof lane:
- `LBA 2047`: existing guarded write/readback proof slot (unchanged)
- `LBA 2046`: **manifest sector** (new)
- `LBA 2038..2045` (8 sectors = 4096 bytes): `/disk/sexfiles-proof-v1` fixed object data region

Rationale:
- Does not collide with existing `LBA 2047` write proof slot.
- Keeps manifest+object near tail for bounded proofs.
- Uses fixed range (no allocator).

## Canonical First Object
- Path: `/disk/sexfiles-proof-v1`
- `name_hash`: deterministic fixed hash of this exact byte string
- `start_lba`: `2038`
- `len_bytes`: `4096`
- `flags`: `READ|WRITE`

## Bad-Manifest Behavior (Fail-Safe)
- Invalid `magic`: treat as **empty manifest**, return not-found for `/disk/...` lookups.
- Invalid `version`: fail-safe reject manifest (empty view).
- `entry_count > 15`: reject manifest (empty view).
- Any entry with `len_bytes == 0`: reject manifest.
- Any entry where `start_lba == 2047`: reject manifest.
- Any entry range including `LBA 2046` or `LBA 2047`: reject manifest.
- Overlapping entry ranges: reject manifest.

V1 policy: if validation fails, do **not** auto-repair; mount with empty disk-manifest view and emit explicit proof marker.

## Why This Is Safe and Tiny
- No VFS redesign.
- No ABI/kernel changes.
- No directory tree/allocator/delete/rename/journaling.
- Single fixed sector + fixed entry schema.
- Bounded parse and strict range validation.

## STOP FIRST Conditions for Implementation
- If discovered NVMe image geometry conflicts with `LBA 2038..2047` reservation.
- If existing proof flow implicitly depends on `LBA 2046` contents.
- If file-level API expectations force allocator or journaling immediately.

## Exact Next Implementation Prompt
`SEXFILES_DISK_MANIFEST_MIN_IMPL_V1`

### Prompt body (ready to run)
```text
MISSION: SEXFILES_DISK_MANIFEST_MIN_IMPL_V1

Goal:
Implement minimal fixed disk manifest parsing/writing for one proof object mapping.

Scope:
- servers/sexfiles/src/backends/diskfs.rs
- servers/sexfiles/src/proof.rs
- docs/handoff/SEXFILES_DISK_MANIFEST_MIN_IMPL_V1.md

Do not change ABI/kernel/sex-pdx.
Do not add allocator/journaling/tree/delete/rename.

Implement:
1. Fixed constants:
   - MANIFEST_LBA=2046
   - PROOF_WRITE_LBA=2047 (existing)
   - PROOF_OBJ_START_LBA=2038
   - PROOF_OBJ_LEN_BYTES=4096
2. Fixed structs/pack-unpack helpers for 512-byte manifest sector.
3. Name-hash helper for `/disk/sexfiles-proof-v1` (deterministic, bounded).
4. Validation rules:
   - magic/version/entry_count bounds
   - no overlap
   - no range collision with LBA 2046 or 2047
5. Proof helpers:
   - write_manifest_single_entry()
   - read_manifest_validate()
   - lookup `/disk/sexfiles-proof-v1` -> (start_lba,len_bytes,flags)
6. Proof markers:
   - sexfiles.disk.manifest.begin
   - sexfiles.disk.manifest.write.ok
   - sexfiles.disk.manifest.read.ok
   - sexfiles.disk.manifest.lookup.ok
   - sexfiles.disk.manifest.bad_magic.empty
   - sexfiles.disk.manifest.overlap.reject
   - sexfiles.disk.manifest.err
7. No behavior changes to existing guarded write/readback lane.

Success:
- Build passes.
- New manifest markers appear.
- Existing persistence/negative markers still pass.
```
