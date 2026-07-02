# ASYNC_STORAGE_ACK_PHASE_B_SEXFILES_LOCAL_OPCODE_REVIEW_V1

## Verdict: PASS REVIEW ONLY

Docs-only.  No source changes.  Design ready for implementation when tx_id
blocker is resolved or when caller/source-level status is sufficient.

## 1. SexFiles Opcode Audit

| Opcode | Value | Used By |
|--------|-------|---------|
| OP_RAMFS_OPEN | 0x30 | Spindle, Quil, Linen |
| OP_RAMFS_READ | 0x31 | Quil, Linen |
| OP_RAMFS_WRITE | 0x32 | All 3 producers |
| OP_RAMFS_CLOSE | 0x33 | All 3 |
| OP_RAMFS_LIST | 0x34 | Spindle |
| OP_RAMFS_STAT | 0x35 | — |
| OP_RAMFS_CREATE_OWNER | 0x36 | Linen |
| OP_RAMFS_OBJECT_ID | 0x37 | Linen |
| OP_DISKFS_WRITE..FLUSH | 0x38-0x3A | Linen, Quil |
| OP_DISKFS_STAT | 0x3B | Linen, Quil |
| OP_DISKFS_MANIFEST_HASH | 0x3C | Linen |
| OP_RAMFS_READNAME | 0x3D | Linen |
| OP_DISKFS_SELECT | 0x3E | Linen, Quil |
| **FREE** | **0x3F** | — |

## 2. Safety Verdict

Adding OP_RAMFS_STATUS = 0x3F is **safe** (local app protocol, same
pattern as OP_LINEN_SEARCH_OBJECTS=0x47 which is proven at 65+ gates):

- SexFiles defines `pub const OP_RAMFS_STATUS: u64 = 0x3F` in messages.rs
- SexFiles adds handler in vfs.rs: receive op, query local state, reply
- Producers define their own local copy of 0x3F
- No kernel edit. No sex-pdx edit. No global ABI change.

## 3. Design Options

### Option A: Per-Caller Last-Write Status
```
pdx_call(SLOT_STORAGE, 0x3F, 0, 0, 0)
→ SexFiles replies with: (last_write_ok, last_write_size, last_write_time)
```
- **Pros**: Simple, no tx_id needed. Works for "did my last write succeed?"
- **Cons**: Only works for single-producer scenarios. Ambiguous with concurrent writes.
- **Status**: Implementable now. Low value (was already known from sync reply).

### Option B: Recent-Write Ring Query
```
pdx_call(SLOT_STORAGE, 0x3F, ring_index, 0, 0)
→ SexFiles replies with packed: (index, source_id, op, size, status)
```
- **Pros**: Multi-entry ring, producers can scan recent history.
- **Cons**: Producer must poll to find its write. No unique correlation.
- **Status**: Implementable. Medium value. Needs ring in SexFiles server.

### Option C: Object-ID Based Status
```
pdx_call(SLOT_STORAGE, 0x3F, object_id, 0, 0)
→ SexFiles replies with: (object_id, exists, size, generation)
```
- **Pros**: Unique correlation by object. Queries specific file.
- **Cons**: Only works for named files (OPEN+CREATE), not for handle-based ops.
- **Status**: Implementable. Good for named-file producers (Quil "quil_doc_01").

### Option D: Do Nothing (Defer)
- Wait until extended PDX args allow tx_id in payload.
- **Pros**: No wasted implementation on partial solution.
- **Cons**: No progress on storage confirmation.
- **Status**: Default conservative choice.

## 4. The tx_id Blocker

All options A-C lack unique write-level correlation because:
- PDX `pdx_call(slot, opcode, a0, a1, a2)` has 3 args, all used by write protocol
- No free arg for tx_id
- Option C sidesteps this by using object_id (filename-based, not write-based)

**Fundamental issue**: without a tx_id in the WRITE call, the status query
can only report aggregate/heuristic status, not "this specific write was applied."

## 5. STOP-FIRST Boundaries
- ❌ Correlation needs unique tx_id per write (requires arg or opcode space)
- ❌ Durable confirmation needs DiskFS/NVMe flush (hardware-dependent)
- ✅ Per-caller or per-object status query is safe (Option A or C)
- ✅ SexFiles-local opcode 0x3F is safe (matches proven 0x47 pattern)

## 6. Recommended Phases

### Phase B1 (implementable now): Option C — Object-ID Status
- `OP_RAMFS_OBJECT_STATUS = 0x3F`
- arg0=object_id, reply=(exists, size, generation)
- Low risk, high value for Quil (check if "quil_doc_01" was saved)
- Unique correlation by object name

### Phase B2 (future): Write Ring (Option B)
- Requires ring buffer in SexFiles server
- Pollable by producers
- Still no unique tx_id

### Phase C (requires kernel): Extended PDX Args
- 4+ argument pdx_call → tx_id in arg3
- Unique per-write correlation
- STOP FIRST: kernel ABI change

## 7. Decision

**REVIEW ONLY** — Phase B1 (Object-ID Status) is safe and implementable.
Proceed when storage confirmation for named files is prioritized over write-level tx_id.
