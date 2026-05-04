# ACCESSIBILITY_STACK_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** SexOS accessibility is a capability-scoped semantic layer, not a screen scraper. Apps may eventually expose intentional semantic surfaces; Silk validates focus/navigation; Bell, Mesh, Collar, Quil, and future SexAudio/Theremin integrations remain bounded by capability policy.

---

## 1. Mission

MISSION: D_ACCESSIBILITY_STACK_PLAN_V1 — Design a capability-safe accessibility stack for Silk DE: semantic roles, keyboard navigation, focus narration, input alternatives, privacy-safe proof events, and future app semantic protocols. Docs/plan only. No implementation.

---

## 2. Dependency Gates

1. Track A (COMPOSITOR_LIFECYCLE) A4 focus validity guards must be complete before keyboard navigation validates focus targets.
2. Track A must provide stable Frame/Tab ordering before D3 (keyboard navigation) can define FocusOrder. If Track A does not provide stable ordering, D3 must define a shell-local stable order independently — this must be documented as a STOP FIRST gap in D1 audit.
3. Track C (TOUCHPAD_GESTURES) C8 gate must verify gesture alternatives exist before gesture settings become user-configurable.
4. Must not require app memory scraping — app semantics require explicit future capability protocol.
5. Must not require OCR/screen scraping — sexdisplay never infers semantics from pixels.
6. Must not require speech/audio engine implementation — V1 narration is proof/event-log-only.
7. May reference Theremin/SexAudio future narration sound hooks, but must not depend on real audio output for V1.
8. Must not add POSIX/AT-SPI/D-Bus accessibility assumptions.
9. No accessibility implementation before D1 audit is complete.
10. **Critical: Track A stall guard** — If Track A A4 focus validity guards are not available by D3 handoff, D3 must define a minimal local guard (surface aliveness check based on existing SurfaceId lifetime semantics) to unblock keyboard navigation. This local guard is a temporary bridge, not a replacement for A4.
11. **Critical: FocusOrder stall guard** — If D1 audit finds no stable Frame/Tab ordering in shell model, D3 must define an explicit FocusOrder (e.g., spatial left-to-right, top-to-bottom based on frame positions) independently of Track A. Keyboard navigation must not depend on a Track A ordering that may not exist.

---

## 3. Context

SexOS / Silk DE capability-native microkernel:
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- No POSIX accessibility assumptions (no AT-SPI, no DBus, no Linux accessibility bus)
- No audio/speech engine implementation in V1 — narration is event-log/proof-first
- No app memory scraping — apps expose intentional semantic surfaces only through explicit future capability protocol
- Track A (COMPOSITOR_LIFECYCLE) A4 focus validity guards must be complete before keyboard navigation validates targets
- Track C (TOUCHPAD_GESTURES) C8 gate must verify gesture alternatives exist before gesture settings become user-configurable
- Bell may surface accessibility/security events but does not own navigation policy (Bell integration is future)
- Quil may inspect proof logs but does not own runtime policy (Quil integration is future)
- Collar may mediate grants for sensitive semantic exposure (Collar integration is future)
- Mesh may visualize semantic/capability graph but does not decide policy (Mesh integration is future)
- sexdisplay does not infer semantics from pixels — no OCR/screen scraping

---

## 4. Why Separate

Accessibility is not a feature — it is a cross-cutting semantic layer that must be designed independently from compositor, input, and app protocol work. It requires its own object model (SemanticNode, SemanticTree, FocusPath), its own policy (what gets narrated, what is private), and its own proof layer (what was announced, skipped, denied). Coupling it to the rapid 12-prompt plan would force accessibility through a pixel/focus-only lens without semantic intent. Separating it ensures the semantic model is capability-scoped from day one, not retrofitted onto a pixel-focused compositor.

---

## 5. Innovation Goal

SexOS accessibility should be a capability-scoped semantic layer, not an afterthought screen scraper. Apps expose intentional semantic surfaces; Silk validates focus/navigation; Bell, Mesh, Collar, Quil, and future SexAudio/Theremin integrations are bounded by capability policy. No AT-SPI clone, no D-Bus, no app memory scraping, no OCR-from-pixels. Every navigation decision, narration event, and redaction is explainable through proof logs.

---

## 6. Accessibility Object Model

- **SemanticNode:** a shell-accessible element with role, label, state, children, and redaction_class. Owned by shell for chrome; by app for app content (future). V1: flat list (no deep tree hierarchy — tree nesting reserved for future app content merging).
- **SemanticTree:** ordered list of navigable SemanticNodes for the current Scene. V1: flat list ordered by deterministic FocusOrder.
- **SemanticRole:** the kind of UI element. V1 enum covers shell chrome only: SilkBar, SceneToggle, Panel, FrameLight (Close/Minimize/Zoom), FrameBody, Tab, TabStrip, FrameRim, AtlasOverview, SettingsPanel, Unknown.
- **SemanticLabel:** human-readable text for a SemanticNode. Shell chrome labels are shell-owned (deterministic from model state). App content labels are untrusted assertions requiring capability validation (future).
- **SemanticHint:** optional contextual hint describing the action result or navigation direction ("switches to Scene 2", "closes Frame 3"). Derived from shell model state, not user-provided.
- **FocusPath:** current navigation path through the SemanticTree — which node is focused, what navigation direction was taken, what was skipped. Deterministic per NavigationIntent.
- **FocusOrder:** deterministic ordering of SemanticNodes for keyboard traversal. V1: spatial (left-to-right, top-to-bottom) or explicit order list. Must be stable within a Scene layout — if current shell model lacks stable order, STOP FIRST or define shell-local stable order.
- **AccessibleAction:** an action that can be performed on a SemanticNode (Activate, Close, Minimize, Zoom, SwitchToTab, SwitchScene, OpenAtlas, etc.). Each action maps to an explicit shell intent or app capability.
- **NavigationIntent:** a user navigation request (next, previous, first, last, activate, escape). Filtered through AccessibilityPolicy and FocusPath validity.
- **NavigationResult:** the result of a NavigationIntent — focused_node, skipped_nodes (list), rejected_reason (if denied).
- **NarrationEvent:** a structured log of what was narrated/focused/denied. Contains tick_count, FocusPath, SemanticRole, SemanticLabel, AccessibleAction list (derived on-demand), result (narrated/skipped/denied/redacted).
- **NarrationPolicy:** deterministic rules for what is narratable, what is redacted, what is denied. V1: hardcoded in shell source code — no runtime configuration.
- **InputAlternative:** a non-gesture input path for an action (keyboard shortcut, switch device, future speech). V1: keyboard alternatives only.
- **AccessibilityPolicy:** deterministic rules for focus traversal, narration, redaction, input alternatives, and capability-gated semantic exposure. V1: hardcoded in shell source code.
- **AccessibilityProofEvent:** logged event with role, label, target_id, action, result, redaction_class. Used by Bell (attention) and Quil (proof console).
- **RedactionClass:** classification of semantic sensitivity. Levels: Public (shell chrome), Session (surface title, tab label), Private (document name, user data), Secure (security-critical). V1: shell chrome is Public; app content labels are Private or Secure by default until capability policy allows.
- **SemanticCapability:** a capability required to access or expose semantic information. Kinds: SemanticOutput (receive narration), SemanticExpose (expose app content to accessibility tree), SemanticCapture (capture semantic tree for debugging/audit). Future: Collar-mediated grants.

---

## 7. Semantic Role Model (V1 shell chrome)

```
SemanticRole ::=
    | SilkBar              // top bar clock/chip/panel area
    | SceneToggle          // workspace toggle (chips)
    | Panel                // launcher, status, clock, bell panels
    | FrameLight           // close/minimize/zoom lights (subtype: Close|Minimize|Zoom)
    | FrameBody            // app content area (opaque role — no app content scraping)
    | Tab                  // tab strip slot
    | TabStrip             // tab strip container
    | FrameRim             // resize/move rim
    | AtlasOverview        // Scene overview (future)
    | SettingsPanel        // scene/chrome settings UI
    | Unknown              // safe fallback
```

V1: shell chrome only. App content roles are reserved for future capability protocol. FrameLight has a subtype to distinguish Close, Minimize, and Zoom actions without requiring the consumer to infer from position.

---

## 8. Keyboard Navigation Model (V1)

```
Navigation flow: FocusPath follows FocusOrder within active Scene.

Primary navigation:
- Tab / Shift+Tab: cycle forward/backward through navigable SemanticNodes
- Arrow keys: directional navigation within a container (e.g., tabs within TabStrip, lights within FrameLight cluster)

Action keys:
- Enter/Space: activate AccessibleAction for focused node
- Escape: close panel/dialog, or return to Scene-level navigation

Frame/Tab actions (equivalent to Frame Lights):
- Ctrl+W: close focused frame/tab (equivalent to red light)
- Ctrl+M: minimize focused frame (equivalent to yellow light)
- Ctrl+Shift+Enter: zoom/unzoom focused frame (equivalent to green light)
- Ctrl+Tab: cycle focused tab within active frame
- Ctrl+Shift+Tab: cycle tabs backward

Scene/Atlas navigation:
- Alt+[1-9]: switch to Scene N
- Alt+Up: open Atlas overview
```

**WARNING:** These key bindings are **speculative** — audit current keyboard dispatch in silk-shell before implementing. Some bindings may already exist (e.g., F5 for scene settings, F6 for custom tint). Conflicts must be documented and resolved before V1 keyboard navigation is wired.

Policy: keyboard navigation cannot focus destroyed/tombstoned/minimized surfaces. Navigation skips hidden/focus-ineligible nodes. Focus traversal order is deterministic — if current shell model lacks stable FocusOrder, STOP FIRST and define shell-local stable order.

---

## 9. Focus Narration/Event Model (V1)

```
Narration is event-log-first, not speech-first.

On each focus change (or poll on shell state in V1 if focus events do not yet exist):
1. Shell resolves focused SemanticNode (role + label + redaction_class)
2. Shell checks NarrationPolicy:
   - If redaction_class is Private or Secure AND policy does not allow → redact label
   - If target is invalid/destroyed/tombstoned → deny narration
3. Shell logs NarrationEvent with:
   - tick_count, FocusPath, SemanticRole, SemanticLabel (or redacted)
   - AccessibleAction list (derived on-demand from node type + state)
   - result: narrated | skipped | denied | redacted
4. If label missing: fallback to role name + node ID ("Frame Light Close, frame 2")
5. If node is private/secure and policy denies: log [access.narration.event] with redacted label, result=redacted
6. V1 NarrationEvents are proof markers (serial_println! format). Structured event logs for Bell/Quil are future — do not require Bell/Quil infrastructure for V1.
7. No speech/audio output in V1 — narration is purely proof/event-log.
```

---

## 10. Input Alternatives Model (V1)

Every gesture action from Track C must have a keyboard alternative:

| Gesture / Action            | Input Alternative              |
|-----------------------------|--------------------------------|
| One-finger tap/click        | Click (unchanged)              |
| Two-finger scroll           | Arrow keys / scroll wheel      |
| Pinch zoom (frame zoom)     | Ctrl+Shift+Enter (zoom toggle) |
| Three-finger swipe (scene)  | Alt+[1-9] (Scene switch)       |
| Three-finger swipe (atlas)  | Alt+Up (Atlas open)            |
| Edge reveal                 | Ctrl+` or dedicated key        |
| Focus next/previous (Tab)   | Tab / Shift+Tab                |

V1 input alternatives are keyboard-only. Future may add switch devices, speech, or other alternatives. Track C C8 gate must verify this mapping before gesture settings become user-configurable.

---

## 11. Privacy/Security Model

**Redaction classes and handling:**

| Class   | Scope                        | V1 Handling                                      |
|---------|------------------------------|--------------------------------------------------|
| Public  | Shell chrome labels          | Narrated normally; logged without redaction      |
| Session | Surface title, tab label     | Narrated but not persistently logged in V1       |
| Private | Document name, user data     | Redacted from narration; denied if policy blocks |
| Secure  | Security-critical surfaces   | Always redacted; denied from narration           |

Rules:
- Shell chrome labels are always Public — no redaction needed for SilkBar, Frame Lights, Scene toggles, Tab positions.
- App content labels default to Private or Secure — never narrated unless capability policy explicitly allows.
- Persistent accessibility logs must NOT store Private or Secure labels unless redaction policy exists and is approved.
- NarrationEvent logged with result=redacted for redacted labels — proof marker preserves role, not content.
- No accessibility event may leak private content across PD boundaries.
- If Persistent Log is requested containing private labels → rejected or redacted.

---

## 12. Ownership Boundaries

### Shell (silk-shell) owns:
- Accessibility navigation policy for shell chrome
- SemanticTree construction for chrome (SemanticNodes, FocusOrder)
- NavigationIntent filtering and FocusPath traversal
- NarrationEvent production (V1: proof markers)
- AccessibilityPolicy enforcement (hardcoded in V1)
- InputAlternative dispatch

### Apps own (future):
- App content SemanticNode production through explicit capability protocol
- Shell validates and merges app semantics into SemanticTree

### sexdisplay owns:
- Nothing — no semantic inference from pixels, no narration rendering, no accessibility policy

### SilkBar owns:
- Nothing — silkbar chips/chrome may be semantic sources, but shell constructs the tree

### Bell owns (future):
- Surfacing accessibility/security events from NarrationEvent stream — V1 does not require Bell infrastructure

### Quil owns (future):
- Inspecting AccessibilityProofEvent logs for debug/audit — V1 proof markers are serial_println! only

### Collar owns (future):
- Mediating grants for SemanticOutput, SemanticExpose, SemanticCapture capabilities

### Mesh owns (future):
- Visualizing semantic/capability graph — does not decide policy

### SexAudio/Theremin (future):
- May render narration/sound intents, but do not own semantic truth — D must work without audio

### Linen/sexfiles (future):
- May store accessibility preferences, but must NOT store private semantic logs unless redaction policy exists

---

## 13. Invariants

1. Focus narration must correspond to a shell-valid focus target — never a destroyed/tombstoned/invalid surface.
2. Keyboard navigation cannot focus destroyed/tombstoned/invalid objects — same guards as Track A A4.
3. Focus traversal order (FocusOrder) must be deterministic within a stable Scene layout. If current shell model lacks stable order, STOP FIRST or define shell-local stable order.
4. Semantic nodes cannot grant authority — they describe UI state, not control access.
5. Shell chrome labels are shell-owned (Public redaction class). App-provided labels are untrusted assertions requiring capability validation.
6. Hidden/private/secure surfaces expose semantics only if policy allows — denied/redacted narration is logged.
7. Every AccessibleAction maps to an explicit shell intent or app capability — no hidden action execution.
8. Accessible actions must revalidate target lifecycle/generation on commit — target may have been destroyed between navigation and action.
9. Input alternatives must not bypass lifecycle/focus guards — keyboard navigation respects same target validation as pointer.
10. AccessibilityPolicy must be deterministic and reversible — settings changes rebuild the tree, no hidden state.
11. No accessibility event may leak private content across PD boundaries.
12. Persistent accessibility logs must be redacted or disabled by default until privacy policy exists and is approved.
13. Missing app semantics degrade safely to shell chrome navigation — app content area has opaque role (FrameBody).
14. Accessibility alternatives exist for every Track C V1 gesture intent — verified at D5 gate.
15. Global shortcuts must respect secure/private surface policy — no shortcut bypasses secure/private focus.
16. Destructive actions (close, minimize) require explicit valid target — no action on destroyed/tombstoned nodes.
17. Close/minimize/zoom actions are deterministic and idempotent where possible — same action on same valid target produces same result.
18. sexdisplay never derives semantics from framebuffer pixels — no OCR, no scraping, no pixel-reading for accessibility.
19. No speech/audio engine required for V1 — narration is proof/event-log only.
20. SemanticTree construction must NOT create PDX message storms. Tree rebuilds are bounded per focus change (max N nodes, where N is the number of shell chrome elements — no iteration over app surfaces without explicit capability). Proof markers for navigation are single events, not per-node enumerations.
21. "Hidden" in keyboard navigation context means: surface is minimized, tombstoned, destroyed, behind a closed panel/chrome overlay, or in a non-active Scene. Partially obscured surfaces in the active Scene are still navigable — only fully obscured or lifecycle-ineligible surfaces are skipped.
22. sexdisplay must NOT be asked to render focus indicators, selection highlights, or accessibility overlays. Focus indication (if any) is a shell rendering concern via sexdisplay's existing bounded update path — sexdisplay never decides what constitutes a focus indicator.
23. SemanticRole enum must have an extension mechanism for future roles without breaking existing navigation. V1: use Unknown as safe fallback for unrecognized roles. Future: versioned role tables.

---

## 14. STOP FIRST Conditions

- Any OCR/screen scraping proposed as accessibility foundation
- Any app memory reading for semantic labels
- Any sexdisplay semantic inference from pixels
- Any POSIX/AT-SPI/D-Bus accessibility assumptions
- Any std audio/speech/thread/time dependencies for V1 narration
- Any app semantic protocol that bypasses capability validation
- Any accessibility action that bypasses shell lifecycle/focus guards
- Any global keyboard shortcut that bypasses secure/private surface policy
- Any persistent logging of private semantic content without approved redaction policy
- Any storing of document titles or private labels without redaction policy
- Any kernel/PDX ABI edits
- Any new kernel timing ABI
- Any broad shell/layout refactor driven by accessibility needs
- Any audio/speech output requirement for V1 implementation
- Any accessibility event route that crosses PD boundaries with raw pointers
- Any app semantic protocol delivery before Collar SemanticCapability model exists
- Any request for sexdisplay to render focus indicators, selection highlights, or accessibility overlays — focus indication is a shell rendering concern via existing bounded sexdisplay paths
- Any SemanticTree structure beyond flat list without explicit V2 approval — tree hierarchy implies parent/child navigation, event bubbling, and mutation observers that exceed V1 scope
- Any keyboard shortcut that conflicts with an existing shell binding without documented resolution — conflicts must be resolved (rebind or negotiate) before D3 can ship
- Any narration format or proof marker design that implies speech/audio output exists — V1 proof markers are serial_println! only, not structured event streams for audio engines

---

## 15. Proof Scenarios

### Proof markers

```
[access.audit.start] phase=D1|D2|...
[access.nav.focus.next] from=N to=N role=R
[access.nav.focus.prev] from=N to=N role=R
[access.nav.focus.skip] target=N reason=minimized|tombstoned|hidden|invalid
[access.nav.focus.reject] target=N reason=no_focus_order|no_valid_target
[access.action.commit] action=close|minimize|zoom|scene_switch|atlas_open target=N result=dispatched
[access.action.reject] action=close|minimize|zoom|scene_switch|atlas_open target=N reason=target_invalid|no_capability|idempotent
[access.narration.event] role=R label="S" target=N result=narrated|skipped|denied|redacted
[access.semantic.allow] target=N role=R label="S" capability=C
[access.semantic.deny] target=N role=R reason=private|secure|no_capability
[access.semantic.missing] target=N role=R fallback=safe
[access.privacy.redact] target=N class=Private|Secure reason=policy_denied
```

### Scenarios

1. Keyboard Tab cycles forward through valid visible frames only — skips destroyed/tombstoned/hidden.
2. Keyboard Tab cycles through valid tabs within active frame — skips closing/tombstoned tabs.
3. Hidden/minimized/invalid targets are skipped during navigation unless explicit restore action is taken.
4. Ctrl+W (close) on focused frame/tab → close intent dispatched through existing close path — equivalent to red light.
5. Ctrl+M (minimize) on focused frame → minimize intent dispatched — equivalent to yellow light.
6. Ctrl+Shift+Enter (zoom/unzoom) on focused frame → zoom toggle dispatched — equivalent to green light.
7. Alt+[1-9] (Scene switch) → ACTIVE_SCENE_IDX changes — same effect as workspace chip click or three-finger swipe.
8. Alt+Up (Atlas open) → Atlas overview revealed — same effect as three-finger upward swipe.
9. Focus on frame light → NarrationEvent logged with role=FrameLight label="Close light, frame 2" result=narrated.
10. Focus on surface with no semantic label → fallback to role+ID ("Frame body, frame 3") → NarrationEvent label is "Frame body, frame 3".
11. Hidden/private surface focus request → [access.narration.event] result=redacted or result=denied — no label leak.
12. Destroyed/tombstoned surface cannot be narrated — guard rejects before narration → [access.narration.event] absent.
13. Accessible action without required capability → [access.action.reject] reason=no_capability.
14. App without semantic protocol → shell chrome navigation still works; app content area has opaque role FrameBody → [access.semantic.missing] role=FrameBody fallback=safe.
15. No speech/audio engine exists → proof/event log still works — NarrationEvents are serial_println! markers only.
16. Private/secure surface focused → narration redacted → [access.privacy.redact] class=Private|Secure — no label leak in log.
17. Malicious app label/action attempts authority escalation → [access.semantic.deny] reason=no_capability.
18. Focus target destroyed between navigation and action commit → action cancelled → [access.action.reject] reason=target_invalid.
19. Keyboard alternative exists for every Track C V1 gesture → verified cross-reference at D5 gate.
20. Persistent log request containing private labels → rejected or redacted → [access.privacy.redact] reason=persistent_log_denied.

---

## 16. Minimal Phase Ladder

1. **D1_ACCESSIBILITY_AUDIT_V1** — Audit current shell focus, frame/tab/scene models, input paths, labels, and proof logging. Identify focus validity gaps, existing keyboard shortcuts, and shell state for semantic role extraction. No code.

2. **D2_SEMANTIC_ROLE_SPEC_V1** — Define shell chrome SemanticRole enum, SemanticNode structure, SemanticTree construction rules, FocusOrder determination, RedactionClass model. Handoff doc.

3. **D3_KEYBOARD_NAVIGATION_MODEL_V1** — Define deterministic FocusPath traversal through SemanticTree. Specify key bindings, skip rules, fallback behavior, and conflict resolution with existing shortcuts. Shell model only — no implementation.

4. **D4_FOCUS_NARRATION_EVENT_LOG_V1** — Define NarrationEvent format, NarrationPolicy rules, redaction rules for Private/Secure classes. Proof-marker-only — no speech/audio. Handoff doc.

5. **D5_INPUT_ALTERNATIVES_MODEL_V1** — Map every Track C V1 gesture intent to a keyboard/pointer alternative. Cross-reference with D3 keyboard bindings. Verify no gesture is accessibility-hostile.

6. **D6_ACCESSIBILITY_CAPABILITY_POLICY_V1** — Define Collar grant model for SemanticOutput, SemanticExpose, SemanticCapture. Define app semantic protocol boundaries. Handoff doc.

7. **D7_SHELL_CHROME_ACCESSIBILITY_V1** — Plan shell-only implementation: build SemanticTree from current shell model (Frames, Tabs, Scene, FrameLights, SilkBar). Wire keyboard navigation through FocusPath. Wire NarrationEvent log on focus change. No behavior change — tree is constructed and logged.

8. **D8_ACCESSIBILITY_PROOF_SCENARIOS_V1** — Deterministic proof scenarios covering all 20 scenarios with proof markers.

---

## 17. Handoff Files

- `docs/handoff/D_ACCESSIBILITY_STACK_PLAN_V1.md` — this document (overview)
- `docs/handoff/ACCESSIBILITY_SEMANTIC_ROLES_V1.md` — SemanticRole enum, SemanticNode structure, FocusOrder (D2)
- `docs/handoff/ACCESSIBILITY_KEYBOARD_NAVIGATION_V1.md` — key bindings, FocusPath traversal, skip rules (D3)
- `docs/handoff/ACCESSIBILITY_FOCUS_NARRATION_V1.md` — NarrationEvent format, NarrationPolicy, redaction rules (D4)
- `docs/handoff/ACCESSIBILITY_INPUT_ALTERNATIVES_V1.md` — gesture-to-keyboard mapping, C8 cross-reference (D5)
- `docs/handoff/ACCESSIBILITY_CAPABILITY_POLICY_V1.md` — Collar grant model, app semantic protocol boundaries (D6)
- `docs/handoff/ACCESSIBILITY_PROOF_SCENARIOS_V1.md` — proof scenarios and markers (D8)

---

## 18. Future Sub-Prompt Names

- `D1_ACCESSIBILITY_AUDIT_V1`
- `D2_SEMANTIC_ROLE_SPEC_V1`
- `D3_KEYBOARD_NAVIGATION_MODEL_V1`
- `D4_FOCUS_NARRATION_EVENT_LOG_V1`
- `D5_INPUT_ALTERNATIVES_MODEL_V1`
- `D6_ACCESSIBILITY_CAPABILITY_POLICY_V1`
- `D7_SHELL_CHROME_ACCESSIBILITY_V1`
- `D8_ACCESSIBILITY_PROOF_SCENARIOS_V1`

---

## 19. Cross-Track Dependency Notes

- **Track A (COMPOSITOR_LIFECYCLE):** A4 focus validity guards must be complete before keyboard navigation validates focus targets. Surface/tab lifecycle states (Tombstoned, Destroyed, Minimized) are required for skip/deny rules.
- **Track C (TOUCHPAD_GESTURES):** C8 gate must verify gesture alternatives exist before gesture settings become user-configurable. D5 maps each gesture to a keyboard alternative; C8 gate confirms no gesture is accessibility-hostile.
- **Theremin/SexAudio (future):** May render narration sounds or accessibility audio intents, but D must work without audio. NarrationEvent log is independent of audio output — audio is a future consumer.
- **Bell (future):** May surface accessibility/security events from NarrationEvent stream, but does not own navigation or semantic policy.
- **Mesh (future):** May visualize semantic/focus graph, but cannot grant authority or modify focus.
- **Collar (future):** Owns grants for SemanticOutput, SemanticExpose, SemanticCapture capabilities. No app semantic protocol without Collar model.
- **Linen/sexfiles (future):** Must not store private accessibility logs until approved redaction policy exists. V1 logs are non-persistent proof markers.
- **sexdisplay:** Remains pixels only — no semantic inference, no narration rendering, no accessibility policy.
- **Quil (future):** Inspects AccessibilityProofEvent logs for debug/audit. No runtime control.
- **SilkBar:** Shell chrome element — may be a semantic source (chip labels, panel state) but shell constructs the SemanticTree, not SilkBar.

---

## 20. Premortem Analysis

**Premise:** Assume this plan failed 6 months after acceptance. Below are the identified failure modes, categories, and hardening applied.

### Failure Mode Table

| # | Failure Mode | Category | Severity | Hardening Applied |
|---|-------------|----------|----------|-------------------|
| 1 | **FocusOrder never stabilized** — Track A provides no stable Frame/Tab ordering, keyboard nav collapses | Invariant violation (§13.3) / dependency stall | **Critical** | §2.2: D3 must define shell-local stable order if Track A lacks it; §2.11: FocusOrder stall guard |
| 2 | **Track A A4 never implemented** — accessibility indefinitely blocked | Dependency stall (§2.1) | **Critical** | §2.10: D3 defines minimal local surface liveness guard as temporary bridge |
| 3 | **SemanticTree becomes full DOM** — tree hierarchy, parent/child nav, mutation observers | Scope creep (§13.20 "flat list") | **High** | §14.19: STOP FIRST for any tree hierarchy beyond flat list without V2 approval |
| 4 | **sexdisplay asked to render focus rings** — violates pixels-only boundary | Renderer ownership (§13.22) | **High** | §13.22: sexdisplay must NOT render focus indicators; §14.18: STOP FIRST for any such request |
| 5 | **Keyboard shortcut conflicts unresolved** — Ctrl+W, Ctrl+M already bound | Integration fault | **High** | §14.20: STOP FIRST if any binding conflicts without documented resolution |
| 6 | **Narration becomes speech engine** — "proof markers aren't useful without audio" erodes no-speech constraint | Scope creep (§13.19) | **High** | §14.21: STOP FIRST for narration format implying audio output |
| 7 | **C8 gate never verifies alternatives** — Track C never implements C8, gesture settings unsafe | Cross-track dependency failure (§13.14) | **High** | §2.3: C8 gate must verify before gesture settings configurable |
| 8 | **Redaction granularity too coarse** — Session class lumps generic tab labels with private doc titles | Privacy leak (§13.6) | **Moderate** | §11: Session scope defined as "surface title, tab label"; V1 logs non-persistent so leak surface is bounded |
| 9 | **Proof log metadata side channel** — nav sequence reveals user patterns without labels | Privacy leak (§13.11) | **Moderate** | Noted: proof markers contain role, target_id, tick_count. Target ID alone creates behavioral trace. D4 must define whether target IDs are stable across rebuilds. |
| 10 | **PDX message storm on tree rebuild** — per-focus-change enumeration floods IPC | MPK/PDX fault | **Moderate** | §13.20: bounded rebuilds, single-event markers |
| 11 | **App semantic protocol before Collar model** — PDX messages designed without capability gates | ABI drift (§14.16) | **Moderate** | §14.16: STOP FIRST for protocol before Collar exists |
| 12 | **"Hidden" not well-defined for tiled layouts** — partially obscured vs lifecycle-ineligible | Invariant violation (§13.2, §13.21) | **Moderate** | §13.21: explicit definition: only fully obscured or lifecycle-ineligible are skipped |
| 13 | **NarrationEvent format drift between D4 and D7** — spec vs implementation mismatch | Implementation drift | **Low** | Standard phase discipline: D4 handoff doc must be the source of truth for D7; D8 proof scenarios validate against it |
| 14 | **SemanticRole enum unversioned** — new roles break existing navigation | Scope creep | **Low** | §13.23: Unknown fallback + versioned role tables future |

### Revised Safest Path Summary

1. **FocusOrder independence** — D3 must not depend on Track A for ordering. If Track A provides stable order, use it. If not, D3 defines shell-local spatial or explicit order. The keyboard navigation model must work with or without Track A.
2. **Local liveness guard** — D3 defines a minimal surface-liveness check (SurfaceId exists and is not destroyed/tombstoned) independently of Track A A4. This is a temporary bridge, not a replacement.
3. **Flat list forever in V1** — SemanticTree is a flat list. No hierarchy, no parent/child, no DOM. Tree hierarchy is a V2+ concern after app semantic protocol exists.
4. **No sexdisplay pixel semantics** — Focus indication is shell-owned visual state rendered by the existing display rendering path from shell-provided model; sexdisplay must not infer focus, decide semantics, or own accessibility policy.
5. **Audit-first shortcuts** — D1 audit must enumerate all existing keyboard bindings before D3 proposes new ones. Conflicts must be resolved explicitly, not silently overridden.
6. **Redaction review** — D4 must define whether target IDs in proof markers are stable across SemanticTree rebuilds. If stable, they create a behavioral trace that partially bypasses label redaction.
