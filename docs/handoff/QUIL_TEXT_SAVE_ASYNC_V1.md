# QUIL_TEXT_SAVE_ASYNC_V1 — Handoff

## Goal
Audit and exercise fire-and-forget async text buffer save for Quil via the
existing SexFiles RamFS storage path.  Preserves HID stash/replay.  No blocking
save/load.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Save async proof gate, audit + send stages, inline in _start | +33 |

## Architecture
- **Gate**: `QUIL_TEXT_SAVE_ASYNC_PROOF_ENABLED` via `SEXOS_QUIL_TEXT_SAVE_ASYNC_PROOF=1`
- **Stage 0**: Audit — Quil has SLOT_STORAGE, RamFS opcodes, pack_name helpers; `pdx_call()` is fire-and-forget. OPEN via pdx_call is safe.
- **Stage 1**: Attempt fire-and-forget OPEN via `pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, ..., flags=O_CREATE)` — no reply wait
- **Stage 2**: Document limitation — no async WRITE path (requires handle from OPEN reply)
- Emit done marker

## Markers (serial)
```
[quil.text.save.audit] safe=N reason=...
[quil.text.save.send] len=N status=N err=N
[quil.text.save.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_TEXT_SAVE_ASYNC_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_text_save`: PASS (save audit complete)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No blocking `pdx_storage_call()` — pure `pdx_call()` fire-and-forget
- ❌ No HID stash/replay disruption
- ✅ Uses existing RamFS OPEN opcode (0x30) with O_CREATE flag
- ✅ Existing sync save/load paths (CMD_SAVE_DOCUMENT / CMD_LOAD_DOCUMENT) unchanged

## Known Limitations
- WRITE/CLOSE not possible without handle from OPEN reply (blocking requirement)
- No readback verification (no handle, no reply wait)
- Async OPEN may race with RamFS server startup
- Existing sync `quil_save()` / `quil_load()` remain the primary save path

## Future Follow-up
- Kernel-side async reply ring for fire-and-forget handle delivery
- Bundle OPEN+WRITE+CLOSE in one async RamFS transaction opcode
- Async save triggered from keyboard shortcut (not just boot proof)
- SexFiles DiskFS async write path
