# DISKFS_V4 Lane 3, slice 1 — OP_DISKFS_READ_V2 (sign-bit ambiguity fix)

## Status: COMPLETE for READ. WRITE audited, not ambiguous — no fix needed.

## The bug

`OP_DISKFS_READ` (0x39) packs up to 8 raw content bytes directly into the
single reply u64. Every client (`spindle_storage_sync`, quil's
`pdx_storage_call_bounded`) treats a reply with bit 63 set as a negative
`ERR_*` code. A full 8-byte read whose last byte (bit 63's byte) is >= 0x80
is indistinguishable from an error — a legitimate read gets rejected as if
the server had failed. This was already known and deliberately kept
reproducible: `apps/spindle/src/main.rs`'s `filldocx`/`catdocx` debug
commands and `scripts/diskfs_v4_read_signbit_regression_gate.sh` existed
specifically to keep this failing case alive as a Lane 3 starting point
(not written this session — found already in place).

`OP_DISKFS_WRITE`'s reply (bytes written, 0..16, or a small negative error)
was audited and is NOT ambiguous: a byte count can never collide with the
tiny negative `ERR_*` range. No fix needed there.

## Why not just widen the existing opcode

`pdx_reply()` (`crates/sex-pdx/src/lib.rs`) carries exactly one u64 from
server to client — there's no second field at the transport layer to
split "status" from "payload" without also changing the kernel IPC relay
(a materially bigger, cross-subsystem change). The fix instead uses a
canonical bit layout within that single u64, and — per instruction — adds
a NEW opcode rather than quietly changing `OP_DISKFS_READ`'s semantics
out from under existing callers, and migrates clients to it deliberately.

## Design

`OP_DISKFS_READ_V2` (0x4A), documented in full in
`servers/sexfiles/src/messages.rs`:

```
bits 63..56 (top byte)   status: 0x00 OK, 0x01 EOF, 0xFF error-follows
bits 55..48              bytes_read (0..=6), valid only when status==0x00
bits 47..0  (low 6 bytes) payload data, LE; when status==0xFF, this field
                          instead holds the ERR_* magnitude (i.e. -e)
```

Status and payload never share bits — a data byte >= 0x80 anywhere in the
payload can never set the status byte, by construction, independent of
content. Max payload dropped from 8 to 6 bytes/call to make room for the
header; that's the deliberate trade for a hard correctness guarantee
without touching the kernel.

EOF is explicit (status 0x01, offset == size) rather than folded into the
error path — matches the "explicit EOF semantics" requirement.

Per-caller state: already correct before this slice.
`DISKFS_SELECTED_PATH_ID` is indexed by `caller_pd % DISKFS_CLIENT_SLOTS`
(32 slots) — `SEXFILES_DEFER_V1`, an earlier fix, already replaced a single
global selection with this. No work needed here; confirmed by reading
`servers/sexfiles/src/vfs.rs` rather than assumed.

`buf_va` (the shared scratch buffer `handle_diskfs_read`/`write` use to
talk to the disk backend) is private to SexFiles and never shared with
client PDs ("SexFiles owns this buffer for its lifetime; Linen never sees
it" — existing comment) — ruled out as a payload channel for clients. This
is why the fix works within the existing single-u64 reply instead of
trying to hand clients a bigger shared-memory window.

## Server implementation

`handle_diskfs_read_v2` (`servers/sexfiles/src/vfs.rs`, next to the
existing `handle_diskfs_read`), wired at `OP_DISKFS_READ_V2` in
`handle_vfs_message`'s dispatch table. All error paths (bad want_len,
grant failure, `v4_ensure` failure, unknown path_id, past-end read, disk
read failure) route through a `pack_err` closure that recovers the
`ERR_*` magnitude from the existing `u64`-encoded-negative-i64 convention
used everywhere else in this file, so every error still maps back to the
same constants (`ERR_OVERFLOW`, `ERR_NOT_FOUND`, etc.) — no new error
vocabulary.

## Client migration

`apps/spindle/src/main.rs`'s `catdocx` (the exact command the pre-existing
regression gate exercises) rewritten to call `OP_DISKFS_READ_V2` and decode
the packed reply instead of `OP_DISKFS_READ`. Deliberately kept as the
same command name and the same 32-byte deterministic test pattern
(`filldocx` unchanged) so the regression gate is a direct, unmodified
before/after proof rather than a new test needing its own validation.

`spindle_storage_sync`'s existing `if v < 0 { Err }` convention needed no
changes: status 0x00/0x01 always leave bit 63 clear (top byte 0x00 or
0x01), status 0xFF always sets it — the existing sign-check is already
correct for this reply shape, by the same construction that makes payload
bytes never collide with status.

Quil's own DiskFS load path (`servers/quil/src/main.rs`, still on the
original `OP_DISKFS_READ`) was NOT migrated this slice — quil's content is
text, ASCII-only in practice, so it was never actually hitting this bug;
migrating it is straightforward (same pattern as spindle's) but out of
scope for this slice's proof, which only needed one real client to
demonstrate the fix against the known-reproducible case.

## Result

`scripts/diskfs_v4_read_signbit_regression_gate.sh` (pre-existing,
unmodified) flips from its documented "expected to fail right now" state
to:

```
ROW read_signbit_bug_absent PASS (protocol fixed — fold this into the general suite)
[diskfs.v4.read_signbit_regression.gate.result] PASS (bug fixed)
```

Byte 31 (`0x86`, bit 7 set) — the exact byte that used to land as the top
byte of the 4th 8-byte `OP_DISKFS_READ` reply and get rejected — now reads
back correctly through `OP_DISKFS_READ_V2`.

## Explicitly not done this slice (real remaining Lane 3 scope)

- Quil's client migration to `OP_DISKFS_READ_V2`.
- `OP_DISKFS_WRITE_V2` / any chunked-write opcode — audited as
  unnecessary for correctness (WRITE isn't ambiguous), but larger
  per-call transfers than 16 bytes would still be a real throughput win
  (observed ~11B/s application-level for the current 16-byte round-trip
  pattern) if this lane continues.
- Explicit length-query opcode distinct from `OP_DISKFS_STAT`.
- Interleaved multi-client transfer proof — per-caller SELECT state is
  already correct (see above), but no gate in this codebase yet exercises
  two concurrent clients reading/writing different objects at once to
  prove it live.
