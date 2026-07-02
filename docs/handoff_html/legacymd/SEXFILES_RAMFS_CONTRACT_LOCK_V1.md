# SEXFILES_RAMFS_CONTRACT_LOCK_V1

## Status: **LOCKED** ✅

RamFS contract is fully locked. The sexfiles server now implements a bounded,
flat-namespace RAM-backed filesystem with full handle validation, deterministic
errors, and no POSIX semantics. A built-in proof suite (`SEXFILES_RAMFS_PROOF=1`)
validates all contract guarantees at startup.

## Precondition Audit

The precondition document `SEXFILES_RAMFS_CONTRACT_AUDIT_V1.md` was not found at
start of this task — it was never created. This handoff serves as both audit and
lock document.

### What was found during audit

The existing `sexfiles` server had:

| Component | Status | Issue |
|-----------|--------|-------|
| `Cargo.toml` | ❌ Not in workspace | Added to `Cargo.toml` members |
| `src/lib.rs` | ✅ Module layout ok | Removed `cache` module (broken deps) |
| `src/main.rs` | ⚠️ Entry point ok | Added fallback loop |
| `src/messages.rs` | ❌ Broken imports | `PageHandover`, `MessageType` not in sex-pdx |
| `src/pdx.rs` | ❌ Broken imports | `PdxRequest` not in sex-pdx |
| `src/vfs.rs` | ❌ Broken ring imports | `PdxReply` not available |
| `src/backends/mod.rs` | ⚠️ Trait ok | Changed `read`/`write` signatures |
| `src/backends/ramfs.rs` | ❌ Mock only | No real storage, no validation |
| `src/backends/diskfs.rs` | ❌ Broken API | Used nonexistent `MessageType::DmaCall` |
| `src/backends/tmpfs.rs` | ✅ Stub ok | Just returns errors |
| `src/trampoline.rs` | ❌ Compile errors | `AtomicRing<PdxReply>` type mismatch |
| `src/cache.rs` | ❌ Broken imports | `AtomicRing` not exposed in sex-pdx |
| `sex-pdx ring module` | ⚠️ Module exists but hidden | Not needed for new approach |

Root cause: The code was written for an older version of `sex-pdx` and `libsys` that
exposed different types and APIs. The newer `sex-pdx` architecture uses
`pdx_listen_raw(slot)` / `pdx_reply()` instead of ring buffers.

## Changes

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Added `servers/sexfiles` to workspace members |
| `servers/sexfiles/Cargo.toml` | Reduced deps: removed `linked_list_allocator`, `serde`, `bitflags`, `libsys` |
| `servers/sexfiles/src/main.rs` | Cleaned up; removed `cache`/`alloc_error_handler` |
| `servers/sexfiles/src/lib.rs` | Removed `cache` module; added `proof` module |
| `servers/sexfiles/src/messages.rs` | **Rewritten**: protocol constants, error codes, bounds |
| `servers/sexfiles/src/pdx.rs` | **Rewritten**: thin wrappers over `pdx_listen_raw`/`pdx_reply` |
| `servers/sexfiles/src/vfs.rs` | **Rewritten**: routes PDX messages to `FsBackend` trait |
| `servers/sexfiles/src/backends/mod.rs` | **Rewritten**: trait uses buffers, returns `(u64, u32)` for list |
| `servers/sexfiles/src/backends/ramfs.rs` | **Full implementation**: bounded storage, handle validation |
| `servers/sexfiles/src/backends/diskfs.rs` | **Rewritten**: stub (no disk backend yet) |
| `servers/sexfiles/src/backends/tmpfs.rs` | **Rewritten**: stub (no tmp backend yet) |
| `servers/sexfiles/src/trampoline.rs` | **Rewritten**: standard PDX message loop (matches sexstore pattern) |
| `servers/sexfiles/src/proof.rs` | **New**: built-in contract validation (7 proofs) |
| `servers/sexfiles/src/cache.rs` | **Removed**: broken, unnecessary for contract |

## Contract Operations (Locked)

### Internal API (`FsBackend` trait via `RamFs`)

| Operation | Signature | Validation |
|-----------|-----------|------------|
| `open` | `(name: &[u8], flags: u32, mode: u32) -> Result<u64, i64>` | Name length ≤ 24, name non-empty, max 64 files |
| `read` | `(handle: u64, offset: u64, buf: &mut [u8]) -> Result<u64, i64>` | Handle valid, OOB reads return 0 (clamped) |
| `write` | `(handle: u64, offset: u64, data: &[u8]) -> Result<u64, i64>` | Handle valid, end ≤ 4096, overflow = error |
| `close` | `(handle: u64) -> Result<(), i64>` | Handle valid; releases handle, keeps data |
| `stat` | `(handle: u64) -> Result<(u64, u32), i64>` | Handle valid; returns (size, name_len) |
| `list_at` | `(index: usize) -> Option<(u64, u32)>` | Returns (handle, name_len) or None |
| `len` | `() -> usize` | Active file count |

### External PDX Protocol (slot 1 = `SLOT_STORAGE`)

| Opcode | Name | Args | Reply |
|--------|------|------|-------|
| `0x30` | `OP_RAMFS_OPEN` | arg0-2 = name (≤24 bytes), bits 24+ of arg2 = flags | handle or error |
| `0x31` | `OP_RAMFS_READ` | arg0=handle, arg1=offset, arg2=max_len | packed data (8 bytes) or error |
| `0x32` | `OP_RAMFS_WRITE` | arg0=handle, arg1=offset, arg2=8 data bytes | bytes written or error |
| `0x33` | `OP_RAMFS_CLOSE` | arg0=handle | 0 or error |
| `0x34` | `OP_RAMFS_LIST` | arg0=index | packed (handle<<32 \| name_len) or 0 |
| `0x35` | `OP_RAMFS_STAT` | arg0=handle | packed (size<<32 \| name_len) or error |

### Error Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `ERR_INVALID_HANDLE` | -1 | Handle not found or inactive |
| `ERR_NAME_TOO_LONG` | -2 | Name > 24 bytes |
| `ERR_NOT_FOUND` | -3 | File not found (or O_EXCL on existing) |
| `ERR_OVERFLOW` | -4 | Write would exceed 4096 byte limit |
| `ERR_FULL` | -5 | Max 64 files reached |

### Bounds

| Bound | Value |
|-------|-------|
| Max files | 64 |
| Max name length | 24 bytes |
| Max file size | 4096 bytes |
| Namespace | Flat (no directories) |

## Proof Markers

Compile with `SEXFILES_RAMFS_PROOF=1` to enable built-in contract validation.

### Proof 1: Create/write/read roundtrip
- Create file with `O_CREATE`
- Write data, verify byte count
- Read data back, verify content matches
- Close file

### Proof 2: Invalid handle rejection
- Read with invalid handle → `ERR_INVALID_HANDLE`
- Close with invalid handle → `ERR_INVALID_HANDLE`

### Proof 3: Oversized name rejection
- Open with name > 24 bytes → `ERR_NAME_TOO_LONG`

### Proof 4: Out-of-bounds write rejection
- Write > 4096 bytes → `ERR_OVERFLOW`
- Write exactly 4096 bytes at offset 0 → succeeds
- Write at offset 1 with 4096 bytes → `ERR_OVERFLOW`

### Proof 5: Out-of-bounds read clamping
- Read beyond EOF → returns 0 (not error)
- Partial read at boundary → returns remaining bytes

### Proof 6: Max files limit
- Create 64 files → succeeds
- Create 65th file → `ERR_FULL`
- Close all files → count = 0

### Proof 7: Close+reopen data persistence
- Create file, write data
- Close (handle released)
- Reopen by name → data still intact

### Proof Marker Output
```
[sexfiles.ramfs.proof.start]
[sexfiles.ramfs.proof.1] create/write/read roundtrip OK
[sexfiles.ramfs.proof.2] invalid handle rejected OK
[sexfiles.ramfs.proof.3] oversized name rejected OK
[sexfiles.ramfs.proof.4] OOB write rejected OK
[sexfiles.ramfs.proof.5] OOB read clamped OK
[sexfiles.ramfs.proof.6] max files limit enforced OK
[sexfiles.ramfs.proof.7] close+reopen data persistence OK
[sexfiles.ramfs.proof.done] ALL CHECKS PASSED
```

## Build

```sh
# Normal build
RUSTC_BOOTSTRAP=1 cargo build -p sexfiles -Zbuild-std=core,alloc --config 'profile.dev.panic="abort"'

# With proof marker enabled
SEXFILES_RAMFS_PROOF=1 RUSTC_BOOTSTRAP=1 cargo build -p sexfiles -Zbuild-std=core,alloc --config 'profile.dev.panic="abort"'
```

### Build result
```
# cargo check output:
    Checking sexfiles v0.1.0
    Finished dev profile [unoptimized + debuginfo] in 0.11s
```

Zero warnings, zero errors in both lib and bin targets.

## Standalone Proof (Linux host)

A standalone Rust program at `/tmp/ramfs_standalone_test.rs` exercises the same
contract checks on the host system:

```
RESULTS: 16 pass, 0 fail
ALL CHECKS PASSED
```

## Runtime Gate

`./scripts/master_runtime_gate.sh` — sexfiles is NOT currently spawned by the
kernel at boot. Adding sexfiles to the spawn list requires kernel edits (STOP FIRST).
The contract is locked and build-ready for future integration.

### Expected markers when sexfiles is spawned
- `[sexfiles.ready]` — server started
- `[sexfiles.ramfs.proof.*]` — 8 markers when proof enabled

## No Persistence Claim

The RamFS contract explicitly does NOT claim persistence:
- All data is volatile (lost on power loss)
- Close releases handles but retains data for reopen-by-name
- No write-back, no fsync, no journal

## No POSIX Semantics Claim

The RamFS contract explicitly does NOT claim POSIX semantics:
- Flat namespace (no `/` paths, no directories)
- No `mode`/permission bits enforced
- No `O_APPEND`, `O_TRUNC`, `O_RDWR` etc.
- Error codes are internal constants, not POSIX errno values

## Safety

- All handles validated before use (returns `ERR_INVALID_HANDLE`)
- All name lengths bounded (max 24 bytes)
- All file sizes bounded (max 4096 bytes)
- Max file count bounded (max 64)
- OOB reads return 0 bytes (not an error)
- OOB writes return `ERR_OVERFLOW`
- Write locks use spinlock `RwLock` (single-core safe)
- No shared-memory or backing-buffer design
- No kernel ABI changes
- No sex-pdx ABI changes

## Files Changed

```
M Cargo.toml                          (add sexfiles to workspace)
M servers/sexfiles/Cargo.toml         (trim deps)
M servers/sexfiles/src/lib.rs         (add proof, remove cache)
M servers/sexfiles/src/main.rs        (clean entry)
M servers/sexfiles/src/messages.rs    (protocol constants)
M servers/sexfiles/src/pdx.rs         (thin wrapper)
M servers/sexfiles/src/vfs.rs         (message routing)
M servers/sexfiles/src/backends/mod.rs (trait)
M servers/sexfiles/src/backends/ramfs.rs (full impl)
M servers/sexfiles/src/backends/diskfs.rs (stub)
M servers/sexfiles/src/backends/tmpfs.rs (stub)
M servers/sexfiles/src/trampoline.rs  (message loop)
A servers/sexfiles/src/proof.rs       (proof suite)
D servers/sexfiles/src/cache.rs       (removed)
A docs/handoff/SEXFILES_RAMFS_CONTRACT_LOCK_V1.md  (this document)
```

## Recurring Issue

The old sexfiles code was written for a different PDX API version. Future audits
should verify:
1. All servers use the same PDX version (`sex-pdx` crate, NOT `libsys::pdx`)
2. The `AtomicRing` in `sex-pdx/src/ring.rs` is unused and could be removed or properly exposed
3. sexfiles spawn integration requires kernel changes (separate task)
