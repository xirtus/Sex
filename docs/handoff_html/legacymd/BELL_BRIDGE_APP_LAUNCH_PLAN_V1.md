# BELL_BRIDGE_APP_LAUNCH_PLAN_V1

**Status:** PLAN ONLY — no implementation, no protocol changes, no source edits.
**Date:** 2026-05-16
**Type:** Future-phase design document (docs-only).
**Depends on:** SLOT_SHELL launch (proven), Bell V1 (proven), Frame Lights stub (proven).

---

## 0. Pass/Fail

**PASS** — PLAN DOCUMENT. 0 gates executed, 0 faults. This is a future
design/roadmap document only. No code was changed, no protocol was changed,
no launch route was changed.

---

## 1. Bridge Role Summary

Bell Bridge is a **capability-scoped attention/event mediation layer** that
augments — but never replaces — the proven SLOT_SHELL launch transport.

### What Bell Bridge IS

| Trait | Description |
|-------|-------------|
| **Attention mediator** | Receives launch outcome events from the shell and publishes them as structured Bell notifications that other components (SilkBar, Spindle, future Mesh) can observe |
| **Event stream owner** | Owns the notification timeline for app-launch lifecycle: enqueue, dismiss, clear, list, mute |
| **Policy boundary** | Future home for per-app attention policy — e.g. which apps may surface launch-failure prompts, which may ring the SilkBar Bell dot, which are silent |
| **Deferred prompt authority (future)** | May surface user-visible deferred-launch prompts ("App X failed: retry?") through the Bell notification ring — but **never** decides whether to launch |
| **Background workflow annotator** | May annotate background workflow events (restore, sync, compile) that originate from app launch or session lifecycle |

### What Bell Bridge IS NOT

| Trait | Why Not |
|-------|---------|
| **Launch transport** | SLOT_SHELL already delivers launch requests with 0 faults (84/84). Bell has no surface focus, no renderer, no PID tracking beyond PD caps, and no task lifecycle. |
| **Focus authority** | Only silk-shell owns focus. Bell cannot open, close, minimize, zoom, or tile surfaces. |
| **Renderer policy** | Only sexdisplay renders pixels. Bell has no GPU, no surface registration, no framebuffer access. |
| **Browser/networking** | WebStub is network=0 engine=0. Bell has no HTTP, no fetch, no socket capability. |
| **Close/minimize/zoom action authority** | Red close is disabled across all frames (FRAME_LIGHTS_STATUS_STUB_V1). Bell must never wire into frame-light actions. |

---

## 2. When Bell Bridge SHOULD Be Used

These are the approved Bell Bridge responsibilities — **future-phase** and
**docs-only** unless an explicit implementation phase gate is passed.

| # | Scenario | Bell Role | Phase |
|---|----------|-----------|-------|
| S1 | App launch result notifications | Shell publishes `[bell.bridge.launch.result]` after SLOT_SHELL launch completes — success or failure. Bell enqueues as WARN/INFO notification visible in SilkBar Bell dot. | Phase 2–3 |
| S2 | Failed launch reason events | Shell captures failure reason (missing app, PD spawn error, cap denied) and sends structured event to Bell. User sees notification with reason and possible retry hint. | Phase 2–3 |
| S3 | User-visible deferred launch prompts | Bell queue holds a NOTICE-lane event with `action_id` pointing to a retry callback. SilkBar or future notification center renders the prompt. User action dispatches back through shell. | Phase 4 (attention policy) |
| S4 | Background workflow events | Spindle or shell sends INFO-lane events for session restore progress, document sync, background compile. Bell holds these passively; SilkBar may suppress the dot for passive lanes. | Phase 2+ |
| S5 | Future approval/attention policy | Collar (future) sends approve/deny decisions to Bell. Bell annotates pending events with attention-policy metadata. Shell queries Bell before surfacing sensitive launch prompts. | Phase 5 |
| S6 | Event/authority graph visualization | Mesh (future) queries Bell LIST for aggregate lane counts and event topology. Mesh renders graph — Bell is the source of truth for event presence but **not** the authority graph itself. | Phase 5 |

---

## 3. When Bell Bridge MUST NOT Be Used

These boundaries are **hard** and must never be crossed without a separate
STOP FIRST audit that explicitly re-evaluates each boundary.

| # | Forbidden Use | Why / What to Use Instead |
|---|---------------|---------------------------|
| F1 | **Direct app launch execution** | SLOT_SHELL (OP_SHELL_LAUNCH_REQUEST=0x15) is the proven transport. Bell has no task lifecycle, no PID tracking, no surface registration. Launch must always flow through silk-shell. |
| F2 | **Shell focus authority** | Only silk-shell owns `FOCUSED_SURFACE`, `focus_surface()`, `clear_focus_if_dead()`, and fallback focus. Bell must never set or clear focus. |
| F3 | **Renderer/pixel policy** | Only sexdisplay renders. Bell has no surface_id, no framebuffer, no GPU. Bell's only visual output is the SilkBar Bell dot — rendered by sexdisplay, not Bell. |
| F4 | **Browser networking capability increase** | WebStub is network=0, engine=0. Bell must never grant network capability or open sockets. No browser engine. |
| F5 | **Close/minimize/zoom action authority** | Red close is disabled across all frames (close_allowed=0, close_impl=0). Yellow/green are available but wire through shell's existing FSM, not Bell. Bell must never dispatch frame-light actions. |
| F6 | **New Bell ABI opcode without STOP FIRST** | Any new OP_BELL_* opcode requires a separate STOP FIRST audit (kernel/ABI boundary). This plan defines no new opcodes. |
| F7 | **Kernel/sex-pdx global ABI change** | No new syscalls, no capability slot renumbering, no PDX wire format changes. Bell Bridge must work within existing Bell V1 opcodes (0xC0–0xC7) with no new ones unless separately audited. |
| F8 | **Launch authority moving out of shell** | Spindle sends launch → silk-shell receives and executes → shell owns the launch decision. Bell observes outcomes; never decides. |
| F9 | **Bell directly focusing apps** | Bell has no surface concept. Apps cannot be "focused by Bell." Shell's `open_app_in_active_scene()` is the only focus path for launched apps. |

---

## 4. Ownership Table

| Component | Role in Launch | Role in Bell Bridge | Capability Grant |
|-----------|---------------|---------------------|-----------------|
| **Spindle** | Sends launch request via `pdx_call(SLOT_SHELL, 0x15, app_id, 0, 0)` | May send Bell events for background workflows; receives Bell bridge commands (`bell`, `events`) | Needs SLOT_BELL → sexbell (currently pending) |
| **silk-shell** | Owns launch/focus policy. Receives OP_SHELL_LAUNCH_REQUEST, calls `open_app_in_active_scene()`. | **Publishes launch outcome events to Bell** (Phase 3). Shell is the authoritative source of launch truth. | Already has SLOT_BELL (granted) |
| **sexbell** | None — does not participate in launch. | **Records/announces outcomes**. Receives OP_BELL_NOTIFY from shell, enqueues, exposes via OP_BELL_LIST to SilkBar. Owns the notification ring. | Self-cap only (SLOT_BELL=12) |
| **SilkBar** | None — observes, does not launch. | **Shows Bell presence** (dot + count badge). Polls Bell every ~2s via OP_BELL_LIST. May show deferred launch prompts (Phase 4). | Already has SLOT_BELL (granted) |
| **sexdisplay** | Renders final pixels only. | Renders SilkBar Bell dot + badge via `SetBellPresence` update. No direct Bell contact. | SLOT_DISPLAY only |
| **Collar** | None — deferred authority. | **May approve sensitive grants later** (Phase 5). Sends approve/deny to Bell for attention-policy annotation. | Future (no Collar spawn yet) |
| **Mesh** | None — deferred visualization. | **May visualize event/authority graph** (Phase 5). Queries Bell LIST for aggregate data; renders topology. | Future (no Mesh spawn yet) |

---

## 5. Phase Ladder

### Phase 0 — THIS DOCUMENT (2026-05-16)
- Docs-only plan.
- No source edits, no scripts edits, no kernel/pdx/ABI edits.
- No Bell protocol changes, no launch route changes.
- Handoff: `docs/handoff/BELL_BRIDGE_APP_LAUNCH_PLAN_V1.md`

### Phase 1 — Bell Status Stub
- Add `bell-bridge` spindle command: prints Bell bridge status stub.
- Marker: `[bell.bridge.status]` — role, phase, ownership summary.
- No PDX calls. No Bell contact. Marker-only, like Atlas Scene status stub.
- Gates: ~85, all marker assertion gates.

### Phase 2 — Bell Launch Outcome Event Markers
- Shell emits `[bell.bridge.launch.event]` marker after each SLOT_SHELL launch completes (success or failure).
- Marker carries: app_name, app_id, result (ok/fail), failure_reason if applicable.
- Bell receives no actual events yet — marker-only proof that shell knows *when* to notify.
- Gates: ~85–90, all marker assertion gates.
- STOP FIRST: no OP_BELL_NOTIFY call from shell yet.

### Phase 3 — Shell Publishes Launch Result Events to Bell
- Shell calls `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)` with structured launch result.
- Bell enqueues launch outcome notification with appropriate lane (WARN for failure, INFO for success).
- SilkBar Bell dot reflects launch outcomes.
- Spindle `events` command shows Bell events via existing Bell bridge (once SLOT_BELL granted to Spindle).
- Gates: ~85–90, real PDX calls.
- STOP FIRST boundary check: only OP_BELL_NOTIFY (existing V1 opcode), no new opcodes.
- Requires: Spindle SLOT_BELL grant (currently pending) for readback.

### Phase 4 — Bell Attention Policy
- Define per-app attention policy struct (privacy level, prompt_allowed, mute).
- Shell queries Bell attention policy before surfacing deferred prompts.
- Bell uses existing OP_BELL_SET_POLICY (0xC6) — currently `[bell.unknown.reject]`.
- STOP FIRST: implementing SET_POLICY is a Phase E unblock (see BELL_V1_FINAL_STATUS.md §7). Requires policy storage design, possibly sexstore schema.
- No kernel ABI changes required for SET_POLICY (server-side only).
- Phase 4a: policy model + stub.
- Phase 4b: per-app defaults.
- Phase 4c: Collar integration for sensitive grants.

### Phase 5 — Collar / Mesh Integration
- Collar spawned as PD, granted SLOT_BELL.
- Collar sends approve/deny decisions → Bell annotates events.
- Mesh spawned as PD, granted SLOT_BELL (LIST allowlist expansion).
- Mesh queries Bell LIST → renders event/authority graph.
- All future — no Collar or Mesh spawn yet.

---

## 6. Future Markers

Each marker is a **future** serial proof point. None are emitted today.
They are defined here so future implementation phases can reference them.

| Marker | Phase | Meaning |
|--------|-------|---------|
| `[bell.bridge.plan]` | 0 | This plan document exists and has been reviewed |
| `[bell.bridge.status]` | 1 | Status stub proof command executed; role/summary printed |
| `[bell.bridge.launch.event]` | 2 | Shell emitted a launch outcome marker (marker-only, no Bell IPC) |
| `[bell.bridge.launch.result]` | 3 | Shell published a real launch result event to Bell via OP_BELL_NOTIFY |
| `[bell.bridge.launch.result.ok]` | 3 | Launch succeeded; event enqueued with INFO lane |
| `[bell.bridge.launch.result.fail]` | 3 | Launch failed; event enqueued with WARN lane + failure reason |
| `[bell.bridge.launch.result.deferred]` | 4 | Launch failed but user-visible retry prompt is queued |
| `[bell.bridge.attention]` | 4 | Bell attention policy applied to an event |
| `[bell.bridge.attention.prompt]` | 4 | Deferred prompt surfaced to user via SilkBar or notification center |
| `[bell.bridge.attention.dismiss]` | 4 | User dismissed a deferred prompt |
| `[bell.bridge.attention.retry]` | 4 | User requested retry via Bell action → shell re-launches |
| `[bell.bridge.policy.set]` | 4b | Per-app attention policy set via OP_BELL_SET_POLICY |
| `[bell.bridge.collar.approve]` | 5 | Collar approved a sensitive launch grant |
| `[bell.bridge.collar.deny]` | 5 | Collar denied a sensitive launch grant |
| `[bell.bridge.mesh.query]` | 5 | Mesh queried Bell LIST for event graph data |
| `[bell.bridge.proof.done]` | each | Phase gate complete; all markers for that phase proven |

---

## 7. STOP FIRST Boundaries

These boundaries are **hard** and trigger STOP FIRST if any future phase
attempts to cross them. Each must be re-audited separately before being
relaxed.

| # | Boundary | Trigger Condition | Why It's Blocked |
|---|----------|-------------------|-----------------|
| B1 | **New Bell opcode** | Adding OP_BELL_* beyond 0xC0–0xC7 | Kernel ABI freeze. Any new opcode requires kernel config + sex-pdx audit + all server rebuild. Phase 3 uses existing OP_BELL_NOTIFY only. |
| B2 | **Global ABI change** | Modifying `sex-pdx/src/lib.rs` slot assignments, capability numbering, or PDX wire format | All servers rebuild. Capability table shifts cascade. |
| B3 | **Kernel edit** | Any change to `kernel/src/init.rs` beyond adding a capability grant that follows existing pattern | Kernel is the root of trust. Cap grant additions are safe (existing pattern); anything else requires STOP FIRST. |
| B4 | **sex-pdx edit** | Changing opcode constants, slot constants, or reply format | All servers that use sex-pdx must be re-audited. Bell Bridge must work within existing opcodes. |
| B5 | **Launch authority moving out of shell** | Any code path where launch decision is made outside silk-shell's `open_app_in_active_scene()` | Shell is the single launch authority. Bell is an observer only. |
| B6 | **Bell directly focusing apps** | Bell calling any surface focus function or sending focus-related IPC | Bell has no surface concept. Focus is shell-only. |
| B7 | **Browser/network capability increase** | WebStub gaining network > 0, engine > 0, or surface > placeholder | WebStub is network=0 engine=0 by design. Bell Bridge does not change this. |
| B8 | **Frame light action dispatch from Bell** | Bell sending close/minimize/zoom commands | Red close is disabled (close_allowed=0). Frame lights are shell FSM domain. |
| B9 | **SUBSCRIBE opcode implementation without kernel review** | Implementing OP_BELL_SUBSCRIBE (0xC5) | Requires kernel push IPC or shared-memory ring buffer (see BELL_V1_FINAL_STATUS.md §7). Currently blocked on kernel ABI. |
| B10 | **Bell renderer integration** | Bell creating surfaces, registering with sexdisplay, or rendering pixels | Bell is headless. Visual presence is via SilkBar → sexdisplay pipeline. |

---

## 8. Handoff Path

```
docs/handoff/BELL_BRIDGE_APP_LAUNCH_PLAN_V1.md   ← THIS FILE
```

Future dependent docs (not yet created):
```
docs/handoff/BELL_BRIDGE_STATUS_STUB_V1.md         (Phase 1)
docs/handoff/BELL_BRIDGE_LAUNCH_EVENT_MARKERS_V1.md (Phase 2)
docs/handoff/BELL_BRIDGE_LAUNCH_RESULT_V1.md        (Phase 3)
docs/handoff/BELL_BRIDGE_ATTENTION_POLICY_V1.md     (Phase 4)
docs/handoff/BELL_BRIDGE_COLLAR_MESH_V1.md           (Phase 5)
```

---

## 9. Commit Command

```bash
git add docs/handoff/BELL_BRIDGE_APP_LAUNCH_PLAN_V1.md
git commit -m "docs(bell): Bell Bridge app-launch plan V1"
```

---

## 10. References

### Baseline Truth Docs
- `docs/handoff/APP_LAUNCH_EXEC_REVISIT_SLOTSHELL_V1.md` — SLOT_SHELL launch proven (84/84 PASS, 0 faults)
- `docs/handoff/BROWSER_PLACEHOLDER_SURFACE_V1.md` — WebStub launch_exec=1, network=0, engine=0 (85/85 PASS)
- `docs/handoff/ATLAS_SCENE_STATUS_STUB_V1.md` — Atlas status stub only (87/87 PASS)
- `docs/handoff/FRAME_LIGHTS_STATUS_STUB_V1.md` — Red close disabled, yellow/green available (83/83 PASS)
- `docs/handoff/BELL_V1_FINAL_STATUS.md` — Bell V1 complete (6/8 opcodes, SilkBar presence, Phase E blocked)

### Bell Design Docs
- `docs/handoff/BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — Original Bell event model design
- `docs/handoff/BELL_PDX_PROTOCOL_SPEC_V1.md` — Wire format and opcode semantics
- `docs/handoff/BELL_CAPABILITY_POLICY_V1.md` — Two-gate model and allowlist design
- `docs/handoff/BELL_BOOT_SPAWN_V1.md` — Bell spawn (PD 10, PKEY 10)
- `docs/handoff/SPINDLE_BELL_BRIDGE_V1.md` — Spindle Bell bridge (cap pending)
- `docs/handoff/BELL_SILKBAR_PRESENCE_CONTRACT_AUDIT_V1.md` — SilkBar Bell presence pipeline
- `docs/handoff/BELL_PHASE_E2_POLICY_V1.md` — Phase E2 policy design
- `docs/handoff/BELL_PHASE_E_SUBSCRIBE_POLICY_DESIGN_V1.md` — Subscribe policy design (blocked)

### Cross-cutting Docs
- `docs/handoff/STABLE_BASELINE_20260503.md` — Stable baseline as of 2026-05-03
- `docs/handoff/SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md` — Shell interaction authority contract

---

*End of BELL_BRIDGE_APP_LAUNCH_PLAN_V1.md*
