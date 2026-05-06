# LINEN_SESSION_PDX_BIND_V1

## Scope
Round 4 continuation for Linen session object model using existing Linen-local implementation only.

Edited files were restricted to:
- `servers/linen/src/main.rs`
- `docs/handoff/LINEN_SESSION_PDX_BIND_V1.md`
- `docs/handoff/snapshots/LINEN_CONTINUE_*`

No edits were made to `kernel/`, `crates/sex-pdx/`, `sexdisplay`, `sexusb`, `tools/qemu`, or unrelated servers.

## Baseline Snapshot
- Baseline HEAD is recorded in:
  - `docs/handoff/snapshots/LINEN_CONTINUE_BASELINE_HEAD.txt`
- Pre-change status is recorded in:
  - `docs/handoff/snapshots/LINEN_CONTINUE_PRE_STATUS.txt`

## Existing Session/Object Model (Confirmed)
Source: `servers/linen/src/session.rs`

- Bounded table: `LINEN_MAX_OBJECTS = 16`
- Name cap: `LINEN_MAX_NAME = 24` bytes
- Object kinds (`ObjectKind`):
  - `Document = 0`
  - `Session = 1`
  - `Unknown = 2`
- Per-object fields include:
  - `object_id: u64` (monotonic, starts at 1)
  - `kind: ObjectKind`
  - `owner_pd: u32`
  - `name: [u8; 24]` + `name_len: u8`
  - `ramfs_handle: u64` (0 means unlinked)

## Owner PD Validation
- `Session::list(caller_pd, start_idx)` filters by owner unless `caller_pd == 0` (server-internal bypass).
- `Session::get(object_id, caller_pd)` enforces owner match unless `caller_pd == 0`.
- Non-owner `get` returns `-6` (`ERR_PERM_DENIED`-equivalent).

## Behavior Summary
- `create(kind, name, owner_pd)`:
  - Validates name bounds (`1..=24`)
  - Fails with `-1` when table is full
  - Fails with `-2` for invalid name length
  - Assigns monotonic `object_id`
- `list(caller_pd, start_idx)`:
  - Returns first matching owned object from index
  - Returns `None` when exhausted
- `get(object_id, caller_pd)`:
  - Returns object when found + owner-valid
  - `-3` if object ID not found
  - `-6` if owner mismatch
- `count()` and `count_owned(caller_pd)` confirmed and now exercised in proof logging.

## PDX Opcode Routes (Linen-local)
Source: `servers/linen/src/main.rs`

- `0x41` `OP_LINEN_CREATE_OBJECT`
  - input: `arg0(kind+name_len), arg1(name bytes 0..7), arg2(name bytes 8..15)`
  - output: `object_id` or negative error code
- `0x42` `OP_LINEN_LIST_OBJECTS`
  - input: `arg0(start_idx)`
  - output: compact packed summary or `0` when done
- `0x43` `OP_LINEN_GET_OBJECT`
  - input: `arg0(object_id)`
  - output: first 8 bytes of name on success or negative error code

No global `sex-pdx` ABI edits were introduced.

## Proof Gate + Serial Markers
Proof gate now uses:
- `SEXOS_LINEN_SESSION_PROOF=1`

Markers emitted:
- `[linen.session.proof.create]`
- `[linen.session.proof.list]`
- `[linen.session.proof.get]`
- `[linen.session.proof.owner_deny]`
- `[linen.session.proof.bounds]`

Additional marker:
- `[linen.session.proof.count]` to show total and owned counts.

Additional proof checks now exercised in `run_session_proof()`:
- oversized-name create rejection (`Err(-2)`)
- invalid object id rejection (`Err(-3)`)
- table-full bound rejection (`Err(-1)`) after filling available slots

## Warning Cleanup
In `servers/linen/src/main.rs`:
- aligned proof env gate to requested variable name
- ensured `ramfs_handle` is read in logging paths
- exercised `count` and `count_owned` in proof
- added `#![allow(static_mut_refs)]` to suppress Rust 2024 compatibility warnings for existing `static mut` pattern used throughout this server without broad refactor

## Remaining Risks / Gaps
- Create path wire format is explicitly capped at 16 bytes (`arg1+arg2`) and now rejects larger `name_len` at handler boundary. Internal session model remains 24-byte bounded.
- `list/get` reply packing is compact and partial; full object metadata requires protocol extension if richer clients need all fields in one call.
- Delete/close/tombstone path is not bound here because no safe existing implementation was present in scope.
- `static mut` usage remains a design risk; warning suppression avoids noisy compile output but does not change underlying aliasing model.

## Readiness (Current)
Linen session/object implementation status after this pass:
- Model implementation: present
- PDX binding for create/list/get: present
- Owner/bounds enforcement: present
- Proof gate + markers: present
- Documentation handoff: present

Estimated Linen readiness score: **8.5/10** for current bounded in-memory session scope.
