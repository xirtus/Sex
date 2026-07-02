# DISKFS_SUPERBLOCK_OBJECT_TABLE_V1

## Backend Route Used
**Mock scaffold (in-memory) over sexfiles backend boundary.**

Reason:
- `servers/sexfiles/src/backends/diskfs.rs` had only stub `ERR_NOT_FOUND` paths.
- No safe, implemented sexfiles→sexdrive block read/write route is currently wired.
- Implementing true persistent DiskFS would require a defined block contract integration path (and likely additional kernel/syscall plumbing review), which is out of scope for this bounded V1 task.

So V1 here is a deterministic **on-disk-format-aligned in-memory scaffold** for superblock + object table only.

## Files Changed
- `servers/sexfiles/src/backends/diskfs.rs`
- `servers/sexfiles/src/proof.rs`
- `servers/sexfiles/src/trampoline.rs`
- `docs/handoff/DISKFS_SUPERBLOCK_OBJECT_TABLE_V1.md`
- `docs/handoff/MASTER_RUNTIME_GATE_V1.md` (auto-updated by runtime gate)

## Object Table Shape (V1 Scaffold)
In `diskfs.rs`:

- `DISKFS_BLOCK_SIZE = 4096`
- `DISKFS_MAX_OBJECTS = 16`
- `SexfilesSuperblock` fields:
  - `magic`
  - `version_major`, `version_minor`
  - `block_size`
  - `fs_generation`
  - `object_table_start_block`
  - `object_table_entry_count`
  - `feature_flags`
  - `checksum`
- `SexfilesObjectEntry` fields:
  - `object_id`
  - `kind`
  - `owner_pd`
  - `rights_generation`
  - `object_size_bytes`
  - `first_block`
  - `metadata_generation`
  - `checksum`
  - `in_use`

Deterministic operations implemented:
1. `format_init_empty()`
2. `mount()` (superblock validation)
3. `create_object_entry(kind, owner_pd)`
4. `stat_object_entry(object_id)`
5. invalid object id rejection
6. table-full rejection

Checksums:
- Deterministic no_std XOR-fold checksum for scaffold fields (superblock and object entries), matching the format-lock “simple V1 checksum with upgrade path” direction.

## Proof Gate
- `SEXOS_DISKFS_OBJECT_TABLE_PROOF=1`

## Required Markers (Observed)
From runtime serial:
- `[diskfs.proof.format]`
- `[diskfs.proof.mount]`
- `[diskfs.proof.create_object]`
- `[diskfs.proof.stat_object]`
- `[diskfs.proof.invalid_object]`
- `[diskfs.proof.table_full]`

## Build / Runtime Result
- `cargo check --target sex-src/targets/x86_64-unknown-sexos.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -p sexfiles` -> PASS
- `./scripts/entrypoint_build.sh` -> PASS
- `SEXOS_DISKFS_OBJECT_TABLE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log` -> PASS (`GREEN_MASTER`)

## Exact Blockers Before True Persistence
1. **No sexfiles->sexdrive block I/O contract wired in DiskFS backend**
   - DiskFS backend currently has no real block read/write path.
2. **No mounted block device mapping for DiskFS region ownership**
   - Need explicit safe region/partition contract before persistent writes.
3. **No journal/replay path implemented yet**
   - Intentionally out of scope in this prompt; required for crash-safe persistence claims.
4. **No on-disk commit proof yet**
   - Current scaffold is in-memory only; no persistence guarantee is claimed.

## Scope Guard Compliance
- No sex-pdx ABI edits.
- No kernel ABI changes were introduced for this prompt.
- No POSIX directory/path semantics introduced.
- No snapshot/journal scope creep.
