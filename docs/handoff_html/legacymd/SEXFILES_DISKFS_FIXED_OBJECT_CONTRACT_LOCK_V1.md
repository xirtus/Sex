# SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1

Date: 2026-05-25
Mission: `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1`
Status: Contract lock complete (audit + freeze)

## A) Scope (Locked)
- Fixed-object bridge only (`Linen -> SLOT_STORAGE -> SexFiles -> DiskFS -> SLOT_BLOCK -> SexDrive -> NVMe`).
- No general filesystem semantics.
- No POSIX semantics.
- No dynamic path IPC.
- No delete/rename/directories.
- No crash-consistency, journaling-completeness, or power-loss durability claim.
- No true FLUSH/FUA durability claim without explicit proof lane evidence.

## B) Fixed Object Identity
Current code supports path_id-selected fixed objects with V2 manifest slots:
- `path_id=0` -> `/disk/sexfiles-proof-v1`
- `path_id=1` -> `/disk/linen-object-v1`
- `path_id=2` -> `/disk/quil-object-v1`

For this contract lock, canonical fixed object identity remains:
- `/disk/sexfiles-proof-v1`

Object size contract (current):
- `4096` bytes

## C) Opcode Contract (Current, Implemented)
Source of truth: `servers/sexfiles/src/messages.rs` and routing in `servers/sexfiles/src/vfs.rs`.

- `OP_DISKFS_WRITE         = 0x38`
- `OP_DISKFS_READ          = 0x39`
- `OP_DISKFS_FLUSH         = 0x3A`
- `OP_DISKFS_STAT          = 0x3B`
- `OP_DISKFS_MANIFEST_HASH = 0x3C`
- `OP_DISKFS_SELECT        = 0x3E`

`0x3D` is already assigned to `OP_RAMFS_READNAME`, so `0x3E` is used for select.

## D) Payload Contract (Locked to Current Code)
Transport shape:
- PDX request carries `type_id,arg0,arg1,arg2`.
- PDX reply carries one `u64` value (`pdx_reply(caller_pd, value)`).

Read:
- `OP_DISKFS_READ(0x39)`
- args: `arg0=byte_offset`, `arg1=max_len`, `arg2=0`
- max_len accepted: `1..=8`
- rejects: `max_len=0`, `max_len>8`, `offset>=4096`, `offset+len>4096`
- reply width: one `u64`, little-endian packed bytes (`<=8` bytes)

Write:
- `OP_DISKFS_WRITE(0x38)`
- args: `arg0=byte_offset`, `arg1=data_lo`, `arg2=data_hi`
- write width: `16` bytes payload carried via two `u64` words
- boundary policy in current V1 code: reject if full 16-byte write would cross object boundary
- rejects: `offset>=4096` or `offset > 4096-16`
- no silent truncation

Stat:
- `OP_DISKFS_STAT(0x3B)` returns packed `{flags:u32,size:u32}` in one `u64`

Manifest hash:
- `OP_DISKFS_MANIFEST_HASH(0x3C)` returns FNV-1a 64-bit path hash

## E) Flush/Fsync Truth (Locked)
- `OP_DISKFS_FLUSH(0x3A)` routes to `DiskFs::diskfs_fsync()` and returns status.
- Marker truth:
  - success -> `[sexfiles.bridge.diskfs.flush.ok]`
  - failure -> `[sexfiles.bridge.diskfs.flush.err] status=... honest=flush_not_emulated_by_qemu_nvme`
- No power-loss durability claim from this marker alone.
- `fsync` here is bridge-level/block-sync signaling, not POSIX durability equivalence.

## F) Linen Boundary (Locked)
- Linen uses `SLOT_STORAGE` only for this bridge lane (`SLOT_STORAGE=1`).
- No direct `SLOT_BLOCK` usage by Linen in this contract.
- No direct SexDrive calls from Linen in this contract.
- SexFiles owns MemLend grant path to block service (`SLOT_BLOCK`, `SLOT_BUF_LEND`).

## G) Required Next Proof Markers (Strict Next Phase)
- `sexfiles.bridge.diskfs.recv`
- `sexfiles.bridge.diskfs.buf.ready` (or reuse marker)
- `sexfiles.bridge.diskfs.write.ok`
- `sexfiles.bridge.diskfs.read.ok`
- `sexfiles.bridge.diskfs.flush.err` or `sexfiles.bridge.diskfs.flush.ok`
- `sexfiles.bridge.diskfs.stat.ok`
- `sexfiles.bridge.diskfs.manifest_hash.ok`
- `linen.diskfs.direct.begin`
- `linen.diskfs.direct.write.ok`
- `linen.diskfs.direct.read.match`
- `linen.diskfs.direct.done`

## H) Gate Proposal (This Phase: Docs Lock Only)
Proposed future gate name:
- `sexfiles_diskfs_fixed_object_contract`

Policy:
- PASS: explicit runtime marker confirms contract lock in run context.
- SKIP: contract lock marker not requested/enabled in run.
- FAIL: contradiction marker (opcode collision / overclaim / boundary mismatch).

This phase intentionally does not add a new runtime gate to avoid false fail pressure on default daily profile.

## I) Discovered Safety Facts (Audit)
- Opcode collision in `0x38..0x3C`: none found in current bridge namespace.
- `0x3D` already used (`OP_RAMFS_READNAME`), `SELECT` at `0x3E` is deliberate and active.
- PDX reply width is one `u64`; therefore read contract must remain `<=8 bytes/reply`.
- Write transport supports 16 bytes/call via `arg1+arg2` and is already implemented.
- Slot contract is explicit: `SLOT_STORAGE=1`, `SLOT_BLOCK=15`, `SLOT_BUF_LEND=17`.

## J) Non-Claims (Locked)
- Not a general filesystem.
- No directories/delete/rename/dynamic paths.
- No POSIX compliance claim.
- No crash consistency or power-loss durability claim.
- No journaling completeness claim for this bridge lock phase.
