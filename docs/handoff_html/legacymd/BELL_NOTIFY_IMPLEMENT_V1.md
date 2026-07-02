# BELL_NOTIFY_IMPLEMENT_V1

**Status:** Implementation complete.
**Build:** `[SEXOS ENTRYPOINT] success`
**Date:** 2026-05-05
**Depends on:** `BELL_NOTIFY_PLAN_V1.md`

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexbell/src/main.rs` | Add OP_BELL_NOTIFY dispatch with enum validation, lane derivation, proof markers | ~90 |
| `kernel/src/init.rs` | Add temporary one-shot OP_BELL_NOTIFY enqueue (proof scaffolding) | +18 |

**Not touched:**
- `crates/sex-pdx/src/lib.rs` — no edits (OP_BELL_NOTIFY=0xC0 already assigned)
- `limine.cfg` — no change (sexbell already in module list)
- `sexos_build_spec.toml` — no change
- `servers/silk-shell/` — no change
- `servers/sexdisplay/` — no change
- Any other file

---

## Exact Arg Packing

Kernel enqueues `MessageType::IpcCall` on sexbell's message ring:

```rust
MessageType::IpcCall {
    func_id:   sex_pdx::OP_BELL_NOTIFY,   // 0xC0
    arg0:      packed_field,               // category:u8 | urgency_hint:u8 | privacy_level:u8 | redaction_class:u8 | padding:u32
    arg1:      0,                          // action_count = 0
    arg2:      0,                          // object_refs = 0
    caller_pd: 0,                          // kernel-originated
}
```

### arg0 bit layout

| Bits | Field | Value | Meaning |
|------|-------|-------|---------|
| 0-7 | `category` | 0 | Info |
| 8-15 | `urgency_hint` | 2 | URGENT |
| 16-23 | `privacy_level` | 0 | Public |
| 24-31 | `redaction_class` | 0 | StructuralMeta |
| 32-63 | `_reserved` | 0 | Zero |

### sexbell parsing

```rust
let category        = ((msg.arg0 >> 0)  & 0xFF) as u8;
let urgency_hint    = ((msg.arg0 >> 8)  & 0xFF) as u8;
let privacy_level   = ((msg.arg0 >> 16) & 0xFF) as u8;
let redaction_class = ((msg.arg0 >> 24) & 0xFF) as u8;
let action_count    = (msg.arg1 & 0xFF) as u8;
let object_refs     = (msg.arg2 & 0xFF) as u8;
let caller_pd       = msg.caller_pd;  // kernel-authoritative u32
```

---

## Validation Ranges

| Field | Valid Range | Reject Reason |
|-------|-------------|---------------|
| `category` | 0..=5 (Info..=Error) | `invalid_category` |
| `urgency_hint` | 0..=3 | `invalid_urgency` |
| `privacy_level` | 0..=3 (Public..=FullHidden) | `invalid_privacy` |
| `redaction_class` | 0..=3 (StructuralMeta..=SecretContent) | `invalid_redaction` |
| `action_count` | 0 only (first proof) | `action_count_not_zero` |
| `object_refs` | 0 only (first proof) | `object_refs_not_zero` |

First invalid field triggers rejection; subsequent fields are not checked.

---

## Downgrade Rule

First-proof placeholder policy — no BellCap table exists yet:

```
urgency_hint 0 → PASSIVE (no downgrade)
urgency_hint ≥ 1 → PASSIVE (downgrade: "no_caps_untrusted")
```

Every sender is classified as unknown/untrusted because no sender has any BellCap entries. Unknown/untrusted max lane = PASSIVE (0). Any non-zero urgency hint is downgraded to PASSIVE.

---

## Markers

| Marker | Budget | Fields | Condition |
|--------|--------|--------|-----------|
| `[bell.notify.recv]` | 8 | `caller_pd`, `category`, `requested` (urgency_hint) | After successful parse + validation |
| `[bell.notify.downgrade]` | 8 | `from`, `to`, `reason` | When final lane < requested |
| `[bell.notify.ok]` | 8 | `caller_pd`, `final_lane` | After lane derivation |
| `[bell.notify.reject]` | 4 | `caller_pd`, `reason` | On validation failure |

### Expected boot log (success — full downgrade path)

```
[bell.boot]
[bell.notify.recv] caller_pd=0 category=0 requested=2
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.notify.ok] caller_pd=0 final_lane=0
```

### Forbidden marker content

No title, body, sender name, file path, action payload, raw arg dump, or any private content in any marker. All fields are StructuralMeta-only.

---

## Temporary Kernel Scaffolding

**Location:** `kernel/src/init.rs`, after sexbell self-cap grant (line 177), before framebuffer handoff (line 179).

**Guard:** `if sexbell_id != 0` (no-op if sexbell not spawned).

**Content:** Single `MessageType::IpcCall` enqueue with `OP_BELL_NOTIFY`, fixed numeric fields, `caller_pd=0`.

**Marker:** `[kernel.sexbell.notify.test]` emitted on enqueue.

**REMOVAL PROMISE:** This block is proof scaffolding only. After QEMU proof showing `[bell.notify.*]` markers, it MUST be removed in `BELL_NOTIFY_CLEANUP_V1`. The kernel does NOT retain the ability to send OP_BELL_NOTIFY.

---

## Private Content Confirmation

**No private content crosses the wire or appears in markers.**

| Category | Present? | Rationale |
|----------|----------|-----------|
| Title/body/sender name | ❌ | No string fields in protocol |
| File paths | ❌ | No path fields in args |
| Object references | ❌ | arg2 = 0, rejected if non-zero |
| Action payloads | ❌ | arg1 = 0, rejected if non-zero |
| Sender identity token | ❌ | Not included in first proof |
| Raw arg dumps in markers | ❌ | Only parsed named fields logged |
| caller_pd | ✅ | Kernel-authoritative u32, StructuralMeta |
| category / urgency / lane | ✅ | Numeric enums, StructuralMeta |

---

## No Queue / Storage / Rendering / SilkBar / Action / Sound

| Feature | Status | Reason |
|---------|--------|--------|
| Ring buffer / queue | ❌ Not implemented | Deferred to BELL_NOTIFY_RAM_QUEUE_V1 |
| Sexstore persistence | ❌ Not implemented | Deferred to Bell persistence gate |
| Sexdisplay rendering | ❌ Not implemented | Deferred to BELL_INBOX_ROWS_V1 |
| SilkBar presence | ❌ Not implemented | Deferred to BELL_SILKBAR_PRESENCE_V1 |
| Action callbacks | ❌ Not implemented | Deferred to BELL_ACTION_CAPS_V1 |
| Sound / audio | ❌ Not implemented | Deferred to Harp/Theremin gate |
| Reply path | ❌ Not implemented | Kernel one-shot does not use reply |

---

## STOP FIRST Findings

| Condition | Verdict |
|-----------|---------|
| IpcCall arg layout differs from expected | ✅ PASS — standard IpcCall used |
| OP_BELL_NOTIFY value is unavailable | ✅ PASS — 0xC0 in sex-pdx |
| Adding one-shot requires broad kernel changes | ✅ PASS — 18 lines in init.rs |
| sexbell cannot identify op/args safely | ✅ PASS — match on type_id, parse fields |
| Enum validation needs heap/string/model expansion | ✅ PASS — static functions, no heap |
| Notify handler needs queue/storage/render/SilkBar | ✅ PASS — none implemented |
| Build fails due to ABI mismatch | ✅ PASS — no sex-pdx edits, ABI hash unchanged |

---

## Proof Markers Reference

```rust
// sexbell/src/main.rs — OP_BELL_NOTIFY dispatch
//   match msg.type_id {
//       OP_BELL_NOTIFY => { ... }  // lines 45-116
//       _ => { ... }               // unknown reject
//   }

// Kernel scaffolding (init.rs):
//   [kernel.sexbell.notify.test] enqueued OP_BELL_NOTIFY to sexbell
```

---

## Next Phase

1. **BELL_NOTIFY_PROOF_V1** — Boot QEMU, capture log, verify markers
2. **BELL_NOTIFY_CLEANUP_V1** — Remove kernel scaffolding, keep sexbell dispatch

---

*End of BELL_NOTIFY_IMPLEMENT_V1.md*
