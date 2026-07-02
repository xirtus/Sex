# BELL_CAPABILITY_NOTIFICATION_PLAN_V1

**Status:** Docs-only consolidation plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-07
**Depends on:** `BELL_V1_FINAL_STATUS.md` (Bell V1 A–D complete, Phase E blocked)
**References:** `BELL_EVENT_MODEL_DESIGN_GATE_V1.md`, `BELL_CAPABILITY_POLICY_V1.md`, `BELL_PDX_PROTOCOL_SPEC_V1.md`

---

## 1. Purpose

This document consolidates the Bell notification/event service architecture as designed and implemented through Bell V1. It defines ownership boundaries, capability classes, opcode semantics, validation flows, negative cases, proof marker policy, and a next-implementation prompt — without implementing new code or modifying kernel/ABI.

**Bell is a capability-scoped attention policy server for Silk DE — not a notification daemon clone.** It receives event requests from apps, validates them against sender capabilities and shell policy, stores them in a bounded RAM ring buffer, and exposes aggregate presence to SilkBar for rendering by sexdisplay. **V1 is policy proof, not product UI.**

### Core architectural rule

```
apps request  →  Bell validates/policies  →  SilkBar displays compact state  →  sexdisplay renders pixels
                                                                                  Linen may later persist event objects
```

No component downstream of Bell owns notification policy. sexdisplay is a pure renderer. SilkBar is a pure poller/display. silk-shell owns session/focus context but not per-event lane policy.

---

## 2. Bell Ownership Model

| Component | Owns | Does NOT Own |
|-----------|------|-------------|
| **Sending app** | Requests event with `urgency_hint`, `category`, `privacy_level` | Final lane, final urgency, policy, rendering |
| **Bell (sexbell, PD 10)** | Capability validation, lane derivation, spam budget, mute list, policy table, ring buffer, privacy enforcement, generation counter | Focus policy, session state, workspace routing, pixel rendering, persistent storage |
| **silk-shell (PD 3)** | Session lock state, focus context, workspace routing (via LIST allowlist membership) | Per-event lane assignment, ring buffer management, rendering |
| **SilkBar (PD 6)** | Polls Bell via SUBSCRIBE+LIST, maintains `BellState` compact summary, sends `SetBellPresence` update to sexdisplay | Event creation, policy, routing, detailed event data |
| **sexdisplay (PD 1)** | Renders Bell dot + count badge from `BellState` fields via `0xEC`/`0xEF` calls | BellEvent model, lane policy, routing, event storage |
| **Linen (PD 7)** | May persist/project-link events after storage maturity gate | Event creation, policy, routing (not yet implemented) |

### Ownership diagram

```
┌──────────┐     NOTIFY (0xC0)     ┌───────────┐     SUBSCRIBE+LIST      ┌──────────┐     SetBellPresence      ┌────────────┐
│   App    │ ────────────────────▶ │   Bell    │ ──────────────────────▶ │ SilkBar  │ ──────────────────────▶ │ sexdisplay │
│ (any PD) │                       │  (PD 10)  │                         │  (PD 6)  │                           │   (PD 1)   │
└──────────┘                       └───────────┘                         └──────────┘                           └────────────┘
                                        │                                      │
                                        │ LIST (0xC3)                          │
                                        ▼                                      │
                                   ┌───────────┐                              │
                                   │ silk-shell│ ◀────────────────────────────┘
                                   │  (PD 3)   │   (future: policy commands)
                                   └───────────┘
```

---

## 3. Minimal BellEvent Fields

The Bell queue entry is a fixed-size `repr(C)` struct. No heap, no raw pointers, no strings. All content fields are marker-only in V1 (no title/body text stored).

```rust
#[repr(C)]
#[derive(Copy, Clone)]
struct BellQueueEntry {
    event_id:         u64,   // Monotonic event ID assigned by Bell (0 = invalid)
    caller_pd:        u32,   // Kernel-authoritative sender PD
    category:         u8,    // BellCategory (0=Info .. 5=Error)
    requested_lane:   u8,    // Urgency hint from sender (0..3)
    final_lane:       u8,    // Final lane after policy (0=PASSIVE .. 5=SECURITY)
    final_urgency:    u8,    // Final urgency after policy (0..3)
    privacy_level:    u8,    // BellPrivacyLevel (0=Public .. 3=FullHidden)
    redaction_class:  u8,    // BellRedactionClass (0=StructuralMeta .. 3=SecretContent)
    action_count:     u8,    // Number of action callbacks (V1: 0 or 1)
    action_id:        u8,    // Action capability token (valid when action_count >= 1)
    object_ref_count: u8,    // Number of object references (V1: 0 or 1)
    object_ref:       u8,    // Object reference ID (valid when object_ref_count >= 1)
    dismissed:        u8,    // 0 = active, 1 = dismissed by CLOSE/CLEAR
    _pad:             [u8; 2],
}
// Total: 8+4+1+1+1+1+1+1+1+1+1+1+1+2 = 26 bytes per entry
```

### Field budget and rationale

| Field | Bytes | Purpose | Privacy |
|-------|-------|---------|---------|
| `event_id` | 8 | Monotonic identifier, unique per boot | StructuralMeta |
| `caller_pd` | 4 | Sender identity (kernel-authoritative) | StructuralMeta |
| `category` | 1 | Event type (Info/Project/Document/System/Security/Error) | StructuralMeta |
| `requested_lane` | 1 | What sender asked for (urgency_hint) | SenderMeta |
| `final_lane` | 1 | What Bell assigned (0=PASSIVE .. 5=SECURITY) | StructuralMeta |
| `final_urgency` | 1 | Derived urgency level (0..3) | StructuralMeta |
| `privacy_level` | 1 | Visibility tier for rendering | StructuralMeta |
| `redaction_class` | 1 | What may appear in proof markers | StructuralMeta |
| `action_count/action_id` | 2 | Callback tokens (marker-only in V1) | SecretContent |
| `object_ref_count/object_ref` | 2 | Linked object IDs (marker-only in V1) | TitleMeta |
| `dismissed` | 1 | Lifecycle flag | StructuralMeta |

### Ring buffer properties

| Property | Value |
|----------|-------|
| Capacity | 16 entries (`BELL_QUEUE_CAPACITY`) |
| Overflow policy | Drop lowest-priority active entry: **PASSIVE (0) → NORMAL (1) → URGENT (2) → PERSISTENT (3) → SYSTEM (4) → SECURITY (5)**. Tiebreaker within same lane: oldest entry first (smallest distance from head). Never drop SYSTEM/SECURITY if they exist (future). |
| Invalid sentinel | `event_id == 0` |
| Lifecycle | Events marked `dismissed = 1` by CLOSE/CLEAR; skipped by LIST |
| Generation counter | `BELL_GENERATION` bumped on every queue or mute-visible state change |

---

## 4. Capability Classes (BellLane)

Lanes classify the routing and visibility of a BellEvent. Each event is assigned exactly one lane by Bell policy. Lane 0 (PASSIVE) is the fallback for untrusted senders with no capability grants.

| Lane | Value | Visual | Dismiss | Persist | Sender Requires |
|------|-------|--------|---------|---------|-----------------|
| **PASSIVE** | 0 | SilkBar dim indicator | Auto-expire | No | No cap needed (fallback) |
| **NORMAL** | 1 | SilkBar bright indicator | Manual | Session | `NotifyNormal` cap |
| **URGENT** | 2 | SilkBar amber accent pulse | Manual | Until read | `NotifyUrgent` cap + trust_label >= 1 |
| **PERSISTENT** | 3 | SilkBar rose pinned | Manual | Until dismissed | `NotifyPersistent` cap + trust_label >= 2 |
| **SYSTEM** | 4 | SilkBar steel blue | Manual | Boot cycle | `NotifySystem` cap (reserved) |
| **SECURITY** | 5 | SilkBar red | Session unlock | Boot cycle | `NotifySecurity` cap (reserved) |

### SYSTEM/SECURITY lane gate

SYSTEM and SECURITY lanes are **reserved for trusted system PDs only**. No app can request them successfully. Non-system requests for these categories are **hard-rejected** (not downgraded):

| Request | Sender Class | Result |
|---------|-------------|--------|
| `category=SYSTEM` | No `NotifySystem` cap | **Reject** — `[bell.notify.reject] reason=missing_system_cap` |
| `category=SECURITY` | No `NotifySecurity` cap | **Reject** — `[bell.notify.reject] reason=missing_security_cap` |
| `category=SYSTEM` | System PD with cap | Allow, lane=SYSTEM |
| `category=SECURITY` | Security PD with cap | Allow, lane=SECURITY |

These are hard rejections. SYSTEM and SECURITY cannot be downgraded to lower lanes — the sender either has the required cap or the event is rejected entirely.

### Capability-to-lane mapping (current, implemented)

Bell V1 uses a first-proof placeholder policy: **every sender is unknown/untrusted**. All `urgency_hint > 0` downgrades to PASSIVE (lane 0).

```
fn derive_lane_first_proof(urgency_hint: u8) -> (u8, u8, Option<&'static str>) {
    if urgency_hint == 0 {
        (0, 0, None)  // PASSIVE lane, passive urgency
    } else {
        (0, 0, Some("no_caps_untrusted"))  // Downgrade: all non-zero → PASSIVE
    }
}
```

### Future capability class design (from `BELL_CAPABILITY_POLICY_V1.md`)

Bell capabilities are 12-bit bitmasks assigned at spawn time via PDX cap slots:

```rust
#[repr(u8)]
enum BellCap {
    NotifyPassive     = 0,   // May send PASSIVE lane events
    NotifyNormal      = 1,   // May send NORMAL lane events
    NotifyUrgent      = 2,   // May send URGENT lane events
    NotifyPersistent  = 3,   // May send PERSISTENT lane events
    NotifySystem      = 4,   // May send SYSTEM lane events (reserved)
    NotifySecurity    = 5,   // May send SECURITY lane events (reserved)
    SoundAllowed      = 6,   // May include sound hint (Harp/Theremin gate)
    ActionCallback    = 7,   // May include action callbacks
    ObjectReference   = 8,   // May include Linen object IDs
    LockscreenVisible = 9,   // May be visible on lockscreen
    ProjectContext    = 10,  // May tag with project/workspace context
    DeveloperEvent    = 11,  // May send developer/tooling events
}
```

**STOP FIRST:** Do not implement the full capability matrix until the cap table mechanism is reviewed. The current first-proof placeholder is sufficient for V1 integration testing.

---

## 5. PDX Opcodes

All opcodes are defined in `crates/sex-pdx/src/lib.rs`. Bell (sexbell, PD 10) is reached via `SLOT_BELL = 12`.

| Opcode | Constant | Direction | Description | Bell V1 Server | Cap Plan |
|--------|----------|-----------|-------------|---------------|----------|
| NOTIFY | `0xC0` | App → Bell | Request to create a BellEvent | Running (first-proof policy) | Specified |
| CLOSE | `0xC1` | Shell → Bell | Dismiss event by ID | Running | Specified |
| ACTION | `0xC2` | Shell → Bell | Execute action callback | Running (stub) | Specified |
| LIST | `0xC3` | Shell → Bell | List aggregate lane counts (summary only) | Running | Specified |
| CLEAR | `0xC4` | Shell → Bell | Clear events by lane or all | Running | Specified |
| SUBSCRIBE | `0xC5` | SilkBar → Bell | Poll generation counter for change detection | Running | Specified |
| SET_POLICY | `0xC6` | silk-shell → Bell | Set per-PD policy override (privacy, lane, mute) | Running | Specified |
| MUTE_SENDER | `0xC7` | Shell → Bell | Add/remove PD from runtime mute list | Running | Specified |

### Opcode collision gate

Before any Phase E code begins, audit sex-pdx for opcode collisions at `0xC0–0xC7`:

```
Known opcode ranges:
  OP_SEXFILES_*  = 0x80–0x8F
  OP_SEXSTORE_*  = 0x90–0x9F
  OP_QUIL_*      = 0xA0–0xAF
  OP_BELL_*      = 0xC0–0xC7  ← Bell range
  OP_SILK_*      = 0xD0–0xDF
  OP_SEXDISPLAY  = 0xEC, 0xEF

No collision at 0xC0–0xC7. Gate passes.
```

**STOP FIRST** before Phase E1: re-run opcode collision audit against current sex-pdx master.

### 5.1 NOTIFY (0xC0) — Wire Format

Bell parses numeric fields from PDX `IpcCall` args. No fixed struct deserialization — fields are bit-packed into `arg0`, `arg1`, `arg2`.

```
arg0 bits [7:0]   = category        (0=Info .. 5=Error)
arg0 bits [15:8]  = urgency_hint    (0..3, sender's requested urgency)
arg0 bits [23:16] = privacy_level   (0=Public .. 3=FullHidden)
arg0 bits [31:24] = redaction_class (0=StructuralMeta .. 3=SecretContent)

arg1 bits [7:0]   = action_count    (0 or 1 in V1)
arg1 bits [15:8]  = action_id       (opaque token)

arg2 bits [7:0]   = object_ref_count (0 or 1 in V1)
arg2 bits [15:8]  = object_ref       (opaque ID)
```

`caller_pd` comes from `msg.caller_pd` (kernel-authoritative, never from request payload).

### 5.2 CLOSE (0xC1)

```
arg0 = event_id (u64) — event to dismiss
```

Marks matching entry's `dismissed = 1` in the queue. Returns `[bell.close.ok]` or `[bell.close.reject]`.

### 5.3 ACTION (0xC2)

```
arg0      = event_id (u64)
arg1[7:0] = action_id (u8) — must match entry's action_id
```

Searches queue for matching event_id with matching action_id. Marker-only dispatch in V1 (no actual callback routing). Returns `[bell.action.dispatch]` or `[bell.action.reject]`.

### 5.4 LIST (0xC3)

```
arg0[7:0]   = lane_filter (0xFF = all lanes, 0..5 = specific lane)
arg0[15:8]  = max_results (1..=4, validated; out of range rejected)
```

**Reply format:** Packed u64 reply via `pdx_reply(caller_pd, packed)`:
```
[7:0]   = total_visible       (sum of all lane counts visible to caller)
[15:8]  = lane0 count (PASSIVE)
[23:16] = lane1 count (NORMAL)
[31:24] = lane2 count (URGENT)
[39:32] = lane3 count (PERSISTENT)
[47:40] = lane4 count (SYSTEM)
[55:48] = lane5 count (SECURITY)
[63:56] = redacted_count      (FullHidden entries excluded from visible counts)
```

**Two-gate access model:**
1. **Kernel gate:** Caller must hold `SLOT_BELL` domain capability (PD 3, PD 6, PD 10)
2. **Server gate:** Caller must be in `BELL_LIST_ALLOWLIST` (`[3, 6]`)
3. **Privacy gate:** Caller's `max_privacy_for_caller()` determines what privacy levels are visible (PD 3 sees FullHidden; all others see Public only)
4. Default-deny: any PD failing either gate receives `u64::MAX` reply

### 5.5 CLEAR (0xC4)

```
arg0[7:0] = lane_filter (0xFF = all lanes, 0..5 = specific lane)
```

- `0xFF`: Reset entire queue (head=0, tail=0, count=0)
- `0..5`: Mark all matching-lane active entries as `dismissed = 1`
- Invalid lane (>5 and ne 0xFF): `[bell.clear.reject]`

### 5.6 SUBSCRIBE (0xC5)

```
No args. Returns current BELL_GENERATION counter via pdx_reply.
```

Caller must pass `BELL_LIST_ALLOWLIST` check. SilkBar uses this for lightweight change detection:
- Polls SUBSCRIBE every ~2s
- If `gen != bell_gen_cached`, calls LIST for aggregate counts
- If gen unchanged, skips LIST (no queue scan needed)

### 5.7 SET_POLICY (0xC6)

```
arg0 = target_pd (u32) — PD whose policy is being set
arg1 = packed policy:
       bits [2:0]   = active_flags (bit0=privacy, bit1=lane, bit2=mute)
       bits [9:8]   = privacy_override (0..3)
       bits [18:16] = lane_override (0..5)
       bit  [24]    = force_mute (0=unmuted, 1=muted)
```

**Author gate:** Only PDs in `BELL_POLICY_AUTHOR_ALLOWLIST` (`[3]`) may call.
**Privacy invariant:** Policy can only INCREASE privacy restriction, never reduce it.
**Table capacity:** 8 entries (`POLICY_TABLE_CAPACITY`). Rejects with `table_full` if exceeded.
**Clear policy:** Set `active_flags = 0` to remove entry entirely.

### 5.8 MUTE_SENDER (0xC7)

```
arg0[31:0]  = mute_pd (u32) — PD to mute/unmute
arg0[39:32] = action (u8) — 0=add, 1=remove
```

Mute list capacity: 16 entries. Muted senders' NOTIFY is rejected at earliest check point. Mute overrides policy table (independent mechanism).

---

## 6. Validation Flow (NOTIFY)

```
1. Receive PDX message → type_id == OP_BELL_NOTIFY (0xC0)
   │
2. Extract fields from arg0/arg1/arg2 + caller_pd from msg.caller_pd
   │
3. MUTE CHECK (first, before any processing)
   ├── is_muted(caller_pd)?        → reject: [bell.notify.reject] reason=muted
   └── is_policy_muted(caller_pd)? → reject: [bell.notify.reject] reason=muted
   │
4. VALIDATE ENUM RANGES
   ├── valid_category(category)?           invalid → reject: "invalid_category"
   ├── valid_privacy_level(privacy)?       invalid → reject: "invalid_privacy"
   ├── valid_redaction_class(redaction)?   invalid → reject: "invalid_redaction"
   ├── urgency_hint in 0..3?              invalid → reject: "invalid_urgency"
   ├── action_count in 0..1?              invalid → reject: "action_count_invalid"
   ├── action_count==1 && action_id==0?   invalid → reject: "action_id_zero"
   └── object_ref_count in 0..1?          invalid → reject: "object_refs_invalid"
   │
5. EMIT RECV MARKER
   └── [bell.notify.recv] caller_pd= category= requested=
   │
6. DERIVE LANE (first-proof placeholder)
   └── derive_lane_first_proof(urgency_hint) → (final_lane, final_urgency, downgrade_reason)
   │
7. SPAM BUDGET CHECK
   ├── check_spam_budget(caller_pd)?
   └── Exceeded → reject: [bell.policy.reject] reason=spam_budget
   │
8. APPLY POLICY OVERRIDES
   ├── apply_policy_privacy(caller_pd, privacy_level) → effective_privacy
   └── apply_policy_lane(caller_pd, final_lane) → effective_lane
   │
9. PUSH TO QUEUE
   ├── Queue full? → drop lowest-priority active entry
   ├── Push new entry with assigned event_id
   └── bump_generation()
   │
10. EMIT RESULT MARKER
    ├── OK:    [bell.notify.ok] caller_pd= final_lane= event_id=
    └── ERROR: [bell.notify.reject] caller_pd= reason=
```

### Rejection priority order

Rejections short-circuit in this order (first match wins):
1. **Muted** (earliest, avoids any processing)
2. **Invalid args** (enum range failures)
3. **Spam budget** (rate-limit)
4. **Queue push failure** (rare, lowest-priority drop usually prevents this)

---

## 7. SilkBar to sexdisplay Presence Pipeline

```
SilkBar (PD 6)                         Bell (PD 10)                      sexdisplay (PD 1)
─────────────                          ────────────                       ────────────────
│                                                                               │
│  Every ~2s:                                                                    │
│  pdx_call(SLOT_BELL, OP_BELL_SUBSCRIBE) ────▶                                  │
│                                             │                                  │
│                                      Reply: BELL_GENERATION                    │
│  ◀──────────────────────────────────── gen                                     │
│                                                                               │
│  if gen != bell_gen_cached:                                                    │
│    pdx_call(SLOT_BELL, OP_BELL_LIST,                                          │
│             lane_filter=0xFF, max_results=1) ───▶                             │
│                                             │                                  │
│                                      Reply: packed u64                        │
│  ◀──────────────────────────────────── total_visible, lane_counts, redacted    │
│                                                                               │
│  Unpack -> BellState:                                                          │
│    total_visible, redacted_count, flags                                        │
│                                                                               │
│  send_update(SetBellPresence, packed) ────────────────────────────────────▶   │
│                                                                               │
│                                                                       Render: │
│  Bell dot color:                                                              │
│    flags & 1 == 0        → Muted (dim)                                        │
│    total_visible == 0    → Muted (dim)                                        │
│    redacted_count > 0    → Amber (0x00FFAA44)                                 │
│    Otherwise             → Gold  (0x00FFD700)                                 │
│  Count badge: max 99                                                          │
│    FONT digits rendered top-right of Bell module                              │
```

### Render state rules

| Condition | Bell Dot Color | Meaning |
|-----------|---------------|---------|
| `flags & 1 == 0` | Muted (dim grey) | Bell unreachable or cap missing |
| `total_visible == 0` | Muted (dim grey) | No events in queue |
| `redacted_count > 0` | Amber (`0x00FFAA44`) | Privacy-redacted events exist |
| `total_visible > 0`, no redacted | Gold (`0x00FFD700`) | Active events present |

Count badge shows `min(total_visible, 99)` rendered with FONT digits at top-right of Bell module.

---

## 8. Capability Grants (Kernel-Side)

Defined in `kernel/src/init.rs`. Three PDs hold `SLOT_BELL` domain capability:

| PD | Name | Slot | Purpose |
|----|------|------|---------|
| 3 | silk-shell | SLOT_BELL (12) -> sexbell (10) | Policy control (SET_POLICY, LIST, CLEAR, MUTE) |
| 6 | silkbar | SLOT_BELL (12) -> sexbell (10) | Poll for aggregate presence (SUBSCRIBE, LIST) |
| 10 | sexbell | SLOT_BELL (12) -> sexbell (10) | Self-cap for listen loop |

**No app PDs hold SLOT_BELL.** Apps send NOTIFY via kernel `pdx_call` which routes to the destination domain regardless of whether the caller holds Bell's capability. Bell validates sender identity via `msg.caller_pd` and enforces policy server-side. This is intentional: capability grants control privileged operations (LIST, SET_POLICY, CLEAR, MUTE), not event submission.

---

## 9. Server-Side Allowlists

Two independent allowlists gate privileged operations beyond the kernel capability check:

### 9.1 LIST Allowlist

```rust
const BELL_LIST_ALLOWLIST: &[u32] = &[3, 6];
// 3 = silk-shell (policy owner, may view FullHidden)
// 6 = silkbar (aggregate poller, Public-only visibility)
```

**Privacy tiers by caller:**
| Caller PD | max_privacy | Visibility |
|-----------|------------|------------|
| 3 (silk-shell) | 3 (FullHidden) | All events visible |
| 6 (silkbar) | 0 (Public) | Only Public-level events in counts |
| Any other | 0 (Public, default-deny) | Reply = `u64::MAX` |

### 9.2 Policy Author Allowlist

```rust
const BELL_POLICY_AUTHOR_ALLOWLIST: &[u32] = &[3];
// 3 = silk-shell — sole policy authority
```

Only silk-shell (PD 3) may call `OP_BELL_SET_POLICY`. SilkBar (PD 6) is explicitly excluded.

---

## 10. Spam Budget

Prevents any single PD from flooding Bell with events.

| Property | Value |
|----------|-------|
| Window | 64 ticks (~1 second) |
| Max events per PD per window | 8 |
| Tracking slots | 16 (LRU-evicted when full) |
| Rejection marker | `[bell.policy.reject] reason=spam_budget` |
| Budget | 8 markers |

### Algorithm

```
fn check_spam_budget(caller_pd: u32) -> bool:
    now = get_ticks()
    find slot for caller_pd:
        if window expired (now - window_start >= 64):
            reset window, count = 1 -> ALLOW
        if count >= 8:
            -> DENY
        count += 1 -> ALLOW
    if no slot found:
        find empty or oldest slot, assign -> ALLOW
```

---

## 11. Mute List

Runtime mechanism for silencing specific PDs. Independent from policy table (SET_POLICY's force_mute is persistent, MUTE_SENDER is volatile).

| Property | Value |
|----------|-------|
| Capacity | 16 PD slots |
| Structure | Static array, shift-remove on removal |
| Operations | Add (idempotent if already present), Remove (no-op if not found) |
| Rejection marker | `[bell.notify.reject] reason=muted` |
| Budget | 8 markers |
| Check point | Earliest in NOTIFY flow (before any processing) |

---

## 12. Policy Table (SET_POLICY)

Per-PD policy overrides set by silk-shell. Volatile — lost on Bell restart.

| Property | Value |
|----------|-------|
| Capacity | 8 entries |
| Author | silk-shell only (PD 3) |
| Override types | privacy_level, lane_override, force_mute |
| Privacy invariant | Can only increase restriction, never reduce |
| Clear | Set `active_flags = 0` removes entry |
| Marker budget | 8 per operation type (set/deny/reject) |

### Policy fields

| Field | Bits | Values | Effect |
|-------|------|--------|--------|
| `active_flags` | [2:0] | bit0=privacy, bit1=lane, bit2=mute | Which overrides are active |
| `privacy_level` | [9:8] | 0=Public..3=FullHidden | Minimum privacy level for events from this PD |
| `lane_override` | [18:16] | 0=PASSIVE..5=SECURITY | Forces all events from this PD to this lane |
| `force_mute` | [24] | 0=unmuted, 1=muted | Mutes this PD (acts like mute list) |

---

## 13. Negative Cases (Rejection Catalog)

### 13.1 Unknown/Invalid Sender

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| Unknown PD (no cap table entry) | `derive_lane_first_proof` returns downgrade | Downgrade to PASSIVE (not reject) | `[bell.notify.downgrade]` |
| Invalid category enum (>5) | `valid_category()` | Reject: "invalid_category" | `[bell.notify.reject]` |
| Invalid privacy_level (>3) | `valid_privacy_level()` | Reject: "invalid_privacy" | `[bell.notify.reject]` |
| Invalid urgency_hint (>3) | Range check | Reject: "invalid_urgency" | `[bell.notify.reject]` |

### 13.2 Urgency Without Capability

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| urgency_hint > 0, no NotifyNormal cap | `derive_lane_first_proof` | Downgrade to PASSIVE | `[bell.notify.downgrade] reason=no_caps_untrusted` |
| urgency_hint > 1, no NotifyUrgent cap | (future) | Downgrade to NORMAL | `[bell.notify.downgrade]` |
| urgency_hint > 2, no NotifyPersistent cap | (future) | Downgrade to URGENT | `[bell.notify.downgrade]` |

### 13.3 Too Many Events (Spam)

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| >8 events in 64-tick window | `check_spam_budget()` returns false | Reject silently (continue loop) | `[bell.policy.reject] reason=spam_budget` |
| Queue full, no droppable entry | `find_lowest_priority_index()` returns None | Reject: "queue_full" | `[bell.queue.reject.full]` |

### 13.4 Action Without Capability

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| action_count > 1 in V1 | Range check | Reject: "action_count_invalid" | `[bell.notify.reject]` |
| action_count==1 with action_id==0 | Validation | Reject: "action_id_zero" | `[bell.notify.reject]` |
| ACTION opcode on non-existent event_id | Queue scan misses | Reject silently | `[bell.action.reject] reason=not_found` |
| ACTION opcode with mismatched action_id | Queue scan matches event_id but not action_id | Reject silently | `[bell.action.reject] reason=not_found` |

### 13.5 Muted Sender

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| PD in mute list | `is_muted(caller_pd)` at step 3 | Reject (continue loop) | `[bell.notify.reject] reason=muted` |
| PD with policy force_mute | `is_policy_muted(caller_pd)` at step 3 | Reject (continue loop) | `[bell.notify.reject] reason=muted` |
| Mute list full (16 entries) | `add_mute()` returns Err | Reject: "mute_list_full" | `[bell.mute.reject] reason=mute_list_full` |

### 13.6 LIST/Policy Access Without Capability

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| LIST from non-allowlisted PD | `is_list_reader_allowed()` false | Reply `u64::MAX` | `[bell.readcap.deny]` |
| SET_POLICY from non-author PD | `is_policy_author_allowed()` false | Reply `u64::MAX` | `[bell.policy.deny]` |
| LIST with invalid lane_filter | `lane_filter != 0xFF && lane_filter > 5` | Reject (continue loop) | `[bell.list.reject] reason=invalid_lane` |
| LIST with invalid max_results | `max_results == 0 || max_results > 4` | Reject (continue loop) | `[bell.list.reject] reason=invalid_count` |
| SET_POLICY privacy reduction | New privacy < existing privacy | Reject | `[bell.policy.reject] reason=privacy_reduction` |

### 13.7 CLOSE/CLEAR Edge Cases

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| CLOSE on non-existent event_id | Queue scan misses | Reject silently | `[bell.close.reject] reason=not_found` |
| CLOSE on already-dismissed event | `dismissed == 1` check | Reject silently | `[bell.close.reject] reason=not_found` |
| CLEAR on invalid lane (>5, not 0xFF) | Range check | Reject | `[bell.clear.reject] reason=invalid_lane` |
| CLEAR on empty lane (no match) | No entries match | No-op, no bump | No marker emitted |

### 13.8 SUBSCRIBE Denial

| Case | Detection | Response | Marker |
|------|-----------|----------|--------|
| SUBSCRIBE from non-allowlisted PD | `is_list_reader_allowed()` false | Reply `u64::MAX` | `[bell.subscribe.deny]` |

---

## 14. Proof Marker Policy

### 14.1 Marker Budgets

All markers have configurable budget limits via `static mut` counters. Budgets prevent serial log flooding.

| Marker | Budget | When |
|--------|--------|------|
| `[bell.notify.recv]` | 8 | NOTIFY received, args valid |
| `[bell.notify.ok]` | 8 | Event accepted into queue |
| `[bell.notify.reject]` | 4 | Event rejected (any reason) |
| `[bell.notify.downgrade]` | 8 | Lane downgraded by policy |
| `[bell.queue.push]` | 64 | Entry written to ring buffer |
| `[bell.queue.drop]` | 16 | Low-priority entry evicted for space |
| `[bell.queue.reject.full]` | 16 | Queue truly full (no droppable entry) |
| `[bell.list.reply]` | 8 | LIST reply sent with aggregate counts |
| `[bell.list.reject]` | 4 | LIST args invalid (lane or count) |
| `[bell.list.item]` | 8 | Individual event scanned in LIST |
| `[bell.list.redact]` | 8 | FullHidden events counted but hidden |
| `[bell.readcap.deny]` | 8 | LIST caller not in allowlist |
| `[bell.close.ok]` | 8 | Event dismissed by CLOSE |
| `[bell.close.reject]` | 4 | CLOSE failed (not found) |
| `[bell.action.dispatch]` | 8 | ACTION matched event+action_id |
| `[bell.action.reject]` | 4 | ACTION failed (not found) |
| `[bell.clear.ok]` | 4 | CLEAR succeeded |
| `[bell.clear.reject]` | 4 | CLEAR arg invalid |
| `[bell.mute.add]` | 8 | PD added to mute list |
| `[bell.mute.remove]` | 8 | PD removed from mute list |
| `[bell.mute.reject]` | 4 | MUTE operation failed |
| `[bell.policy.set]` | 8 | SET_POLICY applied |
| `[bell.policy.deny]` | 8 | SET_POLICY caller unauthorized |
| `[bell.policy.reject]` | 8 | SET_POLICY validation failed |
| `[bell.subscribe.reply]` | 4 | SUBSCRIBE reply sent with gen counter |
| `[bell.subscribe.deny]` | 8 | SUBSCRIBE caller unauthorized |
| `[bell.unknown.reject]` | 8 | Unknown opcode received |

### 14.2 Allowed in Markers (StructuralMeta/SenderMeta)

| Field | Redaction Class | Example |
|-------|----------------|---------|
| `event_id` | StructuralMeta | `event_id=42` |
| `caller_pd` | StructuralMeta | `caller_pd=7` |
| `category` | StructuralMeta | `category=1` |
| `requested_lane` | StructuralMeta | `requested=2` |
| `final_lane` | StructuralMeta | `final_lane=0` |
| `final_urgency` | StructuralMeta | `final_urgency=0` |
| `privacy_level` | StructuralMeta | `privacy=0` |
| `redaction_class` | StructuralMeta | `redaction=0` |
| `action_count` | StructuralMeta | `actions=1` |
| `object_ref_count` | StructuralMeta | `refs=1` |
| `downgrade_reason` | StructuralMeta | `reason=no_caps_untrusted` |
| `reject_reason` | StructuralMeta | `reason=invalid_category` |
| `lane` | StructuralMeta | `lane=0` |
| `count` | StructuralMeta | `count=3` |
| `gen` | StructuralMeta | `gen=7` |

### 14.3 Forbidden in Markers (SecretContent)

| Field | Why |
|-------|-----|
| Event title/body text | Private content (not stored in queue in V1) |
| Sender display name | Could identify PII |
| Action capability details | Capability token leak |
| Object reference names | Could identify documents/projects |
| File paths | System structure leak |

---

## 15. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. **Giving apps untrusted urgent/system notifications.** Currently all urgency > 0 downgrades to PASSIVE. The full capability matrix (12 BellCap bits) must be reviewed before enabling lane escalation.
2. **Implementing notification persistence.** Linen/DiskFS persistence requires storage maturity gate review. Bell V1 ring buffer is RAM-only, lost on reboot. Do not add sexstore/Linen persistence without a separate design gate.
3. **Letting sexdisplay own Bell policy.** sexdisplay is a pure renderer. It must never read, store, or act on BellEvent fields beyond the `BellState` aggregate (total_visible, redacted_count, flags).
4. **Adding kernel/ABI changes.** Bell operates entirely in userspace over existing PDX mechanism. No new syscalls, no shared memory, no kernel capability table changes.
5. **Adding renderer-owned policy.** SilkBar owns Bell display state. sexdisplay receives `SetBellPresence` update and renders. No Bell semantic interpretation in sexdisplay.
6. **Implementing SUBSCRIBE push.** Current SUBSCRIBE is poll-based (returns generation counter). True push notifications would require kernel IPC changes (callback or shared ring). Not in scope for V1.
7. **Adding sound dispatch.** Harp/Theremin gate must precede any audio integration.
8. **Adding lockscreen rendering.** Lockscreen display policy requires a separate design gate.
9. **Exposing private content in proof markers.** Any marker containing event body/title is a PRIORITY-0 bug. V1 queue does not store body/title text.
10. **Allowing app-owned notification priority.** Final lane/urgency is always Bell-derived. Apps provide `urgency_hint` as a request, not a directive.
11. **Opcode collision with existing sex-pdx protocols.** Before Phase E1 code begins, audit sex-pdx opcode range `0xC0–0xC7` against all current opcode assignments. Bell owns this range; any collision is a STOP FIRST.

---

## 16. Integration Points (Existing, Do Not Modify)

### 16.1 SilkBar (PD 6)
- Polls `OP_BELL_SUBSCRIBE` every ~2s (when `uptime_seconds % 2 == 0`)
- If generation changed, calls `OP_BELL_LIST` with `lane_filter=0xFF, max_results=1`
- Sends `send_update(SetBellPresence=7, packed)` to sexdisplay
- Maintains `bell_gen_cached` and `bell_pending_list` state
- See `servers/silkbar/src/main.rs` lines 58–248

### 16.2 sexdisplay (PD 1)
- Receives `SetBellPresence` update (message type 7)
- Renders Bell dot color based on `total_visible`, `redacted_count`, `flags`
- Renders count badge (max 99) using FONT digits
- See `servers/sexdisplay/src/main.rs`

### 16.3 silkbar-model
- `BellState` struct: `bell_available`, `total_visible`, `redacted_count`, `flags`
- `SetBellPresence = 7` variant in SilkBar update enum
- See `crates/silkbar-model/src/lib.rs`

### 16.4 Kernel Init (init.rs)
- Spawn order: sexbell is PD 10 (last in module_paths before sexfiles/spindle)
- Three SLOT_BELL grants: silk-shell (PD 3), silkbar (PD 6), sexbell self (PD 10)
- boot marker: `[bell.boot]`
- See `kernel/src/init.rs`

### 16.5 sex-pdx Constants
- `OP_BELL_NOTIFY = 0xC0` through `OP_BELL_MUTE_SENDER = 0xC7`
- `SLOT_BELL = 12`
- See `crates/sex-pdx/src/lib.rs`

---

## 17. Proof Plan

### 17.1 Proof Goals

| # | Goal | Verification Method |
|---|------|-------------------|
| P1 | Every NOTIFY is either accepted (event_id assigned) or rejected with a reason marker | Serial log audit: count `[bell.notify.ok]` + `[bell.notify.reject]` = count `[bell.notify.recv]` |
| P2 | No sender can bypass mute check | Code audit: mute check at line 489, before any validation or processing |
| P3 | Spam budget prevents flooding | Code audit: `SPAM_WINDOW_TICKS=64`, `SPAM_MAX_PER_WINDOW=8`; rejects over limit |
| P4 | Queue overflow preserves highest priority | Code audit: `find_lowest_priority_index()` drops lowest `final_lane`; ties broken by oldest |
| P5 | LIST only reveals to allowlisted PDs | Code audit: `is_list_reader_allowed()` gates access; non-allowlisted get `u64::MAX` |
| P6 | LIST privacy gate works | Code audit: `max_privacy_for_caller()` limits PD 6 to Public-only; PD 3 sees FullHidden |
| P7 | SilkBar presence pipeline is complete | Runtime gate: `[silkbar.bell.poll.reply]` appears at least once per 45s probe |
| P8 | sexdisplay renders Bell dot | Runtime gate: `[sexdisplay.bell.render]` appears when events present |
| P9 | No private content in markers | Grep audit: no title/body/sender_name fields in any `serial_println!` call |
| P10 | Generation counter bumps on state change | Code audit: `bump_generation()` called after queue push, clear, close, mute, policy set |

### 17.2 Proof Marker Chain (Happy Path)

```
[bell.boot]                          ← Bell spawned (PD 10)
[bell.demo.boot] event_id=1          ← Self-notify for pipe exercise
[bell.queue.push] id=1 final_lane=0  ← Entry in ring buffer
[bell.notify.ok] final_lane=0        ← Self-notify accepted

... SilkBar polls every ~2s ...

[silkbar.bell.gen.reply] gen=2       ← SUBSCRIBE returns changed gen
[silkbar.bell.poll.reply] total=1    ← LIST returns 1 visible event
[silkbar.bell.state] total=1         ← SilkBar updates BellState
[sexdisplay.bell.render]             ← sexdisplay renders Bell dot
```

### 17.3 Proof Marker Chain (Rejection Path)

```
[bell.notify.reject] caller_pd=7 reason=muted
[bell.notify.reject] caller_pd=7 reason=invalid_category
[bell.policy.reject] caller_pd=7 reason=spam_budget window=64 max=8
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.queue.reject.full] count=16
```

---

## 18. Files Reference

| File | Role |
|------|------|
| `servers/sexbell/src/main.rs` | Bell server (1239 lines) — queue, policy, all 8 opcode handlers |
| `crates/sex-pdx/src/lib.rs` | PDX constants: `OP_BELL_*` (0xC0–0xC7), `SLOT_BELL = 12` |
| `kernel/src/init.rs` | Spawn order, three SLOT_BELL capability grants |
| `servers/silkbar/src/main.rs` | Bell poll every ~2s, SUBSCRIBE+LIST, `SetBellPresence` update |
| `servers/sexdisplay/src/main.rs` | Bell dot color + count badge rendering |
| `crates/silkbar-model/src/lib.rs` | `BellState` struct, `SetBellPresence = 7` |

### Key Handoff Documents

| Document | Scope |
|----------|-------|
| `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` | Original event model, lanes, privacy design |
| `BELL_CAPABILITY_POLICY_V1.md` | Full 12-cap matrix, lane derivation algorithm, default-deny rules |
| `BELL_PDX_PROTOCOL_SPEC_V1.md` | Opcode placeholders, message shapes, validation flow |
| `BELL_V1_FINAL_STATUS.md` | Bell V1 completion status, feature matrix, known issues |
| `BELL_CAPABILITY_NOTIFICATION_PLAN_V1.md` | This document — consolidated architecture plan |

---

## 19. Recommended Implementation Sequence

```
1. BELL_PLAN_REFINEMENT_AUDIT_V1          (docs-only, complete: 2026-05-07)
2. BELL_PHASE_E1_POLICY_CAP_TABLE_V1      (code: policy proof, no UI)
3. BELL_PHASE_E2_QUEUE_AND_LIST_V1        (code: event ring/list/clear)
4. BELL_PHASE_E3_SILKBAR_COMPACT_INDICATOR_V1  (code: SilkBar poll+display)
5. BELL_PHASE_E4_SEXDISPLAY_RENDER_STUB_V1     (code: sexdisplay render)
6. FINAL_BELL_CAPABILITY_AUDIT_V1         (docs-only)
```

**Do not start Bell code until Linen/Quil object pipeline stabilizes.** Bell is a parallel design lane, not the next implementation lane.

### Phase boundaries

| Phase | Scope | Depends On | Sexdisplay Changed? | Persistence? |
|-------|-------|-----------|-------------------|-------------|
| **E1** | Policy table, cap derivation, negative tests | Refinement audit | ❌ No | ❌ No |
| **E2** | Event ring buffer, LIST/CLEAR/CLOSE | E1 | ❌ No | ❌ No |
| **E3** | SilkBar poll (SUBSCRIBE+LIST), `BellState` update | E2 | ❌ No (silkbar-model only) | ❌ No |
| **E4** | sexdisplay Bell dot + count badge render | E3 | ✅ Yes | ❌ No |
| **Audit** | Full capability boundary review | E1–E4 | ❌ No | ❌ No |

### Next Implementation Prompt (E1)

```
MISSION: BELL_PHASE_E1_POLICY_CAP_TABLE_V1

Goal:
Replace Bell's first-proof placeholder lane derivation with a Bell-local
policy table and capability derivation. No UI. No rendering. No persistence.

Pre-conditions:
- Bell V1 server runs with first-proof policy (all urgency → PASSIVE)
- Queue, spam budget, mute list, policy table all exist
- SilkBar presence pipeline is proven

Scope:
1. Define BellCap bitmask constants in Bell-local namespace (no sex-pdx changes).
2. Implement sender classification from cap table (classify() function).
3. Implement lane derivation algorithm:
   - urgency_hint 0 → PASSIVE (always)
   - urgency_hint 1 → NORMAL (if NotifyNormal cap, else PASSIVE)
   - urgency_hint 2 → URGENT (if NotifyUrgent + trust_label>=1, else downgrade)
   - urgency_hint 3 → PERSISTENT (if NotifyPersistent + trust_label>=2, else downgrade)
   - category=SYSTEM → SYSTEM (if NotifySystem cap, else REJECT)
   - category=SECURITY → SECURITY (if NotifySecurity cap, else REJECT)
4. Replace derive_lane_first_proof() with derive_lane().
5. Add capability grant entries to kernel init.rs for:
   - sexbell self: all caps for server operation
   - silk-shell: NotifySystem, NotifyPersistent for shell-owned events
   - silkbar: NotifyPassive for low-priority bar events
6. No app grants — apps remain untrusted (PASSIVE fallback).
7. Add proof markers:
   - [bell.lane.derive] caller_pd= category= requested= final= reason=
   - [bell.cap.classify] caller_pd= class= max_lane= caps=
   - [bell.cap.reject] caller_pd= reason= (for SYSTEM/SECURITY hard rejections)
8. Negative tests:
   - Unknown PD → PASSIVE fallback
   - URGENT without cap → downgrade to NORMAL/PASSIVE
   - SYSTEM without cap → hard reject (not downgrade)
   - SECURITY without cap → hard reject (not downgrade)
   - PERSISTENT without cap → downgrade to URGENT/NORMAL

STOP FIRST if:
- Cap table mechanism requires kernel ABI changes (use existing PDX slots)
- Lane derivation reads from uninitialized cap table
- Any app gets URGENT without trust_label >= 1
- Any app gets PERSISTENT without trust_label >= 2
- SYSTEM/SECURITY caps grantable to non-system PDs
- sexdisplay code is touched (must not change in E1)
- Persistence code is added (must not exist until post-E4 storage gate)
```

---

*End of BELL_CAPABILITY_NOTIFICATION_PLAN_V1.md*
