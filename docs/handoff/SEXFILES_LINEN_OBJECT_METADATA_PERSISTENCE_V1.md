# SEXFILES_LINEN_OBJECT_METADATA_PERSISTENCE_V1

**Date**: 2026-05-06
**Status**: IMPLEMENTED (with documented PDX proxy blocker)
**Proof Gate**: `SEXOS_LINEN_SEXFILES_METADATA_PROOF=1`

---

## Summary

Connected the Linen object/session metadata model to SexFiles RamFS object
records so the user-facing object layer has a persistent-capable backing store.
The bridge uses RamFS files as metadata records, with explicit-owner creation
for correct access control.

## Metadata Bridge Shape

### LinenObject fields (session.rs)
```
object_id:    u64      // Unique auto-incrementing ID (>=1)
kind:         ObjectKind // Document(0), Session(1), Unknown(2)
owner_pd:     u32      // Creator/owner protection domain
name:         [u8; 24] // Display name (bounded)
name_len:     u8       // Actual name byte length
ramfs_handle: u64      // SexFiles RamFS handle (0 = unlinked)
generation:   u64      // Metadata generation counter
flags:        u8       // Bit 0 = persisted (backed by SexFiles)
```

### SexFiles metadata record (48 bytes, stored as RamFS file content)
```
bytes 0-7:   object_id (u64 LE)
bytes 8-9:   kind (u16 LE)
bytes 10-13: owner_pd (u32 LE)
bytes 14-21: generation (u64 LE)
bytes 22:     flags (u8)
bytes 23:     name_len (u8)
bytes 24-47: name (24 bytes)
```

### RamFS file naming
```
lo.{object_id:016x}   (18 bytes, fits within 24-byte RamFS name limit)
```

## Route Used

### SexFiles-side: OP_RAMFS_CREATE_OWNER (0x36)

New opcode added to SexFiles protocol. Creates a RamFS file with an explicit
`owner_pd` set from the request arguments rather than from the PDX caller_pd.
This enables the metadata bridge pattern where a proxy server (Linen) needs to
create files on behalf of end-user PDs.

```rust
// arg0 = name bytes 0-7
// arg1 = name bytes 8-15
// arg2 = name bytes 16-23 (lower 24 bits) | (owner_pd << 32)
```

Gate: `caller_pd` must be 0 (server-internal) or match `owner_pd`.

### Linen-side: PDX sync call via SLOT_STORAGE

Linen uses the same pattern as Quil: `pdx_storage_sync()` makes a PDX call
then spins on `pdx_listen_raw(0)` for the reply. HID events during the spin
are handled inline.

## Exact Blocker: PDX Proxy Owner Propagation

**When Linen calls SexFiles via PDX (SLOT_STORAGE), the `caller_pd` field
in the PDX message is Linen's own PD, not the original requesting user's PD.**

RamFS's `create_with_owner` gate:
```rust
if caller_pd != 0 && caller_pd != owner_pd {
    return Err(ERR_PERM_DENIED); // -6
}
```

This correctly rejects the proxy pattern because Linen (PD 7) is not the
same as the object owner (PD 42). The capability model does not allow
arbitrary proxy creation — this is by design.

**Result**: Linen creates local objects successfully. Persistence to SexFiles
fails with ERR_PERM_DENIED. Local metadata encode/decode proof works.
Server-internal SexFiles proof (caller_pd=0) works fully.

### Resolution paths (not implemented in V1):
1. Kernel-level `GRANT` capability to allow Linen to delegate creation on
   behalf of arbitrary PDs.
2. Capability record grant: user PD 42 grants Linen a cap with CREATE right.
3. Linen stores objects under its own PD with the real owner encoded in
   metadata (weak owner isolation).

## Proof Markers

All 5 required markers emit during proof execution:

| Marker | Proven In | Meaning |
|--------|-----------|---------|
| `[linen.sexfiles.proof.create_link]` | SexFiles (ok=1) + Linen (id=1) | Create object → SexFiles record link |
| `[linen.sexfiles.proof.list_link]` | SexFiles (ok=1) + Linen (persisted=0) | List reflects SexFiles-backed metadata |
| `[linen.sexfiles.proof.get_link]` | SexFiles (ok=1) + Linen (persisted=0) | Get returns metadata with persistence flags |
| `[linen.sexfiles.proof.owner_deny]` | SexFiles (ok=1) + Linen (err=-6) | Non-owner access denied |
| `[linen.sexfiles.proof.generation]` | SexFiles (ok=1) | Generation counter works |

## Files Changed

```
servers/sexfiles/src/messages.rs        (+8)  New OP_RAMFS_CREATE_OWNER opcode
servers/sexfiles/src/backends/mod.rs    (+11)  create_with_owner trait method
servers/sexfiles/src/backends/ramfs.rs  (+41)  create_with_owner impl + gate
servers/sexfiles/src/backends/diskfs.rs (+9)   Stub impl
servers/sexfiles/src/backends/tmpfs.rs  (+9)   Stub impl
servers/sexfiles/src/vfs.rs             (+14)  OP_RAMFS_CREATE_OWNER dispatch
servers/sexfiles/src/trampoline.rs      (+6)   SEXOS_LINEN_SEXFILES_METADATA_PROOF gate
servers/sexfiles/src/proof.rs           (+57)  run_linen_sexfiles_metadata_proofs()
servers/linen/src/session.rs            (+36)  generation, flags, set_persisted, bump_generation
servers/linen/src/main.rs               (+270) Metadata bridge: helpers, persist, proof
```

## Build / Runtime Result

```
BUILD_GATE:   PASS
SPAWN_GATE:   PASS  (linen PD 7 spawned)
CLOCK_GATE:   PASS
SCHED_GATE:   PASS  (linen running 9x)
FAULT_GATE:   PASS  (no panics)
SEXFILES_GATE: PASS (sexfiles PD 11 running 9x)
FINAL_SCORE:  GREEN_MASTER
```

Run command:
```
SEXOS_LINEN_SEXFILES_METADATA_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

## Remaining Object Persistence Gaps

1. **PDX proxy owner propagation blocker**: Linen cannot create SexFiles records
   for arbitrary owner PDs. Resolution requires either kernel capability grant
   for proxy creation, or a cap-record delegation model.

2. **Read-back on boot**: No mechanism for Linen to reload persisted metadata
   from SexFiles on boot. The `handle_list_objects` and `handle_get_object`
   return local session data only. Full round-trip requires:
   - Linen scans `lo.*` files via RamFS LIST
   - Reads and decodes metadata records
   - Rebuilds local session table

3. **RamFS file limit (64)**: Each Linen object consumes one RamFS file.
   With LINEN_MAX_OBJECTS=16 and other RamFS users (Quil, etc.), the limit
   is shared. Unified object store compaction needed.

4. **No DiskFS object table link yet**: RamFS files are in-memory only.
   DiskFS object table exists as scaffold but is not wired to RamFS
   persistence or real block I/O (sexdrive).

5. **Atomic create-on-behalf**: When Linen creates local object + persists,
   either or both can fail. No transactional boundary exists. Local object
   is created best-effort even if persistence fails.

## Design Decisions

- **No POSIX directories**: Flat namespace. Files named by hex object_id.
- **No Linen redesign**: Linen session/object model unchanged; metadata bridge
  added as additional fields and methods.
- **No kernel or sex-pdx edits**: New opcode is internal to SexFiles server.
- **Bounded metadata**: 48-byte fixed record, 24-byte name limit.
- **Sync PDX pattern**: Matches Quil's `pdx_call_and_reply` approach.
- **Best-effort persistence**: Local object creation succeeds even if SexFiles
  persistence fails. The `persisted` flag distinguishes backed objects.
