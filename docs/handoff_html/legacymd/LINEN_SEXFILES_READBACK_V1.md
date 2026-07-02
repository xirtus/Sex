# LINEN_SEXFILES_READBACK_V1

Date: 2026-05-07
Status: LANDED
Requires: SEXFILES_RAMFS_READNAME_V1, LINEN_SEXFILES_LIST_V1

## Files Changed

- `servers/linen/src/main.rs` — 3 edits

No sexfiles changes. No kernel changes. No sex-pdx changes. No silk-shell changes.

## Diff Summary

1. Added `const OP_RAMFS_READNAME: u64 = 0x3D;` (local to Linen, after DiskFS constants)
2. Added `unsafe fn linen_readback_verify(object_id: u64)` between `linen_init_session` and Synthetic Proof section
3. In `linen_init_session` — after each successful persist: added `[linen.sexfiles.readback.begin]` marker and `linen_readback_verify(id)` call

## Handle Source

`linen_persist_object()` returns `Ok((handle, sfid))` but calls `OP_RAMFS_CLOSE` before returning. Handle is closed.

Readback reopens by meta-name: `make_linen_meta_name(object_id)` → "lo.{object_id:016x}" (19 bytes). Uses `OP_RAMFS_OPEN` (flags=0, open-existing) to get a fresh handle. File data persists across close per RamFS contract.

## Exact Readback Protocol

```
OP_RAMFS_OPEN   (0x30): arg0=name[0..7] LE, arg1=name[8..15] LE, arg2=name[16..18] LE | (flags=0 << 24)
OP_RAMFS_READNAME (0x3D): arg0=handle, arg1=byte_offset, arg2=max_len
  chunk 0: off=0,  max_len=8 → bytes 0..7  "lo.00000"
  chunk 1: off=8,  max_len=8 → bytes 8..15 "0000000N"  (N = object_id hex)
  chunk 2: off=16, max_len=3 → bytes 16..18 last 3 hex chars (or 0..2 depending on id)
  EOF: Ok(0) when off >= 19
OP_RAMFS_CLOSE  (0x33): arg0=handle
```

Compare `buf[0..19]` against `make_linen_meta_name(object_id)[0..19]`.

## All 5 Names Round-Tripped

Meta-names for each boot object:
| id | meta-name (19 bytes) |
|----|----------------------|
| 1  | `lo.0000000000000001` |
| 2  | `lo.0000000000000002` |
| 3  | `lo.0000000000000003` |
| 4  | `lo.0000000000000004` |
| 5  | `lo.0000000000000005` |

All are deterministic since SESSION.create assigns IDs monotonically starting from 1.

## Proof Markers

Per object:
```
[linen.sexfiles.init.object] id=N kind=K handle=H sfid=S
[linen.sexfiles.readback.begin] id=N
[sexfiles.ramfs.readname.ok] handle=H off=0 len=8
[sexfiles.ramfs.readname.ok] handle=H off=8 len=8
[sexfiles.ramfs.readname.ok] handle=H off=16 len=3
[linen.sexfiles.readback.ok] id=N len=19
```

On open failure (sexfiles not ready):
```
[linen.sexfiles.readback.err] id=N err=-3 stage=open
```

On read failure:
```
[linen.sexfiles.readback.err] id=N err=E stage=readname off=O
```

On name mismatch:
```
[linen.sexfiles.readback.err] id=N err=name_mismatch stage=compare
```

Complete boot sequence:
```
[linen.sexfiles.list.begin]
[linen.sexfiles.init.object] id=1 ...
[linen.sexfiles.readback.begin] id=1
[sexfiles.ramfs.readname.ok] handle=H off=0 len=8
[sexfiles.ramfs.readname.ok] handle=H off=8 len=8
[sexfiles.ramfs.readname.ok] handle=H off=16 len=3
[linen.sexfiles.readback.ok] id=1 len=19
... (×5)
[linen.sexfiles.list.ok] count=5
```

## Remaining Blockers

### RamFS volatile
RamFS is in-memory. All files lost at power-off. Cross-boot persistence requires DiskFS
general directory support or RamFS snapshot-to-disk. Not claimed in this phase.

### Shell cannot see Linen SESSION rows
SESSION.list(caller_pd=shell_pd) owner filter returns 0 results for Linen-owned objects
(owner_pd=7). Preserved intentionally. Shell still renders its own 6 LINEN_SEED_OBJECTS.

## Next Phase Recommendation

**LINEN_PUBLIC_VIEW_SNAPSHOT_V1** — not WM bypass.

Preferred approach: Linen PD explicitly exports a bounded public snapshot of its object
table (e.g., via a new OP_LINEN_GET_PUBLIC_SNAPSHOT opcode that returns the display-safe
subset). Shell calls this opcode on Focus200 to populate a separate `LINEN_REMOTE_OBJECTS`
array and renders from that instead of the seed table.

This preserves:
- SESSION.list owner filter (private objects stay private)
- Linen as sole authority over what is "public"
- Shell as sole painter of surface 200
- No weakening of sexfiles auth model

The snapshot is explicitly opt-in from Linen's side — Linen decides what the shell sees.

Alternative paths (larger scope):
- RAMFS_READDIR_NAMES_V1 — add opcode to enumerate all visible handles with names (broader sexfiles change)
- Persistent backend after sexdrive proof — DiskFS general FS (large scope)
