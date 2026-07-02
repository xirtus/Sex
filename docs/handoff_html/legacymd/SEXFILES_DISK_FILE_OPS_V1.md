# SEXFILES_DISK_FILE_OPS_V1

- date: 2026-05-07
- status: IMPLEMENTED / RUNTIME PROOF PASSED
- gate_env: SEXOS_SEXFILES_REAL_BLOCK_PROOF=1
- files_changed:
  - servers/sexfiles/src/backends/diskfs.rs
  - servers/sexfiles/src/proof.rs

## Summary

Added minimal file-like operations over the single fixed manifest entry
`/disk/sexfiles-proof-v1`. These wrappers resolve a named path to an
LBA range via the V1 disk manifest, then provide byte-range read/write
within the bounded object.

## What Was Added

### diskfs.rs — Three file-like helpers inside `impl DiskFs`

1. `diskfs_lookup_path(path: &[u8]) -> Result<DiskManifestEntryV1, u64>`
   - Hashes the path argument with FNV-1a 64-bit
   - Reads the manifest sector from LBA 2046
   - Parses the single manifest entry
   - Verifies the path hash matches the expected entry
   - Returns the entry or ERR_NOT_FOUND

2. `diskfs_write_object(path: &[u8], offset: u64, data: &[u8]) -> Result<u64, u64>`
   - Resolves path via lookup
   - Bounds-checks offset+len against the entry's len_bytes (4096)
   - Rejects LBA 2047 collision
   - Performs read-modify-write per affected sector
   - Returns bytes written

3. `diskfs_read_object(path: &[u8], offset: u64, out: &mut [u8]) -> Result<u64, u64>`
   - Resolves path via lookup
   - Bounds-checks offset+len
   - Reads spanning sectors as needed
   - Returns bytes read

### proof.rs — `run_sexfiles_disk_file_ops_proofs()`

Called at the end of `run_sexfiles_real_block_proofs`.

Proof sequence:
1. Lookup known path → success
2. Lookup unknown path → ERR_NOT_FOUND
3. Write deterministic 4096-byte payload at offset 0
4. Read full object back → verify byte-for-byte match
5. Partial read at offset 128, len 512 (cross-sector) → verify match
6. Write past end (offset=4097) → ERR_OVERFLOW
7. Read at end (offset=4096) → ERR_OVERFLOW
8. Read last byte (offset=4095, len=1) → success
9. Verify manifest still parsable after file ops
10. Verify LBA 2047 persistence not collided
11. Confirm negative test integrity

## Markers Emitted

- `[sexfiles.disk.file.lookup.ok]` — path resolved
- `[sexfiles.disk.file.lookup.err]` — path rejected
- `[sexfiles.disk.file.write.begin]` — write starting
- `[sexfiles.disk.file.write.ok]` — write completed
- `[sexfiles.disk.file.read.begin]` — read starting
- `[sexfiles.disk.file.read.ok]` — read completed
- `[sexfiles.disk.file.match]` — full payload match result
- `[sexfiles.disk.file.partial.match]` — partial read match result
- `[sexfiles.disk.file.bounds.err]` — out-of-bounds rejection
- `[sexfiles.disk.file.lookup.negative]` — unknown path rejection
- `[sexfiles.disk.file.bounds.negative]` — bounds rejection
- `[sexfiles.disk.file.read.last_byte]` — last-byte boundary read
- `[sexfiles.disk.manifest.proof.still_ok]` — manifest integrity preserved
- `[sexfiles.disk.persistence.proof.still_ok]` — LBA 2047 not collided
- `[sexfiles.storage.negative.still_pass]` — negative contract intact

## What Was NOT Added

- No directory tree
- No rename/delete
- No dynamic allocation
- No generic allocator
- No journaling or caching
- No POSIX semantics
- No VFS integration (FsBackend trait still returns ERR_NOT_FOUND for DiskFs)
- No cross-PD pointers
- No shared memory redesign
- No ABI or kernel changes

## Success Criteria

- [x] Build passes with SEXOS_SEXFILES_REAL_BLOCK_PROOF=1
- [x] lookup /disk/sexfiles-proof-v1 succeeds
- [x] write/read full 4096-byte payload matches
- [x] partial read (offset=128, len=512) matches
- [x] unknown path rejected
- [x] out-of-bounds rejected
- [x] manifest proof still intact
- [x] LBA 2047 persistence proof still passes
- [x] no #PF/#GP/panic (0 fault hits)
- [x] Compiled (0 errors, 0 new warnings)
- [x] All 18 file ops markers: PASS
- [x] Negative tests: still pass

## Next Prompt

SEXFILES_DISK_FSYNC_FLUSH_V1 — add fsync/flush semantics to DiskFS file ops
