# SEXFILES_RAMFS_READNAME_V1

Date: 2026-05-07
Status: LANDED
Requires: LINEN_SEXFILES_LIST_V1

## Files Changed

- `servers/sexfiles/src/messages.rs` — add OP_RAMFS_READNAME constant
- `servers/sexfiles/src/backends/ramfs.rs` — add `readname()` method in new `impl RamFs` block
- `servers/sexfiles/src/vfs.rs` — add dispatch case

No kernel changes. No sex-pdx changes. No Linen changes. No silk-shell changes.

## Opcode

```
OP_RAMFS_READNAME: u64 = 0x3D
```

Next after existing RamFS range (0x30-0x37) and DiskFS range (0x38-0x3C). No gap.

## Input/Output Packing

| Field | Encoding |
|-------|----------|
| arg0 | handle (u64) — must be an active open file handle |
| arg1 | byte_offset (u64) — offset into filename, 0 = first byte |
| arg2 | max_len (u64) — server clamps to 8 |
| reply | up to 8 filename bytes, little-endian packed u64 |
| reply = 0 | EOF — byte_offset >= name_len (not an error) |
| reply < 0 | Error: -1 = invalid handle, -6 = permission denied |

Example: filename "SexOS Kernel" (12 bytes)
- Call: arg0=handle, arg1=0, arg2=8 → packed 8 bytes "SexOS Ke" LE
- Call: arg0=handle, arg1=8, arg2=8 → packed 4 bytes "rnel" LE (only 4 remain)
- Call: arg0=handle, arg1=12, arg2=8 → 0 (EOF)

## Auth Rule

Same as OP_RAMFS_STAT and OP_RAMFS_READ: `check_access(caps, caller_pd, entry, CAP_RIGHT_READ)`.

- `caller_pd == 0` (server-internal) → always allowed
- `caller_pd == entry.owner_pd` → allowed (fast path in check_access)
- cap grant with CAP_RIGHT_READ for subject_pd == caller_pd → allowed
- otherwise → ERR_PERM_DENIED (-6)

No new auth mechanism. No weakening of existing owner filter.

## Bounds

- `max_len` clamped to 8 (server-side, not trusted from caller)
- `byte_offset >= entry.name_len` → Ok(0), no read performed
- `take = max_len.min(8).min(name_len - byte_offset)` — never reads past name storage
- Name storage is `[u8; RAMFS_MAX_NAME]` (24 bytes max) — statically bounded
- No allocation in the read path

## Implementation

`readname` added to a standalone `impl RamFs {}` block (same pattern as `object_id_for_handle`).
NOT part of the `FsBackend` trait — called directly via `RAMFS.readname(...)` in dispatch.

## Dispatch Location

`servers/sexfiles/src/vfs.rs` — inserted between OP_RAMFS_STAT (0x35) and DiskFS block (0x38).

## Proof Markers

On successful read:
```
[sexfiles.ramfs.readname.ok] handle=<H> off=<O> len=<L>
```

On auth failure or invalid handle:
```
[sexfiles.ramfs.readname.deny] handle=<H> err=<E>
```

## Limitations

- Not readdir: does not enumerate all files in a directory
- Not general FS listing: caller must already have an open handle
- No persistence: RamFS is in-memory; handles lost on power cycle
- No partial-chunk concatenation: caller responsible for assembling chunks
- Negative reply (e.g., -6 for ERR_PERM_DENIED) is returned as `e as u64` —
  caller should treat high-bit-set values as errors (same as existing RamFS protocol)

## Next Prompt: LINEN_SEXFILES_READBACK_V1

After this lands, use the following prompt to prove Linen round-trips names through sexfiles:

```
MISSION: LINEN_SEXFILES_READBACK_V1

After linen_init_session() persists 5 objects to RamFS, read each name back via
OP_RAMFS_READNAME and verify byte-for-byte match against the known fixed name.

SCOPE: servers/linen/src/main.rs only + handoff doc.

FIRST: Add OP_RAMFS_READNAME = 0x3D constant to linen/src/main.rs (local, not sex-pdx).

IMPLEMENT:
- After linen_init_session() creates and persists each object:
  - Call OP_RAMFS_READNAME in 8-byte chunks to reconstruct the name
  - Compare against known expected name bytes
  - Log [linen.sexfiles.readback.ok] id=N name=... on match
  - Log [linen.sexfiles.readback.err] id=N reason=... on mismatch

BOUNDS: LINEN_MAX_NAME=24, 3 chunks max (offsets 0, 8, 16).

DO NOT:
- expose objects to shell
- weaken SESSION.list owner filter
- edit silk-shell or sexdisplay
- add new sex-pdx opcodes
```
