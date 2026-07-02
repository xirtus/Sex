# G1: Bell Event Contract — Spec

**Status:** Approved (Docs/Model only)
**Commit:** *(pending)*
**Build:** N/A (no code changes)

## 1. Purpose

Bell is SexOS's **attention firewall / event router**. It receives events over
PDX from PDs and apps, validates sender authority, determines urgency and lane,
and surfaces user-visible attention state through the shell (Silk/SilkBar
later). Bell is **capability-scoped** — every event is checked against Collar
policy before it reaches the user.

### Core claims

- **Bell is not just "notifications."** It is a full event routing and attention
  management layer. Events carry urgency, lane, category, privacy class, and
  optional action tokens.
- **Bell does not grant authority.** Collar (F2) owns all grant decisions. Bell
  asks Collar whether an event's action token should be approved or denied.
- **Bell is not a renderer.** Bell produces attention state (event count, lane
  occupancy, urgency). silk-shell and SilkBar decide how to display attention
  state. sexdisplay renders final pixels.
- **Bell is not app UI chrome.** Apps cannot inject spoofable chrome or
  fake prompts through Bell. Prompt surfaces are rendered by silk-shell under
  shell authority.
- **Bell is not a logging dump.** Events have a lifecycle (Proposed → Accepted →
  Displayed → Dismissed/Archived). Stale events are coalesced, rate-limited, or
  suppressed — not accumulated indefinitely.

## 2. Event Object Fields

Every Bell event is a fixed-size record with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | u64 | Monotonic event identifier |
| `sender_pd` | u8 | PD slot number of the sender |
| `sender_identity` | u64 | App/PD identity hash (from launch manifest) |
| `category` | enum | Event category (see §3) |
| `urgency` | enum | `Low`, `Normal`, `High`, `Critical` |
| `lane` | enum | `Now`, `Soon`, `Later`, `System`, `Project` |
| `title` | `[u8; 64]` | Fixed-size title string (UTF-8, zero-padded) |
| `body` | `[u8; 256]` | Fixed-size body string (UTF-8, zero-padded) |
| `target_scene` | u8 | Scene index the event relates to, or `0xFF` |
| `target_frame` | u32 | Frame ID the event relates to, or `0` |
| `target_tab` | u8 | Tab index the event relates to, or `0xFF` |
| `target_object` | u64 | Object/file ID the event relates to, or `0` |
| `privacy_class` | enum | `Public`, `Internal`, `Confidential`, `Secret` |
| `expiration` | u64 | Timestamp after which the event auto-expires |
| `action_token_id` | u64 | Optional action token reference, or `0` |
| `action_token_scope` | u64 | Bitmask of allowed operations for the action token |
| `proof_marker` | `[u8; 16]` | Audit proof marker for Collar policy check |
| `lifecycle_state` | enum | Current lifecycle state of this event (see §4) |

**Total fixed size:** approximately 512 bytes per event record, suitable for a
static ring buffer.

## 3. Event Categories

| Category | Description | Example |
|----------|-------------|---------|
| `System` | OS-level event | Low memory, PD fault, device hotplug |
| `App` | Application event | Task complete, progress reached |
| `Build` | Build/compile result | Build success, build failure, test result |
| `Fault` | Crash or error | PD panic, capability denied, I/O error |
| `Security` | Security event | Grant requested, auth failure, policy change |
| `FileObject` | File/object event | File ready, sync complete, share request |
| `Network` | Network event | Connection state, transfer complete |
| `Device` | Device event | Device attached, device removed, battery low |
| `Project` | Project event | Project update, collaborator action |
| `Message` | (Future) Person-to-person message | Chat, comment, review |

## 4. Event Lifecycle

```
                    ┌───────────┐
                    │ Proposed  │
                    └─────┬─────┘
                          │
                    ┌─────▼──────┐
                    │  Accepted  │ ← Collar policy check passes
                    └─────┬──────┘
                          │
              ┌───────────┼───────────┐
              │           │           │
         ┌────▼────┐ ┌───▼────┐ ┌───▼──────┐
         │Displayed│ │Suppress│ │ Expired  │
         └──┬──────┘ │  ed    │ └──────────┘
            │        └────────┘   (terminal)
            │
       ┌────▼────────────┐
       │ ActionRequested  │ ← user interacts with event
       └────┬────────────┘
            │
      ┌─────┼──────┐
      │     │      │
  ┌───▼──┐ ┌▼────┐ │
  │Action │ │Action│ │
  │Appro- │ │Denied│ │
  │ ved   │ └─────┘ │
  └───┬──┘          │
      │        ┌────▼────┐
      │        │Dismissed│
      │        └─────────┘
      │
 ┌────▼─────┐
 │  Action  │ ← action token consumed
 │ Complete │
 └────┬─────┘
      │
 ┌────▼───────┐
 │  Archived   │ ← terminal
 └────────────┘
```

**Transitions:**
| From | To | Trigger |
|------|----|---------|
| Proposed | Accepted | Collar policy check passes (or no policy required) |
| Proposed | Suppressed | Collar policy denies visibility |
| Proposed | Expired | TTL reached before acceptance |
| Accepted | Displayed | Bell routes to shell for user visibility |
| Accepted | Suppressed | Collar policy changes or coalescence |
| Accepted | Expired | TTL reached before display |
| Displayed | ActionRequested | User taps/clicks/activates the event |
| Displayed | Dismissed | User dismisses without action |
| Displayed | Expired | TTL reached while displayed |
| ActionRequested | ActionApproved | Collar approves the action token |
| ActionRequested | ActionDenied | Collar denies the action token |
| ActionRequested | Dismissed | User cancels the action request |
| ActionApproved | ActionComplete | Action token consumed successfully |
| ActionComplete | Archived | Audit record finalized |
| ActionDenied | Archived | Audit record finalized |
| Dismissed | Archived | Audit record finalized |
| Suppressed | Archived | Audit record finalized |
| Expired | Archived | Audit record finalized |

## 5. Policy Rules

1. **Sender must be identifiable.** Every event carries a sender PD slot and
   identity hash. Anonymous events are rejected at the Proposed stage.

2. **Event class must be allowed by Collar policy.** Before an event moves from
   Proposed to Accepted, Collar checks whether the sender PD is authorized to
   send events of that category. Non-authorized events are Suppressed.

3. **Action tokens must be one-shot and minimum-scope.** If an event carries an
   action token, the token must be single-use and scoped to the minimum set of
   operations required. Collar enforces this at approval time.

4. **Private events must redact in public contexts.** Events with privacy class
   `Confidential` or `Secret` must have their title/body redacted when displayed
   in Atlas overview, SilkBar, or lock screen contexts. The redacted form shows
   category and urgency only.

5. **Urgent events cannot bypass authority.** Even `Critical` urgency events
   must pass Collar policy checks. Urgency is a display hint, not an authority
   bypass.

6. **Repeated events must be coalesced and rate-limited.** If the same sender
   sends the same category + title within a coalescence window, the events are
   merged into a single event with an incremented count. Senders that exceed a
   rate limit have their events Suppressed for a cooldown period.

7. **Bell never gives apps focus by itself.** An event may request attention,
   but Bell does not call `try_set_focus()`. The shell decides whether to
   surface the event, switch context, or request user action.

## 6. Relationship to Collar

| Aspect | Bell | Collar |
|--------|------|--------|
| Role | Route events, manage attention | Govern authority |
| Authority | Cannot grant/deny action tokens | Approves/denies all action tokens |
| Policy check | Asks Collar before accepting events | Returns allow/suppress/deny |
| Action tokens | Carries token refs, does not validate | Validates scope, approves consumption |
| Audit | Records event lifecycle | Records grant/deny decisions |

**Rule:** Bell asks, Collar governs. Bell cannot self-grant action tokens or
bypass Collar policy.

## 7. Relationship to Mesh

| Aspect | Mesh | Bell |
|--------|------|------|
| Role | Visualize topology | Route events |
| Reads Bell state? | May read event ring for graph later | N/A |
| Reads Collar state? | May read audit ring for graph later | Reads via policy check |
| Can deliver events? | No — diagnostic only | Yes — event router |
| Can approve actions? | No — diagnostic only | No — asks Collar |

**Rule:** Mesh can visualize Bell event routes and denied/suppressed events
later. Mesh cannot deliver or approve events.

## 8. Relationship to Silk/SilkBar

| Aspect | Silk/SilkBar | sexdisplay |
|--------|-------------|------------|
| Role | Shell UI policy, focus, chrome | Render pixels |
| Bell integration | Show event count/lane/urgency in bar | Render bar pixels only |
| Focus decision | Shell decides if event triggers focus | N/A — no policy |
| Event display | silk-shell renders event surfaces | Renders surface pixels |

**Rules:**
- SilkBar may show Bell presence/count/lane later, but never decides authority.
- silk-shell decides whether an event triggers focus, scene switch, or prompt.
- sexdisplay only renders final pixels — no event logic, no policy.

## 9. Future Implementation Plan

### G1 (this document)
- ✅ Docs/contract definition
- No implementation

### G2 — Bell Placeholder Surface
- Bell surface through proven Scene/Frame/Tab path (mirror D1/E1)
- Toggle via key binding (e.g., Bell icon in SilkBar → F-key)
- Placeholder fill rect only — no real event display
- **Requires STOP FIRST review**

### G3 — Bell Event Ring Model (docs)
- Define fixed-size event ring buffer (similar to A6 tombstone ring)
- Event coalescence and rate-limiting algorithm
- Lifecycle state machine implementation design
- Docs only — no code

### G4 — Bell PDX Opcode Proposal (docs)
- Define opcodes: `BELL_EVENT_SEND`, `BELL_EVENT_ACK`, `BELL_EVENT_DISMISS`,
  `BELL_EVENT_ACTION_APPROVE`, `BELL_EVENT_ACTION_DENY`
- Define sender identity verification flow
- Docs only — **requires STOP FIRST review before any ABI changes**

### G5 — Implementation
- Only after Collar, Bell, and Mesh contracts agree
- Event ring implementation
- PDX opcode addition
- Bell PD server or silk-shell integration
- Collar policy check integration
- **Multiple STOP FIRST reviews required**

## 10. STOP FIRST Triggers

Stop all Bell work and escalate if any of the following are required:

- **Kernel edits** — Bell is userspace only
- **`crates/sex-pdx/` ABI/opcode edits** — Requires G4 contract + STOP review
- **New PDX ops** — Requires G4 contract + STOP review
- **Authority enforcement** — Collar owns policy, not Bell
- **Action token implementation** — Requires Collar contract agreement
- **Secret/key handling** — Collar owns secrets, not Bell
- **Renderer-owned notification policy** — sexdisplay never decides visibility
- **App-controlled spoofable prompts/chrome** — All prompts under shell authority
- **Shared-memory/backing-buffer redesign** — Uses existing PDX/display path
- **Cross-PD raw pointers** — Never stored or transmitted
