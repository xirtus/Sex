# E9_STORAGE_DURABLE_BACKEND_GATE_V1

**Status:** Docs/spec only. No code changed. No backend implementation.

**Date:** 2026-05-05

**Review gate:** "Accept E9 only if it is docs-only and does not implement durable storage."

---

## Summary

Defines the durable-storage backend gate: the set of requirements, constraints, and blocked features that govern when and how disk-backed or session-persistent storage may be added to SexOS. V1 remains RAM-only. E9 is a **policy gate** — no backend implementation, no disk code, no block device integration.

---

## 1. Current Status (E1–E8)

### 1.1 Phase ladder status

| Phase | Title | Status | Type |
|-------|-------|--------|------|
| E1 | Storage Boundary Audit | ✅ Complete | Docs only |
| E2 | Storage Protocol Spec | ✅ Complete | Docs only |
| E3 | StoreCapability Policy Spec | ✅ Complete | Docs only |
| E4 | Schema/Value Validation | ✅ Complete | Code |
| E5 | Generation/Tombstone Spec | ✅ Complete | Docs only |
| E6 | DELETE/Tombstone Implementation | ✅ Complete | Code |
| E7 | Proof Marker Hardening | ✅ Complete | Code |
| E8 | Privacy Redaction Policy | ✅ Complete | Docs only |

### 1.2 E9 pre-audit status

| Check | Status | Evidence |
|-------|--------|----------|
| Pre-audit | ✅ GO | `E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1.md` — no critical/high findings |
| Blocker fixes | ✅ Complete | `68ba28d` — CRITICAL-1, HIGH-1, MEDIUM-3 |
| Re-audit | ✅ GO | `E9_PRE_DURABLE_STORAGE_REAUDIT_V1.md` — no critical/high remaining |
| Build | ✅ Success | `[SEXOS ENTRYPOINT] success` |

### 1.3 Key current properties

- **sexstore is RAM-only** — 16-slot static K/V table, 256 bytes, no disk, no persistence across power cycles
- **`OP_KV_DEL = 0xB2` is local** — defined only in `servers/sexstore/src/main.rs:27`, not in `crates/sex-pdx/src/lib.rs`. Not public ABI canon.
- **Shell client handles status replies** — `STORE_REPLY_STATUS_BIT` dispatch in `servers/silk-shell/src/main.rs:1470-1514`
- **Proof markers log metadata only** — all 18 marker types classified StructuralMeta or PublicProof (per E8)
- **Capability gate present** — `store_cap_allowed()` on all 3 dispatch paths (PUT, GET, DEL)
- **caller_pd is kernel-authoritative** — verified end-to-end through `ipc.rs:193` → `syscalls/mod.rs:255`

---

## 2. Durable Backend Entry Criteria

All criteria must be satisfied before any durable storage implementation begins. E9 is the gatekeeper — no code may bypass.

### 2.1 Gating conditions

| # | Criterion | Status | Notes |
|---|-----------|--------|-------|
| 1 | No critical or high audit findings | ✅ GO | Pre-audit + re-audit both clean |
| 2 | Medium risks documented | ✅ 3 documented | See §4 |
| 3 | Privacy redaction policy enforced | ✅ E8 complete | No SecretContent in proof logs |
| 4 | Backend must be bounded and deterministic | ⬜ E10+ | Design constraint, not yet implemented |
| 5 | No POSIX/filesystem assumptions | ⬜ Enforced | STOP FIRST if violated |
| 6 | No raw paths in storage protocol | ✅ Current code | No path strings in sexstore |
| 7 | No heap/String allocations (or explicit approval) | ✅ Current code | sexstore is no_std, fixed-size |
| 8 | No app direct storage caps | ✅ Current code | Only silk-shell has SLOT_SEXSTORE |
| 9 | No cross-PD raw pointers | ✅ Kernel-enforced | MPK/PKU isolation |
| 10 | No kernel/ABI edits unless STOP FIRST | N/A | Not triggered yet |

### 2.2 Gate flow

```
E9 gate → entry criteria met? → YES → E10+ design/implementation tracks
                              → NO  → blocker fix + re-audit
```

### 2.3 Enforcement

E9 is a **docs-only policy gate**. Enforcement is manual at this stage:
- Any proposal to add durable storage code must reference E9
- Any proposed change that violates a gating condition triggers STOP FIRST
- Code review must verify E9 compliance before durable storage merges

---

## 3. Blocked Features

The following features are **blocked until E9 gate criteria are met and E10+ design tracks produce approved specs**:

### 3.1 Absolutely blocked (STOP FIRST)

| Feature | Block reason | Unblock condition |
|---------|-------------|-------------------|
| Disk-backed persistence | V1 is RAM-only | E11+ design approved |
| Raw file paths in storage protocol | Privacy violation, POSIX assumption | E8 redaction + E10 design |
| App direct storage caps | Capability model not extended | E3 StoreCapability implementation |
| LIST/ENUM on storage keys | No privacy/capability design | Separate spec required |
| Linen/Quil durable documents | E gates not all passed | E2, E3, E4, E5, E6, E8 maturity |
| Kernel/ABI changes for storage | Premature without design | STOP FIRST review |
| sex-pdx protocol promotion of OP_KV_DEL | Premature ABI canon | Explicit promotion decision |

### 3.2 Blocked pending design

| Feature | Block reason | Unblock condition |
|---------|-------------|-------------------|
| Multi-slot K/V values | No bounded-transport design | E10+ design |
| Variable-length messages | No heap/String approval | Architecture decision |
| Background flush / write-back | Reply buffer depth issue (§4.1) | MEDIUM-1 fix |
| Proof marker persistence | Redaction enforcement needed | E8 + E10 design |
| Schema versioning / migration | No implementation design | E10+ design |
| Corruption detection / recovery | No implementation design | E10+ design |

### 3.3 Allowed next implementation tracks

These tracks may begin **after E9 gate passes**:

| Track | Scope | Implementation allowed? |
|-------|-------|------------------------|
| E10_MEDIUM_RISK_CLEANUP_V1 | Fix 3 medium risks from audit | ✅ Code changes allowed |
| E11_DURABLE_BACKEND_DESIGN_V1 | Design durable backend architecture | ❌ Docs only |
| E12_RAM_TO_DURABLE_MIGRATION_SPEC_V1 | Spec for RAM→durable migration | ❌ Docs only |

No backend implementation in E9 or E10. E11+ may include implementation after approved design.

---

## 4. Remaining Medium Risks

Three medium-severity findings from the E9 pre-audit remain unfixed. They are non-blocking for E9 (docs-only gate) but must be resolved **before durable storage implementation**.

### 4.1 MEDIUM-1: Reply buffer depth of 1

| Field | Value |
|-------|-------|
| **File** | `kernel/src/capability.rs:247`, `kernel/src/ipc/router.rs:36` |
| **Risk** | `incoming_replies` is `VecDeque::with_capacity(1)`. `send_reply()` silently drops oldest reply if ≥ 2 arrive before processing. |
| **Why non-blocking for E9** | Current protocol is synchronous (listen → process → reply). No impacted path today. |
| **Must fix before** | Any async storage operation (background flush, write-back cache, batched replies). |
| **Fix** | Increase buffer depth to ≥ 8, or add backpressure with `Err(BufferFull)` return. |

### 4.2 MEDIUM-2: Hardcoded KV_SHELL_CALLER = 3

| Field | Value |
|-------|-------|
| **File** | `servers/sexstore/src/main.rs:111` |
| **Risk** | `const KV_SHELL_CALLER: u64 = 3;` is hardcoded. If silk-shell spawns with a different PD ID, storage silently denies all operations. |
| **Why non-blocking for E9** | `kernel/src/init.rs:39` uses fixed spawn order — silk-shell always gets domain 3. Brittle but stable. |
| **Must fix before** | Multi-shell support, dynamic spawn order, or any durable storage configuration. |
| **Fix** | Replace with boot-time grant (kernel passes authorized PD ID as init arg) or static capability table in sexstore. |

### 4.3 MEDIUM-3: Reclaimed slot keeps old generation

| Field | Value |
|-------|-------|
| **File** | `servers/sexstore/src/main.rs:278` |
| **Risk** | Reclaiming a tombstoned slot calls `bump_generation()` which increments from old value instead of resetting to 1. New key inherits old key's generation counter. |
| **Why non-blocking for E9** | Generation is internal-only (no caller protocol depends on it). No security impact. |
| **Must fix before** | Any CAS/optimistic-concurrency protocol that uses generation as a version token. |
| **Fix** | Reset generation to 1 on reclaim for semantic clarity, or document the continuous-counter behavior as intentional. |

---

## 5. Backend Design Constraints

These invariants govern any future durable backend design. Violating any must trigger STOP FIRST.

### 5.1 Authority model

| # | Invariant | Rationale |
|---|-----------|-----------|
| 1 | Storage authority remains sexstore | sexstore is the single K/V authority. No second storage path. |
| 2 | Clients use PDX only | No raw syscalls, no shared memory, no direct hardware access for storage. |
| 3 | Durable layer must not expose raw paths | Raw paths violate E8 redaction policy. Opaque u32 keys only. |
| 4 | No app PD gets direct storage capability | App storage requests go through shell or Linen, not directly to sexstore. |

### 5.2 Write safety

| # | Invariant | Rationale |
|---|-----------|-----------|
| 5 | Durable writes must be atomic or recovery-safe | Crash during write must not produce partially-written state on reboot. |
| 6 | Corruption must fail closed | Corrupt data is never returned to callers. Defaults/error on corruption. |
| 7 | Tombstones/generations must survive backend model | If generation exists in RAM store, durable backend must preserve generation semantics. |

### 5.3 Boundedness

| # | Invariant | Rationale |
|---|-----------|-----------|
| 8 | Storage must remain bounded | No unbounded growth. Eviction/tombstone/reclamation policy required. |
| 9 | Fixed-size operations (no heap/String) | Current no_std constraint. Heap/String requires explicit architecture approval. |
| 10 | Deterministic dispatch | Every storage operation produces a deterministic outcome + proof marker. |

### 5.4 Redaction (from E8)

| # | Invariant | Rationale |
|---|-----------|-----------|
| 11 | No SecretContent in proof logs | E8 classifies stored values, paths, document titles as SecretContent — never logged. |
| 12 | SensitiveMeta requires capability gate | Object IDs, restore tokens, document references must be redacted in public logs. |
| 13 | All markers must have a redaction class | Before any marker is persisted to durable log, its redaction class must be assigned. |

### 5.5 Prohibited (STOP FIRST in backend design)

| # | Prohibited | Why |
|---|-----------|-----|
| 14 | LIST/ENUM without separate privacy/capability design | Key enumeration reveals existence — privacy leak without capability gate. |
| 15 | Direct hardware block device access | Storage must go through sexstore. Kernel block ABI changes require separate design. |
| 16 | POSIX unlink semantics for delete | Delete is tombstone — always recoverable by StoreAdmin until reclamation. |

---

## 6. STOP FIRST Conditions

Any of the following conditions must halt durable storage work and trigger review:

| # | Condition | Action |
|---|-----------|--------|
| 1 | Needs kernel/ABI change | STOP — design must avoid kernel changes |
| 2 | Needs sex-pdx public protocol promotion | STOP — OP_KV_DEL stays local until explicit promotion |
| 3 | Needs disk/filesystem/raw path assumptions | STOP — no POSIX paths |
| 4 | Needs app direct storage caps | STOP — capability model must be extended first |
| 5 | Needs LIST/ENUM | STOP — privacy + capability design required |
| 6 | Needs Linen/Quil durable docs | STOP — E gates not all passed |
| 7 | Needs heap/String/broad refactor | STOP — requires explicit architecture approval |
| 8 | Needs logging of values/content/titles/paths | STOP — E8 redaction forbids SecretContent |
| 9 | Backend cannot guarantee bounded growth | STOP — unbounded storage is not acceptable |
| 10 | Backend cannot survive crash without corruption | STOP — atomicity required |
| 11 | Backend introduces cross-PD raw pointers | STOP — MPK/PKU isolation must be preserved |
| 12 | E9 gate criteria not met before implementation | STOP — E9 is the gate |

---

## 7. Allowed Next Tracks (E10+)

### 7.1 E10_MEDIUM_RISK_CLEANUP_V1

**Scope:** Fix the 3 medium risks documented in §4.
- **Code changes allowed** — sexstore and kernel changes OK
- Fix reply buffer depth (capability.rs, router.rs)
- Replace `KV_SHELL_CALLER = 3` with boot-time grant or static table
- Fix or document reclaimed generation behavior

### 7.2 E11_DURABLE_BACKEND_DESIGN_V1

**Scope:** Design the durable backend architecture. **Docs only.**
- Backend storage model (block device, disk format, partition layout)
- Crash recovery protocol (write-ahead log, checkpoint, rollback)
- Generation/tombstone persistence model
- Boundedness and eviction policy
- Integration with sexstore K/V dispatch

### 7.3 E12_RAM_TO_DURABLE_MIGRATION_SPEC_V1

**Scope:** Spec for migrating from RAM-only to durable storage. **Docs only.**
- Boot sequence: detect durable backend presence
- Migration: copy RAM contents to durable on first write
- Fallback: operate RAM-only if durable unavailable
- Version detection: detect stale/incompatible durable data

---

## 8. Final Gate Verdict

```
E9_STORAGE_DURABLE_BACKEND_GATE_V1

Status: PASS — gate approved.

Verdict: GO for E10+ design tracks, subject to:
  - E10 may implement medium-risk fixes (code changes allowed)
  - E11 and E12 are docs-only — no backend implementation
  - Any backend implementation requires design approval + E9 re-check
  - STOP FIRST conditions remain in effect for all future work

Blocked until further notice:
  - Disk-backed persistence
  - Raw file paths in storage protocol
  - App direct storage caps
  - LIST/ENUM
  - Linen/Quil durable documents
  - Kernel/ABI changes for storage
  - sex-pdx protocol promotion of OP_KV_DEL

sexstore remains RAM-only. V1 is RAM-only.
```

---

## Appendix A: Files Referenced

| File | Relevance |
|------|-----------|
| `docs/PERSISTENT_STORAGE_MATURITY_PLAN_V1.md` | Master plan |
| `docs/handoff/E1_STORAGE_BOUNDARY_AUDIT_V1.md` | Storage topology audit |
| `docs/handoff/E2_STORAGE_PROTOCOL_SPEC_V1.md` | Protocol spec, key namespace |
| `docs/handoff/E3_STORAGE_CAPABILITY_POLICY_SPEC_V1.md` | Capability model |
| `docs/handoff/E4_STORAGE_SCHEMA_VALIDATION_V1.md` | Schema/value validation |
| `docs/handoff/E5_STORAGE_GENERATION_TOMBSTONE_SPEC_V1.md` | Generation/tombstone spec |
| `docs/handoff/E6_STORAGE_TOMBSTONE_DELETE_V1.md` | DELETE/tombstone implementation |
| `docs/handoff/E7_STORAGE_PROOF_MARKER_HARDENING_V1.md` | Proof marker hardening |
| `docs/handoff/E8_STORAGE_REDACTION_POLICY_V1.md` | Redaction policy |
| `docs/handoff/E9_PRE_DURABLE_STORAGE_SAFETY_AUDIT_V1.md` | Pre-audit report |
| `docs/handoff/E9_PRE_DURABLE_STORAGE_REAUDIT_V1.md` | Re-audit report |
| `docs/handoff/E9_BLOCKER_FIX_CRITICAL_MEDIUM_V1.md` | CRITICAL-1 + MEDIUM-3 fixes |
| `docs/handoff/E9_BLOCKER_FIX_HIGH1_SHELL_STORAGE_CLIENT_V1.md` | HIGH-1 fix |
| `servers/sexstore/src/main.rs` | sexstore implementation |
| `servers/silk-shell/src/main.rs` | Shell storage client |
| `kernel/src/capability.rs` | Reply buffer (MEDIUM-1) |
| `kernel/src/ipc/router.rs` | send_reply (MEDIUM-1) |
