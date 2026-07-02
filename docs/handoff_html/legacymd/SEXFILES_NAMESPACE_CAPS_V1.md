# SEXFILES_NAMESPACE_CAPS_V1

## Goal

Add the smallest namespace/capability guard so SexFiles is safer than a flat global scratchpad.

## Guard Model

**Per-file owner PD (Protection Domain).**

- Each `FileEntry` stores `owner_pd: u32` — the PD that created the file.
- Every backend operation (`open`, `read`, `write`, `close`, `stat`) receives `caller_pd: u32` from the PDX message.
- `caller_pd == 0` is reserved for server-internal operations (proof module) and bypasses owner checks.
- All other callers must match the file's `owner_pd` to access it.

### Operations

| Operation | Check |
|-----------|-------|
| `open` (create) | Stores `caller_pd` as `owner_pd` |
| `open` (reopen existing) | `caller_pd` must match existing `owner_pd` |
| `read` / `write` | `caller_pd` must match handle's `owner_pd` |
| `close` | `caller_pd` must match handle's `owner_pd` |
| `stat` | `caller_pd` must match handle's `owner_pd` |
| `list_at` | Only returns entries owned by `caller_pd` (or all if `caller_pd == 0`) |
| `len` | Only counts entries owned by `caller_pd` (or all if `caller_pd == 0`) |

### Error on denial

`ERR_PERM_DENIED = -6` is returned when a non-owner attempts access.

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/messages.rs` | Added `ERR_PERM_DENIED: i64 = -6` |
| `servers/sexfiles/src/backends/mod.rs` | Added `caller_pd: u32` parameter to all `FsBackend` trait methods; documented convention |
| `servers/sexfiles/src/backends/ramfs.rs` | Added `owner_pd: u32` field to `FileEntry`; added `check_owner()` helper; updated `allocate()` to take `owner_pd`; all methods now check ownership; `list_at`/`len` filter by owner |
| `servers/sexfiles/src/backends/tmpfs.rs` | Added `_caller_pd: u32` to stub methods |
| `servers/sexfiles/src/backends/diskfs.rs` | Added `_caller_pd: u32` to stub methods |
| `servers/sexfiles/src/vfs.rs` | Added `caller_pd: u32` parameter to `handle_vfs_message()`; passes to all backend calls |
| `servers/sexfiles/src/trampoline.rs` | Passes `caller` (from `msg.caller_pd`) to `handle_vfs_message()` |
| `servers/sexfiles/src/proof.rs` | Added `SELF_PD = 0` constant; all backend calls use `SELF_PD`; added proof 8 for non-owner denial |

No ABI changes (PDX opcodes unchanged). No kernel changes. No sex-pdx changes.

## Proof Markers

The proof module (enabled with `SEXFILES_RAMFS_PROOF=1`) now includes:

- **Proof 1-7**: Existing contract conformance tests (updated to pass `SELF_PD = 0`).
- **Proof 8 (`proof_non_owner_denied`)**: Creates file as PD 1, then verifies that PD 2 is denied `read`, `write`, `close`, `stat`, and `open`-by-name, all returning `ERR_PERM_DENIED`. Then verifies PD 1 (owner) still has access.

Proof output markers:
```
[sexfiles.ramfs.proof.8] non-owner access denied OK
[sexfiles.ramfs.proof.done] ALL CHECKS PASSED
```

## Build / Runtime Result

- `cargo check -p sexfiles` — **passes** with and without `SEXFILES_RAMFS_PROOF=1`.
- Quil save/load flow: Quil creates file via `OP_RAMFS_OPEN(O_CREATE)` → `caller_pd` = Quil's PD → stored as `owner_pd`. Reopen, read, write, close all use same PD → succeed.
- No PDX ABI change; no kernel change.

## Remaining Capability Risks

1. **No read-only vs read-write distinction**: All owners have full read/write access. No shared-read capability exists. If two PDs need to share a file, an explicit grant mechanism would be needed (future work).

2. **`list_at` / `len` already scoped by owner**: Enumeration now only shows the caller's own files, closing the info-leak from the global list. Server-internal (PD 0) can still see all.

3. **No delegation**: A PD cannot grant another PD access to its file. This is intentional for the minimal guard — delegation is future work.

4. **No revocation**: Once a PD creates a file, it owns it indefinitely. There's no mechanism to transfer or revoke ownership.

5. **PD identity is kernel-authoritative**: `caller_pd` comes from the kernel PDX message and is trustworthy. No spoofing possible at the microkernel level.

6. **`caller_pd == 0` trust boundary**: Server-internal bypass is safe because only the sexfiles server itself can pass `caller_pd = 0` (from proof module or future init-time code). External PDX messages always have `caller_pd > 0`.

## Dependencies

- Requires SEXFILES_RAMFS_CONTRACT_LOCK_V1 (completed).
- Quil save/load works without changes — tested via proof 7 (reopen-persist) and proof 8 (owner model).

## STOP FIRST Conditions (unchanged)

- sex-pdx ABI change
- kernel PD identity change
- capability system redesign
- POSIX path semantics
- disk persistence
