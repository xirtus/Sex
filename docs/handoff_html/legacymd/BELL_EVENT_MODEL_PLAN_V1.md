# BELL_EVENT_MODEL_PLAN_V1

**Status:** Design — No Implementation
**Date:** 2026-05-06
**Purpose:** Design Bell as the SexOS attention/event firewall, building on
existing opcodes (0xC0–0xC7) and the V1 sexbell queue stub. Docs only.

---

## 1. Bell Model Summary

Bell is an **attention firewall**, not a notification delivery system. Its
role is to **receive, classify, filter, queue, and expose** events from
any domain — but never to deliver them directly to a user. The shell polls
Bell; Bell does not push.

### Design principles

- **Default-deny.** Unknown senders get the lowest lane and urgency.
  Capability grants (future Collar) raise limits.
- **Sender identity is kernel-authoritative.** `caller_pd` from
  `pdx_listen_raw` is trusted. Bell never accepts self-declared identities.
- **No delivery push.** Bell queues events. The shell (via SilkBar) polls
  for lane summaries. No thread wakeup, no interrupt.
- **No storage.** Events live in a fixed-size RAM queue. Overflow drops
  oldest or lowest-priority events.
- **Privacy levels are enforced by Bell.** FullHidden events are never
  listed to the shell — only their lane count increments.
- **Sound is deferred.** No audio in V1. Urgency hints exist for future
  sound policy.

### V1 foundation (current implementation status)

| Feature | Status |
|---------|--------|
| `OP_BELL_NOTIFY` (0xC0) | ✅ Receives events, validates fields, pushes to RAM queue. Spam budget checked. Mute list checked. |
| `OP_BELL_LIST` (0xC3) | ✅ Returns matching events by lane filter, allowlist-gated, skips dismissed entries, privacy-filtered. |
| `OP_BELL_CLOSE` (0xC1) | ✅ Dismiss handler — marks event_id as dismissed, skipped in LIST. |
| `OP_BELL_ACTION` (0xC2) | ✅ Action dispatch — looks up event_id+action_id, emits dispatch marker (no execution). |
| `OP_BELL_CLEAR` (0xC4) | ✅ Clear handler — resets queue (all lanes) or marks matching lane entries as dismissed. |
| `OP_BELL_SUBSCRIBE` (0xC5) | ❌ Stub — no subscription mechanism yet. |
| `OP_BELL_SET_POLICY` (0xC6) | ❌ Stub — no per-app policy override yet. |
| `OP_BELL_MUTE_SENDER` (0xC7) | ✅ Mute/unmute sender PDs; muted senders rejected in NOTIFY. |
| RAM queue (16 entries) | ✅ Fixed-size, ring buffer, push/drop-lowest-priority on overflow. |
| Lane derivation | ✅ First-proof: all unknown senders → PASSIVE (0). |
| Enum validation | ✅ category, privacy, redaction, urgency validated. action_count ≤ 1, object_refs ≤ 1. |
| Read-cap allowlist | ✅ Only domain 3 (silk-shell) may call LIST. |
| Spam budget | ✅ Per-PD rate limit: 8 events per 62-tick window, 16 tracked slots. |
| Queue overflow policy | ✅ Drops lowest-priority active entry when queue full. |
| Privacy enforcement | ✅ FullHidden entries filtered from LIST output, redact marker emitted. |
| Action callbacks | ✅ action_count=1 accepted, OP_BELL_ACTION dispatches marker. |
| Object references | ✅ object_ref_count=1 accepted, stored in entry (no resolution). |

---

## 2. Event Fields (BellQueueEntry — already defined)

| Field | Type | Source | Description |
|-------|------|--------|-------------|
| `event_id` | u64 | Bell-assigned monotonic | 0 = invalid sentinel. Wraps at MAX to 1. |
| `caller_pd` | u32 | Kernel (`msg.caller_pd`) | Sender protection domain. Authoritative. |
| `sender_surface_id` | u64 | Sender arg (future) | Surface that triggered the event, if any. |
| `category` | u8 | Sender arg | 0=Info, 1=Success, 2=Warning, 3=Error, 4=Action, 5=Security |
| `urgency_hint` | u8 | Sender arg | 0=Low, 1=Medium, 2=High, 3=Critical. Bell may downgrade. |
| `final_lane` | u8 | Bell-derived | After policy: 0=PASSIVE .. 5=SECURITY |
| `final_urgency` | u8 | Bell-derived | After policy: may differ from hint |
| `privacy_level` | u8 | Sender arg | 0=Public, 1=Internal, 2=Confidential, 3=FullHidden |
| `redaction_class` | u8 | Sender arg | 0=StructuralMeta, 1=SummaryOnly, 2=DetailContent, 3=SecretContent |
| `action_count` | u8 | Sender arg (future) | Number of action callbacks attached. V1: 0. |
| `object_ref_count` | u8 | Sender arg (future) | Number of object references attached. V1: 0. |
| `expires_tick` | u64 | Sender arg (future) | Tick at which event auto-expires. 0 = no expiry. |
| `trust_label` | u8 | Collar-derived (future) | Trust level from Collar attestation. 0=Unknown. |
| `_pad` | [u8; 6] | — | Alignment padding |

### Fields not yet in BellQueueEntry (future)

| Field | Status | Reason |
|-------|--------|--------|
| `sender_surface_id` | Not queued | No sender currently passes surface context; arg0 is full of category/urgency/privacy/redaction |
| `expires_tick` | Not queued | No tick source passed in V1; would need sender clock or kernel tick |
| `trust_label` | Not queued | Collar does not exist yet |

---

## 3. Ownership Boundaries

| Domain | Owns | Does Not Own |
|--------|------|--------------|
| **Bell** (sexbell) | Event queue, lane derivation, privacy enforcement, read-cap allowlist, spam budget | Surface placement, focus context, user-facing rendering, sound output |
| **SilkBar** | Compact lane-summary indicators (colored dots/counters on SilkBar chips) | Event content, per-event detail view, notification policy |
| **silk-shell** (authority) | Focus context, surface lifecycle, Atlas overlay, command routing | Event queue, lane policy, event storage |
| **Linen** (future) | Object references embedded in events, event→object link resolution | Event queuing, lane derivation |
| **Collar** (future) | Capability grants, trust labels, operation approval | Event content, queue management |
| **sexdisplay** | Framebuffer, surface clipping | Event policy, queue, rendering decisions |

### Invariant

**Bell never renders anything.** It emits markers and returns queue data
via PDX. The shell reads queue data and decides what to render. SilkBar
chips reflect lane counts. The Bell panel surface (0x95) shows event
details — drawn by the shell, not by Bell.

---

## 4. Lanes

### Lane definitions

| ID | Name | Purpose | Default max urgency | V1 urgency cap |
|----|------|---------|---------------------|----------------|
| 0 | PASSIVE | Informational, no user action needed | Low (0) | Low (0) |
| 1 | SOON | Requires attention soon | Medium (1) | Medium (1) |
| 2 | LATER | Tasks deferred to later | Low (0) | Low (0) |
| 3 | SYSTEM | OS/kernel-originated events | High (2) | High (2) |
| 4 | PROJECT (future) | Per-project events from Linen | Medium (1) | — |
| 5 | SECURITY (future) | Security/auth events from Collar | Critical (3) | — |

### Lane derivation (first-proof — already implemented)

Currently every unknown sender → PASSIVE (lane 0), with urgency hint
downgraded to 0. This is the safest possible default: no sender is
trusted to request a higher lane.

### Future lane derivation (with Collar caps)

Each sender PD would have a `max_lane` and `max_urgency` cap granted by
Collar:

```
if urgency_hint > sender.max_urgency:  clamp to max_urgency
if requested_lane > sender.max_lane:    clamp to max_lane
```

---

## 5. Capability Classes (Future — Collar)

| Capability | Effect | Default (no cap) |
|------------|--------|-------------------|
| `passive_notify` | May send events to PASSIVE lane | Allowed (no cap needed) |
| `urgent_notify` | May set urgency_hint up to 3 | Downgraded to 0 |
| `persistent_notify` | Events survive queue rotation | Dropped on overflow |
| `action_callback` | May attach action callbacks | V1: always rejected (action_count != 0) |
| `object_reference` | May attach object refs | V1: always rejected (object_refs != 0) |
| `sound_hint` | Urgency hint may trigger sound (future) | No sound |
| `high_privacy` | May set privacy_level up to 3 | Clamped to 0 (Public) |

### V1 behavior

All capabilities are unimplemented. The sexbell server already rejects
`action_count != 0` and `object_refs != 0`. Urgency hint > 0 is
downgraded. Privacy_level is stored but not enforced beyond validation
(list always returns the entry's stored privacy_level for the shell to
consume).

---

## 6. Rejection Policy

### Currently implemented (sexbell)

| Reason | Condition | Marker |
|--------|-----------|--------|
| `invalid_category` | `category > 5` | `[bell.notify.reject] reason=invalid_category` |
| `invalid_privacy` | `privacy_level > 3` | `[bell.notify.reject] reason=invalid_privacy` |
| `invalid_redaction` | `redaction_class > 3` | `[bell.notify.reject] reason=invalid_redaction` |
| `invalid_urgency` | `urgency_hint > 3` | `[bell.notify.reject] reason=invalid_urgency` |
| `action_count_invalid` | `action_count > 1` | `[bell.notify.reject] reason=action_count_invalid` |
| `action_id_zero` | `action_count == 1 && action_id == 0` | `[bell.notify.reject] reason=action_id_zero` |
| `object_refs_invalid` | `object_refs > 1` | `[bell.notify.reject] reason=object_refs_invalid` |
| `queue_full` | Queue at capacity | `[bell.queue.reject.full]` |
| `no_read_cap` | Caller not in LIST allowlist | `[bell.readcap.deny]` |
| `invalid_lane` | Lane filter > 5 and not 0xFF | `[bell.list.reject] reason=invalid_lane` |
| `invalid_count` | max_results 0 or > 4 | `[bell.list.reject] reason=invalid_count` |

### Additional rejection cases (implemented in Phases A-D)

| Reason | Condition | Marker |
|--------|-----------|--------|
| `muted` | `caller_pd` is in mute list | `[bell.notify.reject] reason=muted` |
| `spam_budget_exceeded` | Sender exceeded 8 events/62 ticks | `[bell.notify.reject] reason=spam_budget_exceeded` |
| `action_count_invalid` | `action_count > 1` | `[bell.notify.reject] reason=action_count_invalid` |
| `action_id_zero` | `action_count == 1 && action_id == 0` | `[bell.notify.reject] reason=action_id_zero` |
| `object_refs_invalid` | `object_refs > 1` | `[bell.notify.reject] reason=object_refs_invalid` |
| `action_not_found` | OP_BELL_ACTION: event_id+action_id not found | `[bell.action.reject] reason=not_found` |
| `close_not_found` | OP_BELL_CLOSE: event_id not found | `[bell.close.reject] reason=not_found` |
| `mute_list_full` | Mute list at capacity (16) | `[bell.mute.reject] reason=mute_list_full` |

### Planned rejection cases (future)

| Reason | Condition | Marker |
|--------|-----------|--------|
| `unknown_sender` | `caller_pd` not found in Collar registry | `[bell.notify.reject] reason=unknown_sender` |
| `missing_cap` | Sender lacks `passive_notify` cap | `[bell.notify.reject] reason=missing_cap` |
| `privacy_mismatch` | Sender's `max_privacy` < requested privacy | `[bell.notify.reject] reason=privacy_mismatch` |
| `invalid_action_mask` | Action callback ID out of range | `[bell.notify.reject] reason=invalid_action` |
| `stale_object_ref` | Object ref points to dead/destroyed Linen object | `[bell.notify.reject] reason=stale_object_ref` |
| `expired` | `expires_tick` is in the past | `[bell.notify.reject] reason=expired` |

---

## 7. V1 Implementation Boundary

### What V1 already has
- Bell server (`servers/sexbell/src/main.rs`) with RAM queue
- `OP_BELL_NOTIFY` with field parsing, validation, mute check, spam budget check
- `OP_BELL_LIST` with lane filtering, read-cap allowlist, dismissed-skip, privacy gate
- `OP_BELL_CLOSE` — dismiss event by ID
- `OP_BELL_ACTION` — dispatch marker for event_id+action_id lookup
- `OP_BELL_CLEAR` — clear all lanes or specific lane
- `OP_BELL_MUTE_SENDER` — mute/unmute sender PDs
- First-proof lane derivation (all → PASSIVE)
- Spam budget: per-PD rate limit (8 events per 62 ticks)
- Queue overflow: drops lowest-priority entry when full
- Privacy enforcement: FullHidden entries redacted from LIST
- Action callback and object reference storage (marker-only, no execution/resolution)
- Silk-bar surface (0x95) and bell placeholder surface (204)
- Shell-local Bell event ring buffer (`BELL_EVENTS` in silk-shell)
- `bell_emit_object_link_event` for Linen→Quil link tracking

### What V1 explicitly does NOT do
- **No new PD spawn.** Bell already exists as `sexbell` (domain 10).
- **No sound.** Urgency hint exists for future audio policy but no audio
  server call or PCM generation.
- **No storage.** Queue is fixed RAM only. No persistence across boots.
- **No lockscreen.** Privacy levels are stored but no lockscreen/ auth
  gate exists to reveal FullHidden events.
- **No ABI/opcode changes.** The 8 opcodes (0xC0–0xC7) are assigned and
  all 8 are handled (6 with real logic, 2 stubs return `[bell.unknown.reject]`).
- **No Collar integration.** Capability classes are designed but not
  enforced. Derivation is hardcoded to PASSIVE.
- **No push/subscribe.** `OP_BELL_SUBSCRIBE` is defined but unhandled.
  The shell must poll via `OP_BELL_LIST`.
- **No action execution.** `OP_BELL_ACTION` emits a dispatch marker but
  does not execute the callback. Shell handler does not exist yet.
- **No object reference resolution.** Object refs are stored but not
  resolved (no Linen lookup).
- **No per-app policy overrides.** `OP_BELL_SET_POLICY` is unhandled.

---

## 8. Implementation Status

### Phase A: Complete V1 opcodes ✅ IMPLEMENTED

```
1. servers/sexbell/src/main.rs:
   ✅ OP_BELL_CLOSE: marks event_id as dismissed (dismissed=1)
   ✅ OP_BELL_CLEAR: resets queue (all) or marks matching lane entries as dismissed
   ✅ OP_BELL_MUTE_SENDER: add/remove from static mute list; reject muted NOTIFY
   ✅ Budgeted markers: [bell.close.ok], [bell.clear.ok], [bell.mute.add], [bell.notify.reject] reason=muted
```

### Phase B: Spam budget and queue overflow policy ✅ IMPLEMENTED

```
2. servers/sexbell/src/main.rs:
   ✅ Per-PD rate limit: 8 events per 62-tick window (16 tracked slots)
   ✅ Reject senders exceeding rate → [bell.notify.reject] reason=spam_budget_exceeded
   ✅ On queue full, drop lowest-priority entry (not newest, not oldest)
   ✅ Marker: [bell.queue.drop] reason=lowest_priority lane=N dropped_lane=M
```

### Phase C: Action callbacks and object references ✅ IMPLEMENTED

```
3. No ABI changes needed. OP_BELL_ACTION (0xC2) already defined.
4. servers/sexbell/src/main.rs:
   ✅ Accept action_count=1; store action_id in entry
   ✅ OP_BELL_ACTION: look up event_id+action_id, emit [bell.action.dispatch] marker
   ✅ No actual execution (marker only)
5. Accept object_refs > 0: stored in entry (no resolution, marker only)
```

### Phase D: Privacy enforcement ✅ IMPLEMENTED

```
6. servers/sexbell/src/main.rs:
   ✅ OP_BELL_LIST: filter entries by caller's max_privacy (3 for silk-shell, 0 for others)
   ✅ FullHidden entries are skipped (never returned as items)
   ✅ Marker: [bell.list.redact] reason=full_hidden count=N
```

### Phase E: Collar capability integration

```
7. Collar PD (future):                        -- STOP FIRST --
   - Grant per-sender caps: max_lane, max_urgency, max_privacy, etc.
   - Bell queries Collar on each NOTIFY, or Collar pushes cap table to Bell
   - Lane derivation uses Collar caps instead of hardcoded PASSIVE
```

---

## 9. Summary

| Question | Answer |
|----------|--------|
| What is Bell? | Attention firewall, not notification delivery. Receives, classifies, filters, queues. |
| Existing foundation? | sexbell server with RAM queue, NOTIFY+LIST opcodes, field validation, first-proof lane derivation. |
| V1 boundaries? | No sound, no storage, no lockscreen, no push, no action callbacks, no object refs, no Collar. |
| Implementation status? | Phases A–D implemented (CLOSE, CLEAR, MUTE, spam budget, overflow drop, action dispatch, privacy). Phase E (Collar) blocked. |
| ABI impact? | **No ABI changes needed.** Existing opcodes (0xC0–0xC7) are sufficient. No new opcodes or kernel changes. |
