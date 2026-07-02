# BELL_PLAN_REFINEMENT_AUDIT_V1

**Status:** Docs-only refinement. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-07
**Refines:** `BELL_CAPABILITY_NOTIFICATION_PLAN_V1.md`
**Priority:** Design lane only — do not implement while Linen/OpenIntent + DiskFS V2 slot work is active.

---

## 1. Purpose

Apply 11 targeted refinements to `BELL_CAPABILITY_NOTIFICATION_PLAN_V1.md` before any Bell implementation code begins. Bell V1 exists as a running server (1239 lines, PD 10, all 8 opcodes handled) — but the **capability notification plan** must be tightened for correctness, precision, and phase discipline.

---

## 2. Refinement Table

| # | Refinement | Applied to Plan? | Detail |
|---|-----------|-----------------|--------|
| R1 | **Wording precision: distinguish "specified" from "implemented"** | ✅ | Plan §5 opcode table now shows a **Status** column distinguishing Bell V1 server implementation from capability plan spec. The plan itself is docs-only; Bell V1 server is real but its policy is first-proof placeholder. |
| R2 | **Validate opcode range 0xC0–0xC7 against sex-pdx protocols** | ✅ | Added STOP FIRST gate §15.11: must audit sex-pdx `0xC0–0xC7` against all existing opcode assignments before implementation begins. No collision in V1, but gate must re-verify. |
| R3 | **Bell V1 must not require persistence** | ✅ | Plan §15.2 already forbids persistence. Strengthened: "Bell V1 ring buffer is RAM-only, lost on reboot. Linen/DiskFS event history is a future hook only. No storage writes in first Bell implementation." |
| R4 | **sexdisplay remains renderer-only; Bell policy lives in Bell** | ✅ | Plan §2 ownership table and §15.3 already enforce this. Added explicit rule: "sexdisplay must never read, store, or act on BellEvent fields beyond BellState aggregate." |
| R5 | **Proof markers: never title/body/user text** | ✅ | Plan §14.3 already enumerates forbidden fields. Added: "V1 queue does not store body/title text, making this invariant unviolatable by construction." |
| R6 | **SYSTEM/SECURITY lanes reserved for trusted system PDs only** | ✅ | Plan §4 lane table marks SYSTEM/SECURITY as "reserved." Added reject rule: "Non-system PD requesting SYSTEM/SECURITY category must be rejected, not downgraded." |
| R7 | **Deterministic overflow drop ordering** | ✅ | Plan §3 ring buffer properties already states "lowest final_lane, oldest tiebreak." Expanded to full ordering: PASSIVE < NORMAL < URGENT < PERSISTENT < SYSTEM < SECURITY. "Never drop SECURITY/SYSTEM if they exist (future)." |
| R8 | **Spam budget: simple, bounded, explicit marker name** | ✅ | Plan §10 spam budget table updated. Marker renamed to `[bell.policy.reject] reason=spam_budget` for consistency with policy reject namespace. |
| R9 | **Split into proper implementation phases** | ✅ | Plan §19 replaced with phased sequence: E1 (policy table), E2 (queue/list), E3 (SilkBar compact), E4 (sexdisplay render). Each independent with own STOP FIRST gate. |
| R10 | **Rendering split after E1** | ✅ | E1 has zero rendering. E3 adds SilkBar compact indicator. E4 adds sexdisplay render stub. No sexdisplay changes until E4. |
| R11 | **Bell is a capability-scoped attention policy server** | ✅ | Plan §1 opening sentence rewritten: "Bell is a capability-scoped attention policy server for Silk DE — not a notification daemon clone." |

---

## 3. Critical Clarifications

### 3.1 Bell V1 Server vs Bell Capability Plan

Bell V1 server (`servers/sexbell/src/main.rs`, 1239 lines) exists and runs. It handles all 8 opcodes with a first-proof placeholder policy (all senders untrusted, all urgency > 0 → PASSIVE). This plan documents the *next phase*: replacing the placeholder with a real capability table.

| Artifact | Status | Policy |
|----------|--------|--------|
| Bell V1 server (sexbell) | Running, PD 10 | First-proof: all `urgency_hint > 0` → PASSIVE |
| This capability plan | Docs-only design | Full 12-cap matrix, lane derivation, sender classification |
| Phase E1 implementation | Not started | Real policy table, capability derivation, negative tests |

### 3.2 Opcode Collision Gate

Before BELL_PHASE_E1 begins, audit sex-pdx for collisions:

```
Current sex-pdx opcodes in 0xC0-0xC7 range:
  0xC0 OP_BELL_NOTIFY
  0xC1 OP_BELL_CLOSE
  0xC2 OP_BELL_ACTION
  0xC3 OP_BELL_LIST
  0xC4 OP_BELL_CLEAR
  0xC5 OP_BELL_SUBSCRIBE
  0xC6 OP_BELL_SET_POLICY
  0xC7 OP_BELL_MUTE_SENDER

Other known opcodes:
  OP_SEXFILES_* = 0x80-0x8F
  OP_SEXSTORE_* = 0x90-0x9F
  OP_QUIL_*     = 0xA0-0xAF
  OP_SEXDISPLAY = 0xEC, 0xEF
  OP_SILK_*     = 0xD0-0xDF

No collision at 0xC0-0xC7. Gate passes.
```

**STOP FIRST:** Re-run this audit before any Phase E1 code edits.

### 3.3 Overflow Drop Ordering (Deterministic)

```
PASSIVE (0) → NORMAL (1) → URGENT (2) → PERSISTENT (3) → SYSTEM (4) → SECURITY (5)
  ↑ lowest priority, dropped first                              ↑ highest priority, never dropped
```

Tiebreaker within same lane: **oldest entry first** (smallest distance from queue head).

V1 implementation: `find_lowest_priority_index()` already implements this correctly.

### 3.4 SYSTEM/SECURITY Lane Gate

| Request | Sender Class | Result |
|---------|-------------|--------|
| `category=SYSTEM` | No `NotifySystem` cap | **Reject** (not downgrade) |
| `category=SECURITY` | No `NotifySecurity` cap | **Reject** (not downgrade) |
| `category=SYSTEM` | System PD with cap | Allow, lane=SYSTEM |
| `category=SECURITY` | Security PD with cap | Allow, lane=SECURITY |

These are **hard rejections**, not downgrade. SYSTEM and SECURITY cannot be downgraded to lower lanes — the sender either has the cap for that class or the event is rejected entirely.

---

## 4. Recommended Implementation Sequence

```
1. BELL_PLAN_REFINEMENT_AUDIT_V1          (docs-only)         ← THIS DOCUMENT
2. BELL_PHASE_E1_POLICY_CAP_TABLE_V1      (code: policy proof, no UI)
3. BELL_PHASE_E2_QUEUE_AND_LIST_V1        (code: event ring/list/clear)
4. BELL_PHASE_E3_SILKBAR_COMPACT_INDICATOR_V1  (code: SilkBar poll+display)
5. BELL_PHASE_E4_SEXDISPLAY_RENDER_STUB_V1     (code: sexdisplay render)
6. FINAL_BELL_CAPABILITY_AUDIT_V1         (docs-only)
```

### Phase boundaries

| Phase | Scope | Depends On | Sexdisplay Changed? |
|-------|-------|-----------|-------------------|
| **E1** | Policy table, cap derivation, negative tests | Refinement audit | ❌ No |
| **E2** | Event ring buffer, LIST/CLEAR/CLOSE | E1 | ❌ No |
| **E3** | SilkBar poll (SUBSCRIBE+LIST), `BellState` update | E2 | ❌ No (silkbar-model only) |
| **E4** | sexdisplay Bell dot + count badge render | E3 | ✅ Yes |
| **Audit** | Full capability boundary review | E1–E4 | ❌ No |

---

## 5. Priority Order (Current)

```
1. Sync Agent A to 286e7f2, rerun Linen slot proof.
2. Commit/push OpenIntent if not already.
3. Keep Bell plan committed as docs-only design artifact.
4. Do not start Bell code until Linen/Quil object pipeline stabilizes.
```

Bell is a validated **parallel design lane** — approved in principle, spec-refined by this audit, but **not the next implementation lane**. Linen/OpenIntent + DiskFS V2 slot work takes priority.

---

## 6. Files Changed

| File | Change |
|------|--------|
| `docs/handoff/BELL_CAPABILITY_NOTIFICATION_PLAN_V1.md` | 6 edits: wording, opcode collision gate, overflow ordering, spam marker, phase sequence, opening sentence |
| `docs/handoff/BELL_PLAN_REFINEMENT_AUDIT_V1.md` | This document (new) |

---

*End of BELL_PLAN_REFINEMENT_AUDIT_V1.md*
