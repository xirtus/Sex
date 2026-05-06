# LINEN_SESSION_OBJECTS_V1

Status: Implemented in Linen with bounded in-memory model; protocol/documentation proof now recorded.

## Scope

This handoff documents the existing Linen session object model implemented in:
- `servers/linen/src/session.rs`
- `servers/linen/src/main.rs`

No kernel changes. No sex-pdx ABI changes. No persistence guarantees.

## Model Shape

- Session table is fixed-size: `LINEN_MAX_OBJECTS = 16`.
- Object name cap: `LINEN_MAX_NAME = 24` bytes.
- Object row (`LinenObject`):
  - `object_id: u64` (monotonic, starts at 1)
  - `kind: ObjectKind`
  - `owner_pd: u32`
  - `name: [u8; 24]`
  - `name_len: u8`
  - `ramfs_handle: u64` (0 = unlinked)
- Storage is no-heap/no-std bounded array: `[Option<LinenObject>; 16]`.

## ObjectKind

`ObjectKind` is `repr(u8)`:
- `0 = Document`
- `1 = Session`
- `2 = Unknown`

PDX-side kind byte mapping in `main.rs` accepts only those values and rejects others.

## Owner Validation

Owner policy is enforced in session operations:
- `list(caller_pd, start_idx)` returns only owned rows unless `caller_pd == 0` (server/internal bypass).
- `get(object_id, caller_pd)`:
  - returns object if owner matches (or caller 0)
  - returns `-6` on owner mismatch (`ERR_PERM_DENIED` equivalent)
  - returns `-3` if object not found

`create(...)` stores `caller_pd` as owner for each row.

## Create/List/Get/Count Behavior

### Create

- Rejects empty name or name length > 24 (`-2` in session layer).
- Rejects when table full (`-1` in session layer).
- On success: assigns `object_id = next_id`, increments counter, stores bounded row.

### List

- Input: `start_idx` (u8), owner-filtered by `caller_pd`.
- Returns first matching row at or after index, else `None`.
- PDX reply in current implementation is compact and currently includes:
  - low 32 bits of `object_id`
  - kind in bits 32..39
  - name_len in bits 40..47
- Current wire reply does not include full 24-byte name or full owner/ramfs fields.

### Get

- Input: `object_id`.
- Returns owner-validated row or error.
- Current PDX reply returns first 8 bytes of name (`name_lo`) on success.

### Count

- `count()` returns number of occupied rows.
- `count_owned(caller_pd)` returns number of owned rows (or all rows for caller 0).

## Current Protocol / Opcode State

Linen session opcodes in `servers/linen/src/main.rs`:
- `OP_LINEN_CREATE_OBJECT = 0x41`
- `OP_LINEN_LIST_OBJECTS = 0x42`
- `OP_LINEN_GET_OBJECT = 0x43`

Current encoding state:
- Create input uses packed `arg0` (`kind` + `name_len`) and name bytes in `arg1/arg2`.
- List/Get currently use reduced single-`u64` replies due to `pdx_reply` single-value return path.
- Error replies are sent as `u64`-cast negatives or explicit reject constants in handler code.

## Proof Markers

Startup proof gate:
- `LINEN_SESSION_PROOF_ENABLED` set by build env `LINEN_SESSION_PROOF`.

Markers:
- `[linen.session.proof] begin`
- Stage markers:
  - `stage=0 create_doc ...`
  - `stage=1 list_owned ...`
  - `stage=2 list_non_owner ...`
  - `stage=3 bad_kind_enum_result ...`
  - `stage=4 oversized_name ...`
  - `stage=5 non_owner_get ...`
- `[linen.session.proof] end`

Runtime operation markers:
- `[linen.session.create] ...`
- `[linen.session.list] ...`
- `[linen.session.get] ...`
- Reject path: `[linen.session.reject] reason=...`

## Bounds Summary

- Max objects: 16 hard cap.
- Max name length: 24 bytes hard cap.
- No heap allocation in session layer.
- No POSIX paths or Linux filesystem assumptions.

## Remaining Risks / Gaps

- Current list/get wire replies are partial representations; full object projection is not yet encoded in one stable reply contract.
- Name transport in create handler currently consumes up to 16 bytes from `arg1/arg2`; 24-byte model exists in storage, but full 24-byte ingress path is not yet fully expressed in this opcode format.
- Session state is in-memory only; no persistence contract is provided.
- Error-code mapping comments reference equivalents; a formal Linen error enum/wire contract is still pending.

## Readiness Score (Current)

Linen session-object readiness: **7.5 / 10**

Rationale:
- Implemented bounded model + owner validation + proof markers: strong.
- Protocol reply surface for full object data is still partial: remaining gap.
