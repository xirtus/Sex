# DISKFS_V4 Lane 3, slice 2 — Quil migrated to OP_DISKFS_READ_V2

## Status: COMPLETE

## Caller audit

`quil_persist_load` (`servers/quil/src/main.rs`) is the single unified
content-restore path — confirmed by tracing every call site, not assumed:

- Palette `CMD_LOAD_DOCUMENT` (`Load` menu action).
- Linen's open-disk-document IPC intent (`[quil.open.disk_doc.recv]`
  message handler) — covers Linen-initiated opens and reopens.

There is no separate "startup restore" or "recovery after service
restart" call site — quil does not auto-load on boot; both real paths
above already funnel through the same function. "Document reload
following save" isn't a distinct path either — save and load are separate
user actions, not chained.

The other real read call site was `run_quil_diskfs_slot_min_proof`, a
boot-time connectivity proof (fixed ASCII payload, not real document
content) — migrated too, for the same reason the audit called out
test-only paths: no diskfs read anywhere in quil should still use the
ambiguous encoding once a fix exists.

`OP_DISKFS_READ`'s constant was removed from `servers/quil/src/main.rs`
entirely — zero remaining callers after migration.

## What changed

- `quil_persist_load`'s header read (8 bytes) and content read loop now
  use `OP_DISKFS_READ_V2` via a new `read_v2_chunk()` helper that decodes
  the packed status/length/payload reply.
- **Real bug fixed along the way**: the content read loop used to write
  decoded bytes directly into `QUIL_BUFFER` as it went. A failure partway
  through left the buffer holding a mix of old and partially-read new
  content, while `QUIL_BUFFER_LEN`/`QUIL_DIRTY` still described the
  pre-load state — inconsistent with what was actually sitting in the
  array. Fixed by staging into a new `QUIL_LOAD_STAGING` buffer and only
  copying into `QUIL_BUFFER` on complete success; every failure path
  returns before `QUIL_BUFFER` is touched at all.
- `run_quil_diskfs_slot_min_proof`'s read loop migrated the same way
  (3 calls of 6+6+4 instead of 2×8).

## Regression proof

New `run_quil_read_v2_highbit_proof()` (gated behind
`SEXOS_QUIL_READ_V2_HIGHBIT_PROOF`, matching the existing proof-routine
convention in this file), exercised by `scripts/quil_read_v2_gate.sh`:

- Creates a real disk object, saves a pattern containing 0x00, 0x7f, 0x80,
  and 0xff spliced into otherwise-printable text
  (`HIGHBIT-PROOF-A-\x00-B-\x7f-C-\x80-D-\xff-END`) via the real
  `quil_persist_save`/`quil_persist_load` path (not a synthetic
  bit-manipulation test), overwrites the buffer with a sentinel first so a
  match proves the load actually happened, then verifies each of the four
  bytes individually AND the whole buffer via FNV-1a hash.
- Negative path: marks the buffer dirty, forces a load failure (invalid
  `path_id`), and confirms `quil_persist_load` returns `Err`, the buffer
  content/length/hash are byte-for-byte unchanged from before the failed
  attempt, and dirty state is still `true` — proving the staging-buffer
  fix actually holds under a real failure, not just reasoned about.

Result (`scripts/quil_read_v2_gate.sh`, all PASS): `fault_free`,
`byte_0x00_exact`, `byte_0x7f_exact`, `byte_0x80_exact`, `byte_0xff_exact`,
`exact_transport_and_hash`, `failed_reload_preserves_state`,
`no_spurious_read_errors`.

## Not done this slice

Slice 3 (canonical shared helpers) is separate — this slice's
`read_v2_chunk` still does its own inline bit decode; migrating it to the
new `sex_pdx::diskfs_v2_*` helpers happens in the Slice 3 commit.
