# BELL_CAPABILITY_POLICY_V1

**Status:** Docs-only capability policy spec. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_EVENT_MODEL_DESIGN_GATE_V1.md`

---

## 1. Purpose

Apps do not choose final urgency. Apps request an event class/lane; Bell derives the allowed lane from sender capability + user policy + shell context. This document defines the capability classes, default-deny matrix, lane derivation algorithm, downgrade/reject rules, and proof logging policy for the Bell attention/event system before any protocol or server implementation work begins.

### Core principle

```
sender capability + category + privacy + context → Bell policy → final lane + final urgency
```

The sender's `urgency_hint` is a **request**, not a directive. Bell policy is the sole authority for final lane assignment.

---

## 2. Capability Classes

Each capability class grants the holder permission to request a specific lane/feature. Capabilities are static — assigned at spawn time via the PDX capability table (same mechanism as `SLOT_SEXSTORE=10` for sexstore). No runtime cap registration or app-store model.

```rust
/// Bell capability tokens. Each maps to a permission bit.
/// Assigned at spawn time via PDX slot table — no runtime registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellCap {
    /// May send PASSIVE lane events (lowest priority, no user action needed).
    NotifyPassive = 0,
    /// May send NORMAL lane events (standard attention).
    NotifyNormal = 1,
    /// May send URGENT lane events (requires attention, amber accent).
    NotifyUrgent = 2,
    /// May send PERSISTENT lane events (pinned until dismissed).
    NotifyPersistent = 3,
    /// May send SYSTEM lane events (system-originated, system color).
    NotifySystem = 4,
    /// May send SECURITY lane events (security color, session-scoped).
    NotifySecurity = 5,
    /// May include sound hint with event (Bell does not play sound — Harp/Theremin gate).
    SoundAllowed = 6,
    /// May include action callbacks (dismiss, open, etc. — future).
    ActionCallback = 7,
    /// May include object references (Linen object IDs).
    ObjectReference = 8,
    /// Event may be visible on lockscreen (requires privacy_level validation).
    LockscreenVisible = 9,
    /// May tag event with project/workspace context.
    ProjectContext = 10,
    /// May send developer/tooling events (debug, build, diagnostic).
    DeveloperEvent = 11,
}
```

### Capability slot budget

12 capability classes, fitting in a `u16` bitmask (12 bits used, 4 spare).

---

## 3. Default-Deny Matrix

For each sender class, the matrix defines the **max allowed lane** and any **denied lanes**. No sender may exceed its max allowed lane regardless of `urgency_hint`. Senders not listed in the matrix are **implicitly denied all lanes** (default-deny).

| Sender Class | Example | Max Lane | Denied Lanes | Notes |
|-------------|---------|----------|-------------|-------|
| **Unknown/untrusted app** | Third-party, no cap entry | PASSIVE | NORMAL, URGENT, PERSISTENT, SYSTEM, SECURITY | Lowest trust. All events downgraded to passive. No sound, no actions, no lockscreen. |
| **Trusted app** | Known first-party app with caps | NORMAL | PERSISTENT, SYSTEM, SECURITY | Can send URGENT but it's downgraded to NORMAL unless trust_label >= 1. |
| **System service** | sexstore, sexdisplay, sexnet | SYSTEM | SECURITY | Can send SYSTEM events. Can send PERSISTENT with trust_label >= 1. |
| **Security service** | sexauth, Collar (future) | SECURITY | — | Full lane access for security events. SYSTEM and SECURITY lanes available. |
| **Developer/tooling** | sexbuild, sex-debug | URGENT | SYSTEM, SECURITY | Developer events get DeveloperEvent cap. SYSTEM/SECURITY reserved. |
| **Shell-owned** | silk-shell, SilkBar, Bell itself | PERSISTENT | SYSTEM, SECURITY | Shell may send persistent native events. SYSTEM/SECURITY reserved for system/security services. |

### Capability grant rules

- A sender class is determined by which `BellCap` entries exist in its PDX cap table at spawn time.
- No caps → **unknown/untrusted app** → max lane PASSIVE.
- `BellNotifySystem` cap → class is **system service**.
- `BellNotifySecurity` cap → class is **security service**.
- `BellNotifyPersistent` + `BellNotifyUrgent` + `BellNotifyNormal` → class is **shell-owned**.
- `BellNotifyUrgent` + `BellDeveloperEvent` → class is **developer/tooling**.
- `BellNotifyNormal` + `BellNotifyPassive` → class is **trusted app**.
- `BellNotifyPassive` only → class is **unknown/untrusted app**.

---

## 4. Lane Derivation Algorithm

### Inputs

| Input | Source | Description |
|-------|--------|-------------|
| `requested_lane` | Derived from sender's `urgency_hint` + `category` | The lane the sender requested |
| `sender_class` | Bell cap table lookup | Determined by which caps the sender holds |
| `sender_caps` | Bell cap table | Bitmask of granted `BellCap` values |
| `category` | Event payload | `BellCategory` enum |
| `privacy_level` | Event payload | `BellPrivacyLevel` enum |
| `focus_context` | silk-shell | Is session locked? Is workspace active? |
| `user_policy` | sexstore K/V (future) | User preferences (per-app overrides) |
| `spam_score` | Bell-local (future) | Rate-limit / spam detection placeholder |

### Algorithm

```
fn derive_lane(inputs) -> (final_lane, final_urgency, sound_allowed, action_allowed):

    // Step 1: Resolve sender class from cap table
    sender_class = classify(sender_caps)

    // Step 2: Apply max lane cap
    let max_lane = max_allowed_lane(sender_class)
    if requested_lane > max_lane:
        requested_lane = max_lane  // downgrade

    // Step 3: Apply category overrides
    if category == SYSTEM:
        if !has_cap(sender_caps, NotifySystem):
            return REJECT(system_cap_required)
        requested_lane = clamp(requested_lane, max_lane)  // already ≤ max_lane
    if category == SECURITY:
        if !has_cap(sender_caps, NotifySecurity):
            return REJECT(security_cap_required)
        requested_lane = clamp(requested_lane, max_lane)

    // Step 4: Apply trust_label downgrade
    if requested_lane == PERSISTENT && sender_trust_label < 2:
        requested_lane = URGENT  // downgrade persistent to urgent
    if requested_lane == URGENT && sender_trust_label < 1:
        requested_lane = NORMAL  // downgrade urgent to normal
    if requested_lane == URGENT && sender_class == UNTRUSTED:
        requested_lane = PASSIVE // untrusted urgent -> passive

    // Step 5: Check privacy_level against caps
    if privacy_level > PUBLIC:
        if !has_cap(sender_caps, LockscreenVisible):
            privacy_level = PUBLIC  // downgrade privacy

    // Step 6: Check sound
    sound_allowed = has_cap(sender_caps, SoundAllowed) && final_lane >= NORMAL

    // Step 7: Check action callbacks
    action_allowed = has_cap(sender_caps, ActionCallback)

    // Step 8: Check object references
    if action_cap_count > 0 && !has_cap(sender_caps, ActionCallback):
        action_cap_count = 0  // strip action caps
    if object_ref_count > 0 && !has_cap(sender_caps, ObjectReference):
        object_ref_count = 0  // strip object refs

    // Step 9: Apply user policy overrides (future)
    // if user_policy.deny_lanes.contains(requested_lane):
    //     requested_lane = user_policy.fallback_lane

    // Step 10: Apply spam/rate-limit (future placeholder)
    // if spam_score > THRESHOLD:
    //     requested_lane = PASSIVE

    final_lane = requested_lane
    final_urgency = lane_to_urgency(final_lane)
    return (final_lane, final_urgency, sound_allowed, action_allowed)
```

### Lane-to-urgency mapping

| Lane | `final_urgency` | Description |
|------|----------------|-------------|
| PASSIVE | 0 | Dim indicator, auto-expire |
| NORMAL | 1 | Standard attention |
| URGENT | 2 | Amber accent, requires attention |
| PERSISTENT | 3 | Pinned until dismissed |
| SYSTEM | 2 (system override) | System color, boot-cycle persistence |
| SECURITY | 2 (security override) | Security color, session-scoped |

---

## 5. Downgrade Rules (Summary)

| Condition | Action |
|-----------|--------|
| Missing `NotifyUrgent` cap, requested URGENT | Downgrade to NORMAL (or PASSIVE if untrusted) |
| Missing `NotifyPersistent` cap, requested PERSISTENT | Downgrade to URGENT |
| Missing `NotifySystem` cap, requested SYSTEM | **Reject** (not downgrade — SYSTEM is reserved) |
| Missing `NotifySecurity` cap, requested SECURITY | **Reject** (not downgrade — SECURITY is reserved) |
| Sender class max lane < requested lane | Downgrade to max allowed lane |
| `trust_label < 2`, requested PERSISTENT | Downgrade to URGENT |
| `trust_label < 1`, requested URGENT | Downgrade to NORMAL |
| Sender class UNTRUSTED, requested URGENT | Downgrade to PASSIVE |
| Missing `LockscreenVisible` cap, privacy > PUBLIC | Downgrade privacy to PUBLIC |
| Missing `SoundAllowed` cap, sound hint present | Silence sound hint (no error) |
| Missing `ActionCallback` cap, actions present | Strip action caps from event |
| Missing `ObjectReference` cap, object refs present | Strip object refs from event |

---

## 6. Reject Rules

Bell **rejects** (returns error, does not create event) under these conditions:

| # | Condition | Reject Reason | Marker |
|---|-----------|--------------|--------|
| 1 | Invalid `category` enum value | `invalid_category` | `[bell.ingest.reject]` |
| 2 | Invalid `privacy_level` enum value | `invalid_privacy` | `[bell.ingest.reject]` |
| 3 | `action_count` > 4 | `malformed_action_count` | `[bell.ingest.reject]` |
| 4 | `object_refs` would expose SecretContent in proof log | `private_field_logged` | `[bell.ingest.reject]` |
| 5 | `object_refs` present but missing `ObjectReference` cap | `missing_object_ref_cap` | `[bell.ingest.reject]` |
| 6 | `action_count > 0` but missing `ActionCallback` cap | `missing_action_cap` | `[bell.ingest.reject]` |
| 7 | `privacy_level > TitleOnly` but missing `LockscreenVisible` cap | `missing_lockscreen_cap` | `[bell.ingest.reject]` |
| 8 | Requested SYSTEM lane but missing `NotifySystem` cap | `missing_system_cap` | `[bell.ingest.reject]` |
| 9 | Requested SECURITY lane but missing `NotifySecurity` cap | `missing_security_cap` | `[bell.ingest.reject]` |
| 10 | `sender_pd` not found in cap table | `unknown_sender` | `[bell.ingest.reject]` |
| 11 | Cap table lookup failed (cap cannot prove authority) | `cap_table_error` | `[bell.ingest.reject]` |
| 12 | Rate-limit hard-deny (future) | `rate_limit_exceeded` | `[bell.ingest.reject]` |

### Reject always returns error

Rejected events are **not** stored in the ring buffer. The sender receives an error reply. No partial event state persists in Bell.

---

## 7. Proof Marker Policy

### Allowed in markers (StructuralMeta or SenderMeta)

| Field | Class | Example |
|-------|-------|---------|
| `sender_class` | StructuralMeta | `[bell.ingest] sender_class=trusted_app` |
| `requested_lane` | StructuralMeta | `requested_lane=urgent` |
| `final_lane` | StructuralMeta | `final_lane=normal` |
| `reject_reason` | StructuralMeta | `reason=missing_urgent_cap` |
| `category` | StructuralMeta | `category=project` |
| `redaction_class` | StructuralMeta | `redaction=StructuralMeta` |
| `event_id` | StructuralMeta | `event_id=42` |
| `sender_pd` | StructuralMeta | `sender_pd=7` |
| `downgrade_reason` | StructuralMeta | `downgrade=untrusted_urgent_to_passive` |

### Forbidden in markers (SecretContent)

| Field | Why |
|-------|-----|
| Event title/body | Private content |
| Sender display name | Could identify PII |
| Private object names | Could identify documents/projects |
| File paths | System structure leak |
| Raw action payloads | Capability token leak |
| Secret tokens | Authentication material |

### Marker examples

```
[bell.ingest] event_id=42 sender_pd=7 category=project requested=urgent final=normal downgrade=untrusted_urgent
[bell.ingest.reject] sender_pd=7 reason=missing_system_cap
[bell.dismiss] event_id=42 final_lane=normal
[bell.expire] event_id=42 reason=timeout
```

```
[bell.ingest] title="Build complete"   ← FORBIDDEN (SecretContent)
[bell.ingest] sender="My App v3.2"     ← FORBIDDEN (SecretContent)
```

---

## 8. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Adding real capability IDs to sex-pdx or kernel cap table.
2. Editing sex-pdx for any `OP_BELL_*` opcode — protocol spec must be reviewed first.
3. Implementing `servers/sexbell/` — server stub design must be reviewed first.
4. Adding persistent policy storage — sexstore integration requires separate design gate.
5. Adding action callback routing — action dispatch requires separate design gate.
6. Adding sound routing — Harp/Theremin gate must precede.
7. Adding lockscreen display — lockscreen policy gate must precede.
8. Allowing app-defined `final_urgency` — Bell policy is the sole authority.
9. Adding runtime cap registration — caps are static, assigned at spawn time.
10. Adding user preference storage in Bell — preferences belong in sexstore K/V, not Bell.

---

## 9. Future Dependency Notes

| Dependency | Phase | Notes |
|-----------|-------|-------|
| Capability table mechanism | PDX (existing) | Uses same PDX slot/cap model as sexstore's `SLOT_SEXSTORE=10`. No new kernel mechanism needed. |
| Collar/sexauth | Future | May own user grant UI for notification caps. Not required for V1 static cap model. |
| sexstore persistence | Future E-series gate | User policy overrides (per-app lane preferences) belong in sexstore K/V, not in Bell. |
| SilkBar presence | After server stub | Uses only `final_lane` and count summary — no cap logic in SilkBar. |
| Bell inbox | After server stub | Uses row canon and redacted fields only — no cap logic in inbox renderer. |

---

## References

- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — parent design gate (event model, lanes, privacy)
- `E3_STORAGE_CAPABILITY_POLICY_SPEC_V1.md` — similar static cap table pattern (sexstore)
- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — canon for future inbox
- `servers/sexstore/src/main.rs` — reference for static cap table enforcement pattern

---

*End of BELL_CAPABILITY_POLICY_V1.md*
