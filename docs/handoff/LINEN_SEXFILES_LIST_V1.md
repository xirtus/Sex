# LINEN_SEXFILES_LIST_V1

Date: 2026-05-07
Status: LANDED (write path proven; true read-from-existing-fs blocked — see below)
Requires: LINEN_VIEWMODEL_BRIDGE_V1

## Files Changed

- `servers/linen/src/main.rs` — 3 edits

No sexfiles changes. No kernel changes. No sex-pdx changes. No silk-shell changes.

## What Changed

### 1. Constant

```rust
const LINEN_OWN_PD: u32 = 7; // deterministic per init.rs spawn order (domain 7)
```

### 2. New function: `linen_init_session()`

Creates 5 fixed well-known objects in SESSION and persists each to SexFiles RamFS.
Called unconditionally at boot, before env-var-gated proofs and before event loop.

```rust
unsafe fn linen_init_session() { ... }
```

Objects created:
| id | name           | kind     |
|----|----------------|----------|
| 1  | SexOS Kernel   | Document |
| 2  | Silk Shell     | Document |
| 3  | SexDisplay     | Document |
| 4  | Sessions       | Session  |
| 5  | SexFiles Root  | Document |

Each object:
- Owned by Linen PD (owner_pd = LINEN_OWN_PD = 7)
- Persisted to RamFS via `linen_persist_object()` → OP_RAMFS_CREATE_OWNER + OP_RAMFS_WRITE
- On success: SESSION.set_persisted(id, handle) + SESSION.set_sexfiles_object_id(id, sfid)
- On persist failure: local-only, SESSION still populated, warns with `local_only=true`

### 3. Boot call

```rust
// After [linen.ready], before env-var proofs
unsafe { linen_init_session(); }
```

## Exact SexFiles Protocol Used

| Step | Opcode | Args | Returns |
|------|--------|------|---------|
| Create file | OP_RAMFS_CREATE_OWNER (0x36) | n0=name[0..7], n1=name[8..15], arg2=name[16..23]|owner_pd<<32 | handle |
| Get global ID | OP_RAMFS_OBJECT_ID (0x37) | handle | sfid (≥1) |
| Write metadata | OP_RAMFS_WRITE (0x32) ×6 | handle, offset, 8-byte chunk | bytes_written |
| Close | OP_RAMFS_CLOSE (0x33) | handle | 0 |

Transport: `pdx_storage_sync()` — sends via pdx_call(SLOT_STORAGE), spins for reply via pdx_listen_raw(0x1). Handles OP_HID_EVENT inline while spinning.

## Slot/Cap

`SLOT_STORAGE = 1` → sexfiles VFS. Granted to Linen PD at kernel/src/init.rs:177.

## Object Ownership

All Linen SESSION objects: owner_pd = LINEN_OWN_PD (7).

Shell's SESSION.list(caller_pd=shell_pd) owner filter returns 0 results for Linen objects.
Shell still shows its own 6 LINEN_SEED_OBJECTS (shell-local, display_name: &'static str).
No visual change in shell's Linen surface.

**WHY** SESSION.list owner filter was preserved: it enforces a clean per-PD ownership
boundary. Weakening it for shell-reads would allow any PD with the SLOT_LINEN cap to
enumerate all session objects, bypassing Linen's per-caller access model.

## True vs Honest Fallback

This IS a real sexfiles-backed data path:
- SESSION objects have valid ramfs_handle and sexfiles_object_id
- RamFS files exist during the session with Linen's metadata written to them
- The write path is proven: Linen SESSION → OP_RAMFS_CREATE_OWNER → RamFS

This is NOT a "read from existing FS at boot":
- RamFS is in-memory (Vec<u8>) — empty at each fresh boot
- No cross-boot persistence from this path

## Proof Markers

Boot serial:
```
[linen.ready]
[linen.sexfiles.list.begin]
[linen.sexfiles.init.object] id=1 kind=0 handle=<H> sfid=<S>
[linen.sexfiles.init.object] id=2 kind=0 handle=<H> sfid=<S>
[linen.sexfiles.init.object] id=3 kind=0 handle=<H> sfid=<S>
[linen.sexfiles.init.object] id=4 kind=1 handle=<H> sfid=<S>
[linen.sexfiles.init.object] id=5 kind=0 handle=<H> sfid=<S>
[linen.sexfiles.list.ok] count=5
```

If sexfiles not ready (timing gap before event loop):
```
[linen.sexfiles.init.warn] id=N persist_err=<E> local_only=true
[linen.sexfiles.list.ok] count=5   (SESSION still populated)
```

## STOP FIRST Blockers for True FS-Backed Read

Two primitives missing before Linen can READ existing data from sexfiles:

### Blocker 1: No readdir-with-names

| Opcode | Returns | Missing |
|--------|---------|---------|
| OP_RAMFS_LIST (0x34) | {handle: u32, size: u32} per entry | Name bytes |
| OP_RAMFS_STAT (0x35) | {size: u32, name_len: u32} | Name bytes |

No opcode returns filename bytes given a handle. Adding `OP_RAMFS_READNAME(handle, byte_offset)
→ 8 bytes of filename` would close this gap (~20 lines in sexfiles/src/backends/ramfs.rs +
messages.rs). Requires sexfiles source change.

### Blocker 2: No cross-boot persistence

RamFS is in-memory. At each fresh boot, RamFS is empty. "Populate from existing sexfiles" is
vacuous — nothing exists to read. Fix options:
- DiskFS general directory support (large scope)
- RamFS snapshot → disk at shutdown (medium scope, requires QEMU disk writes at exit)
- Pre-seeded sexfiles content in ISO (build-time injection — simplest for read-at-boot)

## Remaining Gap to OpenIntent / Project Navigation

1. Shell can't see Linen SESSION objects (owner filter — requires WM bypass in SESSION.list)
2. Cross-boot object persistence requires disk-backed store (RamFS is volatile)
3. Linen SESSION kinds (Document, Session, Unknown) don't map 1:1 to shell's
   richer LinenObjectKind enum (Project, CodeFile, etc.) — kind translation needed
4. Object names from Linen SESSION (8-char limit via OP_LINEN_GET_OBJECT) vs
   display names in shell (full &'static str) — schema reconciliation needed
5. No app launch intent (OpenIntent) wired yet — out of scope
