# E2_STORAGE_PROTOCOL_SPEC_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E2 only if docs/spec only, no ABI/code edits, no durable
backend, no Linen/Quil persistence, no raw paths."

---

## Summary

Specification of a minimum safe storage protocol model for SexOS sexstore K/V.
Defines authority model, key namespace, value envelope, operation semantics,
reply/error codes, and proof marker plan. All spec — no implementation.

E2 is the bridge between E1 (audit of current state) and E3 (capability policy).
E2 specifies *what the protocol should look like*; E3 specifies *who can do what*.

---

## 1. Storage Authority Model

### 1.1 Core invariant

> sexstore owns storage K/V state. No other domain may mutate or bypass
> sexstore's internal state. All storage access must go through PDX IPC
> to SLOT_SEXSTORE.

### 1.2 E2 authority boundaries

| Entity | May read? | May write? | Authority basis |
|--------|-----------|------------|-----------------|
| sexstore (domain 8) | ✅ Owns all state | ✅ Manages table internally | Boot-time spawn |
| kernel (domain 0) | ✅ Can inspect via cap table | ✅ Boot-time cap genesis | Boot DAG |
| silk-shell (domain 3) | ✅ Shell-owned keys only | ✅ Shell-owned keys only | `SLOT_SEXSTORE` cap |
| All other PDs | ❌ Denied | ❌ Denied | No cap granted in E2 |

### 1.3 What E2 does NOT change

- **No new cap grants.** silk-shell remains the only client with `SLOT_SEXSTORE`.
- **No app PD storage caps.** Linen, Quil, SexAudio, Theremin do not receive
  storage caps in E2. They must wait for E3+ capability policy.
- **No kernel/ABI edits.** `kernel/src/init.rs` cap grant is unchanged.
- **No sex-pdx edits.** Opcodes remain local copies in sexstore and silk-shell.

### 1.4 Future authority model (E3+)

```
sexstore (domain 8) ← SLOT_SEXSTORE cap ← silk-shell (domain 3)
                                         ← SexAudio (domain X) — E3+
                                         ← Theremin (domain X) — E3+
                                         ← Linen (domain X) — F-track
```

---

## 2. Key Namespace

### 2.1 Current key

| Key | Owner | Value | Purpose |
|-----|-------|-------|---------|
| `0x01` | silk-shell | 8-byte packed u64 | Scene appearance (preset_idx, chrome_flags, access_flags) |
| `0x00` | (reserved) | — | Invalid key; never stored |

### 2.2 Proposed E2 key ranges

Keys are `u32`. Ranges are non-overlapping. Collision detection at spec/plan
time only — no runtime enforcement in E2 (E3 adds StoreCapability checks).

| Range | Owner | Status | Purpose |
|-------|-------|--------|---------|
| `0x00` | (reserved) | Invalid | Never allocated |
| `0x01–0x0F` | **shell legacy** | Current + E2 | Scene appearance (`0x01`), input config, audio policy flags |
| `0x10–0x1F` | **Theremin settings** | Reserved for E3+ | Sound policy, volume, profile |
| `0x20–0x2F` | **SexAudio policy** | Reserved for E3+ | Audio route config, mixer state |
| `0x30–0x3F` | **Shell appearance** | Reserved for E4+ | **Future home for `0x01` migrate** |
| `0x40–0x4F` | **Shell input config** | Reserved for E4+ | Input device preferences |
| `0x50–0x5F` | **Linen documents** | Reserved for F-track | Document metadata refs |
| `0x60–0x6F` | **App storage** | Reserved for future | Per-app preference keys |
| `0x70–0x7F` | **Admin/debug** | Reserved for E6+ | Tombstone reclamation, repair |
| `0x80–0xFF** | **Reserved** | Future expansion | Unallocated |

### 2.3 Key allocation rules

1. Every allocated key must be documented in this table.
2. No two ranges may overlap. Overlap is a spec bug, caught at plan time.
3. Key `0x00` is always invalid (reserved sentinel).
4. Ranges `0x01–0x0F` are legacy/shell-owned; all new ranges start at `0x10+`.
5. `0x01` stays in shell legacy range for E2. Migration to `0x30+` is deferred
   to E4 when schema versioning is introduced.
6. No raw paths, no string keys, no user-content-derived keys.
7. Key values are opaque `u32` identifiers. No semantic encoding in the key
   number (key `0x01` is not "key 1" in a sequence; it is an opaque ID).

---

## 3. Value Envelope

### 3.1 Current value model

Current sexstore stores a single `u64` per key. The scene appearance blob
packs 8 bytes:

```
Byte 0: magic      = 0xAC
Byte 1: version    = 0x01
Byte 2: preset_idx (0..3)
Byte 3: chrome_flags
Byte 4: accessibility_flags
Byte 5: reserved
Byte 6: reserved
Byte 7: checksum   = XOR(byte0..byte6)
```

### 3.2 Proposed E2 value envelope

E2 does not change the value format — the `u64` max of 8 bytes is a hard
constraint of the current PDX IPC (`arg1` is 8 bytes). But E2 *specifies*
the envelope pattern that all future values should follow:

```
[u8; 8] packed u64 envelope:

  Byte 0: type_class   — discriminator for value interpretation
  Byte 1: version      — schema version for this type_class
  Byte 2..6: payload   — type-specific data (5 bytes)
  Byte 7: checksum     — XOR(byte0..byte6) or CRC8

type_class values:
  0x00 = reserved/invalid
  0x01 = scene appearance settings  (current 0xAC → remap to 0x01)
  0x02 = audio policy flags         (future)
  0x03 = input device config        (future)
  0x04 = admin flag byte            (future)
  0x05–0xFF = reserved for future type classes

Constraints:
  - value must fit in 8 bytes total (u64)
  - no cross-PD pointers
  - no variable-length heap strings
  - no embedded file paths or raw paths
  - checksum is mandatory — caller must validate before use
```

### 3.3 E2 migration for key `0x01`

The current `0x01` value uses `magic=0xAC`. This is compatible with the
proposed envelope (type_class `0x01` would replace `magic`). E2 does **not**
change the stored value — the spec merely notes that when schema versioning
(E4) is implemented, the envelope should be enforced uniformly across all keys.

### 3.4 What is NOT stored in values

- ❌ No file paths or raw path strings
- ❌ No user document content
- ❌ No app-provided names or labels
- ❌ No cross-PD virtual addresses or pointers
- ❌ No heap-allocated data
- ❌ No unbounded strings

---

## 4. Operation Model

### 4.1 Current operations

| Opcode | Name | arg0 | arg1 | Reply |
|--------|------|------|------|-------|
| `0xB0` | `OP_KV_GET` | key (u32) | — | stored u64 (0 = not found) |
| `0xB1` | `OP_KV_PUT` | key (u32) | val (u64) | `0x00` = ok, `0x02` = full |

### 4.2 E2 allowed operations

E2 keeps the same two operations. No new opcodes.

| Operation | Allowed in E2? | Notes |
|-----------|---------------|-------|
| GET by key | ✅ Allowed | Same as current |
| PUT key/value | ✅ Allowed | Same as current |
| DELETE | ❌ Deferred to E6 | Tombstone semantics needed |
| LIST/ENUM all keys | ❌ STOP FIRST | Must be bounded and authority-gated |
| MULTI-GET | ❌ Deferred | Requires multi-slot or chunked IPC |
| ADMIN (table repair) | ❌ Deferred to E6+ | StoreAdmin capability |

### 4.3 GET semantics (specified)

```
Caller: pdx_call(SLOT_SEXSTORE, OP_KV_GET, key_u32, 0, 0)

sexstore behavior:
  1. If key == 0 → reply(KV_INVALID_KEY, 0); [sexstore.key.invalid]
  2. If key not found → reply(KV_NOT_FOUND, 0); [sexstore.get.reject] reason=not_found
  3. If key found, value valid → reply(KV_OK, value); [sexstore.get.allow]
  4. If key found, value checksum mismatch → reply(KV_CORRUPT, 0); [sexstore.corrupt.detect]

Caller responsibility:
  - Validate reply status before using value.
  - On KV_NOT_FOUND or KV_CORRUPT: use built-in defaults. Never fatal.
  - On KV_INVALID_KEY: spec bug — fix caller.
```

### 4.4 PUT semantics (specified)

```
Caller: pdx_call(SLOT_SEXSTORE, OP_KV_PUT, key_u32, value_u64, 0)

sexstore behavior:
  1. If key == 0 → reply(KV_INVALID_KEY, 0); [sexstore.key.invalid]
  2. If value checksum fails (internal validation) → reply(KV_INVALID_VALUE, 0); [sexstore.value.invalid]
  3. If key exists → update in-place; reply(KV_OK, 0); [sexstore.put.allow]
  4. If key not found, slot available → insert; reply(KV_OK, 0); [sexstore.put.allow]
  5. If key not found, table full → reply(KV_FULL, 0); [sexstore.put.reject] reason=full

Caller responsibility:
  - Fire-and-forget: do not block on reply.
  - On KV_FULL: drop silently — non-fatal.
  - On KV_INVALID_KEY or KV_INVALID_VALUE: fix caller.
```

### 4.5 Future DELETE semantics (E6 spec placeholder)

```
Caller: pdx_call(SLOT_SEXSTORE, OP_KV_DEL, key_u32, 0, 0)

sexstore behavior:
  1. If key == 0 → reply(KV_INVALID_KEY, 0)
  2. If key not found → reply(KV_NOT_FOUND, 0) — idempotent
  3. If key found → set tombstone flag; reply(KV_OK, 0); [sexstore.tombstone]

Tombstone semantics:
  - Tombstoned entries excluded from GET.
  - Space may be reclaimed on insert under pressure (LRU or FIFO).
  - StoreAdmin may explicitly compact.
  - No POSIX unlink — delete is a logical tombstone, not physical removal.
```

---

## 5. Reply/Error Model

### 5.1 Status codes

| Code | Name | Meaning | GET context | PUT context |
|------|------|---------|-------------|-------------|
| `0x00` | `KV_OK` | Success | Value follows | Stored |
| `0x01` | `KV_NOT_FOUND` | Key has no entry | Use defaults | N/A |
| `0x02` | `KV_FULL` | Table full | N/A | Drop silently |
| `0x03` | `KV_INVALID_KEY` | key == 0 or reserved | Spec bug | Spec bug |
| `0x04` | `KV_INVALID_VALUE` | Value checksum/format fail | N/A | Fix caller |
| `0x05` | `KV_DENIED` | Caller lacks capability | E3+ | E3+ |
| `0x06` | `KV_CORRUPT` | Entry integrity failure | Use defaults | N/A |
| `0x07` | `KV_STALE_GENERATION` | Generation counter mismatch | E6+ | E6+ |
| `0x08` | `KV_UNSUPPORTED_OP` | Opcode not recognized | N/A | N/A |
| `0xFF` | `KV_INTERNAL_ERROR` | sexstore internal failure | Rare | Rare |

### 5.2 Reply mechanism

Current: `kv_reply()` via syscall 29 (`SYSCALL_PDX_REPLY`):
- `rax = 29`, `rdi = target_pd`, `rsi = value`
- Caller reads via `pdx_listen_raw(0)` → `msg.type_id == 0x1`, `msg.arg0 == value`

E2 keeps this mechanism unchanged. Future phases may promote to a typed
reply format (status + value in separate fields), but E2 spec assumes the
current single-u64 reply.

### 5.3 Caller-side error handling matrix

| Reply | Caller action | Logged? | Fatal? |
|-------|---------------|---------|--------|
| `KV_OK` | Use value | Optional | No |
| `KV_NOT_FOUND` | Use defaults | `[sexstore.get.reject]` | No |
| `KV_FULL` | Drop silently | `[sexstore.put.reject]` | No |
| `KV_INVALID_KEY` | Fix caller (spec error) | `[sexstore.key.invalid]` | No |
| `KV_INVALID_VALUE` | Fix caller (pack bug) | `[sexstore.value.invalid]` | No |
| `KV_DENIED` | Do not retry | `[sexstore.get/put.reject]` | No |
| `KV_CORRUPT` | Use defaults, log warning | `[sexstore.corrupt.detect]` | No |
| `KV_UNSUPPORTED_OP` | Fix caller (version mismatch) | `[sexstore.reply]` | No |

> **Invariant:** All storage errors are non-fatal. The system always boots
> and runs with built-in defaults if storage is absent, corrupt, or denied.

---

## 6. Proof Marker Plan

### 6.1 Current markers

| Marker | Budget | Location |
|--------|--------|----------|
| `[sexstore.kv.put] key=N ok=0\|1` | 32 | `sexstore/main.rs:113` |
| `[sexstore.kv.get] key=N hit=0\|1` | 32 | `sexstore/main.rs:137` |

### 6.2 Proposed E2+ markers (spec only — not implemented in E2)

E2 specifies the marker format. Implementation is deferred to E7
(Deterministic RAM Store Proofs).

```
[sexstore.put.allow]       seg=N key=K caller=C
[sexstore.put.reject]      seg=N key=K reason=full|denied|invalid_key|invalid_value
[sexstore.get.allow]       seg=N key=K caller=C
[sexstore.get.reject]      seg=N key=K reason=not_found|denied|corrupt
[sexstore.key.invalid]     seg=N key=K caller=C reason=zero_key|reserved
[sexstore.value.invalid]   seg=N key=K caller=C reason=checksum_fail|format_error
[sexstore.reply]           seg=N target=T status=S
[sexstore.corrupt.detect]  seg=N key=K reason=checksum_mismatch action=tombstone|skip
[sexstore.tombstone]       seg=N key=K caller=C reason=delete|corrupt|admin (E6+)
```

### 6.3 Marker fields

| Field | Type | Purpose |
|-------|------|---------|
| `seg` | u32 | Sequence ID (monotonic per sexstore boot) |
| `key` | u32 | Target key (hex or decimal) |
| `caller` | u32 | Caller PD ID (from msg.caller_pd) |
| `reason` | string enum | Structured failure reason |
| `status` | string enum | KV_OK / KV_NOT_FOUND / etc. |

### 6.4 Budget

E2 does not implement markers. When E7 implements them, they should be
budgeted per-boot (e.g., 1024 per type) and rate-limited to avoid log
flood. Sequence IDs must never repeat within a boot session.

---

## 7. E3/E4 Implementation Boundary

### 7.1 What E3 adds (capability policy — next phase)

- **StoreCapability kinds:** StoreRead, StoreWrite, StoreDelete, StoreAdmin
- **Caller validation:** sexstore checks caller PD identity before serving
- **Range ownership:** sexstore validates that caller owns the target key range
- **Deny markers:** `[sexstore.put.reject] reason=denied` when cap check fails
- **Collaring model:** how Collar integrates with storage capabilities

E3 is also docs/spec only — no code changes.

### 7.2 What E4 adds (schema/version — after E3)

- **StorageVersion:** monotonic version counter in sexstore metadata
- **Migration rules:** forward migration paths for schema changes
- **Value envelope enforcement:** all values must match the type_class/version
- **Key migration:** move `0x01` → `0x30+` range with schema versioning

E4 may require sexstore code changes (table metadata version field).

### 7.3 What remains after E2

| Phase | Scope | Code changes? |
|-------|-------|---------------|
| E2 (this doc) | Protocol spec | ❌ No |
| E3 | Capability policy spec | ❌ No (docs only) |
| E4 | Schema/version spec + implementation | ✅ sexstore code |
| E5 | Corruption handling | ✅ sexstore code |
| E6 | Delete/tombstone | ✅ sexstore code |
| E7 | Deterministic proofs | ✅ sexstore code |
| E8 | Privacy redaction | ✅ sexstore code |

---

## 8. Proof Scenarios (Spec)

### 8.1 Current scenarios (E2-verifiable by code review)

| Scenario | Expected | Marker |
|----------|----------|--------|
| GET existing key `0x01` | Returns stored u64 | `[sexstore.kv.get] hit=1` |
| GET non-existent key `0xFF` | Returns 0 (not found) | `[sexstore.kv.get] hit=0` |
| PUT new key `0x02` | Stored, returns KV_OK | `[sexstore.kv.put] ok=1` |
| PUT update key `0x01` | Overwrites, returns KV_OK | `[sexstore.kv.put] ok=1` |
| PUT when table full | Returns KV_PUT_FULL | `[sexstore.kv.put] ok=0` |

### 8.2 Future scenarios (E4+)

| Scenario | Expected | Phase |
|----------|----------|-------|
| GET corrupt entry | KV_CORRUPT, use defaults | E5 |
| PUT invalid checksum | KV_INVALID_VALUE | E4 |
| Caller without StoreCapability | KV_DENIED | E3 |
| DELETE existing key | Tombstone set, GET returns not_found | E6 |
| DELETE non-existent key | KV_NOT_FOUND (idempotent) | E6 |
| Schema version mismatch | Safe fallback, migration or skip | E4 |

---

## 9. STOP FIRST Conditions

| Condition | Action |
|-----------|--------|
| Raw path/file design in storage protocol | **STOP** — no paths, no filenames |
| App direct storage caps without E3 capability policy | **STOP** — only silk-shell in E2 |
| Cross-PD pointer payloads in values | **STOP** — no pointers, no virt addresses |
| Unbounded strings/heap in values | **STOP** — fixed 8-byte u64 only |
| Shared-memory backing buffer for sexstore | **STOP** — PDX IPC only |
| Disk/durable backend writes before E9 gate | **STOP** — RAM-only V1 |
| Linen/Quil persistence before F-track gates | **STOP** — E2 does not enable document storage |
| sex-pdx/kernel ABI edits before approved implementation phase | **STOP** — no ABI changes in E2 |
| ENUM/LIST operation without bounded + authority-gated design | **STOP** — deferred to E6+ |
| POSIX unlink semantics for delete | **STOP** — delete is logical tombstone, not unlink |
| Value > 8 bytes without chunked IPD design | **STOP** — u64 max for current PDX |

### 9.1 E2 gate status

> ✅ **E2 passes its own gate.** This document is docs/spec only.
> No ABI/code edits, no durable backend, no Linen/Quil persistence,
> no raw paths.

---

## 10. Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | Current KV implementation (148 lines) |
| `servers/silk-shell/src/main.rs` | Only client; local opcodes, pack/unpack, load/save helpers |
| `crates/sex-pdx/src/lib.rs` | `SLOT_SEXSTORE = 10` |
| `kernel/src/init.rs` | sexstore spawn (domain 8), cap grant |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Track E master plan, ladder, object model |
| `docs/handoff/E1_STORAGE_BOUNDARY_AUDIT_V1.md` | E1 audit — current state this spec builds on |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V implementation handoff |
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API design that KV_RAM implemented |
| `docs/handoff/SEXSTORE_KERNEL_ENABLE_V1.md` | Kernel spawn + cap grant handoff |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Silk-shell persistence implementation |

---

## 11. Next Phase: E3_STORAGE_CAPABILITY_POLICY_V1

Specify StoreCapability kinds, caller validation, range ownership checks,
capability deny markers, and Collar integration model.

E3 is docs/spec only — no code changes. Dependencies:
- E2 (this doc) — key namespace, authority model for validation
- Collar capability model (Track I) — StoreCapability types

---

## Appendix A: Current Protocol Quick Reference

```
Slot:        SLOT_SEXSTORE = 10
GET opcode:  OP_KV_GET = 0xB0
PUT opcode:  OP_KV_PUT = 0xB1
Key size:    u32 (4 bytes)
Value size:  u64 (8 bytes) — hard limit
Reply:       syscall 29 (SYSCALL_PDX_REPLY) → msg.type_id == 0x1, msg.arg0

Current status codes (local consts, not in sex-pdx):
  KV_PUT_OK:   u64 = 0x00
  KV_PUT_FULL: u64 = 0x02

Current key allocation:
  0x01 = silk-shell scene appearance settings
  0x00 = invalid/reserved

Listen: pdx_listen_raw(0) — self message ring
Table:  static mut [KvSlot; 16] — 256 bytes, no heap
Lookup: linear scan (O(n), n <= 16)
Insert: update in-place or first-free
Full:   returns KV_PUT_FULL (0x02) — caller drops silently
```

## Appendix B: Current vs Proposed Error Code Mapping

| Current | Proposed | Notes |
|---------|----------|-------|
| `0x00` (KV_PUT_OK) | `0x00` (KV_OK) | Unchanged semantics |
| — | `0x01` (KV_NOT_FOUND) | New — explicit not-found vs zero-value distinction |
| `0x02` (KV_PUT_FULL) | `0x02` (KV_FULL) | Unchanged |
| — | `0x03` (KV_INVALID_KEY) | New — key == 0 guard |
| — | `0x04` (KV_INVALID_VALUE) | New — value checksum/format fail |
| — | `0x05` (KV_DENIED) | E3+ capability denial |
| — | `0x06` (KV_CORRUPT) | E5+ corruption detection |
| — | `0x07` (KV_STALE_GENERATION) | E6+ generation counter |
| — | `0x08` (KV_UNSUPPORTED_OP) | New — unknown opcode reply |
| — | `0xFF` (KV_INTERNAL_ERROR) | New — sexstore internal fault |

> E2 does not implement these codes. They are spec-only.
> Current code continues to use `0x00` = ok and `0x02` = full.
