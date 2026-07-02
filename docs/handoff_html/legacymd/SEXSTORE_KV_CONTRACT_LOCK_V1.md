# SEXSTORE_KV_CONTRACT_LOCK_V1

## Contract Shape (Bounded, no_std, in-memory)
Implementation file: `servers/sexstore/src/main.rs`

- Table capacity: **16 slots** (`KV_SLOT_COUNT=16`) (existing smaller bound than requested max 64)
- Key shape: **u32 key id** (existing smaller than 32-byte key target)
- Value shape: **u64 value** (existing smaller than 256-byte value target)
- Operations present:
  - `PUT` (`OP_KV_PUT=0xB1`)
  - `GET` (`OP_KV_GET=0xB0`)
  - `DEL` tombstone (`OP_KV_DEL=0xB2`)
- Deterministic status mapping:
  - `KV_OK`, `KV_NOT_FOUND`, `KV_FULL`, `KV_INVALID_KEY`, `KV_INVALID_VALUE`, `KV_DENIED`
  - status encoded via `REPLY_STATUS_BIT` (bit63)
- Owner/caller validation:
  - `store_cap_allowed(caller_pd, key)`
  - currently shell-only policy for key class `0x01..0x0F` and `caller_pd=3`
- No POSIX/file semantics.

## Persistence Statement
No disk persistence claim is made in this contract.
Current durable logic is RAM-backed scaffold (`DURABLE_REGION`) and therefore not persistent across reboot/media.

## Proof Gate
- `SEXOS_SEXSTORE_KV_PROOF=1`

## Proof Markers
- `[sexstore.kv.proof.roundtrip]`
- `[sexstore.kv.proof.missing_key]`
- `[sexstore.kv.proof.oversized_key]` (key shape bound proof: 4-byte key model)
- `[sexstore.kv.proof.oversized_value]` (value shape bound proof: 8-byte model + envelope rejection)
- `[sexstore.kv.proof.table_full]`
- `[sexstore.kv.proof.owner_deny]`

## Notes
- No sex-pdx ABI changes.
- No kernel edits.
- No broad storage redesign.
