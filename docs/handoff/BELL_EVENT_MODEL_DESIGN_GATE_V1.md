# BELL_EVENT_MODEL_DESIGN_GATE_V1

**Status:** Docs-only design gate. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05

---

## 1. Purpose

Bell is the user's attention firewall. A BellEvent is a typed, capability-scoped event object that informs the user without claiming focus, blocking input, or demanding immediate action. Bell is **not** a popup daemon — it is a policy-enforced event routing system that lives at the boundary between app-generated attention signals and the shell's session/workspace model.

### Core principles

- **Events are typed capability objects**, not raw strings or opaque blobs.
- **Sender identity is kernel-authoritative** via PDX caller_pd — Bell trusts the kernel, not the sender's self-declared name.
- **Urgency is validated, not inherited.** The sender may *suggest* urgency; Bell applies policy to determine final urgency.
- **Privacy is structural.** The event model carries a redaction class from creation; proof markers must never log body/title/private object names.
- **The shell owns policy.** Bell validates and routes; silk-shell provides focus/session/workspace context; sexdisplay renders final pixels only (future).
- **No popups.** Bell events appear in the SilkBar compact presence area or the inbox list — never as modal dialogs that steal focus.

---

## 2. Non-Goals (explicitly out of scope for V1)

| Non-goal | Reason |
|----------|--------|
| Server implementation | Design gate only — no `servers/sexbell/` yet |
| PDX opcodes | No `OP_BELL_*` added to sex-pdx until protocol spec |
| ABI changes | No kernel, sex-pdx, or PDX wire format changes |
| Storage persistence | No sexstore K/V or sexshop for events — Bell-local ring only |
| Rendering | No 0xEF/0xEC calls for Bell inbox yet |
| App integration | No app notification API, caps, or registration |
| Sound | No audio integration (deferred to Harp/Theremin gate) |
| Lockscreen | No lockscreen rendering — out of scope for V1 |
| Action callbacks | No `action_cap` dispatch — defined in model but not wired |
| Sexdisplay policy | sexdisplay must never own Bell policy, routing, or event data |

---

## 3. BellEvent V1 Draft Fields

```rust
/// A single attention/event object. Typed, capability-scoped, privacy-tagged.
/// Sender identity is kernel-authoritative via PDX caller_pd.
/// All string-like content is &'static str (no heap, no allocation).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BellEvent {
    /// Monotonic event ID, assigned by Bell on ingest.
    event_id: u64,

    /// Kernel-authoritative sender PD (set by Bell on receive, not by sender).
    sender_pd: u16,

    /// Sender-provided identity token (opaque u32, validated against caps).
    sender_identity_token: u32,

    /// Event category (see BellCategory enum).
    category: BellCategory,

    /// Suggested urgency from sender (0..3). Bell applies policy to derive final_urgency.
    urgency_hint: u8,

    /// Final urgency after policy validation (0=passive, 1=normal, 2=urgent, 3=persistent).
    final_urgency: u8,

    /// Privacy classification for rendering and proof logging.
    privacy_level: BellPrivacyLevel,

    /// Workspace context (0 = current workspace, non-zero = specific workspace).
    workspace_context: u32,

    /// Scene context (0xFF = no scene, otherwise scene_id).
    scene_context: u8,

    /// Number of valid action capability references.
    action_count: u8,

    /// Up to 4 action capability tokens (opaque, routed through Bell's cap table).
    action_caps: [u32; 4],

    /// Expiration tick (0 = no expiry, otherwise ticks since boot).
    expires_at_ticks: u64,

    /// Trust label: 0=untrusted, 1=verified-source, 2=system-authority.
    trust_label: u8,

    /// Up to 2 object references (Linen object IDs or surface IDs).
    object_refs: [u64; 2],

    /// Redaction class for proof logging (see §7).
    redaction_class: BellRedactionClass,
}

/// Event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellCategory {
    /// Generic informational event (no action needed).
    Info = 0,
    /// Project/workspace update (build complete, review requested, etc.).
    Project = 1,
    /// Document event (comment, mention, change notification).
    Document = 2,
    /// System event (update available, hardware change, etc.).
    System = 3,
    /// Security event (login, permission change, auth challenge).
    Security = 4,
    /// Error or failure event.
    Error = 5,
}

/// Privacy classification for a BellEvent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellPrivacyLevel {
    /// Visible on lockscreen, logged in full.
    Public = 0,
    /// Visible on lockscreen but without details.
    SenderOnly = 1,
    /// Title visible, body hidden.
    TitleOnly = 2,
    /// Full content hidden until session unlock.
    FullHidden = 3,
}

/// Redaction class for proof logging (mirrors E8 redaction hierarchy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BellRedactionClass {
    /// Safe to log: event_id, category, final_urgency, workspace_context.
    StructuralMeta = 0,
    /// Log with sender_pd and identity token but no content.
    SenderMeta = 1,
    /// Log title only (no body, no object names).
    TitleMeta = 2,
    /// Never log: body text, private object names, action cap details.
    SecretContent = 3,
}
```

### Field size budget

Total: `8+2+4+1+1+1+1+4+1+1+16+8+1+16+1` = **66 bytes** per event. Fits in two cache lines. A ring buffer of 64 events would consume ~4.2KB — acceptable for a no_std, no-heap kernel environment.

---

## 4. Capability Lanes

Lanes classify the **routing and visibility** of a BellEvent, not its content. Each event is assigned exactly one lane (derived from `final_urgency` + `category` by Bell policy).

| Lane | Slot | Visual | Dismiss | Persist | Description |
|------|------|--------|---------|---------|-------------|
| **PASSIVE** | 0 | SilkBar dim indicator | Auto-expire | No | Low-priority info, no user action needed |
| **NORMAL** | 1 | SilkBar bright indicator | Manual | Session | Standard events, dismissed by user or by opening |
| **URGENT** | 2 | SilkBar accent pulse | Manual | Until read | Requires attention, persists until acknowledged |
| **PERSISTENT** | 3 | SilkBar pinned | Manual | Until dismissed | Stays until user explicitly dismisses |
| **SYSTEM** | 4 | SilkBar system color | Manual | Boot cycle | System-originated (updates, hardware events) |
| **SECURITY** | 5 | SilkBar security color | Session unlock | Boot cycle | Security events, cleared on logout/reboot |

### Lane derivation policy (future)

```
urgency_hint 0 → PASSIVE (always, sender cannot override)
urgency_hint 1 → NORMAL (default)
urgency_hint 2 → URGENT (if trust_label >= 1, else downgrade to NORMAL)
urgency_hint 3 → PERSISTENT (if trust_label >= 2, else downgrade to URGENT)
category SYSTEM → SYSTEM lane (overrides urgency)
category SECURITY → SECURITY lane (overrides urgency)
```

This ensures:
- Untrusted senders cannot create PERSISTENT events.
- Only system-authority sources can bypass lane assignment.
- SYSTEM and SECURITY categories always get their designated lane regardless of urgency hint.

---

## 5. Policy Ownership Boundaries

| Component | Owns | Does not own |
|-----------|------|-------------|
| **Sending process (app)** | Requests event with `urgency_hint`, `category`, `privacy_level`, optional `action_caps` and `object_refs` | Final urgency, lane assignment, policy validation, rendering |
| **Bell** (future server) | Validates sender caps, assigns `final_urgency` + lane, manages ring buffer, enforces privacy redaction in markers | Focus policy, session state, workspace routing, pixel rendering |
| **silk-shell** | Provides `workspace_context`, `scene_context`, session lock state, focus state | Event storage, lane policy, pixel rendering |
| **SilkBar** (future) | Hosts compact presence indicator (badge, dim/bright/pulse) | Event model, policy, routing, inbox list |
| **sexdisplay** | Renders final pixels for SilkBar and inbox surface (0xEC/0xEF calls only) | Event model, policy, routing, Bell semantics |
| **Linen** (future) | May persist/project-link events after storage gate | Event creation, policy, routing |

### Key rule

```
apps request  →  Bell validates  →  silk-shell provides context  →  SilkBar/sexdisplay render
```

No component downstream of Bell owns policy. sexdisplay in particular is explicitly forbidden from reading, storing, or acting on BellEvent fields.

---

## 6. Event Lifecycle

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌───────────┐
│  Sender  │───▶│  Bell    │───▶│ Inbox    │───▶│ History   │
│ (app)    │    │ (policy) │    │ (ring)   │    │ (expired) │
└──────────┘    └──────────┘    └──────────┘    └───────────┘
                     │                │
                     ▼                ▼
               [quil.row.reject]  SilkBar presence
               if caps invalid    indicator
```

1. **Ingest:** Sender sends event PDX message. Bell validates `caller_pd` against capability table.
2. **Policy:** Bell applies lane derivation, checks trust_label, assigns `final_urgency`.
3. **Ring insert:** Event is written to Bell-local ring buffer (FIFO, bounded at 64 entries).
4. **SilkBar presence:** If event is visible (not PASSIVE with dim-only), SilkBar indicator updates.
5. **User interaction:** User opens inbox (adopts SILK_LIST_ROW_VISUAL_CANON), sees events, dismisses or acts.
6. **Expire/dismiss:** Event either expires via `expires_at_ticks` or is explicitly dismissed. Moves to history ring (circular, last 16 entries).

---

## 7. Privacy and Redaction

### Per-field redaction classes

| Field | Redaction Class | Loggable? |
|-------|----------------|-----------|
| `event_id` | StructuralMeta | ✅ Yes |
| `sender_pd` | StructuralMeta | ✅ Yes (kernel-authoritative) |
| `category` | StructuralMeta | ✅ Yes |
| `final_urgency` | StructuralMeta | ✅ Yes |
| `lane` | StructuralMeta | ✅ Yes |
| `workspace_context` | StructuralMeta | ✅ Yes |
| `expires_at_ticks` | StructuralMeta | ✅ Yes |
| `sender_identity_token` | SenderMeta | ✅ Yes (opaque, no PII) |
| `trust_label` | SenderMeta | ✅ Yes |
| `privacy_level` | StructuralMeta | ✅ Yes |
| `urgency_hint` | SenderMeta | ✅ Yes |
| `action_count` | StructuralMeta | ✅ Yes |
| `action_caps[]` | SecretContent | ❌ Never |
| `object_refs[]` | TitleMeta if resolved, otherwise SecretContent | ⚠️ Log only if structural IDs |
| `scene_context` | StructuralMeta | ✅ Yes |
| Event title/body text | SecretContent | ❌ **Never** |

### Proof marker rules

All proof markers generated by Bell must:
- Never log event title, body, or sender-provided display name.
- Never log resolved action capability details.
- Never log resolved object names (only structural IDs if at all).
- Use the `BellRedactionClass` to determine what may be serialized.

```
[bell.ingest] event_id={} sender_pd={} category={} ← StructuralMeta, OK
[bell.ingest.reject] sender_pd={} reason=cap_invalid ← StructuralMeta, OK
[bell.dismiss] event_id={} lane={} ← StructuralMeta, OK
[bell.expire] event_id={} reason=timeout ← StructuralMeta, OK
[bell.inbox.render] count={} ← StructuralMeta, OK
```

```
[bell.ingest] event_id={} title="Build complete" ← **FORBIDDEN** — SecretContent in marker
```

---

## 8. Integration with Silk List Row Canon

The future Bell inbox surface **must** adopt `SILK_LIST_ROW_VISUAL_CANON_V1`:

| Rect | Purpose | Bell Inbox |
|------|---------|------------|
| 0 | Header | Event list header ("Inbox", count, filter lane) |
| 1 | List background | Neutral dark slate (matching Linen/Quil/Command Palette) |
| 2 | Selected row highlight | Active with OOB guard (dismiss/select event) |
| 3-7 | Left accent bars | Event lane color (passive=dim, normal=standard, urgent=accent pulse, system=system color, security=security color) |

### Color mapping from lane to accent bar

| Lane | Accent Bar Color | Style |
|------|-----------------|-------|
| PASSIVE | `0x00404050` | Dim grey |
| NORMAL | `0x004080C0` | Standard blue |
| URGENT | `0x00C08040` | Amber accent |
| PERSISTENT | `0x00C04060` | Rose accent |
| SYSTEM | `0x006080C0` | Steel blue |
| SECURITY | `0x00C04040` | Red |

These are RECOMMENDED. The inbox must use the same rect_index allocation as all other Silk list surfaces. No Bell-specific row semantics may leak into sexdisplay.

### Forbidden

- Creating a Bell-specific rect_index allocation (must share canon).
- Adding Bell event fields to the 0xEF fill rect call.
- sexdisplay reading or inferring lane, urgency, or category from pixel data.

---

## 9. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Adding `OP_BELL_*` to sex-pdx. A protocol spec must be reviewed first.
2. Creating `servers/sexbell/`. A server stub design must be reviewed first.
3. Adding persistence. Storage integration requires a separate design gate.
4. Adding lockscreen rendering. Lockscreen policy requires a separate design gate.
5. Adding action callbacks. Capability dispatch requires a separate design gate.
6. Adding sound. Audio integration requires Harp/Theremin gate.
7. Adding app notification caps. App capability model requires a separate design gate.
8. Exposing private text in proof logs. Any proof marker containing event body/title is a PRIORITY-0 bug.
9. Allowing app-defined `final_urgency`. Only Bell policy may assign the final lane.
10. Letting sexdisplay own Bell policy. sexdisplay must remain renderer-only.

---

## 10. Future Phases (Recommended Order)

| Phase | Scope | Type | Depends On |
|-------|-------|------|------------|
| **BELL_CAPABILITY_POLICY_V1** | Define cap table for senders, lanes, actions | Docs | This gate |
| **BELL_PDX_PROTOCOL_SPEC_V1** | Define opcodes, wire format, reply codes | Docs | Cap policy |
| **BELL_SERVER_STUB_V1** | Minimal `sexbell` server, ring buffer, ingest dispatch | Implementation | Protocol spec |
| **BELL_SILKBAR_PRESENCE_V1** | Compact indicator in SilkBar (badge, dim/bright/pulse) | Implementation | Server stub |
| **BELL_INBOX_ROWS_V1** | Full inbox surface adopting SILK_LIST_ROW_VISUAL_CANON | Implementation | Server stub + canon |
| **BELL_ACTION_CAPS_V1** | Dispatch actions from inbox (dismiss, open, etc.) | Implementation | Inbox rows |
| **BELL_LINEN_OBJECT_LINK_V1** | Project-link events to Linen objects | Implementation | Action caps + storage gate |
| **BELL_PERSISTENT_STORAGE_V1** | Persist events across boot via sexstore | Implementation | E-series storage maturity |

Each phase must pass its own STOP FIRST review before proceeding.

---

## References

- `SILK_LIST_ROW_VISUAL_CANON_V1.md` — canon that Bell inbox must adopt
- `SILK_DE_GLASS_VISUAL_LANGUAGE.md` — SilkBar/global bar visual direction (Bell indicator target)
- `E15_STORAGE_DOCS_CLEANUP_V1.md` — storage canon (sexstore vs sexshop, relevant for persistence)
- `QUIL_COMMAND_PALETTE_ROW_VISUALS_V1.md` — first canonical list implementation (reference for inbox)
- `LINEN_LIST_ROW_VISUAL_MIGRATION_V1.md` — Linen row migration (reference for inbox)
- `QUIL_LIST_ROW_VISUAL_MIGRATION_V1.md` — Quil row migration (reference for inbox)

---

*End of BELL_EVENT_MODEL_DESIGN_GATE_V1.md*
