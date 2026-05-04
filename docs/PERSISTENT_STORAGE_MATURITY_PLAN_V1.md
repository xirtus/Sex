# PERSISTENT_STORAGE_MATURITY_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** Mature storage guarantees below Linen before building document/project abstractions on top. sexstore (RAM K/V) gets deterministic read/write semantics, corruption handling, schema/versioning, privacy/redaction policy, proof markers, partial failure behavior, and capability boundaries. Then Linen builds documents/projects on a proven foundation.

---

## 1. Mission

MISSION: E_PERSISTENT_STORAGE_MATURITY_PLAN_V1 — Mature sexstore and sexfiles storage guarantees: deterministic reads/writes, corruption handling, schema/versioning, privacy/redaction policy, proof markers, partial failure behavior, and capability boundaries. Below Linen. Docs/plan only. No implementation.

---

## 2. Dependency Gates

1. sexstore (RAM K/V) must exist and be reliable at current capability (16-slot, u32 key, u64 value) before schema/versioning upgrades begin.
2. sexfiles VFS backend must be auditable before corruption-handling and privacy/redaction policy are defined.
3. Collar capability model must support storage capability types (StoreRead, StoreWrite, StoreAdmin) before storage access control is enforced.
4. PDX IPC must support variable-length messages or chunked transfers before multi-slot K/V values or file contents cross PD boundaries reliably.
5. sexstore and sexfiles must prove deterministic behavior before Linen builds document/project abstractions on top.
6. Privacy/redaction policy must be defined before any persistent log stores private content.
7. No Linen implementation before sexstore/sexfiles maturity gates (E2 key ranges, E3 capability policy, E4 schema/version, E5 corruption/partial-read, E6 delete/tombstone, E8 privacy redaction) pass.
8. Key range allocation must be collision-checked before StoreCapability enforcement (E3). No two PDs may claim overlapping ranges — collisions must be detected at allocation/plan time, not runtime.
9. E requires an explicit delete/tombstone operation in the active SexOS storage protocol. It must be capability-checked, idempotent, proof-marked, and safe under missing/corrupt keys. Do not assume POSIX unlink semantics.

---

## 3. Context

SexOS storage architecture is currently split across three layers:

- **sexstore** — RAM-based K/V store, fixed 16 slots, u32 key → u64 value. Used for scene settings persistence (same-boot), audio policy flags, and shell state. No disk, no filesystem, no corruption detection, no schema versioning.
- **sexfiles** — PDX VFS server with trampoline architecture, backends, cache, and message routing. Has filesystem-like abstractions but privacy/redaction policy and corruption handling are unproven.
- **Linen** — Document/project server. Currently minimal (placeholder surface on sexdisplay). Depends on sexfiles for storage but cannot safely build until storage guarantees are mature.

Track E focuses on the layer below Linen: maturing sexstore and sexfiles so Linen (and future storage consumers) have deterministic, inspectable, privacy-safe storage.

Current invariants from existing storage:
- sexstore is RAM-only — no persistence across power cycles
- sexstore uses fixed 16-slot table — KV_FULL is a real failure mode
- sexstore ops are OP_KV_GET (0xB0) and OP_KV_PUT (0xB1) — no delete, no list, no multi-key
- sexfiles uses PDX messages with trampoline dispatch for VFS operations
- No storage operation has proof markers or audit trail
- No storage operation has capability/access control beyond PD slot

---

## 4. Why Separate

Storage maturity must be designed independently from document abstractions because:
- Storage guarantees (atomicity, corruption detection, schema evolution) are cross-cutting concerns that affect every storage consumer
- Linen document/project semantics would distract from storage-layer correctness
- Privacy/redaction policy for storage logs must exist before any persistent data stores user-facing content
- Capability boundaries for storage (who can read/write which keys/files) are a Collar integration point that should be designed once and applied to all storage consumers
- Rushing storage would create fragile persistence that Linen cannot safely build upon

---

## 5. Innovation Goal

SexOS storage should be inspectable and deterministic: every read, write, and failure has a proof marker, a capability check, and a deterministic outcome. No silent corruption, no partial writes, no unversioned schema drift, no private data leaked in audit logs, no unbounded storage growth, and no storage operation that bypasses capability policy.

---

## 6. Storage Object Model

- **KvEntry:** a single key-value pair in sexstore. Has key (u32), value (u64), flags (u8: valid, tombstone, version_tag), generation (u32).
- **KvTable:** the complete sexstore state. Has slot_count (fixed or bounded), entries, checksum over metadata, version_tag.
- **StorageSchema:** version identifier for stored data format. V1: implicit magic byte in value blob. Future: explicit schema field in KvEntry flags.
- **StorageVersion:** a version counter incremented on each schema change. Stored as part of KvTable metadata. Used to detect stale or incompatible stored data on boot.
- **StoreIntent:** a request to perform a storage operation (read, write, delete, list, admin). Contains intent_kind, target_key, value (for writes), caller_capability.
- **StoreResult:** the result of a StoreIntent. Contains status (ok, not_found, full, corrupt, denied, partial_failure), value (for reads), proof_sequence_id.
- **ProofMarker:** logged event for any storage operation. Contains sequence_id, operation (get/put/del/list/admin), key, status, caller_role.
- **PrivacyRedactionRule:** a rule specifying which keys or key ranges are redacted from proof logs. Rules: Public (logged fully), Session (key logged, value redacted), Private (key and value redacted), Secure (operation type logged only).
- **CorruptionState:** the current integrity state of a storage backend. States: Clean, Suspect, Corrupt, TombstonePending, Tombstoned.
- **PartialFailureRecord:** a log of incomplete storage operations. Contains sequence_id, operation, key, failure_point, recovery_action (rollback, retry, tombstone).
- **StoreCapability:** a capability to perform storage operations. Kinds: StoreRead (read specific key/range), StoreWrite (write specific key/range), StoreAdmin (schema migration, repair, tombstone). Future: Collar-mediated grants.
- **StorageProofEvent:** a logged proof event for any storage operation. Contains sequence_id, operation, key, status, corruption_state, redaction_class.

---

## 7. Sexstore Maturity Model

### Current sexstore (V0):
- 16 fixed slots, linear scan for get/put
- u32 key, u64 value
- No delete operation
- No corruption detection
- No schema versioning
- No proof markers beyond current LOG_PUT/LOG_GET counters
- RAM-only, lost on power cycle
- No access control beyond PDX slot

### V1 upgrades (planned):

| Feature | V0 | V1 Target | Phase |
|---------|----|-----------|-------|
| Slot count | Fixed 16 | Configurable at compile time (power-of-2, max 64) | E2 |
| Delete operation | None | OP_KV_DEL (0xB2) — sets tombstone flag | E6 |
| Slab/linear scan | Linear scan | Slab with generation counter per slot | E2 |
| Proof markers | Manual LOG_PUT/LOG_GET counters | Structured `[store.kv.get/put/del]` with sequence_id, key, status, redaction_class | E7 |
| Corruption detection | None | Checksum over KvTable metadata; per-entry CRC on value | E5 |
| Schema versioning | Implicit magic byte in value blob | Explicit StorageVersion in KvTable header; migration rules | E4 |
| Partial failure | Silent (full slots = dropped) | PartialFailureRecord logged; client notified | E5 |
| Capability access | None (any PDX caller) | StoreCapability check before get/put/del | E3 |
| Privacy redaction | None | PrivacyRedactionRule applied to proof markers | E8 |
| Tombstone | No delete | Tombstone flag; tombstoned entries excluded from get; space reclaimed on insert under pressure | E6 |

### V1 is still RAM-only:
- Persistence across power cycles requires disk/block device support (future)
- V1 focuses on making the RAM store *provably correct*: deterministic behavior, corruption detection, complete audit trail, capability-gated access
- Future V2 may add disk-backed persistence, but only after block device abstraction exists

---

## 8. Sexfiles Maturity Model

### Current sexfiles:
- PDX VFS server with trampoline dispatch
- Backend abstraction (multiple backends)
- Cache layer
- Message routing

### V1 upgrades:

| Feature | Current | V1 Target | Phase |
|---------|---------|-----------|-------|
| Corruption detection | Unknown | Checksum per file block; integrity check on open | E5 |
| Partial write safety | Unknown | Write-ahead intent log; rollback on failure | E5 |
| Privacy redaction | None | Redaction rules for file metadata in proof logs | E8 |
| Proof markers | None | `[store.file.open/read/write/close]` with sequence_id | E7 |
| Capability access | None | StoreCapability check on file operations | E3 |
| Schema versioning | None | File metadata version header | E4 |

Note: sexfiles VFS backend audit (E1) must determine exact current capabilities before V1 targets are locked.

---

## 9. Privacy/Redaction Policy Model

### Redaction classes for storage:

| Class | Scope | Logged | Redacted |
|-------|-------|--------|----------|
| Public | Storage metadata (schema version, slot count, sequence_id) | Key + value | Nothing |
| Session | Setting keys, config keys, scene state | Key only | Value |
| Private | Document titles, user data, app preferences | Operation type only | Key + value |
| Secure | Security-critical keys (capability refs, auth state) | Operation type only | Key + value; may suppress even operation type |

### Rules:
- Storage health metadata (file sizes, access times) are Public.
- User-facing setting values (theme, volume) are Session — key logged, value redacted.
- Document names, file paths, and user-created content labels are Private — operation type logged only.
- Security-critical keys are Secure — may suppress even the operation type from persistent logs.
- Proof markers in transient (serial_println!) output may be less restrictive than persistent logs.
- Persistent storage logs must never store Private or Secure content unless explicit privacy policy approval exists.

---

## 10. Capability Boundaries

### StoreCapability kinds:
- `StoreRead(key_range)` — read specific key or key range
- `StoreWrite(key_range)` — write/update specific key or key range
- `StoreDelete(key_range)` — delete/tombstone specific key or key range
- `StoreAdmin` — schema migration, repair, tombstone reclamation, full table scan

### Ownership:
- **Shell** owns scene settings, input config, audio policy keys — StoreWrite for its key range
- **SexAudio** owns audio policy keys — StoreRead for audio keys, StoreWrite for own keys
- **Theremin** owns sound settings keys — StoreRead for sound keys
- **Linen (future)** owns document/file keys — StoreRead/StoreWrite for document key range
- **sexfiles** owns file metadata and content — StoreRead/StoreWrite for file system range
- **No PD** may read/write outside its granted key range without explicit StoreAdmin capability
- **StoreAdmin** is restricted to storage layer and authorized shell admin — never granted to app PDs

### Key range allocation:
- `0x00–0x0F`: Reserved (system)
- `0x10–0x1F`: Theremin settings
- `0x20–0x2F`: Audio policy (SexAudio)
- `0x30–0x3F`: Scene appearance (shell)
- `0x40–0x4F`: Input config (shell)
- `0x50–0x5F`: Future Linen documents
- `0x60–0x6F`: Future app storage
- `0x70–0x7F`: Admin/debug
- `0x80–0xFF`: Reserved for future allocation

---

## 11. Corruption Handling Model

### Detection:
- On boot: checksum KvTable metadata; if mismatch → CorruptionState = Corrupt → attempt repair from last known good state or tombstone
- On each read: verify per-entry CRC (if available); if mismatch → log proof marker, return not_found, suspect entry
- On each write: write new entry + compute checksum before acknowledging

### Recovery:
- Corrupt entry: tombstone and notify StoreAdmin
- Corrupt metadata: fall back to last validated snapshot (V1: not available; log corruption and fail safe)
- Partial write (crash during write): detect on next boot via incomplete write marker; roll back or tombstone

### V1 constraints:
- No disk-backed persistence — corruption is detected across same-boot operations only
- Corrupt = fail safe (deny reads, log proof, notify admin). No silent recovery of corrupt data.
- Tombstoned corrupt entries retain space but are excluded from reads

---

## 12. Schema/Versioning Model

### StorageVersion:
- u32 version counter stored in KvTable metadata
- Incremented on schema change (new key format, redaction class change, capability boundary change)
- On boot: compare stored version against compiled expected version
  - Match → proceed
  - Mismatch → check migration table; if migration exists → apply; if not → log error, use default/safe state, do not load incompatible entries

### Migration rules:
- Every schema version must have a forward migration path or a safe rejection
- Migration is a StoreAdmin-only operation
- Migration failure → entries not migrated are tombstoned or skipped with proof marker
- V1: no automatic migration — manual StoreAdmin operation. Future: automatic migration on boot.

---

## 13. Invariants

1. Every storage read/write/delete operation produces a proof marker with sequence_id, key, status, and redaction_class.
2. Every write operation must complete atomically (all-or-nothing within a single PDX message). No partial write visible to readers.
3. Corrupt entries are never returned to callers — return not_found with proof marker.
4. Schema version mismatch on boot → safe fallback (use defaults, log incompatibility, do not load incompatible entries).
5. No storage operation bypasses StoreCapability check (V1: capability check is planned in E3; before E3, PDX slot is the access control).
6. Key ranges are non-overlapping — each key maps to exactly one owning PD.
7. Proof markers for Private/Secure keys redact key name and value — operation type only is logged.
8. Persistent storage logs must NEVER store Private or Secure content unless explicit privacy policy approval exists.
9. Partial failure (slot full, write interrupted, corrupt detected) is logged with proof marker and communicated to caller.
10. Tombstoned entries are excluded from reads. Space may be reclaimed on insert under pressure or by StoreAdmin.
11. StorageVersion increments are monotonic — never decrement.
12. sexstore remains RAM-only in V1 — no filesystem/disk dependency for boot/error paths.
13. sexfiles may use block device backends but must handle missing/unavailable backends gracefully (fail safe, not crash).
14. Linen must not depend on persistence across power cycles until sexstore/sexfiles prove deterministic behavior and corruption handling.
15. Proof sequence_id monotonically increases per storage backend. On wrap-around (u32 overflow), the backend must detect, log a proof marker, and either reset or stall before reusing sequence IDs.
16. Key range allocation is collision-free — every key maps to exactly one owning PD. Collisions are detected at allocation/plan time, not runtime.
17. E8 privacy redaction policy is defined and enforced before any proof marker is persisted to log (phase ordering invariant: E8 before persistent logging).

---

## 14. STOP FIRST Conditions

- Any proposal to add disk-backed persistence before sexstore proves deterministic RAM behavior
- Any storage operation without a proof marker
- Any persistent log storing Private or Secure content without approved privacy/redaction policy
- Any schema migration that silently drops or corrupts data
- Any access control that relies on PD identification alone without capability validation (pre-E3: acceptable; post-E3: STOP FIRST)
- Any Linen document/project implementation before sexstore/sexfiles maturity gates (E2 key ranges, E3 capability policy, E4 schema/version, E5 corruption/partial-read, E6 delete/tombstone, E8 privacy redaction) pass
- Any unbounded storage growth without eviction/tombstone/reclamation policy
- Any kernel block device ABI changes for storage
- Any PDX ABI changes for storage without explicit buffer transport design
- Any std filesystem/POSIX assumptions for storage paths
- Any cross-PD raw pointer storage access
- Any storage operation that bypasses sexstore/sexfiles and writes directly to hardware
- Any persistent proof marker logging before privacy redaction policy (E8) is defined and enforced
- Any sexfiles V1 implementation without delete/tombstone operation
- Any key range allocation without collision check against all existing allocations
- Any StoreCapability check that validates fewer than: caller identity, requested operation, key range ownership, namespace prefix, sequence_id generation, and revoke/tombstone state. PD slot alone is never authority.
- Any E1 (sexfiles/sexstore audit) skipped or deferred below the phase ladder — maturity targets must be based on audited reality, not assumptions

---

## 15. Proof Scenarios

### Proof markers

```
[store.kv.get] seq=N key=K status=ok|not_found|corrupt|denied redact=public|session|private|secure
[store.kv.put] seq=N key=K status=ok|full|denied redact=public|session|private|secure
[store.kv.del] seq=N key=K status=ok|not_found|denied redact=public|session|private|secure
[store.file.open] seq=N path="S" status=ok|not_found|denied redact=session
[store.file.read] seq=N path="S" offset=O length=L status=ok|corrupt|denied redact=session
[store.file.write] seq=N path="S" offset=O length=L status=ok|partial|denied redact=session
[store.file.close] seq=N path="S" status=ok redact=session
[store.corrupt.detect] target=kv_table|entry|metadata location=K status=suspect|corrupt|tombstoned
[store.corrupt.recover] target=kv_table|entry location=K action=skip|tombstone|repair status=ok|failed
[store.schema.migrate] from=V to=V result=ok|incompatible|failed entries_affected=N
[store.capability.deny] operation=get|put|del|admin target=K caller=N reason=no_capability
[store.privacy.redact] marker=store.kv.get|store.kv.put|store.kv.del key=K class=private|secure
```

### Scenarios

1. Get existing key → returns value, proof marker logged with status=ok.
2. Get non-existent key → returns not_found, proof marker logged.
3. Put new key → stored successfully, proof marker logged with status=ok.
4. Put key into full table → returns full, proof marker logged, no data lost.
5. Put update existing key → overwrites, proof marker logged.
6. Delete existing key → tombstone flag set, subsequent get returns not_found.
7. Delete non-existent key → returns not_found, proof marker logged.
8. Corrupt entry detected on read → returns not_found, proof marker with status=corrupt, entry tombstoned.
9. Corrupt metadata detected on boot → safe fallback (defaults), proof marker logged, table marked Suspect.
10. Schema version mismatch on boot → safe fallback, incompatible entries skipped, proof marker logged.
11. Storage operation without StoreCapability → denied (V1: after E3; before E3, PDX slot is gate).
12. Private key access → key/value redacted in proof marker — operation type only logged.
13. Persistent log contains only Public/Session content — verified by log inspection.
14. Partial write failure → PartialFailureRecord logged, client notified, no partial data visible to readers.
15. sexstore table full + tombstoned entry exists → tombstoned slot reclaimed for new insert.
16. sexfile read from corrupt block → returns corrupt status, file marked Suspect.
17. sexfile write with unavailable backend → fails safe with proof marker, no crash.
18. Two PDs attempt to write same key (pre-capability) → last write wins, proof markers show both operations.
19. Multi-key scenario: shell writes scene settings (0x3x range), Theremin writes sound settings (0x1x range) — no collision, both succeed.
20. Linen attempts document write before E maturity gates (E2 key ranges, E3 capability policy, E4 schema/version, E5 corruption/partial-read, E6 delete/tombstone, E8 privacy redaction) pass → rejected at plan level (STOP FIRST).

21. Key range collision detected at allocation time → allocation rejected at compile/plan time, not runtime. Verified by cross-checking the key range map before any StoreCapability deployment.
22. Proof sequence_id wraps on a storage backend → backend detects wrap, logs [store.kv.get] seq=OVERFLOW_RESET status=wrap, resets or stalls before reusing IDs.
23. E8 privacy redaction enforced before any persistent proof marker logging → verified by phase ordering check (E8 handoff must precede any persistent log integration).
24. Sexfile deleted → file tombstoned, subsequent open/read returns not_found, proof marker logged with status=ok and operation=del.
25. Two PDs with different StoreCapabilities attempt same key (post-E3) → capability check evaluates caller identity + key range + operation: one succeeds (in-range), one denied with [store.capability.deny] proof marker.

---

## 16. Minimal Phase Ladder

1. **E1_STORAGE_AUDIT_V1** — Audit current sexstore (capabilities, slots, opcodes, failure modes) and sexfiles (VFS backends, cache, message flow, error handling). Document current invariants, gaps, and proof marker readiness. No code.

2. **E2_KEY_NAMESPACE_RANGE_V1** — Define key namespace allocation, range ownership model, collision detection at plan/compile time, and range allocation table as source of truth. Handoff doc.

3. **E3_STORAGE_CAPABILITY_POLICY_V1** — Define StoreCapability kinds (StoreRead, StoreWrite, StoreDelete, StoreAdmin), caller identity + operation + key range + namespace prefix + sequence_id generation + revoke/tombstone state validation. PD slot alone is never authority. Collar integration model. Handoff doc.

4. **E4_SEQUENCE_SCHEMA_VERSION_V1** — Define sequence_id monotonic model with wrap-around detection, StorageVersion model, migration rules, safe fallback behavior. Handoff doc.

5. **E5_CORRUPTION_PARTIAL_FAILURE_V1** — Define checksum model (KvTable metadata CRC, per-entry CRC), CorruptionState FSM, recovery actions (skip/tombstone/repair), PartialFailureRecord format, write-ahead intent logging, rollback semantics, client notification protocol. Handoff doc.

6. **E6_DELETE_TOMBSTONE_V1** — Define OP_KV_DEL (0xB2), generation-counter slab model, tombstone semantics, idempotent delete, safe behavior under missing/corrupt keys, slot compaction under pressure. Capability-checked and proof-marked. No POSIX unlink semantics. Handoff doc.

7. **E7_DETERMINISTIC_RAM_STORE_PROOFS_V1** — Define structured proof marker format for all storage operations (get/put/del, file open/read/write/close). Add redaction_class field placeholder. Prove deterministic behavior: every read, write, and failure has a proof marker, a capability check, and a deterministic outcome. Handoff doc.

8. **E8_PRIVACY_REDACTION_PROOF_POLICY_V1** — Define PrivacyRedactionRule classes (Public/Session/Private/Secure), redaction enforcement in proof markers, persistent log restrictions. **Must follow E7 directly** — proof markers must be redacted before they reach persistent/public logs (E9). Handoff doc.

9. **E9_PERSISTENT_BACKEND_GATE_V1** — Define the gate for adding persistent (disk/block-device) storage: requirements, constraints, and compatibility model. No implementation in V1 — this is a policy gate that prevents premature persistence. V1 remains RAM-only.

10. **E10_SEXFILES_SEXSHOP_INTEGRATION_V1** — Plan integration of sexfiles (VFS/block-backed paths, handover patterns) and sexshop (KV/object/package semantics) with the maturity guarantees from E1–E9. Define how delete/tombstone, proof markers, capability checks, and redaction apply to each layer. Handoff doc.

---

## 17. Handoff Files

- `docs/handoff/E_PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` — this document (overview)
- `docs/handoff/STORAGE_KEY_NAMESPACE_RANGE_V1.md` — key namespace allocation, range ownership, collision detection (E2)
- `docs/handoff/STORAGE_CAPABILITY_POLICY_V1.md` — StoreCapability kinds, caller+op+range validation, Collar integration (E3)
- `docs/handoff/STORAGE_SEQUENCE_SCHEMA_VERSION_V1.md` — sequence_id, StorageVersion, migration rules (E4)
- `docs/handoff/STORAGE_CORRUPTION_PARTIAL_FAILURE_V1.md` — checksum model, CorruptionState FSM, PartialFailureRecord (E5)
- `docs/handoff/STORAGE_DELETE_TOMBSTONE_V1.md` — OP_KV_DEL, slab model, tombstone semantics, idempotent delete (E6)
- `docs/handoff/STORAGE_DETERMINISTIC_PROOFS_V1.md` — proof marker format, deterministic behavior proofs (E7)
- `docs/handoff/STORAGE_PRIVACY_REDACTION_V1.md` — redaction classes, persistent log policy (E8 — directly after E7)
- `docs/handoff/STORAGE_PERSISTENT_BACKEND_GATE_V1.md` — persistent storage gate, requirements, constraints (E9)
- `docs/handoff/STORAGE_SEXFILES_SEXSHOP_INTEGRATION_V1.md` — sexfiles/sexshop integration with maturity guarantees (E10)
- `docs/handoff/STORAGE_CUSTOMIZATION_POLICY_V1.md` — storage customization policy, preference model, validation rules, non-customizable boundaries, preference lifecycle (E11)

---

## 18. Future Sub-Prompt Names

- `E1_STORAGE_AUDIT_V1`
- `E2_KEY_NAMESPACE_RANGE_V1`
- `E3_STORAGE_CAPABILITY_POLICY_V1`
- `E4_SEQUENCE_SCHEMA_VERSION_V1`
- `E5_CORRUPTION_PARTIAL_FAILURE_V1`
- `E6_DELETE_TOMBSTONE_V1`
- `E7_DETERMINISTIC_RAM_STORE_PROOFS_V1`
- `E8_PRIVACY_REDACTION_PROOF_POLICY_V1`
- `E9_PERSISTENT_BACKEND_GATE_V1`
- `E10_SEXFILES_SEXSHOP_INTEGRATION_V1`
- `E11_STORAGE_CUSTOMIZATION_POLICY_V1`

---

## 19. Cross-Track Dependency Notes

- **Linen (F):** Must NOT build document/project abstractions until E2 (key ranges), E3 (capability policy), E4 (schema/version), E5 (corruption/partial-read), E6 (delete/tombstone), and E8 (privacy redaction) gates pass. Linen depends on sexstore/sexfiles storage trust layer being provably correct.
- **sexstore/sexfiles:** Both must implement capability policy (E3), schema/version (E4), corruption handling (E5), delete/tombstone (E6), deterministic proofs (E7), and privacy redaction (E8) before exposing storage to app PDs beyond shell.
- **Collar:** Must support StoreCapability kinds before E3 can enforce storage access control. Collar integration is a dependency of E3.
- **SexAudio/Harp:** Audio policy keys stored in sexstore (0x20–0x2F range). Must be updated when E3 capability policy, E7 deterministic proofs, and E8 privacy redaction land.
- **Theremin:** Sound settings keys stored in sexstore (0x10–0x1F range). Same dependency as SexAudio.
- **Scene appearance (shell):** Scene settings keys stored in sexstore (0x30–0x3F). Already using sexstore K/V — must migrate when E2 key ranges, E3 capability policy, E6 delete/tombstone, and E7 deterministic proofs land.
- **SilkBar/panel:** May display storage health/proof status (future) but does not own storage policy.
- **Mesh:** May visualize storage graph (K/V entries, files, capability boundaries) in read-only diagnostic view.
- **Quil:** May inspect StorageProofEvent logs for debug/audit.
- **sexdisplay:** No integration — sexdisplay has no storage path.

---

## 20. Premortem Analysis

**Premise:** Assume this plan failed 6 months after acceptance. Below are the identified failure modes, their categories, and the revised safest path hardening applied above.

### Failure Mode Table

| # | Failure Mode | Category | Severity | Hardening Applied |
|---|-------------|----------|----------|-------------------|
| 1 | **Key range collision** — two PDs claim overlapping key ranges → data corruption or security breach | Invariant violation (§13.16) | **Critical** | §2.8 gate requiring collision check before E7; §13.16 invariant; §14.15 STOP FIRST |
| 2 | **Disk persistence scope creep** — someone adds filesystem/block device support before RAM store is provably correct → complexity explosion, kernel ABI changes, non-deterministic behavior | Scope creep (§14.1) | **Critical** | §14.1 already prohibits; premortem confirms no disk in V1 — RAM-only is firm |
| 3 | **E8 ordering violation** — phases reordered so privacy redaction (E8) falls after persistent backend gate (E9) → unredacted proof markers land in persistent/public logs before policy exists | Phase ordering violation (§13.17) | **Critical** | §13.17 invariant; §14.13 STOP FIRST; E8 locked directly after E7 and before E9 (E7→E8→E9 ordering inviolable) |
| 4 | **StoreCapability bypass via raw PDX** — rogue PD constructs raw PDX messages bypassing capability check → unauthorized storage access | MPK/PDX fault (§14.12) | **Critical** | §14.12 already prohibits raw PDX storage access; E7 capability policy is the structural fix |
| 5 | **sexfiles audit never completed (E1)** — E1 skipped, V1 targets based on assumptions not reality → sexfiles maturity plan invalid | Dependency stall (§2.2) | **High** | §14.17 STOP FIRST: E1 cannot be skipped or deferred; gate 2 requires audit before privacy/redaction |
| 6 | **Privacy redaction bypass** — new operation type (e.g., list, admin) logs full Private key/value because redaction rule not extended | Privacy leak (§13.17) | **High** | §13.17 invariant covers all operations; §14.13 STOP FIRST requires redaction before logging |
| 7 | **Schema migration silently drops data** — migration skips incompatible entries without proof marker → data loss with no audit trail | Invariant violation (§13.4) | **High** | §13.4 already requires safe fallback + proof marker; §14.4 already prohibits silent drop |
| 8 | **Capability granularity too coarse** — StoreCapability checks PD slot but not key range → shell reads Theremin keys, SexAudio writes scene settings | Invariant violation (§13.5) | **High** | §14.16 STOP FIRST: post-E3, caller identity + key range + operation validation required; §13.6 non-overlapping ranges enforced |
| 9 | **Partial failure silent data loss** — sexstore full + tombstone reclamation fails → write silently dropped, caller assumes success → settings lost | Invariant violation (§13.9) | **High** | §13.9 already requires proof marker + caller notification on partial failure; E6 formalizes PartialFailureRecord |
| 10 | **No sexfiles delete** — file tombstones accumulate unbounded → storage full with no reclamation path | Invariant violation (§14.14) | **High** | §2.9 gate requires delete before V1 file ops; §14.14 STOP FIRST |
| 11 | **Corruption detection false positive** — CRC fails on benign state change (version tag update, concurrent write) → entry tombstoned unnecessarily → data loss + unnecessary admin | Implementation defect (§11) | **Moderate** | §11 already tombstone-on-corrupt; E4 design must specify CRC scope (metadata vs value) to minimize false positives. Noted for E4 handoff. |
| 12 | **Proof sequence_id wrap-around** — u32 sequence_id wraps, audit trail entries become ambiguous | Invariant violation (§13.15) | **Moderate** | §13.15 invariant: wrap detection + reset/stall before reuse; §15 scenario 22 |
| 13 | **Pre-E3 cross-write between same-slot PDs** — two PDs sharing a PDX slot can cross-write before capability enforcement → data corruption | Invariant violation | **Moderate** | Accepted risk pre-E3; capability policy (E3) is the structural fix. Pre-E3: proof markers document all writes for audit. |
| 14 | **Key range allocation drift** — new PD added mid-project allocates range without checking existing allocations → collision | Process failure (§14.15) | **Moderate** | §14.15 STOP FIRST for allocation without collision check; §13.16 compile-time invariant |

### Revised Safest Path Summary

1. **E1 audit first, always** — sexfiles/sexstore audit must complete before any V1 target is locked. Skipping E1 invalidates the entire maturity plan because targets are based on assumptions, not audited reality.
2. **E7→E8 ordering is inviolable** — proof markers (E7) must never be persisted until privacy redaction policy (E8) exists. The E8 phase is locked directly after E7 in the ladder; any reordering proposal triggers STOP FIRST. Privacy before persistent/public logs (E9).
3. **Key range map as source of truth** — the key range allocation table (§10) must be the single source of truth for all storage capability decisions. Any new key range must be checked against this table at plan/compile time. No runtime discovery.
4. **No disk in V1** — RAM-only is firm. Disk persistence is a future concern that must not leak into V1 design. Block device abstraction, filesystem drivers, and power-cycle persistence are out of scope until RAM store is provably correct.
5. **Capability granularity must include caller identity + operation + key range + namespace prefix + sequence_id generation + revoke/tombstone state** — PD slot alone is never authority post-E3.
6. **Sexfiles delete is not optional** — V1 must include file delete/tombstone semantics that are capability-checked, idempotent, proof-marked, and safe under missing/corrupt keys. No POSIX unlink semantics.
7. **Proof log auditability depends on sequence_id uniqueness** — sequence_id wrap-around must be detected and handled explicitly. Silent wrap makes the audit trail unreliable.

---

## 21. Customization / User Policy Surface

### Overview

Storage-layer customization covers operational policy preferences for sexstore and sexfiles: key namespace behavior, range sizes, sequence generation, schema versioning, corruption handling, retry semantics, logging verbosity, capability caching, and redaction strictness. All customization is bounded by non-customizable safety invariants that the storage layer enforces regardless of preference values.

Customization is a **storage-admin and shell-admin** concern — not app-visible. App PDs cannot read or write storage-layer preferences. The shell may provide a preference UI but must validate through the storage layer's preference API.

### 21.1 Customizable Items

Customizable only as bounded admin/developer policy hints after E validation. Not end-user authority. Cannot change key ownership, key collision checks, capability checks, sequence monotonicity, schema validation, or backend safety behavior.

| # | Domain | Type | Default | Description |
|---|--------|------|---------|-------------|
| 1 | key_namespace_policy | enum {reserved, dynamic, hybrid} | reserved | How key namespace prefixes are assigned. Reserved = compile-time allocation table. Dynamic = runtime registration. Hybrid = base ranges reserved, sub-ranges dynamic. |
| 2 | key_range_size | enum {4, 8, 16, 32} | 16 | Default range granularity (power-of-2). Smaller ranges allow finer capability granularity. |
| 3 | sequence_strategy | enum {monotonic_u32, timestamp_ms, hybrid} | monotonic_u32 | Proof sequence_id generation strategy. Monotonic = u32 counter. Timestamp = wall-clock ms (best-effort). Hybrid = counter + timestamp suffix. |
| 4 | schema_migration_policy | enum {manual_only, auto_forward, auto_forward_with_rollback} | manual_only | How schema version mismatches are handled on boot. Manual = StoreAdmin must approve. Auto_forward = automatic forward migration. Auto_forward_with_rollback = auto with rollback on failure. |
| 5 | corruption_mode | enum {fail_safe, attempt_repair, log_only} | fail_safe | Behavior when corruption is detected. Fail_safe = deny reads, log proof, notify. Attempt_repair = try CRC fix or fallback. Log_only = log warning, serve data (debug only). |
| 6 | retry_policy | struct {max_retries: u8, backoff_ms: u32, fail_fast: bool} | {3, 100, false} | Retry behavior on transient failure. max_retries capped at compiled maximum. fail_fast = abort on first failure instead of retrying. |
| 7 | proof_verbosity | enum {full, normal, minimal} | normal | Level of detail in proof markers. Full = operation + entry-level. Normal = operation-level. Minimal = operation type only (still includes required fields). |
| 8 | capability_cache_ttl_ms | u32 | 1000 | How long capability check results are cached. 0 = no caching (rejected — STOP FIRST). Hard compiled upper bound. |
| 9 | redaction_strictness | enum {strict, standard, relaxed} | standard | How aggressively proof markers are redacted. Strict = redact everything beyond Public. Standard = as defined in §9. Relaxed = session keys logged at Session instead of Private (Private/Secure never degraded). |

### 21.2 Non-Customizable Invariants

These 17 boundaries are enforced by the storage layer regardless of preference values:

1. **Proof markers never optional** — every storage read/write/delete operation produces a proof marker with sequence_id, key, status, and redaction_class. Proof verbosity may suppress detail but never suppress required fields.
2. **Private/Secure redaction never bypassable** — regardless of redaction_strictness, Private and Secure content is always redacted from proof markers. Redaction is never fully disabled.
3. **Capability check order hard-coded** — caller identity → requested operation → key range ownership → namespace prefix → sequence_id generation → revoke/tombstone state. This order cannot be customized.
4. **PD slot alone is never authority** — capability validation never degrades to slot-only check, regardless of preference values.
5. **Key ranges non-overlapping** — no preference allows overlapping key ranges. Collision detection at allocation time is invariant.
6. **Delete is tombstone, not unlink** — no customization re-enables POSIX unlink semantics. Delete always sets tombstone flag.
7. **Schema mismatch = safe fallback** — no preference allows silent loading of incompatible schema versions. Migration always requires proof marker.
8. **Corrupt entries never returned to caller** — corruption_mode = attempt_repair or log_only cannot override fail_safe when CorruptionState = Corrupt. Fail-safe takes precedence on confirmed corruption.
9. **sexstore RAM-only in V1** — no preference can enable disk persistence before E9 gate passes.
10. **Proof sequence_id monotonic** — sequence_strategy may change generation method but never allows non-monotonic sequence_id.
11. **E8 before persistent logging** — phase ordering invariant: no preference can enable persistent proof logging before E8 privacy redaction exists.
12. **No std/POSIX assumptions** — no preference introduces filesystem paths, file descriptors, or POSIX semantics.
13. **No cross-PD raw pointers** — no preference enables shared-memory or raw-pointer storage access.
14. **StoreAdmin restricted** — no preference grants StoreAdmin capability to unprivileged PDs.
15. **Migration failure = tombstone/skip with proof** — no preference allows silent data drop on migration failure.
16. **Partial failure communicated to caller** — no preference suppresses partial failure notification to caller.
17. **Capability cache has hard upper bound** — capability_cache_ttl_ms is capped at a compiled-in maximum. No preference can set unbounded caching.

### 21.3 Preference Lifecycle

Six-step lifecycle for storage customization preferences:

1. **Load** — Preferences loaded from memory or (post-E9) from E-backed storage. Proof marker: `[store.pref.load]`.
2. **Validate** — Preference value validated against type, bounds, non-customizable invariants, and compiled limits. Invalid values rejected or clamped to safe default. Proof markers: `[store.pref.load]` with status=ok or reject.
3. **Apply** — Validated preference applied. If preference affects capability validation (key_range_size, key_namespace_policy), capability re-validation is triggered before apply. Proof marker: `[store.pref.apply]` or `[store.pref.reject]`.
4. **Persist** — Preferences persisted to storage only after E9 gate passes. Before E9, preferences are memory-only. Proof marker: `[store.pref.persist.reject]` if persistence attempted before E9.
5. **Redact** — Preference proof logs redacted per E8 policy. Private preference values never logged fully. Proof marker: `[store.pref.redact]`.
6. **Reset** — Reset-to-safe-default restores all preferences to compiled defaults, clearing any persisted state. Proof marker: `[store.pref.reset]`.

### 21.4 Preference Ownership

- Preferences are storage-layer owned — not app-visible or app-configurable.
- Shell may provide preference UI but must validate through storage-layer preference API.
- Cross-PD preference sync uses RedactedMetadata only — private preference values never transmitted.
- Preference scope determines who can read/write: storage-admin, shell-admin, or system (compiled-in only).
- No PD other than shell-admin or storage-layer internal may read or write storage preferences.

### 21.5 Additional Invariants

11 invariants governing customization:

1. Customization preferences are validated against type, bounds, and policy before apply. Unvalidated preferences never take effect.
2. Non-customizable boundaries (§21.2) are enforced by the storage layer regardless of preference values. Preferences that conflict with non-customizable invariants are rejected at validation time.
3. Preference changes that affect capability validation (key_range_size, key_namespace_policy) require capability re-validation before taking effect. Proof marker: `[store.pref.cap.revalidate]`.
4. Proof verbosity preferences may suppress detail but never suppress the required proof marker format: operation, key, status, and redaction_class are always present.
5. Redaction_strictness = relaxed may not expose Private or Secure content in proof logs — redaction is never fully disabled for these classes.
6. Corruption_mode preferences are advisory when CorruptionState = Corrupt: fail_safe always takes precedence when actual corruption is confirmed.
7. Capability_cache_ttl_ms must have a hard compiled upper bound (e.g., 30000 ms). No preference can set TTL above this bound.
8. Schema_migration_policy preferences cannot override StoreAdmin ownership. Auto_forward still requires StoreAdmin authorization.
9. Retry_policy preferences cannot cause infinite retry. max_retries must have a compiled upper bound (e.g., 10).
10. Customization preference persistence respects E gate ordering. Preferences cannot persist before E9 gate passes. Proof marker: `[store.pref.persist.reject]`.
11. Every preference load, validate, apply, and reset operation produces a proof marker. No preference operation is silent.

### 21.6 STOP FIRST Conditions

12 additional STOP FIRST conditions for customization:

1. Any customization that bypasses redaction for Private or Secure content — redaction_strictness = relaxed may not expose these classes.
2. Any preference that allows non-monotonic sequence_id generation.
3. Any corruption_mode setting that allows confirmed-corrupt data to be returned to callers.
4. Any customization that re-enables POSIX unlink semantics for delete — delete is always tombstone.
5. Any capability_cache_ttl_ms set to 0 (caching disabled) or above compiled maximum.
6. Any retry_policy with max_retries = 0 (retry disabled) or above compiled maximum.
7. Any schema_migration_policy that allows silent data drop without proof marker.
8. Any customization that grants StoreAdmin capability to unprivileged callers.
9. Any preference that conflicts with E8 privacy redaction policy — redaction_strictness = relaxed may not expose Private content.
10. Any preference that enables disk persistence before E9 gate passes.
11. Any customization that introduces std/POSIX assumptions for storage paths.
12. Any preference change applied without validation — validation step is never skippable.

### 21.7 Proof Markers

7 additional proof markers for customization:

```
[store.pref.load] pref=KEY value=VALIDATED|REJECTED reason=REASON
[store.pref.apply] pref=KEY value=VALUE
[store.pref.reject] pref=KEY reason=REASON
[store.pref.reset] pref=KEY to=DEFAULT
[store.pref.cap.revalidate] pref=KEY reason=changed_affects_capability
[store.pref.persist.reject] pref=KEY reason=before_E9_gate
[store.pref.redact] pref=KEY class=private
```

### 21.8 Additional Proof Scenarios

13 additional scenarios for customization:

1. Load preference from memory → validated OK → applied. `[store.pref.load]` status=ok, `[store.pref.apply]`.
2. Load preference → validation rejects (out of bounds) → rejected, default used instead. `[store.pref.load]` status=rejected, `[store.pref.reject]`.
3. Apply corruption_mode = attempt_repair → repair logic enabled for suspect entries (CorruptionState = Suspect), but fail_safe still takes precedence on confirmed Corrupt.
4. Apply corruption_mode = fail_safe → fail-safe enforced. Confirmed-corrupt entries never returned to caller.
5. Apply redaction_strictness = relaxed → Private keys still redacted (non-customizable boundary §21.2.2 enforced). `[store.pref.redact]` logged for each redacted access.
6. Apply capability_cache_ttl_ms = 5000 → caching enabled with 5s TTL, checked against compiled max (e.g., 30000). `[store.pref.apply]` logged.
7. Apply capability_cache_ttl_ms = 0 → rejected (STOP FIRST §21.6.5). `[store.pref.reject]` logged, previous TTL retained.
8. Apply schema_migration_policy = auto_forward → migration runs on version mismatch, still requires StoreAdmin authorization. Migration + authorization both proof-marked.
9. Apply retry_policy with max_retries = 15 (above compiled max of 10) → rejected. `[store.pref.reject]` logged, compiled max used.
10. Reset all preferences to defaults → all 9 preferences revert to compiled-in values. `[store.pref.reset]` logged for each, `[store.pref.apply]` for defaults.
11. Attempt preference persistence before E9 → rejected with `[store.pref.persist.reject]`. Preferences remain memory-only.
12. Preference change affecting key_range_size → capability re-validation triggered before change takes effect. `[store.pref.cap.revalidate]` logged, then `[store.pref.apply]`.
13. Redact private preference from proof log → preference key/value not logged in full. `[store.pref.redact]` marker logged with class=private.

### 21.9 Exceeded Hypothesis — Customization

Assume a rival storage system (e.g., a key-value store with per-tenant configuration) gained adoption over SexOS storage by offering richer customization. Each row identifies the rival advantage, maps the loss category, and describes the SexOS-native countermeasure.

| # | Rival Advantage | Loss Category | SexOS Countermeasure |
|---|----------------|---------------|----------------------|
| 1 | Per-tenant storage with customizable durability (sync vs. async write) | Feature gap | §21.1.5 corruption_mode — customizable failure behavior within fail_safe boundary. Deterministic proof markers for every write guarantee auditability regardless of mode. |
| 2 | Granular per-operation retry policy (per-key backoff, per-key max retries) | Granularity gap | §21.1.6 retry_policy — single policy with compiled max_retries bound. Per-operation retry is unnecessary with deterministic K/V ops (no network, no disk in V1). |
| 3 | Auto-repair on corruption with per-entry recovery policies | Feature gap | §21.1.5 corruption_mode = attempt_repair. Advisory only — fail_safe always wins on confirmed corruption. Safety over flexibility. |
| 4 | Per-customer redaction levels (different redaction for different auditors) | Granularity gap | §21.1.9 redaction_strictness — three levels within non-customizable Private/Secure boundary §21.2.2. Redaction is per-log-target, not per-auditor. Uniform policy prevents accidental exposure. |
| 5 | Schema auto-migration with automatic rollback on failure | Feature gap | §21.1.4 schema_migration_policy = auto_forward_with_rollback. Still requires StoreAdmin authorization (§21.2.8). Safety gate prevents unauthorized migration. |
| 6 | Per-device capability caching with configurable TTL per device | Granularity gap | §21.1.8 capability_cache_ttl_ms — single TTL with compiled max bound (§21.2.17). Per-device TTL adds complexity without safety benefit in single-kernel SexOS. |
| 7 | Unlimited key range size per tenant | Flexibility gap | §21.1.2 key_range_size — bounded power-of-2 sizes (§21.2.5). Range limits enable collision detection and capability granularity. Unlimited ranges increase collision surface. |

### 21.10 Customization Implementation Phase

**E11_STORAGE_CUSTOMIZATION_POLICY_V1** — Define storage-layer customization policy: preference model, validation rules, non-customizable boundaries, preference lifecycle, Capability re-validation triggers, persistence gate. Handoff doc.

Phase E11 depends on:
- E9 (persistent backend gate) — preference persistence requires E9
- E8 (privacy redaction) — preference proof log redaction requires E8
- E7 (deterministic proofs) — preference proof markers depend on E7 proof infrastructure

