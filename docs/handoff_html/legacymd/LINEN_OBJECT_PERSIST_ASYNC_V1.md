# LINEN_OBJECT_PERSIST_ASYNC_V1 — Handoff

## Goal
Audit and exercise fire-and-forget async persistence for Linen objects via the
existing SexFiles RamFS storage path.  No blocking reply wait, no sync readback.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/linen/src/main.rs` | Persist proof gate, audit stages, fire-and-forget CREATE_OWNER sends, wiring in _start | +74 |

## Architecture
- **Gate**: `LINEN_OBJECT_PERSIST_PROOF_ENABLED` via `SEXOS_LINEN_OBJECT_PERSIST_PROOF=1`
- **Proof function**: `run_linen_object_persist_proof()` — 6-stage burst loop
- **Stage 0**: Audit — Linen has SLOT_STORAGE, RamFS opcodes, pack_name helpers; `pdx_call()` is fire-and-forget (AsyncEnqueue edge). Safe for CREATE_OWNER only.
- **Stages 1-3**: For each locally-owned object, send `pdx_call(SLOT_STORAGE, OP_RAMFS_CREATE_OWNER, ...)` — fire-and-forget, no reply wait
- **Stage 4**: Document limitation — no async WRITE path (requires handle from CREATE reply)
- **Stage 5**: Done marker

## Fire-and-Forget Pattern
```rust
let (status, _) = pdx_call(SLOT_STORAGE, OP_RAMFS_CREATE_OWNER, n0, n1, arg2);
// No reply wait — enqueue only. Server processes asynchronously.
```
WRITE/CLOSE require the handle returned in the CREATE reply, so full async
write is NOT possible without blocking. This proof honestly documents that limit.

## Markers (serial)
```
[linen.object.persist.audit] safe=N reason=...
[linen.object.persist.send] object_id=N status=N err=N
[linen.object.persist.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_LINEN_OBJECT_PERSIST_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `linen_object_persist`: PASS (persist audit present)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No blocking `pdx_storage_sync()` — pure `pdx_call()` fire-and-forget
- ❌ No sync readback — write path not exercised
- ✅ Uses existing RamFS CREATE_OWNER opcode (0x36)
- ✅ Bounded burst loop (6 stages max)

## Known Limitations
- WRITE/CLOSE not possible without handle from CREATE reply (blocking requirement)
- Objects created by session proof may fill the table, limiting persist targets
- No readback verification (no handle, no reply wait)
- RamFS server may not process CREATE in time for next boot cycle

## Future Follow-up
- Kernel-side async handle return (reply_ring for fire-and-forget handles)
- SEXFILES_ASYNC_REPLY opcode that bundles OPEN+WRITE+CLOSE in one server-side tx
- Full async write/read via reply_ring or completion callback
- Persist proof with dedicated object slot reservation
