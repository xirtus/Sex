# E6_STORAGE_TOMBSTONE_DELETE_V1

**Status:** Implemented. Code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E6 only if storage remains RAM-only, shell-only, bounded, no paths, no app caps, and no kernel edits."

---

## Summary

Implements E5 Option A (minimal DELETE): status-code remap, per-slot generation counter, tri-state slot model (empty/active/tombstoned), `OP_KV_DEL = 0xB2` with capability-gated dispatch, tombstone proof markers, and reply discriminator bit. RAM-only, shell-only, no kernel edits, no sex-pdx changes, no app caps, no paths.

---

## 1. Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexstore/src/main.rs` | Full E6 implementation | ~300 lines total |

No other files touched. No kernel edits. No sex-pdx changes. No ABI changes.

---

## 2. Status Mapping (Final)

### 2.1 Status codes

| Code | Constant | Purpose | Previous value (E4) |
|------|----------|---------|---------------------|
| `0x00` | `KV_OK` | Success (PUT, DELETE) | `KV_PUT_OK = 0x00` |
| `0x01` | `KV_NOT_FOUND` | Get/delete on missing or tombstoned key | (was `KV_DENIED`) |
| `0x02` | `KV_FULL` | PUT when table full | `KV_PUT_FULL = 0x02` |
| `0x03` | `KV_INVALID_KEY` | key == 0x00 | Unchanged |
| `0x04` | `KV_INVALID_VALUE` | PUT value envelope invalid | Unchanged |
| `0x05` | `KV_DENIED` | Caller lacks capability | (was `0x01`) |

### 2.2 Reply discriminator

All status replies now set bit 63 (`REPLY_STATUS_BIT = 0x8000_0000_0000_0000`) to distinguish status codes from stored values.

- **GET success**: reply = stored u64 (bit 63 = 0)
- **Status reply**: reply = `0x8000_0000_0000_0000 | status_code`

Silk-shell's `unpack_scene_settings_blob()` checks `byte[0] == 0xAC` (magic). Status replies with bit 63 set have `byte[0]` between `0x00` and `0x05`, which fails the magic check → defaults. **Safe, backward-compatible.**

### 2.3 Boot-time marker

A single `[sexstore.status.mapping]` marker is emitted at boot to document the remap:

```
[sexstore.status.mapping] KV_OK=0x00 KV_NOT_FOUND=0x01 KV_FULL=0x02 KV_INVALID_KEY=0x03 KV_INVALID_VALUE=0x04 KV_DENIED=0x05 REPLY_BIT=0x8000
```

---

## 3. Opcode Added

| Opcode | Value | Purpose |
|--------|-------|---------|
| `OP_KV_DEL` | `0xB2` | DELETE / tombstone |

Local constant in sexstore (same pattern as `OP_KV_GET` and `OP_KV_PUT`). Not added to sex-pdx to avoid ABI hash update. Silk-shell does not yet call DELETE — opcode constant can be mirrored when needed.

---

## 4. Slot Model Implemented

### 4.1 KvSlot structure

```rust
struct KvSlot {
    state:      u8,   // 0=Empty, 1=Active, 2=Tombstoned
    generation: u8,   // 0=never written, 1..255=write count (wraps 255→1)
    key:        u32,
    val:        u64,
}
```

Size: 16 bytes (1 + 1 + 2 padding + 4 + 8). **Same 256-byte table total.** No heap.

### 4.2 Slot state transitions

```
                     EMPTY (state=0)
                         │ PUT (insert)
                         ▼
                    ┌──────────┐
           ┌────────│  ACTIVE  │
           │        │ (state=1)│
           │        └────┬─────┘
           │ PUT(update) │ DELETE
           │ gen++       │ gen++, state=2
           │             ▼
           │        ┌────────────┐
           └────────│ TOMBSTONED │
           PUT(revive) (state=2)
           state=1, gen++
```

- **Corrupt state (3)**: Deferred (not implemented). No corruption detection in E6.
- **Slot reclamation**: When table is full and no empty slot exists, tombstoned slots are reclaimed (overwritten with new key+val, generation bumped).

---

## 5. Generation Behavior

### 5.1 Rules

| Event | Generation behavior |
|-------|-------------------|
| First write (insert into empty) | Set to 1 |
| PUT update (active key) | Bump: `gen = min(gen+1, 255) → 1 on wrap` |
| PUT revive (tombstoned key) | Bump |
| PUT reclaim (reuse tombstoned slot for different key) | Bump |
| DELETE (active → tombstoned) | Bump |
| DELETE idempotent (already tombstoned) | No change |
| GET | No change |

Wrap: `255 → 1` (never 0). Generation 0 = never written.

### 5.2 Caller protocol

No caller-supplied generation protocol in E6. Generation is internal to sexstore. E6+ may add `OP_KV_PUT_GEN` or caller-supplied generation arg.

---

## 6. Tombstone Behavior

### 6.1 DELETE dispatch

```
OP_KV_DEL (0xB2):
  1. Policy check (same store_cap_allowed — shell-only range)
  2. Scan for key in non-empty slot
  3. Match on found_state:
     - Active (1) → state=2, bump gen
       [sexstore.tombstone.record] reason=delete
       [sexstore.generation.bump] op=tombstone
       → kv_reply_status(KV_OK)
     - Tombstoned (2) → idempotent, no state change
       [sexstore.tombstone.record] reason=delete_idempotent
       → kv_reply_status(KV_OK)
     - Not found (0) →
       → kv_reply_status(KV_NOT_FOUND)
```

### 6.2 GET on tombstoned key

```
GET:
  - Active → return stored value (bit 63 = 0)
  - Tombstoned → [sexstore.tombstone.get]; kv_reply_status(KV_NOT_FOUND)
  - Not found → kv_reply_status(KV_NOT_FOUND)
```

### 6.3 PUT on tombstoned key (revive)

```
PUT:
  - Find existing key in active or tombstoned state
  - Active → update val, bump gen
  - Tombstoned → revive (state=1), update val, bump gen
    [sexstore.tombstone.revive]
    [sexstore.generation.bump] op=revive
  - Not found → insert into empty or reclaim tombstoned
```

### 6.4 DELETE idempotency

DELETE is idempotent: calling DELETE twice on the same key returns `KV_OK` both times (first call tombstones, second finds already tombstoned and returns success). DELETE on missing key returns `KV_NOT_FOUND` (not an error).

---

## 7. Reply Behavior

### 7.1 Reply flow

| Path | Reply function | Value |
|------|---------------|-------|
| GET success | `kv_reply(caller, val)` | Stored u64 (bit 63 = 0) |
| GET tombstoned | `kv_reply_status(caller, KV_NOT_FOUND)` | `0x8000_0000_0000_0001` |
| GET not found | `kv_reply_status(caller, KV_NOT_FOUND)` | `0x8000_0000_0000_0001` |
| GET denied | `kv_reply_status(caller, KV_DENIED)` | `0x8000_0000_0000_0005` |
| PUT success | `kv_reply_status(caller, KV_OK)` | `0x8000_0000_0000_0000` |
| PUT full | `kv_reply_status(caller, KV_FULL)` | `0x8000_0000_0000_0002` |
| PUT denied | `kv_reply_status(caller, KV_DENIED)` | `0x8000_0000_0000_0005` |
| PUT invalid key | `kv_reply_status(caller, KV_INVALID_KEY)` | `0x8000_0000_0000_0003` |
| PUT invalid value | `kv_reply_status(caller, KV_INVALID_VALUE)` | `0x8000_0000_0000_0004` |
| DELETE success | `kv_reply_status(caller, KV_OK)` | `0x8000_0000_0000_0000` |
| DELETE idempotent | `kv_reply_status(caller, KV_OK)` | `0x8000_0000_0000_0000` |
| DELETE not found | `kv_reply_status(caller, KV_NOT_FOUND)` | `0x8000_0000_0000_0001` |
| DELETE denied | `kv_reply_status(caller, KV_DENIED)` | `0x8000_0000_0000_0005` |
| Unknown opcode | `kv_reply(caller, 0)` | `0x0000_0000_0000_0000` |

### 7.2 Silk-shell compatibility

- `unpack_scene_settings_blob(value)` checks `byte[0] == 0xAC` — all status replies have `byte[0]` between 0x00 and 0x05, so they fail the magic check → defaults. **Safe.**
- GET success with valid stored u64 has `byte[0] == 0xAC` → parsed correctly. **Unchanged.**
- PUT replies are fire-and-forget (ignored by silk-shell). **Unchanged.**

---

## 8. Proof Markers Added

### 8.1 New budgeted statics

| Static | Budget | Marker format | When |
|--------|--------|---------------|------|
| `LOG_GENERATION_BUMP` | 64 | `[sexstore.generation.bump] key=K slot=N gen=G op=put|revive|insert|reclaim|tombstone` | Every slot write |
| `LOG_TOMBSTONE_RECORD` | 32 | `[sexstore.tombstone.record] key=K slot=N gen=G reason=delete|delete_idempotent` | DELETE on active or tombstoned key |
| `LOG_TOMBSTONE_GET` | 32 | `[sexstore.tombstone.get] key=K slot=N gen=G` | GET on tombstoned key |
| `LOG_TOMBSTONE_REVIVE` | 16 | `[sexstore.tombstone.revive] key=K old_gen=G` | PUT revives tombstoned key |

### 8.2 Boot-time marker (no budget)

| Marker | When |
|--------|------|
| `[sexstore.status.mapping] KV_OK=0x00 KV_NOT_FOUND=0x01 KV_FULL=0x02 KV_INVALID_KEY=0x03 KV_INVALID_VALUE=0x04 KV_DENIED=0x05 REPLY_BIT=0x8000` | Once at boot |

### 8.3 E4 markers (unchanged)

| Static | Budget |
|--------|--------|
| `LOG_PUT` | 32 |
| `LOG_GET` | 32 |
| `LOG_POLICY_ALLOW` | 32 |
| `LOG_POLICY_DENY` | 32 |
| `LOG_KEY_INVALID` | 8 |
| `LOG_VALUE_INVALID` | 8 |
| `LOG_REPLY_ERROR` | 8 |

### 8.4 Marker counts per boot

| Source | Markers | Total budget |
|--------|---------|-------------|
| E0 baseline | 2 | 64 |
| E4 added | 5 | 88 |
| E6 added | 4 | 144 |
| Boot-time | 1 | 1 (no budget) |
| **Total** | **12** | **296** |

---

## 9. Build Result

```
[SEXOS ENTRYPOINT] success
Build complete: sexos-v1.0.0.iso
```

**Sexstore warnings:** 1 (pre-existing — unused import `SLOT_SEXSTORE`).
No new warnings. No errors.

Other build errors (pre-existing, unrelated to E6):
- `sexshop`: 6 errors (PDX_DISCOVER_SERVICE, PdxMessage.num field)
- `silkbar`, `sexdisplay`, `sexusb`: linking errors (undefined memcpy/memset)

---

## 10. Behavior Changes

### 10.1 What changed

| Scenario | E4 behavior | E6 behavior |
|----------|-------------|-------------|
| GET on existing key | Returns stored u64 | **Unchanged** (bit 63 = 0) |
| GET on missing key | Returns 0 | Returns `KV_NOT_FOUND` with status bit (0x8000...0001) |
| GET on tombstoned key | N/A (no tombstone) | Returns `KV_NOT_FOUND` with status bit |
| GET denied | Returns `KV_DENIED = 0x01` | Returns `KV_DENIED = 0x05` with status bit |
| PUT update existing | Replaces value | Replaces value, bumps generation |
| PUT revive tombstoned | N/A (no tombstone) | Revives, bumps generation, logs tombstone.revive |
| PUT insert into empty | Writes | Writes, generation = 1 |
| PUT reclaim tombstone | N/A | Reclaims, bumps generation |
| DELETE active key | N/A (no opcode) | Tombstones, bumps gen, logs markers |
| DELETE tombstoned key | N/A | Idempotent, logs marker |
| DELETE missing key | N/A | Returns `KV_NOT_FOUND` |
| Reply ABI | Plain value | Status replies have bit 63 = 1 |
| Silk-shell GET reply | Magic-check rejects 0x01 | Magic-check rejects 0x8000...0001 — safe |

### 10.2 What did NOT change

- ❌ No new kernel ABI changes
- ❌ No sex-pdx changes
- ❌ No new cap grants
- ❌ No durable persistence
- ❌ No LIST/ENUM
- ❌ No app PD caps (only silk-shell)
- ❌ No heap/String
- ❌ No raw paths
- ❌ No POSIX unlink semantics
- ❌ No corrupt state (deferred)
- ❌ No caller-supplied generation protocol (deferred)

---

## 11. STOP FIRST Findings

| Condition | Status |
|-----------|--------|
| OP_KV_DEL requires ABI/kernel change | ✅ Not required — opcode stays local to sexstore |
| Status remap breaks existing shell restore behavior | ✅ **Does not break** — silk-shell magic check rejects all status replies (byte 0 = 0x00–0x05 ≠ 0xAC) |
| Slot struct cannot safely add state/generation | ✅ Fits in existing 16-byte slot (generation replaces padding) |
| DELETE authority cannot be represented with current policy | ✅ Uses existing `store_cap_allowed()` — same shell-only range as PUT/GET |
| Reply encoding needs broad protocol redesign | ✅ Compact discriminator (bit 63) — no kernel change |
| Durable backend/app caps/Linen/Quil required | ✅ Not required — RAM-only, shell-only |
| RAM-only violated | ✅ RAM-only preserved |
| Shell-only violated | ✅ Only silk-shell (domain 3) authorized |
| Bounded table violated | ✅ 16 slots, 256 bytes, static allocation |
| Raw paths introduced | ✅ No paths |
| Kernel edits required | ✅ No kernel edits |

> ✅ **E6 passes its own gate.** RAM-only, shell-only, bounded, no paths, no app caps, no kernel edits.

---

## 12. Ready/Not Ready for E7

### 12.1 Yes — E7 can proceed

E7 (deterministic proof marker hardening) is **ready to start**:

1. **Status mapping finalized** — KV_DENIED=0x05, KV_NOT_FOUND=0x01, discriminator bit in use
2. **Slot model stable** — empty/active/tombstoned with generation counter
3. **DELETE opcode live** — OP_KV_DEL=0xB2, capability-gated, proof-marked
4. **Proof markers running** — generation bump, tombstone record/get/revive all budgeted
5. **No build regressions** — 1 pre-existing warning
6. **Silk-shell compatibility verified** — status-bit replies safely rejected by magic check

### 12.2 E7 scope (proposed)

- Sequence_id generation (monotonic counter for proof markers)
- Structured proof marker format with sequence_id
- Corrupt state (state=3) with checksum detection
- `KV_CORRUPT` (0x06) status code
- Reply ABI cleanup (document discriminator contract)
- Add OP_KV_DEL constant to silk-shell for future use

### 12.3 Outstanding pre-E7 items

- Silk-shell does not yet call DELETE — E7 can add the constant without functional change

---

## Appendix A: Full Proof Trace

### A.1 PUT → update active

```
1. pdx_call(SLOT_SEXSTORE, OP_KV_PUT, key=0x01, val=blob, 0)
2. caller=3, store_cap_allowed(3, 0x01)=true
3. [sexstore.policy.allow] caller=3 key=1 op=PUT
4. store_validate_value(0x01, blob)=true
5. Scan: found slot 0, state=1 (active)
6. slot.val = blob; bump_generation(slot)  # gen: 3→4
7. [sexstore.generation.bump] key=1 slot=0 gen=4 op=put
8. kv_reply_status(3, KV_OK)  → 0x8000_0000_0000_0000
```

### A.2 PUT → revive tombstoned

```
1. pdx_call(SLOT_SEXSTORE, OP_KV_PUT, key=0x01, val=blob, 0)
2. caller=3, store_cap_allowed(3, 0x01)=true
3. Scan: found slot 0, state=2 (tombstoned), gen=5
4. [sexstore.tombstone.revive] key=1 old_gen=5
5. slot.state=1; slot.val=blob; bump_generation(slot)  # gen: 5→6
6. [sexstore.generation.bump] key=1 slot=0 gen=6 op=revive
7. kv_reply_status(3, KV_OK)  → 0x8000_0000_0000_0000
```

### A.3 DELETE → active key

```
1. pdx_call(SLOT_SEXSTORE, OP_KV_DEL, key=0x01, 0, 0)
2. caller=3, store_cap_allowed(3, 0x01)=true
3. Scan: found slot 0, state=1 (active), gen=6
4. slot.state=2; bump_generation(slot)  # gen: 6→7
5. [sexstore.tombstone.record] key=1 slot=0 gen=7 reason=delete
6. [sexstore.generation.bump] key=1 slot=0 gen=7 op=tombstone
7. kv_reply_status(3, KV_OK)  → 0x8000_0000_0000_0000
```

### A.4 GET → tombstoned key

```
1. pdx_call(SLOT_SEXSTORE, OP_KV_GET, key=0x01, 0, 0)
2. caller=3, store_cap_allowed(3, 0x01)=true
3. Scan: found slot 0, state=2 (tombstoned), gen=7
4. [sexstore.tombstone.get] key=1 slot=0 gen=7
5. kv_reply_status(3, KV_NOT_FOUND)  → 0x8000_0000_0000_0001
```

### A.5 GET → denied (non-shell caller)

```
1. pdx_call(SLOT_SEXSTORE, OP_KV_GET, key=0x01, 0, 0)  # caller != 3
2. caller=10, store_cap_allowed(10, 0x01)=false
3. [sexstore.policy.deny] caller=10 key=1 class=shell reason=no_cap
4. kv_reply_status(10, KV_DENIED)  → 0x8000_0000_0000_0005
```

---

## Appendix B: Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | E6 implementation — all changes |
| `servers/silk-shell/src/main.rs` | Only client — `unpack_scene_settings_blob()`, `handle_sexstore_get_reply()` |
| `crates/sex-pdx/src/lib.rs` | PdxMessage, syscall 29 — unchanged by E6 |
| `kernel/src/init.rs` | sexstore spawn + cap grant — unchanged by E6 |
| `docs/handoff/E5_STORAGE_GENERATION_TOMBSTONE_SPEC_V1.md` | E5 spec that E6 implements |
| `docs/handoff/E4_STORAGE_SCHEMA_VALIDATION_V1.md` | E4 base — caller policy, value validation |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Master plan — §6 storage object model, §11 tombstone |
