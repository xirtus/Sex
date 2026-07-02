# BELL_NOTIFY_PLAN_V1

**Status:** Docs-only plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_EVENT_MODEL_DESIGN_GATE_V1.md`, `BELL_CAPABILITY_POLICY_V1.md`, `BELL_PDX_PROTOCOL_SPEC_V1.md`, `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md`, `BELL_UNKNOWN_REJECT_CLEANUP_V1.md`

---

## 1. Purpose

Plan the first real `OP_BELL_NOTIFY` protocol crossing. Currently, sexbell is alive (domain 10, PKEY 10, SLOT_BELL=12 self-cap) and runs a listen loop that emits `[bell.unknown.reject]` for any message. Nobody sends it `OP_BELL_NOTIFY` — the temporary kernel enqueue was removed in `BELL_UNKNOWN_REJECT_CLEANUP_V1`.

This plan defines **one controlled proof cycle**: a single `OP_BELL_NOTIFY` from kernel to sexbell during boot, sexbell validates and replies, proof markers are verified in QEMU, then the kernel enqueue is removed. No retained boot behavior. No app notification grants. No queue, persistence, rendering, actions, or sound.

**No implementation.** This plan only. STOP FIRST gates apply before any code change.

---

## 2. Sender Decision: Kernel One-Shot (Removed After Proof)

### Options considered

| Option | Description | Verdict |
|--------|-------------|---------|
| **A: Kernel one-shot** | Single `MessageType::IpcCall` with `OP_BELL_NOTIFY` enqueued on sexbell's message ring during `init()` | **PREFERRED** — simplest, no permanent cap grants, no silk-shell edits, removed after proof |
| **B: silk-shell with temporary SLOT_BELL cap** | Add temporary `SLOT_BELL` grant to silk-shell in init.rs, send OP_BELL_NOTIFY from silk-shell boot path | Rejected — requires silk-shell code change, permanent cap grant pattern, higher blast radius |
| **C: Existing test sender** | Re-use the 0xFFFF test enqueue | Rejected — was proof scaffolding, already removed in cleanup phase |

### Decision

**Kernel one-shot direct message, removed after proof.**

Rationale:
- No permanent cap grants to any external sender (silk-shell, apps, etc.)
- No silk-shell or sex-pdx edits beyond the kernel's init.rs
- The kernel already has the mechanism (`MessageType::IpcCall` enqueue on `(*pd.message_ring).enqueue(msg)`)
- The kernel does NOT need a `SLOT_BELL` cap to send — kernel bypasses the capability layer for boot-time direct messages
- Clean removal: single `git revert` of the enqueue block after proof verification

### Constraint

The kernel enqueue is **proof scaffolding**, not retained boot behavior.
After proof verification (QEMU boot showing `[bell.notify.*]` markers), the enqueue is removed in a follow-up cleanup phase (`BELL_NOTIFY_CLEANUP_V1`).

---

## 3. Protocol Subset

Only fixed numeric arguments cross the wire in the first proof. No title, body, strings, hashes, file paths, or private payloads.

### PDX message encoding

The kernel enqueues an `IpcCall` message on sexbell's message ring:

```rust
MessageType::IpcCall {
    func_id:  OP_BELL_NOTIFY, // 0xC0
    arg0:     packed_fields,   // category(8) | urgency_hint(8) | privacy_level(8) | redaction_class(8) | padding(32)
    arg1:     0,               // action_count + reserved (zero for first proof)
    arg2:     0,               // object_refs count + reserved (zero for first proof)
    caller_pd: 0,              // kernel-authoritative; 0 = kernel-originated
}
```

### arg0 packed field layout

| Bits | Field | Values | Description |
|------|-------|--------|-------------|
| 0-7 | `category` | `0` = Info | BellCategory enum (see §3 of event model) |
| 8-15 | `urgency_hint` | `2` = URGENT | Tests downgrade (no caps → PASSIVE) |
| 16-23 | `privacy_level` | `0` = Public | BellPrivacyLevel enum |
| 24-31 | `redaction_class` | `0` = StructuralMeta | BellRedactionClass enum |
| 32-63 | `_reserved` | `0` | Zero for first proof |

### Why urgency_hint=2 (URGENT)?

To exercise the lane derivation algorithm with no caps:
- Sender class = unknown/untrusted (no BellCap entries for any sender yet)
- Max lane = PASSIVE (default-deny, untrusted)
- Requested URGENT → downgrade to NORMAL (trust_label < 1) → downgrade to PASSIVE (untrusted sender)
- This proves: enum parsing, cap lookup, three-step downgrade, marker emission, and reply all work

### Fields explicitly excluded from first proof

| Field | Reason |
|-------|--------|
| `action_count` | Would require ActionCallback cap check — excluded until future phase |
| `action_caps[]` | Opaque capability tokens — excluded until action dispatch phase |
| `object_refs[]` | Would require ObjectReference cap check — excluded until Linen integration |
| Title/body/sender name | No string fields in protocol V1 — never on wire |
| `sender_identity_token` | Opaque u32 — excluded from first proof (kernel sender has no identity token) |
| `expires_at_ticks` | No queue yet — excluded until ring buffer phase |

---

## 4. sexbell Dispatch (First Proof Implementation Plan)

When sexbell's listen loop receives an IpcCall with `type_id == OP_BELL_NOTIFY (0xC0)`:

### 4.1 Parse

```rust
// Extract packed fields from arg0
let category      = (msg.arg0 >> 0)  & 0xFF;
let urgency_hint  = (msg.arg0 >> 8)  & 0xFF;
let privacy_level = (msg.arg0 >> 16) & 0xFF;
let redaction_class = (msg.arg0 >> 24) & 0xFF;
let action_count  = msg.arg1 & 0xFF;
let object_ref_count = msg.arg2 & 0xFF;
let caller_pd     = msg.caller_pd; // kernel-authoritative
```

### 4.2 Validate Enum Ranges

| Check | Valid Range | Fail Action |
|-------|-------------|-------------|
| `category` | 0..=5 (Info..=Error) | Reject: `invalid_category` |
| `urgency_hint` | 0..=3 | Reject: `invalid_urgency` |
| `privacy_level` | 0..=3 (Public..=FullHidden) | Reject: `invalid_privacy` |
| `redaction_class` | 0..=3 (StructuralMeta..=SecretContent) | Reject: `invalid_redaction` |
| `action_count` | 0 only (first proof) | Reject: `action_count_not_zero` |
| `object_ref_count` | 0 only (first proof) | Reject: `object_refs_not_zero` |

On any validation failure: emit `[bell.notify.reject]`, reply with status=Invalid (2), return.

### 4.3 Cap Policy (Placeholder for First Proof)

For the first proof, sexbell has no cap table — no sender has been granted any BellCap entries. The minimal policy:

```rust
// First-proof placeholder policy: no caps → untrusted → max lane PASSIVE
// Full cap table implementation deferred to BELL_NOTIFY_CAPS_V1.

fn derive_lane_first_proof(caller_pd: u32, urgency_hint: u8) -> (u8, u8, Option<&'static str>) {
    // Step 1: No cap table → sender is untrusted
    // Step 2: Untrusted max lane = PASSIVE (0)
    // Step 3: urgency_hint 2 (URGENT) downgrade to NORMAL → untrusted downgrade to PASSIVE
    let final_lane = 0; // PASSIVE
    let final_urgency = 0;
    let downgrade_reason = Some("no_caps_untrusted");
    (final_lane, final_urgency, downgrade_reason)
}
```

Key behaviors:
- **No cap table entries** → every sender is `unknown/untrusted`
- **Unknown/untrusted** → max lane PASSIVE (0)
- Any `urgency_hint > 0` → downgraded to PASSIVE
- No reject (downgrade only — unknown senders are tolerated at PASSIVE)

### 4.4 Emit Proof Markers

| Marker | Condition | Fields |
|--------|-----------|--------|
| `[bell.notify.recv]` | Always on valid OP_BELL_NOTIFY | `caller_pd`, `category`, `requested` (lane from urgency_hint) |
| `[bell.notify.downgrade]` | When final_lane < requested_lane | `from`, `to`, `reason` |
| `[bell.notify.reject]` | On validation or cap failure | `caller_pd`, `reason` |
| `[bell.notify.ok]` | After successful lane derivation | `event_id` (0 for first proof, no queue), `caller_pd`, `final_lane` |

### 4.5 Reply

```rust
// Reply to sender (kernel does not use the reply, but sexbell sends it anyway for pattern completeness)
let _ = pdx_call_checked(msg.caller_pd as u64, status, event_id, final_lane as u64, reject_reason as u64);
```

Actually — **sexbell should NOT reply via pdx_call to the kernel.** The kernel's IpcCall direct message does not set up a reply path. Instead, sexbell should continue its listen loop after emitting markers. The reply pattern is only relevant when a userspace sender (like silk-shell) calls `pdx_call` and expects a return value.

For the kernel one-shot proof: sexbell processes, emits markers, and loops. No reply needed.

### 4.6 Marker Budget

| Marker | Per-Call Budget | Total Budget (boot) |
|--------|----------------|---------------------|
| `[bell.notify.recv]` | 1 | 8 |
| `[bell.notify.downgrade]` | 1 | 8 |
| `[bell.notify.ok]` | 1 | 8 |
| `[bell.notify.reject]` | 1 | 4 (only on failure) |

Static budget counters, matching sexstore and quil pattern. Budgets are generous for first proof — can be tightened in later phases.

---

## 5. Kernel Enqueue (Proof Scaffolding)

### Location

In `kernel/src/init.rs`, after the sexbell self-cap grant (after line 177), before the framebuffer handoff (line 179).

### Code sketch

```rust
// ── BELL_NOTIFY_PROOF_SCAFFOLDING ───────────────────────────────────
// REMOVAL PROMISE: This block is proof scaffolding only.
// It enqueues a single OP_BELL_NOTIFY to sexbell during boot to verify
// the first protocol crossing. After QEMU proof showing [bell.notify.*]
// markers, this block MUST be removed in BELL_NOTIFY_CLEANUP_V1.
//
// The kernel does NOT retain the ability to send OP_BELL_NOTIFY.
// No app notification caps are granted.
// No silk-shell, sex-pdx, or limine.cfg changes are needed.
if sexbell_id != 0 {
    use crate::ipc::messages::MessageType;

    // Pack fields: category=Info(0), urgency_hint=URGENT(2), privacy=Public(0), redaction=StructuralMeta(0)
    let arg0: u64 = (0u64 << 0)  | (2u64 << 8)  | (0u64 << 16) | (0u64 << 24);
    let msg = MessageType::IpcCall {
        func_id:   sex_pdx::OP_BELL_NOTIFY,  // 0xC0
        arg0,
        arg1:      0,   // action_count = 0
        arg2:      0,   // object_refs = 0
        caller_pd: 0,   // kernel-originated
    };
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        unsafe { let _ = (*pd.message_ring).enqueue(msg); }
        serial_println!("[kernel.bell.notify.test] enqueued OP_BELL_NOTIFY to sexbell");
    }
}
```

### Cleanup guarantee

This block is **exactly 18 lines** (including comments). After proof verification, `BELL_NOTIFY_CLEANUP_V1` removes it by deleting lines N..M from init.rs. No other file is touched.

---

## 6. Cap Policy for First Proof

### Current state

- sexbell has `SLOT_BELL` self-cap (granted in init.rs) — allows listening on slot 0
- No other domain has `SLOT_BELL` in their cap table
- No domain has any `BellCap` entries (NotifyPassive, NotifyNormal, etc.)

### First-proof cap policy

Since no sender has any BellCap entries, every sender is classified as **unknown/untrusted**:

| Sender Class | Max Lane | Downgrade |
|-------------|----------|-----------|
| Unknown/untrusted (no caps) | PASSIVE (0) | All urgency downgraded to passive |

### Future state (after BELL_NOTIFY_CAPS_V1)

A cap table will be added to sexbell that maps `caller_pd` → bitmask of granted `BellCap` values. The full lane derivation algorithm from `BELL_CAPABILITY_POLICY_V1 §4` will be implemented. This is explicitly **not in the first proof**.

### Key security properties preserved in first proof

| Property | How |
|----------|-----|
| Default-deny | No caps → max lane PASSIVE |
| Kernel-authoritative caller ID | `msg.caller_pd` is kernel-set |
| No private content in markers | All markers log only StructuralMeta fields |
| No retained boot behavior | Kernel enqueue removed after proof |
| No app cap grants | No `SLOT_BELL` granted to any app |

---

## 7. Proof Markers (Detailed Spec)

### Allowed markers

```
[bell.notify.recv]    caller_pd={} category={} requested={}
[bell.notify.downgrade] from={} to={} reason={}
[bell.notify.ok]      caller_pd={} final_lane={}
[bell.notify.reject]  caller_pd={} reason={}
```

### Forbidden patterns

```
[bell.notify.ok] title="..."          ← FORBIDDEN — no title/body fields in protocol
[bell.notify.recv] sender="AppName"    ← FORBIDDEN — no sender name field
[bell.notify.downgrade] file="..."     ← FORBIDDEN — no file paths
[bell.notify.ok] action_payload={}     ← FORBIDDEN — no action payloads in first proof
[bell.notify.recv] raw_args={...}      ← FORBIDDEN — raw arg dumps may contain private padding
```

### Expected boot log sequence (success path)

```
[bell.boot]
[bell.notify.recv] caller_pd=0 category=0 requested=2
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.notify.ok] caller_pd=0 final_lane=0
```

### Expected boot log (validation failure path)

```
[bell.boot]
[bell.notify.recv] caller_pd=0 category=0 requested=2
[bell.notify.reject] caller_pd=0 reason=invalid_category
```

---

## 8. Files Touched (Implementation Phase)

| File | Change | Type |
|------|--------|------|
| `servers/sexbell/src/main.rs` | Add OP_BELL_NOTIFY dispatch, enum validation, lane derivation, markers | Server edit |
| `kernel/src/init.rs` | Add 18-line scaffolding enqueue (removed after proof) | Scaffolding |
| `crates/sex-pdx/src/lib.rs` | (none — OP_BELL_NOTIFY=0xC0 already assigned) | — |
| `limine.cfg` | (none — sexbell already in module list) | — |
| `sexos_build_spec.toml` | (none — sexbell already in build spec) | — |

### Non-targets (explicitly)

- No sex-pdx edits (OP_BELL_NOTIFY=0xC0 already assigned)
- No new sex-pdx constants
- No new SLOT_* constants
- No silk-shell edits
- No sexdisplay edits
- No limine.cfg changes
- No sexos_build_spec.toml changes
- No ABI hash change
- No new workspace members
- No reply path wiring (kernel one-shot does not use reply)

---

## 9. Verification

**Before implementation**, verify these STOP FIRST conditions pass:

1. ✅ OP_BELL_NOTIFY=0xC0 already in sex-pdx (assigned in BELL_SLOT_OPCODE_ASSIGNMENT_V1)
2. ✅ SLOT_BELL=12 already in sex-pdx
3. ✅ sexbell spawn exists in kernel init.rs (domain 10, index 9)
4. ✅ sexbell self-cap granted (SLOT_BELL=12 self)
5. ✅ sexbell module in limine.cfg
6. ✅ sexbell build stage in sexos_build_spec.toml
7. ✅ No other sender has SLOT_BELL cap (kernel bypasses cap layer for direct messages)
8. ✅ No private content in protocol (numeric fields only)
9. ✅ No reply path needed (kernel one-shot — markers suffice)
10. ✅ Cleanup plan exists (BELL_NOTIFY_CLEANUP_V1 removes scaffolding)

**After implementation:**

1. Build: `./scripts/entrypoint_build.sh` → `[SEXOS ENTRYPOINT] success`
2. QEMU boot: verify `[bell.notify.recv]`, `[bell.notify.downgrade]`, `[bell.notify.ok]` appear
3. Verify `[bell.boot]` still present (no regression)
4. Verify `[bell.unknown.reject]` does NOT fire for OP_BELL_NOTIFY (correctly matched)
5. Verify no other PD crashes or shows errors
6. Cleanup: `BELL_NOTIFY_CLEANUP_V1` removes kernel enqueue, keeps sexbell dispatch

---

## 10. Cleanup: BELL_NOTIFY_CLEANUP_V1

After proof verification, the following cleanup is required:

1. **Remove kernel scaffolding** — delete the 18-line enqueue block from `init.rs`
2. **Keep sexbell dispatch** — sexbell keeps `OP_BELL_NOTIFY` matching logic, marker emission, and lane derivation. This is NOT scaffolding — it is the real protocol handler that will receive messages from real senders (silk-shell, etc.) in future phases.
3. **Verify no regression** — sexbell still spawns, listens, and `[bell.boot]` still appears. No OP_BELL_NOTIFY fires (nobody sends it yet), but sexbell is ready to handle it when a real sender connects.

### What persists after cleanup

| Component | Persists? | Rationale |
|-----------|-----------|-----------|
| sexbell OP_BELL_NOTIFY dispatch | ✅ Yes | Real protocol handler, needed for future phases |
| Enum validation | ✅ Yes | Security invariant, always required |
| Lane derivation | ✅ Yes | Core policy function, always required |
| Proof markers | ✅ Yes | StructuralMeta-only, required for all phases |
| Kernel one-shot enqueue | ❌ Removed | Proof scaffolding, not retained boot behavior |
| Reply path | ❌ N/A | Not implemented for kernel sender |

---

## 11. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Adding any string/body/title field to the protocol or proof markers.
2. Keeping the kernel enqueue beyond the proof phase (must be removed in cleanup).
3. Granting SLOT_BELL to any app or external sender without a separate cap policy review.
4. Adding a ring buffer, queue, or event storage to sexbell (deferred to BELL_NOTIFY_RAM_QUEUE_V1).
5. Adding action callback dispatch (deferred to BELL_ACTION_CAPS_V1).
6. Adding persistence via sexstore (deferred to BELL_PERSISTENCE_GATE_V1).
7. Adding SilkBar presence or inbox rendering (deferred to separate phases).
8. Adding sound hints or audio integration (deferred to Harp/Theremin gate).
9. Adding real BellCap entries to any sender without a separate cap grant phase.
10. Adding `sender_identity_token` or any opaque identity field without a design review.

---

## 12. Future Phases (after this plan)

| Phase | Scope | Type |
|-------|-------|------|
| **BELL_NOTIFY_IMPLEMENT_V1** | Add OP_BELL_NOTIFY dispatch to sexbell + kernel scaffolding | Implementation |
| **BELL_NOTIFY_PROOF_V1** | QEMU boot proof showing [bell.notify.*] markers | Proof |
| **BELL_NOTIFY_CLEANUP_V1** | Remove kernel scaffolding, verify sexbell-only | Cleanup |
| **BELL_NOTIFY_CAPS_V1** | Add real BellCap table to sexbell, assign first sender caps | Implementation |
| **BELL_NOTIFY_RAM_QUEUE_V1** | Add bounded ring buffer, event lifecycle (expiry, dismiss) | Implementation |
| **BELL_SILKBAR_PRESENCE_V1** | Compact lane-summary indicator in SilkBar | Implementation |
| **BELL_INBOX_ROWS_V1** | Full inbox surface adopting SILK_LIST_ROW_VISUAL_CANON | Implementation |

---

## References

- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — event model, lanes, privacy, BellEvent struct
- `BELL_CAPABILITY_POLICY_V1.md` — default-deny, sender classes, lane derivation algorithm
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — BellNotifyRequest (56 bytes), BellNotifyReply, status codes
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — SLOT_BELL=12, OP_BELL_NOTIFY=0xC0 assignments
- `BELL_UNKNOWN_REJECT_CLEANUP_V1.md` — previous cleanup removed test enqueue
- `kernel/src/init.rs` — spawn order, sexbell self-cap, location for proof scaffolding
- `servers/sexbell/src/main.rs` — current minimal stub with unknown-reject loop
- `servers/quil/src/main.rs` — reference for sexbell dispatch pattern (same structure)

---

*End of BELL_NOTIFY_PLAN_V1.md*
