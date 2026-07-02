# ASYNC_STORAGE_ACK_MODEL_REVIEW_V1

## Verdict: PASS REVIEW ONLY

No source changes.  Documenting the model for future implementation.

## 1. Current Storage Call Patterns (3 producers, 1 server)

### Spindle → SexFiles (17 PDX calls)
- `pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, ...)` — fire-and-forget
- `pdx_call(SLOT_STORAGE, OP_RAMFS_WRITE, ...)` — fire-and-forget
- No reply readback; save is best-effort
- No load readback possible (pdx_call READ returns (0,0) always)

### Linen → SexFiles (44 PDX calls)
- `pdx_storage_sync(OP_RAMFS_CREATE_OWNER, ...)` — **BLOCKS** waiting for reply
- `pdx_storage_sync(OP_RAMFS_WRITE, ...)` — blocks
- `pdx_storage_sync(OP_RAMFS_CLOSE, ...)` — blocks
- Has async persist audit using fire-and-forget `pdx_call()` for CREATE_OWNER only

### Quil → SexFiles (23 PDX calls)
- `pdx_storage_call(OP_RAMFS_OPEN, ...)` — **BLOCKS** (wraps pdx_call_and_reply)
- `pdx_storage_call(OP_RAMFS_WRITE, ...)` — blocks
- `pdx_storage_call(OP_RAMFS_CLOSE, ...)` — blocks
- Has async save audit using fire-and-forget `pdx_call()` for OPEN only

### SexFiles Server
- Receives PDX calls, processes in RamFS/DiskFS backends
- Writes data to RamFS in-memory store
- Replies with handle/status via `pdx_reply(caller_pd, value)`
- Emits serial markers: `[ramfs.write]`, `[ramfs.create]`, etc.

## 2. Three-Level Acknowledgement Model

| Level | Name | Available? | Mechanism |
|-------|------|-----------|-----------|
| L1 | Send accepted | ✅ | `pdx_call` returns status=0 (enqueued in PDX ring) |
| L2 | Server received | ⚠️ partial | SexFiles emits markers on receipt; caller reads log |
| L3 | Write applied | ✅ (sync) / ❌ (async) | Sync callers get reply; async callers have no handle |
| L4 | Durable | ❌ | No DiskFS flush confirmation on QEMU; NVMe write not verified |

**Key limitation**: L2 correlation between send and receipt requires a tx_id in the payload.  Current PDX `pdx_call(slot, opcode, arg0, arg1, arg2)` has no free argument for tx_id — all 3 args are consumed by the existing protocol (name, handle, offset, data, flags, owner_pd).

## 3. Why tx_id Cannot Be Added Without Protocol Change

Current pdx_call signature: `(slot, opcode, arg0, arg1, arg2) → (status, value)`

For OPEN/CREATE_OWNER: arg0=name[0..7], arg1=name[8..15], arg2=flags|name16_23|owner_pd
For WRITE: arg0=handle, arg1=offset, arg2=data
For CLOSE: arg0=handle, arg1/arg2=unused

No argument is free to carry a tx_id.  Adding one requires:
- **Option A**: New PDX opcode with different arg layout (sex-pdx change ❌)
- **Option B**: Multiplex tx_id into existing arg (e.g., steal bits from name/owner) — fragile, breaks protocol
- **Option C**: Use a separate PDX call for tx_id (ACK opcode) — 2× PDX overhead, still needs correlation

## 4. Phased Implementation Plan (Future)

### Phase A: Marker-Only Correlation (no protocol change)
- Producer emits `[storage.tx.send] source=NAME` marker before pdx_call
- SexFiles emits `[sexfiles.tx.recv] source=NAME` marker after receiving
- Gate matches send+recv by source name in serial log
- **Limitation**: no unique tx_id, concurrent sends may interleave markers
- **Risk**: low (markers only, no protocol change)

### Phase B: tx_id via OP_RAMFS_WRITE_ACK (sexfiles-local opcode)
- New SexFiles opcode: `OP_RAMFS_WRITE_ACK = 0x3F`
- arg0=handle, arg1=tx_id (u64), arg2=data
- SexFiles logs tx_id, replies with (handle, tx_id)
- **Requires**: SexFiles server change only (local opcode, no kernel/pdx)
- **Risk**: low-medium (new SexFiles opcode, backward compatible)

### Phase C: PDX Extended Args (kernel change)
- Kernel extended pdx_call to 4+ arguments
- New: `pdx_call_ext(slot, opcode, arg0, arg1, arg2, tx_id)`
- **Requires**: kernel + sex-pdx change ❌ STOP FIRST
- **Risk**: high (kernel ABI change)

## 5. STOP-FIRST Boundaries
- ❌ No tx_id correlation without protocol change
- ❌ L2 (server received) requires SexFiles marker or new opcode
- ❌ L4 (durable) requires NVMe write confirmation (hardware-dependent)
- ✅ L1 (send accepted) already proven by all 3 producers
- ✅ L3 (write applied) proven by Linen/Quil sync paths

## 6. Decision
**REVIEW ONLY** — Model documented.  Phase A safe for future but low value
(correlation by source name only).  Phase B requires SexFiles server edit.
Phase C requires kernel edit.

Recommended: proceed with Phase A (marker-only) when payload space allows;
defer Phase B/C until app lifecycle requires reliable storage confirmation.
