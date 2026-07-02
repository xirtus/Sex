# SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1

## Date
2026-05-07

## Status
IMPLEMENTED — DiskFS V2 3-object manifest with OP_DISKFS_SELECT.

## Opcode
```
OP_DISKFS_SELECT = 0x3E
```
(0x3D was occupied by OP_RAMFS_READNAME; plan correctly updated to 0x3E.)

## V2 Layout Implemented

| path_id | Object | Path | LBA range | Size | Flags |
|---------|--------|------|-----------|------|-------|
| 0 | SexFiles proof | /disk/sexfiles-proof-v1 | 2038–2045 | 4096 | 0x3 |
| 1 | Linen object | /disk/linen-object-v1 | 2030–2037 | 4096 | 0x3 |
| 2 | Quil object | /disk/quil-object-v1 | 2022–2029 | 4096 | 0x3 |

Manifest at LBA 2046. Write-proof slot at LBA 2047 (untouched).

## SELECT Semantics
- **V1 single-client proof-only** — global `DISKFS_SELECTED_PATH_ID` in vfs.rs.
- Not caller-scoped. Future V3 should use per-caller state.
- Marker `[sexfiles.bridge.diskfs.select.v1_single_client]` on first SELECT.
- Valid path_ids: 0, 1, 2. Invalid → ERR_BAD_CMD.
- No raw path/hash/LBA from client. All resolved server-side via manifest read.

## Files Changed

| File | Changes |
|------|---------|
| `servers/sexfiles/src/messages.rs` | Added `OP_DISKFS_SELECT = 0x3E`, `ERR_BAD_CMD = -7` |
| `servers/sexfiles/src/backends/diskfs.rs` | V2 constants, `proof_manifest_build_v2_entries_sector()`, `proof_manifest_parse_v2_entries()`, `diskfs_lookup_by_path_id()`, `diskfs_path_for_id()`, `diskfs_ensure_manifest_v2()`. Replaced hardcoded `DISKFS_PROOF_OBJECT_START_LBA` with `entry.start_lba` in write/read loops. |
| `servers/sexfiles/src/vfs.rs` | `DISKFS_SELECTED_PATH_ID`, `DISKFS_SELECT_USED` state. `handle_diskfs_select()`, `diskfs_selected_path()`. Bridge handlers updated to use selected path + V2 manifest ensure. SELECT dispatch case added. |
| `servers/sexfiles/src/proof.rs` | `run_diskfs_multi_object_proofs()` — 3-entry manifest validate, Linen write/read/match, Quil write/read/match, proof object intact verify, invalid SELECT negatives. |
| `servers/sexfiles/src/trampoline.rs` | Gate: `SEXOS_DISKFS_MULTI_OBJECT_PROOF`. |

## V1→V2 Upgrade Safety
- `diskfs_ensure_manifest_v2()` detects V1 manifest, upgrades to V2 by writing a new 3-entry manifest sector.
- Never touches existing object data (LBA 2022-2045).
- Never writes LBA 2047.
- Idempotent — repeated calls detect valid V2 and return immediately.

## Proof Markers
```
[sexfiles.disk.manifest.v2.valid] entries=3
[sexfiles.disk.manifest.v2.upgrade] from_version=1 entries=3
[sexfiles.disk.manifest.v2.bootstrap] entries=3
[sexfiles.disk.manifest.v2.ok] entries=3
[sexfiles.disk.manifest.v2.err] reason=...
[sexfiles.bridge.diskfs.select.v1_single_client]
[sexfiles.bridge.diskfs.select.ok] path_id=N
[sexfiles.bridge.diskfs.select.err] path_id=N code=...
[sexfiles.disk.multi.linen.write.ok] size=128
[sexfiles.disk.multi.linen.match] ok=1
[sexfiles.disk.multi.quil.write.ok] size=128
[sexfiles.disk.multi.quil.match] ok=1
[sexfiles.disk.multi.proof_intact] first_byte=...
[sexfiles.disk.multi.select.neg] path_id=N err=...
[sexfiles.disk.multi.summary] ok=1
```

## Opcode Collision Verified
- 0x3D = OP_RAMFS_READNAME (occupied, unchanged)
- 0x3E = OP_DISKFS_SELECT (new)
- 0x38–0x3C = existing DiskFS bridge opcodes (unchanged, now multi-object aware)

## Build
```
./scripts/entrypoint_build.sh → success
```

## Next Phase
`LINEN_DISKFS_SLOT_OBJECT_PROOF_V1` — Linen calls SELECT(path_id=1) via SLOT_STORAGE, writes object payload, reads back, verifies match.
