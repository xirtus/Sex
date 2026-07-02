# E5_STORAGE_GENERATION_TOMBSTONE_SPEC_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E5 only if docs-only and it does not sneak in DELETE/tombstone code before opcode/status ABI boundaries are clear."

---

## Summary

Specification of generation/version/tombstone behavior for sexstore's RAM-only K/V. Documents current E4 status, identifies the status-code collision with the E2 spec (KV_DENIED = 0x01 vs KV_NOT_FOUND = 0x01), proposes a generation-counter model for per-slot write tracking, a tombstone state model for future DELETE semantics, and the ABI boundaries that must be resolved before E6 implementation can proceed.

Docs-only. No code changed.

---

## 1. Current E4 Status Summary

### 1.1 Caller policy

- Only silk-shell (domain 3) on shell legacy range (`0x01–0x0F`) is authorized
- Key `0x00` → `KV_INVALID_KEY` (0x03)
- Keys `0x10+` (reserved ranges) → `KV_DENIED` (0x01)
- Non-shell callers (domain != 3) on shell range → `KV_DENIED` (0x01)

### 1.2 Key validation

- `store_key_owner_class()`: returns 0 (invalid) for key 0x00, 1 (shell) for `0x01–0x0F`, 2 (reserved) for `0x10+`
- `store_cap_allowed()`: returns true only if class == 1 AND caller_pd == 3

### 1.3 Value envelope validation

- `store_validate_value()`: key `0x01` PUTs validated for magic=0xAC, version=0x01, XOR checksum
- All other keys pass validation unconditionally (validation deferred to their schema definition)
- No value validation on GET (callee validates the returned blob)

### 1.4 Current reply/status mapping

| E4 code | Constant | Meaning | Reply value |
|---------|----------|---------|-------------|
| `0x00` | `KV_PUT_OK` | PUT succeeded | Status-only |
| `0x01` | `KV_DENIED` | Caller lacks cap | Status-only |
| `0x02` | `KV_PUT_FULL` | Table full | Status-only |
| `0x03` | `KV_INVALID_KEY` | key == 0x00 | Status-only |
| `0x04` | `KV_INVALID_VALUE` | Value envelope invalid | Status-only |
| stored u64 | (value) | GET success | Value-only |
| `0` | (not found) | GET on missing key | Value-only |

### 1.5 KvSlot structure

```rust
struct KvSlot {
    used: u8,  // 0 = empty, 1 = occupied
    key:  u32, // u32 key identifier
    val:  u64, // 8-byte packed value
}
// 16 × 16 = 256 bytes total. Static allocation. No heap.
```

---

## 2. Status-Code Mismatch Audit

### 2.1 The collision

The E2 spec (`docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md`, §5.1) proposed:

| Code | Name | Purpose |
|------|------|---------|
| `0x01` | `KV_NOT_FOUND` | Key has no entry (GET) |
| `0x05` | `KV_DENIED` | Caller lacks capability |

But E4 implemented:

| Code | Name | Purpose |
|------|------|---------|
| `0x01` | `KV_DENIED` | Caller lacks capability |
| (none) | `KV_NOT_FOUND` | GET on missing key returns 0 |

### 2.2 Root cause

The reply mechanism (`kv_reply()` via syscall 29) can only send a single `u64`.
This single value must serve **both** as a status code (for denied/invalid
replies) and as the stored value (for GET success). There is no way to
return `(status, value)` in a single u64.

E4 chose to use `0x01` for `KV_DENIED` because:
- `0x00` was already taken by `KV_PUT_OK`
- `0x02` was already taken by `KV_PUT_FULL`
- GET success returns the stored u64 (which can be any value, including 0)

### 2.3 Why it is safe (but wrong)

Silk-shell handles GET replies via `unpack_scene_settings_blob()`, which checks
for `magic == 0xAC` (byte 0) and `version == 0x01` (byte 1). Any reply value
that doesn't match (including `0x01` = KV_DENIED, `0x03` = KV_INVALID_KEY,
or plain `0` = not found) fails the check and is treated as "not found/corrupt"
— silk-shell uses built-in defaults. **No functional harm**, but the error
reported to the operator is misleading: a capability denial looks like a
corrupt or missing entry.

PUT calls from silk-shell are fire-and-forget — the reply value is ignored
entirely — so the status collision has no effect on PUT paths.

### 2.4 What must be fixed

Before E6 can add DELETE/tombstone with proper status codes, the reply ABI
must be expanded. Two options:

**Option A: Compact status encoding**
Pack status into the reply u64 with a discriminator bit:
```
bit 63 = 0 → reply is a stored value (GET success)
bit 63 = 1 → reply is a status code (bits 0..6 = code, bit 7..62 = reserved)
```
Status codes 0x00–0x7F fit in 7 bits. This preserves the single-u64 ABI
while distinguishing status from value.

**Option B: Two-reply protocol**
Use two separate syscall 29 replies: first sends status, second sends value.
Caller reads both from the reply ring. Requires caller-side state machine.

**Option C: syscall 29 ABI extension**
Add a second reply register (e.g., rdx = status, rsi = value). Requires
kernel change (STOP FIRST for E5 — deferred to E6).

### 2.5 E5 recommendation

**Document only — no implementation.** E5 recommends Option A (compact status
encoding) for E6 implementation because it requires no kernel changes, no
sex-pdx changes, and no caller-side state machine. The discriminator bit is
backward-compatible: existing stored values (which never have bit 63 set, since
the current magic is 0xAC and version is 0x01, both in the lower byte) are
correctly interpreted as values.

### 2.6 Mapped vs actual status codes

| E2 spec code | E2 name | E4 actual | E5+ recommended | Notes |
|---|---|---|---|---|
| `0x00` | KV_OK | KV_PUT_OK (0x00) | `0x00` (OK) | Unchanged |
| `0x01` | KV_NOT_FOUND | KV_DENIED (0x01) | `0x01` (NOT_FOUND) | **Collision** — E6 must remap KV_DENIED to `0x05` |
| `0x02` | KV_FULL | KV_PUT_FULL (0x02) | `0x02` (FULL) | Unchanged |
| `0x03` | KV_INVALID_KEY | KV_INVALID_KEY (0x03) | `0x03` (INVALID_KEY) | Matches E2 |
| `0x04` | KV_INVALID_VALUE | KV_INVALID_VALUE (0x04) | `0x04` (INVALID_VALUE) | Matches E2 |
| `0x05` | KV_DENIED | (collision — not used) | `0x05` (DENIED) | Remapped in E6 |
| `0x06` | KV_CORRUPT | (not implemented) | `0x06` (CORRUPT) | Deferred to E6+ |
| `0x07` | KV_STALE_GENERATION | (not implemented) | `0x07` (STALE_GEN) | Deferred to E6+ |
| `0x08` | KV_UNSUPPORTED_OP | (not implemented) | `0x08` (UNSUPPORTED) | Deferred to E6+ |
| `0xFF` | KV_INTERNAL_ERROR | (not implemented) | `0xFF` (INTERNAL_ERR) | Deferred to E6+ |

**E5 spec requires:** E6 MUST remap KV_DENIED from `0x01` to `0x05` and
introduce KV_NOT_FOUND as `0x01`. This is the first code change in E6.

---

## 3. Generation Model

### 3.1 Purpose

A generation counter per slot enables:
1. **Write ordering detection** — detect whether a newer write has occurred since a previous read
2. **Stale write rejection** — if caller supplies expected generation, reject writes that would overwrite a newer generation
3. **Tombstone tracking** — tombstone events increment generation, making tombstones observable
4. **Slot reuse ordering** — when a tombstoned slot is reclaimed, generation resets (or increments beyond all previous values)

### 3.2 Per-slot generation counter

Extend `KvSlot` with a `generation: u8` field:

```rust
/// E5 proposed slot structure.
struct KvSlot {
    used:       u8,   // 0 = empty, 1 = active, 2 = tombstoned
    generation: u8,   // incremented on every write, tombstone, and reclaim
    key:        u32,  // u32 key identifier (0 for empty/tombstoned)
    val:        u64,  // 8-byte packed value
}
// 16 × (1+1+2pad+4+8) = 16 × 16 = 256 bytes. Same total size.
```

### 3.3 Generation rules

1. **Initial value:** 1 (not 0). Generation 0 means "never written."
2. **Increment on PUT update:** Each PUT to an existing key increments the generation of that slot.
3. **Increment on PUT insert:** Each PUT to a new/free slot sets generation to 1 (first write).
4. **Increment on tombstone:** Each DELETE that tombstone-flags a slot increments generation.
5. **Increment on reclaim:** When a tombstoned slot is reclaimed for a new PUT, generation increments.
6. **Monotonic:** Generation always increases, never decreases. Wraps from 255 to 1 (0 is never used).
7. **Slot reuse with generation preserved:** When a slot transitions empty→active, tombstoned→reclaimed, or active→tombstoned, generation increments.

### 3.4 Caller-supplied generation (deferred to E6+)

E5 does NOT define a caller-supplied generation protocol. The generation counter
is internal to sexstore in E5. Callers cannot yet pass an expected generation
for stale-write rejection. This is deferred to E6+ when the opcode ABI is
expanded (new PUT variant or additional arg).

**E5 spec decouples generation tracking (internal) from generation validation
(external).** Tracking is immediate; validation is deferred.

### 3.5 Packed-value generation (not recommended)

An alternative approach would pack generation into the 8-byte value envelope
(e.g., use byte 5 or 6 for generation). This is NOT recommended because:
- Generation is slot metadata, not value data
- Schema-versioned values need those bytes for payload
- Packing creates coupling between schema and slot management
- Separating generation into the slot struct keeps slot management orthogonal to value schema

### 3.6 Proof markers

```
[sexstore.generation.bump] key=K slot=N gen=G operation=put|tombstone|reclaim
```

Budget: 64 per boot (64 PUTs across 16 slots = 4 full table rotations).

```
[sexstore.generation.stale] caller=C key=K slot=N expected=E actual=A
```

Budget: 16 per boot. E6+ only (requires caller-supplied generation protocol).

---

## 4. Tombstone Model

### 4.1 Purpose

Tombstone semantics allow key deletion without immediate physical removal.
This preserves the audit trail (the key was present and then deleted) and
avoids slot reclamation races. Tombstoned entries are excluded from GET
results but retain their key, value, and generation for diagnostic purposes.

### 4.2 Slot states

| State | `used` | Value accessible? | Can be PUT? | Can be tombstoned? | Generation behavior |
|-------|--------|-------------------|-------------|-------------------|---------------------|
| **Empty** | `0` | N/A | Yes (insert) | No | Never written |
| **Active** | `1` | Yes (GET returns it) | Yes (update) | Yes (DELETE) | Increments on update |
| **Tombstoned** | `2` | No (GET returns NOT_FOUND) | Yes (revive) | No (already tombstoned) | Incremented on tombstone, increments on reclaim |
| **Corrupt** | `3` | No (GET returns CORRUPT) | Yes (overwrite) | Yes (tombstone) | Increments on any state change |

### 4.3 Tombstone operations

**DELETE (E6 — spec only):**
```
Caller: pdx_call(SLOT_SEXSTORE, OP_KV_DEL, key_u32, 0, 0)
Sexstore:
  1. If key == 0 → reply(KV_INVALID_KEY); [sexstore.key.invalid]
  2. If caller lacks cap → reply(KV_DENIED); [sexstore.policy.deny]
  3. If key found (used == 1) → used = 2, generation += 1
     reply(KV_OK, generation);
     [sexstore.tombstone.record] key=K slot=N gen=G
  4. If key tombstoned (used == 2) → reply(KV_OK, generation); [sexstore.tombstone.record] — idempotent
  5. If key not found (no slot with matching key, used==1 or 2) → reply(KV_NOT_FOUND, 0); [sexstore.get.reject]
  6. If corrupt (used == 3) → tombstone it; reply(KV_OK); [sexstore.tombstone.record] reason=corrupt
```

**GET on tombstoned key:**
```
1. If key found but used == 2 → reply(KV_NOT_FOUND, 0); [sexstore.tombstone.get] key=K
2. If key found but used == 3 → reply(KV_CORRUPT, 0); deferred to E6+
```

**PUT on tombstoned key (revive):**
```
1. If key found but used == 2 → revive: used = 1, generation += 1
   reply(KV_OK); [sexstore.tombstone.revive] key=K old_gen=G
2. Value envelope validation still applies before revive
3. Capability check still applies
```

### 4.4 Slot reclamation under pressure

When PUT cannot find an empty or active slot (table full), sexstore may
reclaim tombstoned slots:

```
Reclaim policy:
  1. Scan for tombstoned (used == 2) slot
  2. Reclaim: overwrite key, value, set used=1, generation += 1
  3. [sexstore.tombstone.reclaim] key=K slot=N old_gen=G
  4. If no tombstoned slot either → KV_PUT_FULL
```

### 4.5 Tombstone without opcode

E5 does NOT add a DELETE opcode. Tombstone state transitions only happen
within existing PUT/GET operations when E6 adds DELETE. Until then:
- No path to set `used = 2` exists in E4 code
- All slots stay in Empty (0) or Active (1) states
- This is intentional: tombstones are spec-only until the opcode boundary is resolved

### 4.6 No durable deletion

Tombstoned entries remain in RAM. No space is freed. No POSIX-style unlink.
Tombstone is a logical state, not physical removal. Logging/audit can inspect
tombstoned entries (StoreAdmin only).

### 4.7 Proof markers

```
[sexstore.tombstone.record]  key=K slot=N gen=G reason=delete|corrupt [budget: 32]
[sexstore.tombstone.get]     key=K slot=N gen=G            [budget: 32]
[sexstore.tombstone.revive]  key=K slot=N old_gen=G        [budget: 16]
[sexstore.tombstone.reclaim] key=K slot=N old_gen=G        [budget: 8]
[sexstore.delete.unsupported] caller=C op=0xNN             [budget: 8]
```

---

## 5. DELETE Operation Boundary

### 5.1 Current status

- **No DELETE opcode exists.** Only OP_KV_GET (0xB0) and OP_KV_PUT (0xB1).
- **No tombstone path.** The `used` field is binary (0 = empty, 1 = active).
- **No way to delete.** Callers can only overwrite a key with a new value, or the key stays forever (until power cycle).

### 5.2 E5 spec does NOT add DELETE

E5 is docs-only. No opcodes, no code changes, no slot structure changes.

DELETE is deferred to E6 with the following prerequisites:

1. **Status-code remap must happen first** — KV_DENIED must move from 0x01 to 0x05, KV_NOT_FOUND must take 0x01 (see §2.6).
2. **Reply ABI must be decided** — E6 must choose Option A (compact status encoding) or Option B/C before DELETE can return distinct status codes.
3. **Slot struct must be extended** — `used` field must support value 2 (tombstoned).
4. **DELETE opcode must be defined** — proposed `OP_KV_DEL = 0xB2`.
5. **Capability check must be extended** — DELETE requires StoreWrite (same as PUT) in E6; StoreDelete when capability granularity increases.
6. **Proof markers must be added** — tombstone.record, tombstone.get, tombstone.revive, tombstone.reclaim.

### 5.3 No LIST/ENUM

E5 does NOT add LIST or ENUM operations. Any proposal to enumerate keys
requires bounded design, StoreAdmin capability, and explicit proof marker
format. STOP FIRST for E5.

### 5.4 DELETE idempotency

DELETE is idempotent by spec:
- First DELETE on active key: succeeds (tombstone set)
- Second DELETE on tombstoned key: succeeds (already tombstoned — no state change)
- DELETE on missing key: succeeds (not_found is not an error)
- DELETE on corrupt key: succeeds (corrupt entry is tombstoned)

---

## 6. In-Memory Slot Model

### 6.1 Current (E4)

```rust
struct KvSlot {
    used: u8,  // 0 = empty, 1 = occupied
    key:  u32,
    val:  u64,
}
// sizeof = 16 bytes (1 + 3 padding + 4 + 8)
// 16 slots × 16 bytes = 256 bytes total
// Static mut array, no heap, no pointers
```

### 6.2 Proposed (E6+)

```rust
struct KvSlot {
    used:       u8,  // 0 = empty, 1 = active, 2 = tombstoned, 3 = corrupt
    generation: u8,  // monotonic counter, starts at 1, wraps 255→1
    pad:        [u8; 2], // padding to align key to 4 bytes (same total size)
    key:        u32,
    val:        u64,
}
// sizeof = 16 bytes (unchanged — 1+1+2+4+8)
// 16 slots × 16 bytes = 256 bytes total (unchanged)
```

The `generation` field fits into the existing padding bytes. No increase in
table size. No heap. No pointers. No raw paths. No strings.

### 6.3 Slot state transitions

```
                     ┌─────────────────────────────────────┐
                     │               EMPTY                 │
                     │        used=0, gen=0, key=0        │
                     └──────────┬──────────────────────────┘
                                │ PUT (insert)
                                ▼
                     ┌─────────────────────────────────────┐
            ┌────────│           ACTIVE                    │
            │        │   used=1, gen++, key=K, val=V      │
            │        └──────────┬──────────────────────────┘
            │                   │                    │
            │ PUT (update)      │ DELETE             │ CORRUPT detected
            │ gen++             ▼                    ▼
            │        ┌──────────────────┐  ┌──────────────────┐
            │        │   TOMBSTONED     │  │    CORRUPT       │
            └────────│  used=2, gen++   │  │  used=3, gen     │
                     │  key=K, val=V   │  │  key=K, val=?    │
                     └──────────┬──────┘  └────────┬─────────┘
                                │ PUT (revive)     │ DELETE (tombstone)
                                │ gen++            │ gen++
                                ▼                  ▼
                          ACTIVE             TOMBSTONED
```

### 6.4 Corrupt state (deferred to E6+)

The `used = 3` (corrupt) state is defined here but not implemented until
E6+ when corruption detection (checksum over slot metadata) is added. E5
notes it but does not require it.

---

## 7. Proof Marker Plan

### 7.1 Current markers (E4)

| Marker | Budget | Source |
|--------|--------|--------|
| `[sexstore.kv.put] key=N ok=0|1` | 32 | E0 baseline |
| `[sexstore.kv.get] key=N hit=0|1` | 32 | E0 baseline |
| `[sexstore.policy.allow] caller=C key=K op=PUT|GET` | 32 | E4 |
| `[sexstore.policy.deny] caller=C key=K class=...` | 32 | E4 |
| `[sexstore.key.invalid] caller=C key=0x00` | 8 | E4 |
| `[sexstore.value.invalid] caller=C key=K` | 8 | E4 |
| `[sexstore.reply.error] caller=C op=0xNN` | 8 | E4 |

### 7.2 Proposed E5+ new markers (spec only — not implemented)

| Marker | Budget | When | Phase |
|--------|--------|------|-------|
| `[sexstore.generation.bump] key=K slot=N gen=G op=put|tombstone|reclaim` | 64 | PUT, DELETE, reclaim | E6 |
| `[sexstore.generation.stale] caller=C key=K slot=N expected=E actual=A` | 16 | Stale-write rejected | E6+ |
| `[sexstore.tombstone.record] key=K slot=N gen=G reason=delete|corrupt` | 32 | DELETE | E6 |
| `[sexstore.tombstone.get] key=K slot=N gen=G` | 32 | GET on tombstoned key | E6 |
| `[sexstore.tombstone.revive] key=K slot=N old_gen=G` | 16 | PUT on tombstoned key | E6 |
| `[sexstore.tombstone.reclaim] key=K slot=N old_gen=G` | 8 | Slot reclamation | E6 |
| `[sexstore.delete.unsupported] caller=C op=0xNN` | 8 | Unknown del opcode | E5+ |
| `[sexstore.status.mapping] code=C name=N action=remap|add|remove` | 8 | Status code change | E6 |

### 7.3 Budget total

| Phase | Marker count | Total budget |
|-------|-------------|--------------|
| E0 (baseline) | 2 | 64 |
| E4 (added) | 5 | 88 |
| E6 (proposed new) | 8 | 184 |
| **Grand total** | **15** | **336** |

---

## 8. E6 Implementation Options

### 8.1 Option A: Minimal DELETE (recommended for E6)

```
Changes:
  1. Remap status codes: KV_DENIED = 0x01 → 0x05, KV_NOT_FOUND = 0x01
  2. Extend KvSlot: add generation: u8, used values 0/1/2
  3. Add OP_KV_DEL = 0xB2
  4. Implement DELETE dispatch arm
  5. Add generation bump on every PUT and DELETE
  6. Add proof markers for generation bump and tombstone events
  7. Compact status encoding (Option A, §2.4): bit 63 discriminator

No kernel changes. No sex-pdx changes. No caller-supplied generation protocol.
No LIST/ENUM. No corrupt state (deferred to E7+).
```

### 8.2 Option B: DELETE + generation validation

```
Adds to Option A:
  5a. caller-supplied generation in DELETE arg0 (optional)
  5b. caller-supplied generation in PUT variant (new opcode OP_KV_PUT_GEN = 0xB3)
  5c. KV_STALE_GENERATION (0x07) returned when caller generation mismatches

Requires: opcode ABI change (new PUT variant). Caller must track generations.
```

### 8.3 Option C: DELETE + full reply ABI expansion

```
Adds to Option B:
  2a. Two-reply protocol (Option B, §2.4): status + value via two syscall 29 replies
  Or:
  2b. Kernel ABI change (Option C, §2.4): additional reply register

Requires: kernel ABI change or caller-side state machine.
STOP FIRST — requires review.
```

**E5 recommends Option A for E6.** It provides DELETE/tombstone with minimal
code changes and no kernel/ABI impact. Caller-supplied generation and full
reply ABI expansion can follow in E6+ or E7.

---

## 9. E5 STOP FIRST Conditions

| Condition | Status |
|-----------|--------|
| Adding DELETE opcode code before E6 boundaries clear | ✅ Not done — DELETE is spec-only |
| Changing KvSlot struct in code | ✅ Not done — slot model is spec-only |
| Adding generation counter to running code | ✅ Not done — generation is spec-only |
| Any kernel/ABI edits | ✅ Not done — no code changed |
| Any sex-pdx ABI edits | ✅ Not done — no code changed |
| Any durable backend writes | ✅ Not done — RAM-only |
| Any app PD storage caps | ✅ Not done — only silk-shell |
| Any Linen/Quil persistence | ✅ Not done — not touched |
| Any raw path or LIST/ENUM | ✅ Not done — not proposed |
| Any POSIX unlink semantics | ✅ Not done — tombstone model is logical, not unlink |
| Snoaking in DELETE code before review | ✅ Not done — E5 is docs-only |
| Overlapping E6 implementation into E5 | ✅ Not done — clear E6 boundary |
| Caller-supplied generation protocol before ABI design | ✅ Not done — deferred to E6+ |
| Any change to reply ABI without STOP FIRST review | ✅ Not done — Option A/B/C documented for E6, not implemented |

> ✅ **E5 passes its own gate.** Docs-only. No code changes. Status-code
> collision documented. DELETE/tombstone model specified. E6 boundaries clear.

---

## 10. Ready/Not Ready for E6

### 10.1 Ready — E5 has defined:

1. **Status-code collision** — mapped, documented, remap plan for E6
2. **Slot state model** — empty/active/tombstoned/corrupt with transition diagram
3. **Generation counter design** — per-slot u8, monotonic, increment on every state change
4. **Tombstone semantics** — GET returns NOT_FOUND, PUT revives, DELETE idempotent, reclaim under pressure
5. **DELETE opcode boundary** — OP_KV_DEL = 0xB2, capability-gated, proof-marked
6. **Reply ABI expansion options** — Option A (compact encoding) preferred for E6
7. **Proof marker set** — 8 new markers specified
8. **E6 implementation options** — Option A (minimal DELETE) recommended

### 10.2 Not ready — E6 must resolve:

1. **Status-code remap** — KV_DENIED from 0x01 to 0x05, introduce KV_NOT_FOUND as 0x01. This is a semantic breaking change: any external observer of the reply ABI must be updated.
2. **Compact status encoding design** — exact bit layout for reply discriminator
3. **Caller compatibility** — silk-shell's `unpack_scene_settings_blob()` will still work (magic check), but status-code path must be verified
4. **KvSlot extension** — add `generation: u8` field, extend `used` to [0,1,2]
5. **DELETE opcode dispatch** — new arm in sexstore match statement
6. **Generation bump integration** — modify existing PUT code to increment generation

### 10.3 E6 prerequisites summary

```
Status-code remap:     REQUIRED (E6 first task)
Reply ABI expansion:   REQUIRED (use Option A — compact encoding)
KvSlot extension:      REQUIRED (add generation, used values)
DELETE opcode:         REQUIRED (OP_KV_DEL = 0xB2)
Proof markers:         REQUIRED (8 new types)
Kernel changes:        NOT REQUIRED (Option A)
sex-pdx changes:       NOT REQUIRED (opcodes stay local)
Caller gen protocol:   OPTIONAL (deferred to E6+)
Sexstore opcode consts: REQUIRED (OP_KV_DEL = 0xB2)
Silk-shell opcode consts: REQUIRED (mirror OP_KV_DEL)
```

---

## Appendix A: E4-to-E6 Transition Map

```
E4 status        E5 (spec)        E6 (planned)
──────────────────────────────────────────────────
KV_DENIED=0x01   Collision doc'd  KV_DENIED=0x05
(none)           KV_NOT_FOUND     KV_NOT_FOUND=0x01
KV_PUT_OK=0x00   OK=0x00           OK=0x00
KV_PUT_FULL=0x02 FULL=0x02         FULL=0x02
KV_INVALID_KEY=0x03  same           same
KV_INVALID_VALUE=0x04  same         same
(none)           (corrupt doc'd)   KV_CORRUPT=0x06
(none)           (stale doc'd)     KV_STALE_GENERATION=0x07
(none)           (unsupported doc'd) KV_UNSUPPORTED_OP=0x08
(none)           KV_INTERNAL_ERROR=0xFF
(none)           OP_KV_DEL=0xB2    OP_KV_DEL=0xB2
(struct unused)   generation: u8   generation: u8
(used: 0/1)       used: 0/1/2/3   used: 0/1/2
(no proof for gen) [gen.bump] spec'd [gen.bump] implemented
(no tombstone)    tombstone spec'd tombstone implemented
```

---

## Appendix B: Proof Scenarios

### B.1 Current E4 scenarios (verifiable by code review)

| Scenario | Expected | Marker |
|----------|----------|--------|
| Silk-shell GET key 0x01 (exists) | Returns stored u64 | `[sexstore.kv.get] hit=1` |
| Silk-shell GET key 0x01 (missing) | Returns 0 | `[sexstore.kv.get] hit=0` |
| Silk-shell PUT key 0x01 valid | Stored, KV_PUT_OK | `[sexstore.kv.put] ok=1`, `[sexstore.policy.allow]` |
| Silk-shell PUT key 0x01 corrupt | Rejected, KV_INVALID_VALUE | `[sexstore.value.invalid]` |
| Non-shell caller PUT key 0x01 | Rejected, KV_DENIED | `[sexstore.policy.deny]` |
| Any caller PUT key 0x00 | Rejected, KV_INVALID_KEY | `[sexstore.key.invalid]` |
| Unknown opcode | Reply(0) | `[sexstore.reply.error]` |

### B.2 Future E6+ scenarios (spec only)

| Scenario | Expected | Marker | Phase |
|----------|----------|--------|-------|
| GET tombstoned key | KV_NOT_FOUND (0x01) | `[sexstore.tombstone.get]` | E6 |
| GET corrupt key | KV_CORRUPT (0x06) | `[sexstore.corrupt.detect]` | E7 |
| DELETE active key | Tombstone set, KV_OK | `[sexstore.tombstone.record]`, `[sexstore.generation.bump]` | E6 |
| DELETE tombstoned key | KV_OK (idempotent) | `[sexstore.tombstone.record]` | E6 |
| DELETE missing key | KV_NOT_FOUND | `[sexstore.get.reject]` | E6 |
| PUT revive tombstoned key | Active, KV_OK | `[sexstore.tombstone.revive]`, `[sexstore.generation.bump]` | E6 |
| PUT with stale generation | KV_STALE_GENERATION | `[sexstore.generation.stale]` | E6+ |
| Table full, tombstoned slot reclaimed | Active, KV_OK | `[sexstore.tombstone.reclaim]`, `[sexstore.generation.bump]` | E6 |
| Status code remap KV_DENIED 0x01→0x05 | KV_DENIED=0x05 | `[sexstore.status.mapping]` | E6 |

---

## Appendix C: Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | Current E4 implementation — all changes will be here |
| `servers/silk-shell/src/main.rs` | Only client — `unpack_scene_settings_blob()`, `handle_sexstore_get_reply()`, PUT fire-and-forget |
| `crates/sex-pdx/src/lib.rs` | PdxMessage struct, SLOT_SEXSTORE, syscall 29 |
| `kernel/src/init.rs` | sexstore spawn + cap grant (unchanged by E5) |
| `docs/handoff/E4_STORAGE_SCHEMA_VALIDATION_V1.md` | E4 implementation — base for E5 design |
| `docs/handoff/E3_STORAGE_CAPABILITY_POLICY_SPEC_V1.md` | Cap policy — DELETE will need StoreWrite/StoreDelete |
| `docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md` | Protocol spec — status code collision with §5.1 |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Master plan — §6 storage object model, §11 tombstone, §16 deletion/tombstone |
