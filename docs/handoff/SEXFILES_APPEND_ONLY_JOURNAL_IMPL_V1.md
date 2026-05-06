# SEXFILES_APPEND_ONLY_JOURNAL_IMPL_V1

## Purpose
Implement the smallest bounded append-only journal inside SexFiles `DiskFs` so object metadata updates are recorded through begin/append/commit transaction records.

## Journal Structure (Implemented)
- `DISKFS_JOURNAL_CAPACITY = 64` fixed records
- `JournalRecordKind`:
  - `TxBegin`
  - `ObjectMetadataUpdate`
  - `TxCommit`
- `JournalRecord` fields:
  - `kind`
  - `tx_id`
  - `generation`
  - `object_id`
  - `metadata_generation`
  - `payload_len`
  - `checksum`
- Per-record checksum validation with deterministic reject (`ERR_OVERFLOW`) on mismatch

## Wiring (Bounded Scope)
- `create_object_entry()` now journals:
  1. begin transaction
  2. append metadata update record
  3. append commit record
- `create_object_entry()` rejects `ERR_FULL` when journal is full
- `create_object_entry()` and `stat_object_entry()` call journal integrity verification; corrupted journal checksum rejects path with `ERR_OVERFLOW`
- No file data journaling
- No snapshot logic

## Files Changed
- `servers/sexfiles/src/backends/diskfs.rs`
- `servers/sexfiles/src/proof.rs`
- `servers/sexfiles/src/trampoline.rs`
- `docs/handoff/SEXFILES_APPEND_ONLY_JOURNAL_IMPL_V1.md`

## Proof Gate / Markers
Gate:
- `SEXOS_SEXFILES_JOURNAL_PROOF=1`

Markers:
- `[sexfiles.journal.proof.begin]`
- `[sexfiles.journal.proof.append]`
- `[sexfiles.journal.proof.commit]`
- `[sexfiles.journal.proof.full]`
- `[sexfiles.journal.proof.checksum_reject]`

Observed (runtime serial): all markers emitted with `ok=1`.

## Build / Runtime
- `cargo check --target sex-src/targets/x86_64-unknown-sexos.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -p sexfiles`: PASS
- `./scripts/entrypoint_build.sh`: PASS
- `SEXOS_SEXFILES_JOURNAL_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)

## Non-Goals (Kept)
- No kernel edits
- No `sex-pdx` ABI edits
- No app-visible journal/raw disk surface
- No POSIX semantics
- No broad DiskFs rewrite

## Remaining Replay Blockers
1. No journal replay engine yet (scan, committed-tx filter, apply order, corruption policy).
2. No persistent block-device write/read route (still bounded in-memory scaffold).
3. No checkpoint record integration with object-table generation advancement.
4. No committed/uncommitted recovery simulation across reboot.
5. No capability/revocation persistence records in journal yet.
