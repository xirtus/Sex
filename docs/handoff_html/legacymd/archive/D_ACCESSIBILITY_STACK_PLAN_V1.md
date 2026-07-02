# D_ACCESSIBILITY_STACK_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** Capability-safe accessibility stack for Silk DE: semantic roles, keyboard navigation, focus narration, and input alternatives. SexOS accessibility is a capability-scoped semantic layer, not an afterthought screen scraper. Apps expose intentional semantic surfaces; Silk validates focus/navigation; Bell/Quil can prove what was announced, selected, skipped, or denied. No POSIX/AT-SPI/DBus. No audio/speech engine in V1.

## 1. Mission

Design capability-safe accessibility stack for Silk DE. Define semantic roles, keyboard navigation, focus narration, input alternatives, proof markers, and safety boundaries. Docs/plan only. No implementation.

## 2. Context / References

- **`docs/SEPARATE_TRACKS_AFTER_12_PROMPTS.md`** — D-track sub-prompt defining object model, semantic roles, keyboard navigation, invariants, STOP FIRST
- **`docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md`** — Track A: keyboard navigation validates focus targets through A4 guards
- **`docs/C_TOUCHPAD_GESTURES_PLAN_V1.md`** — Track C: D provides input alternatives for every gesture action; D gates gesture customization
- **`docs/handoff/FOCUS_CONTRACT_V1.md`** — current focus model
- **`docs/handoff/SHELL_INTERACTION_STATE_V1.md`** — current interaction FSM
- **`docs/handoff/HIT_TEST_PRIORITY_V1.md`** — hit test model for target resolution

Current state: Keyboard handling exists for some actions (F4 top bar toggle, F5 appearance cycle, PageUp minimize, Escape close panel, 1/2/3 scene switch). No semantic role model, no SemanticTree, no FocusPath, no NarrationEvent, no systematic keyboard navigation (Tab/Shift+Tab cycling), no input alternatives mapping for gestures.

## 3. Ownership Boundaries

- **silk-shell** (exclusive): SemanticTree construction for chrome, NavigationIntent filtering, FocusPath traversal, NarrationEvent production (V1: proof markers), AccessibilityPolicy enforcement
- **sexdisplay:** owns nothing — no semantic inference from pixels, no narration rendering
- **silkbar:** silkbar chips/chrome may be semantic sources, but shell constructs the tree
- **Apps (future):** app content SemanticNode production through explicit capability protocol; shell validates and merges into tree
- **Bell (future):** surfacing accessibility/security events from NarrationEvent stream — V1 does not require Bell infrastructure
- **Quil (future):** inspecting AccessibilityProofEvent logs for debug/audit — V1 proof markers are serial_println! only
- **Collar (future):** mediating grants for sensitive app semantic exposure
- **Mesh (future):** visualizing semantic/capability graph — does not decide policy

## 4. Object Model

- **SemanticNode:** shell-accessible element with role, label, state, children. Owned by shell for chrome; by app for app content (future). V1 children are flat (no deep tree — reserved for future app content).
- **SemanticTree:** ordered collection of navigable SemanticNodes for the current Scene. V1 is flat list — tree hierarchy reserved for future app content merging.
- **SemanticRole:** enum of UI element kinds (Frame, Tab, SceneToggle, FrameLight, Button, Slider, TextOutput, etc.). Fixed enum in V1.
- **SemanticLabel:** human-readable text for a node. For shell chrome: deterministic from model state. For app content: future capability protocol.
- **FocusPath:** current navigation path through SemanticTree — which node is focused, direction taken, what was skipped.
- **AccessibleAction:** action on a SemanticNode (Activate, Close, Minimize, Zoom, SwitchToTab, SwitchScene, OpenAtlas). Maps to shell intents or app capabilities.
- **NavigationIntent:** user navigation request (next, previous, first, last, activate, escape). Filtered through AccessibilityPolicy.
- **NarrationEvent:** structured log of narrated/focused/denied. Contains timestamp, FocusPath, SemanticRole, SemanticLabel, AccessibleAction, result.
- **InputAlternative:** non-gesture input path for an action (keyboard shortcut, switch device, future speech). V1 covers keyboard alternatives only.
- **AccessibilityPolicy:** deterministic rules for what is narratable, what is private, navigation directions, input alternatives. V1 hardcoded in shell — no runtime configuration.
- **AccessibilityProofEvent:** logged event with role, label, target_id, action, result. Used by Bell (attention) and Quil (proof console) in future.

## 5. Semantic Role Model (V1 Shell Chrome)

```
SemanticRole ::=
    | SilkBar            // top bar clock/chip/panel area
    | SceneToggle        // workspace toggle (chips)
    | Panel              // launcher, status, clock, bell panels
    | FrameLight         // close/minimize/zoom lights
    | FrameBody          // app content area (opaque role — no scraping)
    | Tab                // tab strip slot
    | TabStrip           // tab strip container
    | FrameRim           // resize/move rim
    | AtlasOverview      // Scene overview (future)
    | SettingsPanel      // scene/chrome settings UI
    | Unknown            // safe fallback
```
V1: shell chrome only. App content roles reserved for future capability protocol.

## 6. Keyboard Navigation Model (V1)

```
Navigation flow: FocusPath follows SemanticTree order within active Scene.
- Tab / Shift+Tab: cycle forward/backward through navigable SemanticNodes
- Arrow keys: directional navigation within a container (tabs, lights)
- Enter/Space: activate AccessibleAction for focused node
- Escape: close panel/dialog, or return to Scene-level navigation
- Ctrl+W: close focused frame/tab (equivalent to red light)
- Ctrl+M: minimize focused frame (equivalent to yellow light)
- Ctrl+Shift+Enter: zoom/unzoom focused frame (green light)
- Ctrl+Tab: cycle focused tab within active frame
- Ctrl+Shift+Tab: cycle tabs backward
- Alt+[1-9]: switch to Scene N
- Alt+Up: open Atlas overview
```

**WARNING:** These key bindings are speculative — audit current keyboard dispatch before implementing. Some bindings may already exist (F4, F5, PageUp, Escape, 1/2/3). Conflicts must be documented and resolved before V1 keyboard navigation wiring. Policy: keyboard navigation cannot focus destroyed/tombstoned/minimized surfaces. Navigation skips hidden/focus-ineligible nodes.

## 7. Focus Narration/Event Model

Narration is event-log-first, not speech-first. On each focus change:
1. Shell resolves focused SemanticNode (role + label).
2. Shell logs NarrationEvent: timestamp, FocusPath, SemanticRole, SemanticLabel, AccessibleAction list (derived on-demand from node type + state, not stored per node), result (narrated/skipped/denied).
3. If label missing: fallback to role name + node ID ("Frame Light Close, frame 2").
4. If node is private/secure: log `[accessibility.narrate.denied]` reason=private, do NOT expose semantics.
5. V1 NarrationEvents are proof markers (serial_println! format). Structured event logs for Bell/Quil are future.

## 8. Input Alternatives Model (Track C Cross-Reference)

Every gesture action in Track C must have a keyboard alternative in V1:

| Gesture | Alternative |
|---------|-------------|
| One-finger tap/click | Click (unchanged) |
| Two-finger scroll | Arrow keys / scroll wheel |
| Pinch zoom | Ctrl+Shift+Enter (zoom toggle) |
| Three-finger swipe horizontal | Alt+[1-9] (scene switch) |
| Three-finger swipe vertical | Alt+Up (Atlas open) |
| Edge reveal | Ctrl+` / dedicated key |

V1 input alternatives are keyboard-only. Future may add switch devices, speech, or other alternatives.

## 9. Track A Lifecycle Dependency

- Keyboard navigation validates focus targets through Track A A4 guards (alive, focusable, non-tombstoned, in active scene, frame accepts input).
- NavigationIntent targeting destroyed/tombstoned/minimized surfaces is skipped before FocusPath traversal — never reaches Track A guards.
- FocusPath traversal skips hidden/focus-ineligible nodes. Only navigable SemanticNodes in active Scene are candidates.
- AccessibleAction dispatch (close/minimize/zoom) routes through Track A lifecycle transitions (Closing→Tombstoned, Minimized, zoom toggle).
- No accessibility action may bypass Track A lifecycle validity.

## 10. Track C Gesture Alternative Gate

- D provides keyboard alternatives for every Track C gesture action. Verified at C7 gate.
- Track C gesture customization (Scan 8) is gated by D: no gesture customization until D verifies alternatives exist.
- Keybindings in A/B/C/Quil/Linen customization are gated by D shortcut/conflict audit.

## 11. Future Integration Boundaries

- **Bell (future):** may surface accessibility/security events from NarrationEvent stream. V1 does not require Bell infrastructure — proof markers suffice.
- **Quil (future):** may inspect AccessibilityProofEvent logs for debug/audit. V1 proof markers are serial_println! only.
- **Collar (future):** may mediate grants for sensitive app semantic exposure (e.g., app providing semantic tree to shell). V1 shell chrome semantics are shell-owned — no Collar grants needed.
- **Mesh (future):** may visualize semantic/capability graph but does not decide policy.

## 12. Invariants

1. Focus narration must correspond to a shell-valid focus target — never destroyed/tombstoned/invalid surface.
2. Semantic nodes cannot grant authority — they describe UI state, not control access.
3. Semantic labels are untrusted until validated by capability/policy — app-provided labels are assertions, not facts.
4. Hidden/private/secure surfaces do not expose semantics unless explicit policy allows. Denied narration is logged.
5. Keyboard navigation cannot focus destroyed/tombstoned/invalid surfaces — same guards as Track A A4.
6. Every AccessibleAction maps to an explicit shell intent or app capability — no hidden action execution.
7. Input alternatives must not bypass lifecycle/focus guards — keyboard navigation respects same target validation as pointer.
8. AccessibilityPolicy must be deterministic and reversible — settings changes rebuild the tree, no hidden state.
9. No NarrationEvent may leak private document/app content across PD boundaries — role+label are shell-validated.
10. sexdisplay never derives semantics from framebuffer pixels — no OCR, no scraping, no pixel-reading for accessibility.
11. Missing semantic label must degrade safely to role-based fallback identification.
12. Accessibility must provide keyboard alternatives for touchpad gestures where possible (Track C C7/C8 gate).
13. V1 narration is event-log-only — no speech, no audio, no TTS engine dependency.
14. V1 semantics are shell chrome only — app content semantics require future capability protocol.

## 13. STOP FIRST Gates

- Any OCR/screen scraping proposed as accessibility foundation
- Any app memory reading for semantic labels
- Any sexdisplay semantic inference from pixels
- Any POSIX/AT-SPI/DBus accessibility assumptions
- Any std audio/speech/thread/time dependencies for V1 narration
- Any app semantic protocol that bypasses capability validation
- Any accessibility action that bypasses shell lifecycle/focus guards
- Any global keyboard shortcut that bypasses secure/private surface policy
- Any semantic content storage without privacy policy
- Any kernel/PDX ABI edits
- Any broad UI refactor
- Any speech/audio engine requirement for V1 implementation
- Any AccessibleAction that does not map to a validated shell intent or capability

## 14. Proof Markers

```
[accessibility.narrate] role=FrameLight label="Close light, frame 2" target=N result=narrated
[accessibility.narrate.denied] role=FrameBody target=N reason=private|secure|invalid
[accessibility.narrate.fallback] target=N fallback=role+id role=FrameBody
[accessibility.navigate] direction=next|prev|first|last|activate|escape from=N to=N
[accessibility.navigate.skipped] target=N reason=minimized|tombstoned|hidden|invalid
[accessibility.action] action=close|minimize|zoom|scene_switch|atlas_open target=N result=dispatched|denied
[accessibility.action.denied] action=close|minimize|zoom|scene_switch target=N reason=target_invalid|no_capability
[accessibility.pref.load]      [accessibility.pref.validate.ok]     [accessibility.pref.validate.reject]
[accessibility.pref.apply]     [accessibility.pref.reset]           [accessibility.pref.redact]
```

## 15. Negative Tests

| # | Scenario | Expected Result | Guard | Reject Marker |
|---|----------|----------------|-------|--------------|
| 1 | Tab cycle across visible frames only | Skips destroyed/tombstoned/hidden | Track A A4 focus validity | `[accessibility.navigate.skipped]` reason=tombstoned |
| 2 | Keyboard navigation skips minimized frames | No focus on minimized unless restore | frame_accepts_input()=false | `[accessibility.navigate.skipped]` reason=minimized |
| 3 | Ctrl+W on focused frame | Close intent dispatched via Track A | Closeable, alive, non-tombstoned | `[accessibility.action]` result=dispatched or `.denied` |
| 4 | Ctrl+M on minimized frame | No-op (already minimized) | FRAME_FLAG_MINIMIZED check | `[accessibility.action.denied]` reason=target_invalid |
| 5 | Alt+1 through Scene N | Scene switch dispatched | Scene ID within WORKSPACE_COUNT | `[accessibility.action]` result=dispatched |
| 6 | Focus on frame light with missing label | Fallback to role+ID | label empty check | `[accessibility.narrate.fallback]` |
| 7 | Hidden/private surface focus request | Narration denied, logged | privacy policy check | `[accessibility.narrate.denied]` reason=private |
| 8 | Destroyed/tombstoned surface cannot be narrated | Guard rejects before narration | `surface_is_alive()` check | `[accessibility.narrate.denied]` reason=invalid |
| 9 | App without semantic protocol | Shell chrome navigation works; app content opaque | V1 shell-only semantics | navigation continues on shell chrome |
| 10 | Keyboard navigation during active gesture | Suspended until gesture completes | InteractionState gate | gesture completes before navigation resumes |
| 11 | Missing narratable target in Scene | Empty Scene fallback reported | SemanticTree empty check | `[accessibility.navigate.skipped]` reason=empty_scene |
| 12 | Input alternative missing for gesture action | Track C C7/C8 gate catches gap | D provides alternative mapping | `[accessibility.action.denied]` reason=no_alternative |

## 16. Minimal Phase Ladder

1. **D1_ACCESSIBILITY_AUDIT_V1** — Inspect current keyboard handling, focus model, frame/tab actions. Identify gaps for semantic roles, narration, keyboard equivalents. No code.
2. **D2_SEMANTIC_ROLE_SPEC_V1** — Write `docs/handoff/SEMANTIC_ROLE_SPEC_V1.md` defining SemanticRole enum, SemanticNode structure, SemanticTree construction rules for shell chrome.
3. **D3_KEYBOARD_NAVIGATION_MODEL_V1** — Design FocusPath traversal through SemanticTree. Add keyboard navigation constants (direction, skip rules, fallback). Shell model only.
4. **D4_FOCUS_NARRATION_EVENT_LOG_V1** — Implement NarrationEvent production on focus change. Log role+label per focus. No speech/audio.
5. **D5_INPUT_ALTERNATIVES_MODEL_V1** — Wire keyboard equivalents for gesture actions (Scene switch, Atlas open, zoom, minimize, close). Cross-check against Track C gestures.
6. **D6_ACCESSIBILITY_CAPABILITY_POLICY_V1** — Define AccessibilityPolicy rules for private/secure surfaces, denied narration, capability-gated semantic exposure.
7. **D7_SHELL_CHROME_SEMANTIC_TREE_V1** — Build SemanticTree from current shell model (Frames, Tabs, Scene, FrameLights). No behavior change — tree is constructed and logged but not yet used for navigation.
8. **D8_NARRATION_KEYBOARD_NAVIGATION_WIRE_V1** — Wire NarrationEvent log to focus changes. Wire keyboard navigation through FocusPath traversing SemanticTree. Wire key bindings through existing shell keyboard dispatch.

## 17. Scan 7 — Exceeded Hypothesis

Assume a rival shell beat Silk accessibility across 10 dimensions:

| Rival Advantage | Why Silk Would Lose | SexOS-Native Fix | Invariant Preserved | Proof Gate |
|----------------|---------------------|------------------|-------------------|------------|
| Screen reader works out of box | No speech/audio engine in V1 | Event-log-first: narration as proof markers, not speech. Speech can be added later without redesign. | §12.13: V1 narration is event-log-only | D4 |
| Keyboard nav reaches all elements | SemanticTree may miss chrome elements | SemanticTree captures all shell chrome (FrameLights, TabStrip, SceneChips, SilkBar). Flat list in V1. | §12.14: V1 semantics are shell chrome only | D7 |
| Tab order matches visual order | SemanticTree could be out of sync | SemanticTree built from model state (Frames array, FrameLights, Tab array). Order matches render order. | §12.8: Deterministic and reversible | D7 |
| Private content is never announced | Narrator could leak private surface content | Narration denied for private/secure surfaces. Logged as denied with reason. | §12.4: Hidden surfaces do not expose semantics | D6 |
| Dead surfaces never receive focus | Nav could land on closing/tombstoned surface | FocusPath skips invalid nodes. Track A A4 guards validate before navigation. | §12.1: Never destroyed/tombstoned target | D3 |
| Custom shortcuts never conflict | Keyboard bindings could collide with existing | D shortcut/conflict audit before any new binding. Conflicts documented in D1 audit. | §12.5: Keyboard nav respects Track A guards | D8 |
| Gesture alternatives always exist | Gestures may lack keyboard equivalent | D provides input alternative for every gesture action. Track C customization gated by D. | §12.12: Input alternatives for gestures | D5 |
| Command/control alternatives don't need screen scrape | No OCR pipeline exists | Apps expose intentional semantic surfaces. Shell validates before use. No scraping. | §12.10: sexdisplay never derives semantics from pixels | D6 |
| Proof markers make failures obvious | Accessibility failure silently swallowed | Every narration/navigation produces proof marker with result. Denials include reason. | §12.6: Action maps to validated intent | D8 |
| Customization is rich but safe | Custom narration could leak private content | AccessibilityPolicy is hardcoded in V1. Future customization cannot bypass privacy rules. | §12.8: Policy is deterministic and reversible | D6 |

## 18. Scan 8 — Customization / User Policy Surface

Customization is shell-owned, validated, reversible, accessible, and unable to customize away accessibility safety, privacy, or capability boundaries.

### Customizable (10 domains)

| Preference | Options | Constraint |
|-----------|---------|------------|
| Keyboard navigation order | tab_order matches tree order (no custom reorder in V1) | Must include all navigable nodes. Cannot skip lifecycles. |
| Focus highlight visibility | enabled/disabled | Visual only. Cannot suppress narration. |
| Narration verbosity | minimum/normal/debug | Cannot suppress required safety markers. |
| Narration fallback style | role+id / role_only | Cannot suppress narration entirely. Must degrade safely. |
| Reduced motion | enabled/disabled | Visual indicators only. Does not affect navigation/narration. |
| Shortcut key remapping (future) | scancode+modifiers (after D audit) | Must pass shortcut/conflict + accessibility audit. |
| Alternative input device (future) | switch/keyboard_only (after device protocol) | Must not bypass lifecycle/focus guards. |
| Semantic tree depth (future) | flat/hierarchical (after app protocol) | Shell validates hierarchy depth cap. |
| Proof verbosity | minimum/normal/debug | Cannot suppress required accessibility markers. |
| Focus-follows-keyboard (future) | enabled/disabled (after D audit) | Must not bypass Track A focus validity. |

### Not Customizable (11 hard boundaries)

No semantic label scraping of private content. No keyboard shortcut that bypasses secure/private surface policy. No AccessibleAction without validated shell intent. No semantic tree exposure across PD boundaries without policy. No sexdisplay pixel-reading for semantics. No POSIX/AT-SPI/DBus assumptions. No audio/speech engine for V1. No app memory reading for labels. No global shortcuts without conflict + accessibility audit. No kernel/PDX ABI edits. sexdisplay ownership boundary (renders shell state only; no semantic inference).

### Customization Proof Scenarios

1. Focus highlight disabled still logs narration → `[accessibility.narrate]` result=narrated. Visual highlight suppressed, narration unchanged.
2. Narration verbosity=minimum still emits `[accessibility.narrate]`, `[accessibility.narrate.denied]`, `[accessibility.navigate]` — required safety markers never suppressed.
3. Reduced motion enabled → `[accessibility.pref.apply]` motion=reduced. Navigation/narration FSMs unchanged.
4. Shortcut remapping before audit rejected → `[accessibility.pref.reject]` reason=no_audit. Planned-only until D audit.
5. Narration fallback style=role_only still provides identification → `[accessibility.narrate.fallback]` with role only.
6. Focus-follows-keyboard cannot bypass Track A → `[accessibility.navigate.skipped]` if target tombstoned/destroyed.
7. Proof verbosity minimum suppresses non-safety markers only → `[accessibility.pref.apply]` fires, safety markers unchanged.
8. Semantic tree depth=flat (V1 default) produces flat list ✅.
9. Custom tab order must include all navigable nodes → validation rejects if node missing from order.
10. Reset-to-safe-default restores canonical behavior → `[accessibility.pref.reset]`. All preferences back to compiled defaults.

### Preference Lifecycle

1. **Load** → `[accessibility.pref.load]`. 2. **Validate** → `[accessibility.pref.validate.ok]` or `.reject`. 3. **Apply** → `[accessibility.pref.apply]` (immediate for UI prefs; policy prefs need guard re-validation). 4. **Persist** → blocked until E gates pass (memory-only in V1). 5. **Redact** → `[accessibility.pref.redact]` per E8 policy. 6. **Reset** → `[accessibility.pref.reset]`.

## 19. Handoff Files

- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — this document (overview)
- `docs/handoff/SEMANTIC_ROLE_SPEC_V1.md` — SemanticRole enum, SemanticNode structure, SemanticTree construction (D2)
- `docs/handoff/KEYBOARD_NAVIGATION_MODEL_V1.md` — FocusPath traversal, keyboard constants, skip rules (D3)
- `docs/handoff/FOCUS_NARRATION_EVENT_LOG_V1.md` — NarrationEvent format, fallback rules, denied logging (D4)
- `docs/handoff/INPUT_ALTERNATIVES_MODEL_V1.md` — keyboard equivalents for gestures, cross-check (D5)
- `docs/handoff/ACCESSIBILITY_CAPABILITY_POLICY_V1.md` — private/secure surfaces, denied narration, policy rules (D6)
- `docs/handoff/SHELL_CHROME_SEMANTIC_TREE_V1.md` — SemanticTree capture from shell model (D7)

## 20. Final Safest Path

1. **D1 audit first** — Current keyboard handling, focus model, frame/tab actions must be audited before semantic role design. Skipping D1 means semantic roles target assumed state, not reality.
2. **Roles before navigation** — D2 (semantic roles) must precede D3 (keyboard navigation) because FocusPath traverses SemanticTree using SemanticRole types.
3. **Navigation before narration** — D3 (FocusPath) must precede D4 (narration) because narration describes focus changes through the SemanticTree.
4. **Alternatives after navigation** — D5 (input alternatives) requires D3 keyboard navigation to exist as the alternative target.
5. **Policy before capture** — D6 (capability policy) must precede D7 (SemanticTree capture) because policy determines what nodes are included/excluded.
6. **Integration last** — D8 wires narration, navigation, and bindings together. Must come after all preceding phases are complete.
7. **No implementation before A4 or C7** — STOP FIRST for accessibility code before Track A A4 focus validity guards complete and Track C C7/C8 gesture alternative verification exists.
