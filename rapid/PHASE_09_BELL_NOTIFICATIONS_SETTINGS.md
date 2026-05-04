# PHASE 09: Bell — The Attention Firewall

## Revolutionary Vision

Bell is not a notification daemon.
Bell is not a popup spam system.
Bell is not D-Bus notifications for SexOS.

**Bell is the user's attention firewall for a capability-native computer.**

Every other OS treats notifications as a UI problem: an app produces a message, the system displays a popup, and the user manages the chaos manually. The best systems (Apple, Android, KDE) have learned to filter, batch, and prioritize — but they all work with the same fundamental model: **apps declare urgency, the system believes them.**

SexOS is different. In a capability-native OS, urgency is not a suggestion. It is a **right** that must be granted.

### Why Existing Systems Fall Short

| System | Weakness |
|--------|----------|
| **Apple** | Beautiful UX but no architectural control. Apps can still spam with user permission. Focus modes are reactive, not proactive. |
| **Android** | Granular channel control but no sender identity verification. Notification listeners can read everything. Per-app importance is user-managed. |
| **KDE** | Rich history and desktop integration but no capability model. Any app can send any notification class. DND is binary. |
| **GNOME** | Minimalist but no control beyond app-level toggle. No urgency classes, no history. |
| **mako/dunst** | Fast and scriptable but zero trust model. They trust whatever D-Bus sends them. |
| **COSMIC** | Rust-native but still conventional: app → notification daemon → layer-shell popup. No capability gating. |

### What Bell Does Differently

**Capability-scoped urgency.** An app does not get to scream "URGENT" just because it wants attention. It needs the right notification capability class. Bell enforces this at the protocol level — not heuristics, not training, not AI. The architecture itself prevents urgency abuse.

```
Capability classes for notifications:
  Bell::Passive    → banner only, no sound, no wake
  Bell::Normal     → banner + sound (default)
  Bell::TimeSensitive → banner + sound + wake (needs grant)
  Bell::Urgent     → banner + sound + wake + persistent (needs grant)
  Bell::Persistent  → stays until dismissed (needs grant)
  Bell::LockScreen  → visible when locked (needs grant)
  Bell::Sound       → allows sound (separate from urgency)
  Bell::Action      → allows action buttons (separate from urgency)
```

An app that only holds `Bell::Normal` can suggest Urgent all it wants — Bell silently downgrades it. The sender never knows. The user never sees spam.

**Verified sender identity.** The sender PD is authenticated by the PDX slot. Apps cannot spoof:
- System prompts (Bell checks `sender_pd != sexdisplay`)
- Security prompts (Bell checks `sender_pd != Collar`)
- Other app notifications (Bell checks sender identity against registered name)
- Phishing attacks ("Your system is infected" from an app = blocked, logged, reported to Mesh)

**Action capabilities.** A notification button is not an arbitrary app callback. It is a scoped, single-use capability grant:

```
Notification: "Build failed"
  Action: "Open log" → Capability: { read: "/tmp/build.log", once: true }
  Action: "Rebuild"  → Capability: { execute: "entrypoint_build.sh", once: true }
```

The app cannot use the "Open log" grant to open anything else. It cannot use the "Rebuild" grant more than once. Action capabilities are consumed on use.

**Attention budget.** Bell enforces per-app, per-category, per-hour limits:
- Passive: unlimited
- Normal: 10/hour per app
- TimeSensitive: 5/hour per app  
- Urgent: 2/hour per app
- Exceeded → auto-batched as "App sent 15 notifications" summary at end of hour

This is not a suggestion. Bell **drops** notifications that exceed the budget. The app gets a `BELL_BUDGET_EXCEEDED` response and can choose to batch or wait.

**Notification lanes — not one dumb stack:**

```
LANE      POLICY                    VISIBILITY
──────────────────────────────────────────────────────
NOW       urgent, current           Shows immediately, interrupts
SOON      time-sensitive            Banner, no interrupt  
LATER     inbox, history            Silent, accumulates
SYSTEM    power, network, security   Always through (unless DND)
PROJECT   Linen objects, builds     Workspace-scoped
DEV       PD faults, PDX errors     Never interrupts, badge only
```

Each lane has its own attention policy, rendering, and history. DEV notifications never interrupt focus. PROJECT notifications follow the current workspace.

**Notifications as Linen objects.** A notification is not a transient UI element. It is an `Object` in Linen:
- Can be saved to a project
- Pinned to a workspace
- Linked to a build, trace, or prompt
- Attached to a task or PR
- Replayed in time-travel via Mesh

This makes Bell's history not a separate "notification log" — it falls out of Linen's object graph naturally.

**Time-travel audit.** Bell stores every notification event with:
- What happened
- Who sent it
- What urgency was suggested vs granted
- What action was taken
- Which capabilities were used
- What was denied or auto-muted

Combined with Mesh's temporal graph, the user can ask: "Show me all notifications from App X yesterday" and get a complete timeline with capability provenance.

**Dev mode — uniquely SexOS.** Bell's DEV lane surfaces operating system events that no other notification system shows:
- PD crashed → link to fault log (+ Mesh graph showing dead node)
- PDX route denied → link to Collar (+ grant request prompt)
- Build finished with warnings → link to Quil (+ open file at warning line)
- ABI mismatch detected → link to handoff (+ STOP-FIRST warning)
- USB device connected/disconnected → link to sexusb state
- Framebuffer ownership violation → link to sexdisplay audit

This makes Bell the diagnostic dashboard for the OS developer, not just an app notification center.

### The Bell/Mesh/Collar Triangle

This is where Bell transcends "notification system" entirely:

```
Mesh = The living graph
  Shows: PDs, routes, devices, surfaces, edges
  Knows: who exists, who calls whom, what's normal

Collar = The authority wallet
  Shows: grants, capabilities, trust, revocation
  Knows: who can do what, who granted it, when it expires

Bell = The attention firewall
  Shows: events, breaches, downgrades, denials
  Knows: who tried to send what, what was blocked, what was downgraded
```

When an app misbehaves:

```
App tries to send URGENT notification
  → Bell checks capability: missing Bell::Urgent
  → Bell downgrades to Normal
  → Bell records: "App attempted Urgent, downgraded to Normal (missing cap)"
  → Mesh creates edge: App──attempted:Urgent→Bell──granted:Normal
  → Collar notes: app only holds Bell::Normal, no escalation available
  → User sees normal notification, never knows app tried to escalate
  → Dev sees in DEV lane: "App X attempted URGENT without capability"
```

That is not Apple. That is not Android. That is a **capability-native OS explaining itself and protecting the user's attention at the architectural level.**

---

## What Already Exists
- SilkBar has module slots for chips (clock, battery, network, bell — ModuleSlot enum)
- sexdisplay renders SilkBar chips with fixed positions
- silk-shell has panel toggle infrastructure (Launcher, Status, Clock, Bell panels)
- Notification surface concept doesn't exist
- Settings data (appearance tokens, input config) is in-memory but not editable via UI
- F4 toggles top bar, F5 cycles appearance presets — but no settings panel to change these
- PDX protocol infrastructure exists (OP_* constants, PDX dispatch)
- Ring buffer pattern exists in Mesh's temporal graph design (4096 events)

## Ownership
- **Bell** (server/exclusive): notification PD, event routing, priority/trust model, policy enforcement
- **SilkBar** (integration): Bell chip indicator, unread count, lane indicators
- **sexdisplay** (rendering): notification toast rendering (flat ARGB only — no alpha/blur)
- **Quil** (consumer): settings panels, notification history viewer, action dispatch
- **silk-shell** (integration): Bell surface lifecycle, notification dispatch, focus-aware context
- **Linen** (consumer): notification persistence as objects (V2)
- **Collar** (consumer): notification capability grants (V2)
- **Mesh** (consumer): notification route visualization (V2)

## Bundle: Bell Protocol

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| **BellEvent V1 struct** | Fixed-size: id, sender_pd, urgency, category, title[32], body[128], action_count, expires_at | 2h | HIGH |
| **BellAction V1 struct** | Fixed-size: label[16], capability_slot, once_flag | 2h | HIGH |
| **Urgency/Category/Privacy enums** | BellUrgency { Passive, Normal, TimeSensitive, Urgent, Persistent, LockScreen }, BellCategory { App, System, Security, Dev, Project }, BellPrivacy { Public, Sensitive, Private } | 1h | HIGH |
| **OP_BELL_NOTIFY handler** | Validate sender PD, check caps, enforce urgency budget, store in ring buffer, forward to SilkBar | 4h | HIGH |
| **OP_BELL_CLOSE handler** | Dismiss by id, notify sender if action callback pending | 2h | High |
| **OP_BELL_LIST handler** | Return ring buffer slice by lane, max 16 per query | 2h | High |
| **OP_BELL_CLEAR handler** | Clear all/LANE/all-by-sender | 1h | Medium |
| **OP_BELL_ACTION handler** | Execute action: grant capability, consume token, report result | 3h | High |
| **OP_BELL_SET_POLICY handler** | Per-app: max_per_hour, allowed_lanes, dnd_override | 2h | Medium |
| **OP_BELL_MUTE_SENDER handler** | Mute by PD identity + duration | 1h | Medium |
| **Ring buffer storage** | `[BellEvent; 64]` fixed-size, oldest dropped when full | 1h | HIGH |
| **Policy storage** | `[BellPolicy; 8]` fixed-size, keyed by PD identity | 1h | Medium |

## Bundle: UX Integration

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| **SilkBar bell chip** | Bell icon, unread count, lane indicators (colored dots) | 3h | HIGH |
| **Click → panel toggle** | Click chip → create Bell panel surface → show notification list | 3h | High |
| **Notification toast** | Flat ARGB colored bar (lane-colored) at top of screen, no alpha/blur | 4h | Medium |
| **Action button rendering** | Text label(s) on toast, click → OP_BELL_ACTION | 3h | Medium |
| **DEV lane integration** | PD crash, PDX deny, build finish, ABI mismatch events → DEV lane | 4h | High |
| **Notification lanes UI** | Tab/filter UI: NOW, SOON, LATER, SYSTEM, PROJECT, DEV | 3h | Medium |

## Bundle: Settings (unchanged from previous — display, input, network, security, apps)

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| Display settings panel | Top bar toggle, appearance presets, rim thickness | 4h | Medium |
| Input settings panel | Mouse speed, keyboard repeat rate (model only) | 2h | Low |
| Network settings panel | Link status display, IP info (read-only) | 2h | Low |
| Security panel | App permissions, granted capabilities view | 3h | Low (after Collar) |
| Apps panel | Installed apps list, launch, remove | 2h | Low (after sexshop) |

## Smallest First Step
Bell V1 prototype: A PDX server that accepts `OP_BELL_NOTIFY`, validates the sender is a known app PD, stores the event in a fixed ring buffer, and forwards a "new notification" signal to SilkBar (which shows an unread count). No toasts, no actions, no lanes, no policy — just "message received → chip lights up." This proves the entire notification pipeline in one 4-hour session.

Second step: Add urgency downgrade. If sender doesn't hold Bell::Urgent capability, downgrade to Normal. Log the attempt. This proves the capability-scoped urgency model.

## Dependencies
- **Bell blocks on**: sexdisplay toast rendering (needs new surface or overlay), SilkBar chip slots (already exist)
- **Bell does NOT block on**: Linen (history as Linen objects is V2), Quil (history panel can be simple for V1), Mesh/Collar (visualization is V2)
- **Settings blocks on**: Phase 6 (Collar for security panel), Phase 7 (app list for apps panel)
- **Can parallelize with**: Phase 04 (Linen), Phase 05 (Quil), Phase 07 (App Launch)

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Notification toast rendering needs alpha/blur (looks bad without glass) | High | Medium | Flat ARGB colored bar per lane. Solid background color. No transparency. Document as V1 limitation. |
| Bell becomes popup spam despite capability gating | Medium | High | Attention budget (max N/hour per app per lane). Auto-batch exceeding apps. User never sees spam — Bell drops it. |
| Sender identity spoofing via PDX | Low | High | Impossible at PDX level — PD identity is bound by kernel slot. Bell logs `sender_pd` and checks against registered name. PHY_ATTACK style spoofing doesn't exist in microkernel. |
| Action capabilities are complex | Medium | Medium | V1: actions are simple slots — no capability grant. V2: add capability-backed actions. |
| DEV lane floods during development | Medium | Low | DEV lane never interrupts. Accumulates silently. Per-session clear. User must open DEV lane to see. |
| Settings panels multiply | High | HIGH | Ship only 3 panels in V1: Display, Security, Apps. Input/Network read-only. Defer full settings to later phase. |

## Exit Criteria (Done Checklist)

**Phase 09A (Bell):**
- [ ] Bell server boots, listens on PDX slot, accepts OP_BELL_NOTIFY
- [ ] Sender PD validated — unknown senders rejected
- [ ] Capability-scoped urgency enforced: app without Bell::Urgent gets downgraded
- [ ] Ring buffer stores last 64 events, oldest dropped when full
- [ ] SilkBar bell chip shows unread count
- [ ] Click bell chip opens notification panel (surface create/focus)
- [ ] Notification lanes: NOW, SOON, LATER, SYSTEM, PROJECT, DEV — each with own rendering
- [ ] DEV lane captures: PD crash, PDX deny, build finish, ABI mismatch
- [ ] Attention budget enforced: per-app per-hour limits
- [ ] Notification toast appears as flat ARGB bar (lane-colored, no alpha/blur)
- [ ] Action buttons render on toast — click dispatches OP_BELL_ACTION
- [ ] OP_BELL_CLOSE, OP_BELL_LIST, OP_BELL_CLEAR, OP_BELL_SET_POLICY, OP_BELL_MUTE_SENDER all functional

**Phase 09B (Settings):**
- [ ] Display settings panel: top bar toggle, preset selector, rim thickness slider
- [ ] Security panel: app permissions list, grant/revoke capability (via Collar from Phase 6)
- [ ] Apps panel: installed apps list, click to launch
- [ ] All panels are Quil surfaces (standard create/focus/destroy lifecycle)
- [ ] Settings changes persist only in memory (no sexstore yet)
- [ ] Build passes. Boot passes. No panic.

## Testing Strategy
- **Bell protocol**: Send OP_BELL_NOTIFY → verify event stored → verify SilkBar chip updates → verify OP_BELL_LIST returns event → verify OP_BELL_CLOSE removes it
- **Urgency enforcement**: Send with Urgent from app that only holds Normal → verify downgraded to Normal → verify DEV lane shows downgrade log
- **Attention budget**: Send 11 Normal notifications in one tick → verify 10th accepted, 11th dropped → verify app receives BUDGET_EXCEEDED
- **Bell panel**: Click chip → verify surface created → verify list shows events → verify lanes filter correctly
- **Settings**: Open display panel → toggle top bar → verify F4 behavior changes → change preset → verify tokens update
- **Integration**: App launches → sends notification → Bell validates → SilkBar updates → click chip → panel shows it
- **Regression**: All existing markers fire

## Efficiency Opportunity
**Bell V1 is a PDX server with a ring buffer — AI can write 90% of it in one session.** The protocol, structs, storage, and dispatch are pure pattern. The only novel code is the urgency downgrade logic (a single match statement checking sender capabilities against declared urgency). This makes Bell the fastest path to a visible UX improvement.

**DEV lane can be populated before any app notifications exist.** Hardcode events for: PD boot ("sexdisplay online"), PD crash (simulated), build complete (from last build). This proves the DEV lane value before any app integration.

## Completeness Gain
Desktop/user utility: **+10–15%** overall (Bell makes it feel alive, Settings makes it feel controllable)

## Files Changed
- `servers/bell/src/main.rs` (new PDX server — notification routing, policy, ring buffer, lanes)
- `servers/silk-shell/src/main.rs` (Bell surface lifecycle, notification dispatch, DEV lane events)
- `servers/sexdisplay/src/main.rs` (toast rendering — flat ARGB bar per lane, action buttons)
- `servers/quil/src/main.rs` (settings panels: display, security, apps)
- `crates/sex-pdx/src/lib.rs` (OP_BELL_NOTIFY, OP_BELL_CLOSE, OP_BELL_ACTION, OP_BELL_LIST, OP_BELL_CLEAR, OP_BELL_SET_POLICY, OP_BELL_MUTE_SENDER opcodes)

## Forbidden
- Popup spam model (capability-scoped only — urgency is granted, not declared)
- Renderer notification compositing without safety plan
- Alpha/blur in toast rendering (flat ARGB only)
- Full settings app (panels in Quil only — no separate settings server)
- Persistence/storage for settings (in-memory only in V1)
- Sender spoofing (sender identity is PDX-bound, not app-declared)
- Action callbacks without capability scoping

## Next Phase
PHASE_10_COMPATIBILITY_APPS_BUNDLE.md

## Ordering Note
Bell can be moved earlier in the execution order. It depends on Phase 01 (display rendering) and Phase 02 (shell surface lifecycle), but does **not** depend on Linen, Quil, Mesh, or Collar for V1. A reasonable ordering is:

```
1. 01 Display (finish existing contract + rendering)
2. 02 Shell (finish surface lifecycle, focus, frames)
3. 09 Bell (attention firewall — fast PDX server, visible UX)
4. 04 Linen (object layer)
5. 05 Quil (language workstation)
6. 06 Mesh+Collar (living graph + authority)
```
