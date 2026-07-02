# E8_STORAGE_REDACTION_POLICY_V1

**Status:** Docs/spec only. No code changed.

**Date:** 2026-05-05

**Review gate:** "Accept E8 only if docs-only and it classifies current markers without allowing content/path/title logging."

---

## Summary

Defines storage proof-log redaction classes, classifies all current E7 sexstore markers, identifies forbidden log fields, specifies enforcement shapes for E9+, draws Linen/Quil persistence boundaries, and provides a negative test matrix. All current markers are **StructuralMeta** — no SecretContent violations found.

Docs-only. No code changed.

---

## 1. Redaction Classes

### 1.1 Class definitions

Adapted from `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` §9 with E8 refinement.

| Class | Tag | Logged | Examples | Persistable? |
|-------|-----|--------|----------|-------------|
| **PublicProof** | `redact=public` | Marker name, status, reason only | `[sexstore.put.allow] status=ok` | Yes — permanent audit |
| **StructuralMeta** | `redact=structural` | Caller PD, op, key range, slot state, generation, class | `caller=3 key=1 state=1 gen=4` | Yes — session-scoped |
| **SensitiveMeta** | `redact=sensitive` | Document IDs, restore IDs, owner IDs, object references | `doc_id=0x7F`, `restore_token=0xA3` | Capability-gated; value redacted |
| **SecretContent** | `redact=secret` | Stored values, user text, document names, raw paths, cryptographic material | `val=0xAC01...`, `path=/home/...` | **Never enter proof logs** |

### 1.2 Class hierarchy

```
PublicProof ⊂ StructuralMeta ⊂ SensitiveMeta ⊂ SecretContent

Each class includes all fields of the parent class, plus its own fields.
Logging a higher class requires permission to log all lower classes.
```

| Class | Logs fields from |
|-------|-----------------|
| PublicProof | Marker name, status string, reason code |
| StructuralMeta | + caller_pd, op name, key number, slot state, generation, class |
| SensitiveMeta | + object IDs, restore tokens, owner references |
| SecretContent | + stored values, user text, paths, crypto material |

### 1.3 Redaction enforcement rules

1. **SecretContent never enters proof logs.** Any marker that would include a stored value, user text, file path, document name, or cryptographic secret must redact that field before emission.
2. **SensitiveMeta requires capability gate or admin scope.** Object IDs and restore tokens may only appear in logs that are capability-gated (StoreAdmin) or session-scoped (not persisted across boots).
3. **StructuralMeta is the default class for current operations.** Key numbers are opaque u32 identifiers — not user content. Caller PD is a domain ID. Slot state and generation are counters.
4. **PublicProof is a subset of StructuralMeta** — some markers may only need to log status/reason without caller/key detail.
5. **Redaction is applied at emission time** — before the marker string is formed. If any field of a marker would violate its class restriction, that field is replaced with `redacted` or omitted.
6. **Transient output** (`serial_println!` to debug console) may be less restrictive than persistent logs. But E8 policy applies uniformly: no SecretContent in any proof log, transient or persistent.
7. **Label hashes are not a secrecy boundary.** A hash of a document name is still derived from user content — it must be treated as SensitiveMeta or higher, not StructuralMeta.

---

## 2. Current E7 Marker Classification

### 2.1 All markers classified

| Marker | Class | Fields logged | SecretContent violations? |
|--------|-------|---------------|--------------------------|
| `[sexstore.put.allow]` | StructuralMeta | caller, key, status=ok, state, gen | None |
| `[sexstore.put.reject]` | StructuralMeta | caller, key, status, reason | None |
| `[sexstore.get.allow]` | StructuralMeta | caller, key, status=ok, state, gen | None |
| `[sexstore.get.reject]` | StructuralMeta | caller, key, status, reason | None |
| `[sexstore.delete.allow]` | StructuralMeta | caller, key, status=ok, state, gen, reason | None |
| `[sexstore.delete.reject]` | StructuralMeta | caller, key, status, reason | None |
| `[sexstore.policy.allow]` | StructuralMeta | caller, key, op | None |
| `[sexstore.policy.deny]` | StructuralMeta | caller, key, class, reason | None |
| `[sexstore.key.invalid]` | StructuralMeta | caller, key=0x00 | None |
| `[sexstore.value.invalid]` | StructuralMeta | caller, key | None |
| `[sexstore.status.mapping]` | PublicProof | Status code mapping only | None |
| `[sexstore.generation.bump]` | StructuralMeta | key, slot, gen, op | None |
| `[sexstore.tombstone.record]` | StructuralMeta | key, slot, gen, reason | None |
| `[sexstore.tombstone.get]` | StructuralMeta | key, slot, gen | None |
| `[sexstore.tombstone.revive]` | StructuralMeta | key, old_gen | None |
| `[sexstore.reply.error]` | StructuralMeta | caller, op | None |
| `[sexstore.kv.put]` (legacy) | StructuralMeta | key, ok flag | None |
| `[sexstore.kv.get]` (legacy) | StructuralMeta | key, hit flag | None |

### 2.2 Key observation

**All 18 current marker types are StructuralMeta or PublicProof.** No marker logs:
- The stored u64 value (`val` field of KvSlot)
- File paths or raw path strings
- Document titles or user-generated names
- Object IDs (none exist yet in sexstore)
- Cryptographic material
- Caller identity beyond domain ID

**This is by design:** E4–E7 explicitly avoided logging stored values. E8 confirms no violations exist.

### 2.3 Marker field classification

| Field | Class | Notes |
|-------|-------|-------|
| Marker name (e.g., `put.allow`) | PublicProof | Operation type |
| `status` | PublicProof | Structured status string |
| `reason` | PublicProof | Structured reason code |
| `caller` (domain ID) | StructuralMeta | Opaque u32 domain identifier |
| `op` | StructuralMeta | `PUT`, `GET`, `DEL` |
| `key` (u32) | StructuralMeta | Opaque numeric key identifier |
| `class` (u8) | StructuralMeta | Key owner class (0=invalid, 1=shell, 2=reserved) |
| `state` (u8) | StructuralMeta | Slot state (0=empty, 1=active, 2=tombstoned) |
| `gen` (u8) | StructuralMeta | Generation counter |
| `slot` (usize) | StructuralMeta | Slot index (0..15) |
| `ok` (legacy) | StructuralMeta | Binary success flag |
| `hit` (legacy) | StructuralMeta | Binary found flag |
| `stored value` (u64) | **SecretContent** | **Never logged** |
| file path / document name | **SecretContent** | **Never logged** |
| cryptographic secret | **SecretContent** | **Never logged** |

---

## 3. Forbidden Log Fields

The following fields must **never** appear in any storage proof log (transient or persistent):

1. **Stored u64 value** — the `val` field of a KvSlot entry. Contains the packed scene settings blob (magic, version, preset_idx, chrome_flags, access_flags). Even the magic byte is not user content, but the packed blob as a whole is opaque and must not be exposed.
2. **Raw file paths** — any string starting with `/` or containing path separators. SexOS has no POSIX filesystem, but if a future backend uses path-like references, they must be redacted.
3. **Document titles** — user-created document names in Linen or other document servers.
4. **User text content** — any string typed by the user or derived from user input.
5. **Cryptographic material** — keys, hashes (unless used as opaque identifiers with SensitiveMeta classification), tokens, secrets.
6. **Quil buffer contents** — pixel data, workspace content, document buffers.
7. **App object IDs without classification** — any future object reference that is not explicitly classified as StructuralMeta or higher.

### 3.1 Field validation at code review

Every `serial_println!` call in sexstore dispatch that produces a proof marker must be verified:
1. Does it include the stored value? → **STOP FIRST**
2. Does it include a raw path? → **STOP FIRST**
3. Does it include user text? → **STOP FIRST**
4. Does it include a key that is not opaque u32? → **REVIEW — may need SensitiveMeta**
5. Does it include caller identity beyond domain ID? → **STOP FIRST**
6. Does it include generation/state/class that reveals user behavior patterns? → **ACCEPTABLE — StructuralMeta**

---

## 4. Future Enforcement Helper Shape

For E9+ (persistent log backend), redaction helpers should follow this shape.
No implementation in E8 — these are prototypes for future enforcement.

```rust
/// Log a PublicProof marker (marker name + status + reason only).
/// No caller, key, or structural metadata.
fn log_public(target: &str, status: &str, reason: &str) {
    // [target] status=S reason=R
}

/// Log a StructuralMeta marker (caller, key, state, gen, etc.).
/// caller_pd is domain ID (u32), key is opaque u32.
fn log_structural(target: &str, caller: u32, key: u32, status: &str,
                  state: u8, gen: u8, reason: &str) {
    // [target] caller=C key=K status=S state=ST gen=G reason=R
}

/// Log a SensitiveMeta marker with redacted value.
/// id is capability-gated; the value field is replaced with "redacted".
fn log_sensitive_redacted(target: &str, caller: u32, id: u64,
                          status: &str, reason: &str) {
    // [target] caller=C id=REDACTED status=S reason=R
}

/// Reject a marker that would log SecretContent.
/// This is a compile-time/logic error — never reaches output.
fn log_reject_secret(target: &str, reason: &str) -> ! {
    // [target] FATAL: attempted to log SecretContent — reason=R
    // This should never happen; indicates a code bug.
}
```

### 4.1 Constraints

- All helpers are `no_std` compatible — no heap, no String, no allocation.
- All helpers use fixed-size formatting (array on stack or `serial_println!` macro).
- No helper accepts a raw `&str` for value, path, or user content.
- `log_sensitive_redacted` accepts a `u64` id but always redacts it in the output.
- `log_reject_secret` is a diverging function — calling it indicates a storage-layer bug.

---

## 5. Linen/Quil Boundary

### 5.1 Current state

- **Linen:** Document/project server (placeholder). No document content stored in sexstore.
- **Quil:** App surface server. No buffer content stored in sexstore.
- **sexstore:** RAM-only K/V for shell scene settings only.

### 5.2 Persistence boundary

**No Linen or Quil persistence may proceed until E8 redaction policy is enforced.**

| Condition | Status | Gate |
|-----------|--------|------|
| Linen document metadata in sexstore keys | ❌ Not allowed in E8 | Requires SensitiveMeta or higher |
| Linen document content in sexstore values | ❌ Not allowed in E8 | Requires SecretContent handler |
| Quil workspace IDs in proof markers | ❌ Not allowed in E8 | Requires SensitiveMeta |
| Quil buffer content in proof markers | ❌ Never allowed | SecretContent — forbidden |
| Shell scene settings in sexstore keys (key 0x01) | ✅ Allowed in E8 | StructuralMeta — opaque u32 key |
| Shell scene settings in sexstore values (packed blob) | ✅ Allowed in E8 (internal) but **never logged** | Not exposed in markers |

### 5.3 OpenIntent/RestoreIntent

When Linen document restore is implemented (future), `OpenIntent` and `RestoreIntent`
IDs may appear in storage operations. These are **SensitiveMeta** — they must be
redacted in public logs and capability-gated in persistent logs.

```text
Allowed:  [store.doc.restore] caller=3 doc_id=REDACTED status=ok
Forbidden: [store.doc.restore] caller=3 doc_id=0xA3F7 status=ok
```

### 5.4 Label hashes

If Linen uses hashed document titles as storage keys, those hashes are still
derived from user content. They are **SensitiveMeta**, not StructuralMeta.
Using a hash does not automatically make a value safe for public logging.

---

## 6. Negative Test Matrix

### 6.1 Current E7 negative tests (pass)

| Test | Violation attempted | E7 result | E8 classification |
|------|--------------------|-----------|-------------------|
| Stored u64 value in PUT allow marker | Would expose scene settings blob | **Not logged** ✅ | SecretContent — forbidden |
| Stored u64 value in GET allow marker | Would expose scene settings blob | **Not logged** ✅ | SecretContent — forbidden |
| Raw path in any marker | Would expose filesystem layout | **Not logged** ✅ | SecretContent — forbidden |
| Document title in any marker | Would expose user content | **Not logged** ✅ | SecretContent — forbidden |
| Quil buffer content in any marker | Would expose workspace data | **Not logged** ✅ | SecretContent — forbidden |
| Caller identity beyond domain ID | Would expose user identity | **Not logged** ✅ | Not applicable |
| Key semantic meaning in marker | Key 0x01 logged as opaque number | **Not exposed** ✅ | StructuralMeta — opaque u32 |
| Generation as behavioral fingerprint | Gen 1..255 reveals write count | **Logged** but acceptable | StructuralMeta — counter |

### 6.2 Future E9+ negative test scenarios

| Test | Expected | Gate |
|------|----------|------|
| Linen stores document title in proof marker | **STOP FIRST** — SecretContent | E8 violation |
| Quil stores buffer hash in proof marker | **STOP FIRST** — SecretContent | E8 violation |
| sexstore value logged in debug marker | **STOP FIRST** — SecretContent | E8 violation |
| Raw path appears in storage log | **STOP FIRST** — SecretContent | E8 violation |
| OpenIntent ID logged without redaction | **STOP FIRST** — SensitiveMeta | E8 violation |
| RestoreIntent ID logged without redaction | **STOP FIRST** — SensitiveMeta | E8 violation |
| App object ID without classification | **REVIEW** — must classify | E8 requirement |
| Label hash treated as StructuralMeta | **REVIEW** — is SensitiveMeta | E8 requirement |
| Caller PD with user-identifiable name | **STOP FIRST** — never use names | E8 violation |
| Any doc/value/path in public/persistent log | **STOP FIRST** — redact before persist | E9 gate |

---

## 7. STOP FIRST Conditions

| Condition | Action |
|-----------|--------|
| Any proof marker needs to log stored u64 value | **STOP** — SecretContent never enters proof logs |
| Any durable restore logs raw IDs unredacted | **STOP** — SensitiveMeta must be redacted |
| Any raw path or file name appears in proof log | **STOP** — SecretContent, forbidden |
| Any app gets direct storage capability without redaction class | **STOP** — must classify all marker fields |
| Any LIST/ENUM leaks key inventory without redaction | **STOP** — key inventory is StructuralMeta but enumeration pattern is sensitive |
| Any redaction implementation requires heap/String | **STOP** — must use fixed-size formatting |
| Linen/Quil document persistence before E8 enforcement | **STOP** — E8 must be enforced before document storage |
| Label hash treated as StructuralMeta without review | **STOP** — hashes derived from user content are SensitiveMeta |
| Any marker logged without a redaction class | **STOP** — all markers must have a class before persistent logging (E9) |
| Any code change to sexstore in E8 | **STOP** — E8 is docs-only |

> ✅ **E8 passes its own gate.** Docs-only. Current markers classified as StructuralMeta.
> No SecretContent violations found. No code changed.

---

## 8. Ready/Not Ready for E9

### 8.1 Yes — E9 can proceed

E9 (persistent backend gate) is **ready to start**:

1. **All current markers classified** — 18 types, all StructuralMeta or PublicProof
2. **No violations** — no SecretContent in any current marker
3. **Redaction classes defined** — PublicProof, StructuralMeta, SensitiveMeta, SecretContent
4. **Forbidden fields enumerated** — stored values, paths, documents, crypto, user text
5. **Enforcement helper shapes prototyped** — `log_public`, `log_structural`, `log_sensitive_redacted`, `log_reject_secret`
6. **Linen/Quil boundary defined** — no document/object persistence before E8 enforcement
7. **Negative tests specified** — 14 scenarios (6 current, 8 future)

### 8.2 E9 scope (proposed)

- Define the persistent backend gate: requirements for adding disk-backed or session-persistent storage
- No implementation in V1 — E9 is a policy gate that prevents premature persistence
- Must reference E8 redaction classes: no persistent log may store unredacted StructuralMeta+
- Must verify marker classification before any marker is persisted

### 8.3 Outstanding pre-E9 items

- E8 enforcement helpers are prototyped but not implemented — E9 must decide whether to implement them
- `[sexstore.status.mapping]` is currently unbudgeted (one boot-time marker) — E9 may need to budget it if persisted
- Legacy `[sexstore.kv.put]` and `[sexstore.kv.get]` markers are deprecated but still emit — E9 may suppress or replace them

---

## Appendix A: Full Redaction Classification Matrix

### A.1 All E7 markers with redaction class

```text
sexstore.status.mapping       → PublicProof
sexstore.put.allow            → StructuralMeta
sexstore.put.reject           → StructuralMeta
sexstore.get.allow            → StructuralMeta
sexstore.get.reject           → StructuralMeta
sexstore.delete.allow         → StructuralMeta
sexstore.delete.reject        → StructuralMeta
sexstore.policy.allow         → StructuralMeta
sexstore.policy.deny          → StructuralMeta
sexstore.key.invalid          → StructuralMeta
sexstore.value.invalid        → StructuralMeta
sexstore.generation.bump      → StructuralMeta
sexstore.tombstone.record     → StructuralMeta
sexstore.tombstone.get        → StructuralMeta
sexstore.tombstone.revive     → StructuralMeta
sexstore.reply.error          → StructuralMeta
sexstore.kv.put (legacy)      → StructuralMeta
sexstore.kv.get (legacy)      → StructuralMeta

No marker currently requires PublicProof-only or SensitiveMeta.
All 18 markers are StructuralMeta (or PublicProof subset).
```

### A.2 Field-level classification example

```
[sexstore.put.allow] caller=3 key=1 status=ok state=1 gen=4
                    ──────  ───       ────── ─────  ───
                    Struct  Struct    Public  Struct Struct
```

```
[sexstore.get.reject] caller=10 key=1 status=denied reason=no_cap
                      ──────    ───       ──────    ──────
                      Struct    Struct    Public     Public
```

---

## Appendix B: Files Referenced

| File | Relevance |
|------|-----------|
| `servers/sexstore/src/main.rs` | All current markers — verified no SecretContent logged |
| `servers/silk-shell/src/main.rs` | Client-side sexstore callers — no proof markers in storage paths |
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | §9 privacy/redaction model — base for E8 classes |
| `docs/handoff/E7_STORAGE_PROOF_MARKER_HARDENING_V1.md` | Current marker inventory — classified in E8 |
| `docs/handoff/E6_STORAGE_TOMBSTONE_DELETE_V1.md` | Generation/tombstone markers — classified |
| `docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md` | Key namespace — opaque u32 keys are StructuralMeta |
