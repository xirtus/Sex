# BELL_PDX_PROTOCOL_SPEC_V1

**Status:** Docs-only protocol spec. No code changed. No opcode allocation.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_EVENT_MODEL_DESIGN_GATE_V1.md`, `BELL_CAPABILITY_POLICY_V1.md`

---

## 1. Purpose

Bell accepts event requests over PDX. The protocol is bounded, no-heap, no-raw-pointer, no-private-content-logging. Bell validates capability policy before producing internal BellEvents. This document specifies the proposed opcode placeholders, message shapes, status codes, validation flow, and security invariants — **without implementing any of them**.

### Protocol principles

- **Caller identity is kernel-authoritative.** Bell never trusts a `sender_name` or `sender_pd` field from the request payload. The PDX `caller_pd` is the sole source of sender identity.
- **Request is bounded.** All request structs are fixed-size, no_std, no raw pointers, no heap strings.
- **Reply is synchronous.** Bell replies on the same PDX call. No async registration or callback PDX.
- **No private content in protocol.** The wire format never carries event title, body, or sender display name. Those are resolved later by the inbox from internal BellEvent storage (future).
- **No opcode assignment.** All opcodes are `TBD` — no numeric values assigned to sex-pdx or kernel cap tables.

---

## 2. Proposed Slot and Opcode Placeholders

### Slot

```
SLOT_BELL = TBD
```

**STOP FIRST** before assigning a numeric slot value. Slot assignment requires:
- Confirmed slot is not conflicting with existing slots (SLOT_SEXSTORE=10, SLOT_DISPLAY, etc.)
- Kernel cap table entry added
- Init.rs spawn order updated
- sex-pdx constant added

### Opcodes

| Opcode | Name | Direction | Description |
|--------|------|-----------|-------------|
| `OP_BELL_NOTIFY` | `TBD` | App → Bell | Request to create a BellEvent |
| `OP_BELL_CLOSE` | `TBD` | App/Shell → Bell | Dismiss/close an existing event by ID |
| `OP_BELL_ACTION` | `TBD` | App/Shell → Bell | Execute an action callback on an event |
| `OP_BELL_LIST` | `TBD` | Shell → Bell | List current events (summary, no private content) |
| `OP_BELL_CLEAR` | `TBD` | Shell → Bell | Clear all events in a lane or all lanes |
| `OP_BELL_SUBSCRIBE` | `TBD` | SilkBar → Bell | Subscribe to lane-summary updates (future) |
| `OP_BELL_SET_POLICY` | `TBD` | Shell → Bell | Set per-app user policy override (future) |
| `OP_BELL_MUTE_SENDER` | `TBD` | Shell → Bell | Mute a sender PD (future) |

**State clearly:** No numeric opcode values are assigned in this phase. No edits to sex-pdx, kernel, or cap tables.

---

## 3. Message Shapes

All request/reply structs are fixed-size, no_std, no raw pointers, no heap strings. Text fields are future token IDs or bounded hashes only. Action refs are cap IDs/tokens only. Object refs are bounded IDs only.

### 3.1 BellNotifyRequest

```rust
/// Request from app to create a BellEvent.
/// Total size: 56 bytes (fixed, no heap, no pointers).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellNotifyRequest {
    /// Sender-assigned identity token (opaque u32, validated against caps).
    /// Bell does NOT trust this as authoritative identity — caller_pd is king.
    sender_identity_token: u32,

    /// Event category (BellCategory enum).
    category: u8,

    /// Urgency hint (0..3). Bell derives final urgency via policy.
    urgency_hint: u8,

    /// Privacy level (BellPrivacyLevel enum).
    privacy_level: u8,

    /// Number of action caps in action_caps array.
    action_count: u8,

    /// Up to 4 action capability tokens.
    action_caps: [u32; 4],

    /// Up to 2 object references (Linen object IDs or surface IDs).
    object_refs: [u64; 2],

    /// Reserved for future use (zero for now).
    _reserved: [u8; 12],
}
```

**Size:** `4 + 1 + 1 + 1 + 1 + 16 + 16 + 12 = 52 bytes` + padding to 56 bytes.

### 3.2 BellNotifyReply

```rust
/// Reply to BellNotifyRequest.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellNotifyReply {
    /// Status code (BellStatus enum).
    status: u8,

    /// Assigned event ID (0 if rejected).
    event_id: u64,

    /// Final lane after policy derivation (BellLane enum, 0..5).
    final_lane: u8,

    /// Final urgency after policy derivation (0..3).
    final_urgency: u8,

    /// Reject reason code (0 if OK, otherwise BellRejectReason enum).
    reject_reason: u8,

    /// Reserved for future use.
    _reserved: [u8; 5],
}
```

**Size:** `1 + 8 + 1 + 1 + 1 + 5 = 17 bytes`.

### 3.3 BellCloseRequest

```rust
/// Request to dismiss/close an event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellCloseRequest {
    /// Event ID to close.
    event_id: u64,
    /// Reserved.
    _reserved: [u8; 8],
}
```

**Size:** `8 + 8 = 16 bytes`.

### 3.4 BellActionRequest

```rust
/// Request to execute an action callback on an event.
/// Actions require BellActionCallback cap.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellActionRequest {
    /// Event ID.
    event_id: u64,
    /// Action cap index (0..action_count-1).
    action_index: u8,
    /// Reserved.
    _reserved: [u8; 7],
}
```

**Size:** `8 + 1 + 7 = 16 bytes`.

### 3.5 BellListRequest

```rust
/// Request to list current events.
/// Returns summary only — no private content on the wire.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellListRequest {
    /// Filter by lane (0xFF = all lanes).
    lane_filter: u8,
    /// Maximum events to return in summary.
    max_results: u8,
    /// Reserved.
    _reserved: [u8; 6],
}
```

**Size:** `1 + 1 + 6 = 8 bytes`.

### 3.6 BellListReplySummary

```rust
/// Summary of a single event for listing.
/// No private content — only structural fields.
/// Inbox resolves full display from internal BellEvent storage (future).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellEventSummary {
    event_id: u64,
    category: u8,
    final_lane: u8,
    final_urgency: u8,
    privacy_level: u8,
    trust_label: u8,
    workspace_context: u32,
    scene_context: u8,
    expires_at_ticks: u64,
    action_count: u8,
    _reserved: [u8; 7],
}

/// Reply to BellListRequest.
/// Contains up to 8 event summaries (bounded, no heap).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellListReply {
    /// Status code.
    status: u8,
    /// Number of summaries in the events array.
    count: u8,
    /// Total events matching filter (may exceed count).
    total_matching: u8,
    /// Reserved.
    _reserved: [u8; 5],
    /// Up to 8 event summaries.
    events: [BellEventSummary; 8],
}
```

**Size per summary:** `8 + 1+1+1+1+1+4+1+8+1+7 = 34 bytes`.
**Total reply:** `1+1+1+5 + 8*34 = 280 bytes`. Fits within PDX message size limits.

### 3.7 BellClearRequest

```rust
/// Request to clear events.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellClearRequest {
    /// Lane to clear (0xFF = all lanes).
    lane: u8,
    /// Reserved.
    _reserved: [u8; 7],
}
```

**Size:** `8 bytes`.

### 3.8 BellSubscribeRequest (future)

```rust
/// Request to subscribe to lane-summary updates.
/// For SilkBar to receive push-style count/lane changes (future).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellSubscribeRequest {
    /// PD of the subscriber (SilkBar or shell component).
    subscriber_pd: u16,
    /// Reserved.
    _reserved: [u8; 6],
}
```

**Size:** `8 bytes`.

### 3.9 BellSetPolicyRequest (future placeholder)

```rust
/// Request to set per-app user policy override.
/// Policy storage belongs in sexstore K/V, not in Bell.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellSetPolicyRequest {
    /// Target sender PD.
    target_pd: u16,
    /// Max allowed lane for this sender (user override).
    max_lane: u8,
    /// Reserved.
    _reserved: [u8; 5],
}
```

**Size:** `2 + 1 + 5 = 8 bytes`.

### 3.10 BellMuteSenderRequest (future placeholder)

```rust
/// Request to mute a sender PD.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellMuteSenderRequest {
    /// Sender PD to mute.
    target_pd: u16,
    /// Mute duration in ticks (0 = permanent until unmute).
    duration_ticks: u32,
    /// Reserved.
    _reserved: [u8; 2],
}
```

**Size:** `2 + 4 + 2 = 8 bytes`.

---

## 4. Status Codes

```rust
/// Status codes for Bell PDX replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellStatus {
    /// Event created successfully.
    Ok = 0,
    /// Request denied by capability policy.
    Denied = 1,
    /// Request contained invalid enum or field value.
    Invalid = 2,
    /// Sender rate-limited (future).
    RateLimited = 3,
    /// Request would expose redacted/private content.
    Redacted = 4,
    /// Event ID not found (for close/action).
    NotFound = 5,
    /// Ring buffer full (try again later).
    Full = 6,
    /// Opcode not supported (future).
    Unsupported = 7,
    /// Policy decision deferred to user (future, Collar/sexauth).
    PolicyDeferred = 8,
}
```

### Status-to-action mapping

| Status | Meaning | Sender Action |
|--------|---------|---------------|
| `Ok` | Event created | Read `event_id`, `final_lane` from reply |
| `Denied` | Cap policy blocked | Check `reject_reason` — do not retry with same payload |
| `Invalid` | Bad enum/field | Fix payload and retry |
| `RateLimited` | Too many events | Back off and retry later |
| `Redacted` | Private content would leak | Remove private fields and retry |
| `NotFound` | Event ID unknown | Verify event_id is correct |
| `Full` | Ring buffer full | Retry later (or dismiss old events first) |
| `Unsupported` | Opcode not available | Do not retry |
| `PolicyDeferred` | Needs user grant (future) | Wait for Collar/sexauth grant |

---

## 5. Validation Flow for OP_BELL_NOTIFY

```
1. Parse BellNotifyRequest from PDX message
   │
2. Validate enum ranges
   ├── category ∈ valid BellCategory values
   ├── privacy_level ∈ valid BellPrivacyLevel values
   ├── urgency_hint ∈ 0..3
   └── action_count ∈ 0..4
   │   FAIL → return INVALID + reject_reason
   │
3. Resolve sender_pd from PDX caller_pd (kernel-authoritative)
   └── sender_pd = msg.caller_pd  // NOT from request struct
   │
4. Classify sender from Bell cap table
   ├── Look up sender_pd in cap table
   ├── Determine sender class from granted BellCap bits
   └── sender_pd not found → return DENIED + unknown_sender
   │
5. Check BellCap bitmask
   ├── Compare requested lane against sender's max allowed lane
   └── NotifySystem/NotifySecurity caps checked for SYSTEM/SECURITY categories
   │
6. Derive final lane/urgency
   ├── Apply lane derivation algorithm (see BELL_CAPABILITY_POLICY_V1 §4)
   └── May downgrade or reject
   │   REJECT → return DENIED + reject_reason
   │
7. Apply privacy/redaction
   ├── Strip fields that exceed sender's privacy caps
   ├── Strip action_caps if missing ActionCallback cap
   ├── Strip object_refs if missing ObjectReference cap
   └── privacy_level downgraded if missing LockscreenVisible cap
   │
8. Allocate slot in ring buffer
   ├── If ring buffer full → return FULL
   └── Write sanitized BellEvent to next slot
   │
9. Emit structural proof marker
   └── [bell.notify.ok] event_id= sender_pd= requested= final= downgrade=
   │
10. Return BellNotifyReply
    └── status=OK, event_id, final_lane, final_urgency
```

---

## 6. Reply Semantics

- **Synchronous PDX reply.** Bell replies on the same PDX call. No async registration.
- **No private content in reply.** The reply contains only structural fields (event_id, final_lane, final_urgency, status, reject_reason).
- **event_id returned only on OK.** If status != OK, event_id is 0.
- **final_lane/final_urgency returned on OK.** The sender learns its derived lane/urgency.
- **reject_reason on DENIED/INVALID.** A machine-readable enum, not a human-readable string.

---

## 7. Queue/Storage Model Placeholder

```
First implementation: bounded RAM ring buffer
  - 64 entries (fixed at compile time)
  - Circular FIFO
  - No persistence across boot
  - No sexstore/sexshop usage
  - No heap allocation

Future: persistence gate adds sexstore K/V
  - Requires E-series storage maturity gate
  - Not in protocol V1
  - Not in server stub V1
```

The ring buffer is owned by Bell (future `servers/sexbell/`). It is not shared with sexdisplay, silk-shell, or any other component. Inbox reads events by requesting `BellListRequest` over PDX — no shared memory.

---

## 8. Security Invariants

| # | Invariant | Enforcement | Violation Consequence |
|---|-----------|------------|----------------------|
| 1 | `caller_pd` is kernel-authoritative | PDX message struct, set by kernel | Cannot spoof — kernel sets the field |
| 2 | App-provided sender name never trusted | No sender_name field in protocol | N/A — field doesn't exist |
| 3 | Final urgency/lane is Bell-derived | Lane derivation algorithm | Downgrade or reject |
| 4 | Action callback requires `BellActionCallback` cap | Cap check in derivation step | Strip actions or reject |
| 5 | Object refs require `BellObjectReference` cap | Cap check in derivation step | Strip refs or reject |
| 6 | Lockscreen visibility requires cap + privacy validation | Cap check + privacy_level validation | Downgrade privacy |
| 7 | SYSTEM/SECURITY categories require system caps | Cap check for NotifySystem/NotifySecurity | Reject with missing_*_cap |
| 8 | No private content on wire | No title/body fields in protocol | N/A — fields don't exist |
| 9 | Ring buffer is Bell-local | Not shared with any component | No information leak via shared state |
| 10 | Proof markers never log private content | Marker format enforced in code review | PRIORITY-0 bug if violated |

---

## 9. Proof Markers

### Allowed markers

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.notify.ok]` | 64 | `event_id`, `sender_pd`, `requested_lane`, `final_lane`, `downgrade_reason` | Event created |
| `[bell.notify.reject]` | 64 | `sender_pd`, `reject_reason` | Event rejected |
| `[bell.notify.downgrade]` | 64 | `event_id`, `from_lane`, `to_lane`, `reason` | Lane downgraded |
| `[bell.close.ok]` | 64 | `event_id` | Event dismissed |
| `[bell.close.reject]` | 64 | `event_id`, `reason` | Close failed |
| `[bell.action.reject]` | 16 | `event_id`, `reason` | Action rejected |
| `[bell.list.summary]` | 16 | `count`, `total_matching` | List returned |
| `[bell.clear.ok]` | 8 | `lane`, `count` | Events cleared |
| `[bell.ring.full]` | 8 | — | Ring buffer at capacity |

### Forbidden patterns

```
[bell.notify.ok] title="Build complete"           ← FORBIDDEN (SecretContent)
[bell.notify.ok] sender="My App"                   ← FORBIDDEN (SecretContent)
[bell.list.summary] event_0_title="..."            ← FORBIDDEN (SecretContent)
```

---

## 10. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Assigning a numeric value to `SLOT_BELL`.
2. Assigning numeric values to `OP_BELL_*`.
3. Editing `sex-pdx` for any Bell opcode or constant.
4. Editing kernel cap grant table for Bell slot.
5. Implementing `servers/sexbell/` — server stub design must be reviewed first.
6. Adding persistence — requires separate Bell persistence gate.
7. Adding rendering — SilkBar and inbox are separate phases.
8. Adding action callbacks — action dispatch requires separate design gate.
9. Adding private text payload transport — protocol V1 explicitly has no title/body fields.
10. Allowing request-provided sender identity — `caller_pd` is the sole authority.

---

## 11. Future Implementation Order

| Phase | Scope | Type | Depends On |
|-------|-------|------|------------|
| **BELL_SERVER_STUB_V1** | Minimal sexbell server, ring buffer, OP_BELL_NOTIFY dispatch | Implementation | Protocol spec + cap policy |
| **BELL_NOTIFY_RAM_QUEUE_V1** | Ring buffer + event lifecycle (expiry, dismiss) | Implementation | Server stub |
| **BELL_SILKBAR_PRESENCE_V1** | Compact lane-summary indicator in global bar | Implementation | Server stub |
| **BELL_INBOX_ROWS_V1** | Full inbox surface adopting SILK_LIST_ROW_VISUAL_CANON | Implementation | Server stub + canon |
| **BELL_ACTION_CAPS_V1** | Action callback dispatch (dismiss, open, etc.) | Implementation | Inbox rows |
| **BELL_PERSISTENCE_GATE_V1** | Persist events across boot via sexstore | Implementation | E-series storage maturity + server stub |

---

## References

- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — parent design gate (event model, lanes, privacy)
- `BELL_CAPABILITY_POLICY_V1.md` — capability policy (default-deny, lane derivation)
- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — canon for future inbox
- `E15_STORAGE_DOCS_CLEANUP_V1.md` — storage canon (sexstore vs sexshop)
- `servers/sexstore/src/main.rs` — reference for PDX dispatch + cap check pattern

---

*End of BELL_PDX_PROTOCOL_SPEC_V1.md*
