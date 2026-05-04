# C_TOUCHPAD_GESTURES_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** Shell-owned touchpad gesture policy for Silk DE after USB HID pointer producer exists. Gestures are capability-aware spatial commands, not laptop-driver magic. Raw HID stays below; Silk interprets intentional movement against Scene/Frame/Tab state with deterministic guards, visible proof, and reversible actions. No POSIX/libinput/evdev assumptions.

## 1. Mission

Design shell-owned touchpad gesture policy for Silk DE. Define gesture FSMs, target validation, interaction state boundaries, proof markers, and safety properties. Docs/plan only. No implementation.

## 2. Context / References

- **`docs/SEPARATE_TRACKS_AFTER_12_PROMPTS.md`** — C-track sub-prompt defining input pipeline boundary, 6 gesture FSMs, invariants, STOP FIRST, proof scenarios
- **`docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md`** — Track A: gesture targets validated against compositor lifecycle (focus validity, surface liveness)
- **`rapid/PHASE_03_INPUT_COMPLETION_USB_MOUSE.md`** — USB input completion (reference only; not build authority)
- **`docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md`** — current USB input state audit
- **`docs/handoff/SEXINPUT_TO_SHELL_ROUTE_AUDIT_V1.md`** — sexinput→shell routing audit
- **`docs/handoff/SHELL_INTERACTION_STATE_V1.md`** — current shell interaction FSM
- **`docs/handoff/FOCUS_CONTRACT_V1.md`** — focus contract for gesture target validity

Current state: USB HID pointer producer exists. NormalizedPointerEvent delivery to shell exists (verify in audit). Shell InteractionState covers Idle/DragActive/HoverTarget. Track A lifecycle exists but A4 focus validity guards may not be complete.

## 3. Ownership Boundaries

- **sexinput:** raw HID parsing, NormalizedPointerEvent production, device-level filtering. No gesture awareness.
- **silk-shell** (exclusive): contact tracking, GestureCandidate FSM, GestureIntent validation, GestureTarget lookup, GestureCommit dispatch, GestureCancel, GestureProofEvent logging. Owns all gesture policy.
- **sexdisplay:** renders shell-provided visual feedback (scroll indicators, zoom bounds, edge glow) — only after shell sends render intents. Never owns gesture policy.
- **Apps:** own nothing in V1 — gesture targets are shell objects (Frame, Scene, Atlas), not app content. Future app scroll protocol may deliver scroll intents to focused app.
- **Track A:** gesture targets validated against lifecycle states. Gesture cannot bypass focus validity or surface liveness rules.

## 4. Input Pipeline Boundary

```
Raw HID device → sexinput → NormalizedPointerEvent → shell → GestureCandidate → GestureIntent → shell Action → sexdisplay render
[USB stack]    [HID parser] [normalized coords/buttons]  [gesture FSM]  [validated target] [shell policy] [bounded pixels]
```

- USB/HID normalizes device reports only (sexinput domain).
- Shell owns gesture policy (silk-shell domain).
- sexdisplay only renders shell-provided visual state.
- Apps never receive raw touchpad contacts in V1.
- Gestures operate on focused/hovered/valid shell targets only.
- No gesture may bypass Track A lifecycle validity.
- No hidden app command execution from gestures.

## 5. Object Model

- **PointerSample:** raw timestamped coordinates + button state from HID (sexinput domain).
- **NormalizedPointerEvent:** device-independent coordinates, button mask, contact count (cross-domain safe).
- **ContactSlot:** tracked finger/contact with position, velocity, age, slot_id.
- **GestureCandidate:** detected pattern of contact movement that may become a gesture (shell domain).
- **GestureState:** current FSM state: Idle, Tracking, Triggered, Committed, Cancelled.
- **GestureIntent:** validated gesture mapping to a shell policy Action (e.g. `GestureIntent::ScrollFrame`, `GestureIntent::SwitchScene`).
- **GestureTarget:** the shell object a gesture targets (Surface, Frame, Tab, Scene, Atlas, EdgeReveal).
- **GestureCommit:** a committed intent dispatched as a shell action.
- **GestureCancelReason:** why a gesture was cancelled (target_invalid, focus_changed, threshold_unmet, contact_lost, timeout).
- **GestureProofEvent:** logged proof event with gesture type, target, state, result.

## 6. Gesture FSMs

### Tap/Click FSM
```
Idle → (contact_count==1 && movement<threshold) → TapTracking → (contact_released && within_timeout) → TapCommitted
                                                              → (movement>=threshold) → TapCancelled
                                                              → (contact_released && outside_timeout) → TapCancelled
```
Output: Click action through existing shell click-dispatch path. Chrome targets (frame light, tab strip, rim) dispatch chrome actions. App content targets dispatch focus+click. Thresholds: `TAP_MOVEMENT_THRESHOLD_PX=10`, `TAP_TIMEOUT_TICKS=<shell tick count>`. No std/time/sleep/thread. If no deterministic tick source exists, V1 tap is movement-only (no timeout).

### Two-Finger Scroll FSM
```
Idle → (contact_count==2) → ScrollTracking → (cumulative_movement>=scroll_threshold) → ScrollIntent
                                           → (contact_count<2) → ScrollCancelled
```
Output: ScrollIntent for shell UI only (V1). No app scroll protocol. If no app scroll path exists, V1 logs proof placeholder. Threshold: `SCROLL_THRESHOLD_PX=20`.

### Pinch Zoom FSM
```
Idle → (contact_count==2 && distance_changes) → PinchTracking → (distance_delta>=zoom_threshold) → ZoomIntent
                                                              → (contact_count<2) → ZoomCancelled
```
Output: Shell Frame zoom/unzoom (flag toggle) or Atlas scale. V1 does not zoom app content. Only valid on frames with FRAME_FLAG_ZOOMED support.

### Three-Finger Swipe FSM
```
Idle → (contact_count==3) → SwipeTracking → (horizontal_movement>=scene_switch_threshold) → SceneSwitchIntent
                                         → (vertical_up_movement>=atlas_threshold) → AtlasOpenIntent
                                         → (contact_count<3 || wrong_direction) → SwipeCancelled
```
Output: Scene switch or Atlas open intent. Validated against valid Scene IDs within WORKSPACE_COUNT.

### Edge Reveal FSM
```
Idle → (cursor in edge proximity && contact_count>=1) → EdgeRevealPending → (movement_away_from_edge>=reveal_threshold) → RevealIntent
                                                                          → (contact_lost || movement_back) → RevealCancelled
```
Output: SilkBar/overlay reveal intent. Edge zone: configurable constant (e.g. 10px from screen edge). Edge gestures detected by cursor position at gesture start, not touchpad hardware zone.

### Cancel/Recovery FSM
```
AnyGestureState → (target_invalid) → CancelWithReason(target_invalid)
AnyGestureState → (focus_changed && policy==lock_target) → ContinueLocked or CancelWithReason(focus_changed)
AnyGestureState → (contact_lost) → CancelWithReason(contact_lost)
AnyGestureState → (timeout_exceeded) → CancelWithReason(timeout)
```
Cancel policy: V1 uses lock-target-at-start; if locked target becomes invalid, cancel with proof.

## 7. Gesture States vs InteractionState

Gesture FSMs are orthogonal to the shell's existing InteractionState until gesture commit:

- Pre-commit: gesture FSMs run in parallel with InteractionState. A two-finger scroll in progress does not change InteractionState.
- On commit: GestureIntent maps to a shell Action, which may transition InteractionState (e.g. rim drag commit → `InteractionState::DragActive`).
- On cancel: InteractionState unchanged.
- Conflict rule: if `InteractionState::DragActive`, gesture FSMs suspended until drag completes. Gesture processing resumes from Idle.
- Exception: tap/click may commit during `InteractionState::Idle` only. No click dispatch during active drag.

## 8. Track A Lifecycle Dependency

- Gesture targets must be validated against Track A lifecycle before commit.
- Focused surface at gesture start must pass Track A focus validity (alive, focusable, non-tombstoned, in active scene).
- If target becomes invalid mid-gesture (surface tombstoned/destroyed), cancel with `GestureCancelReason::TargetInvalid`.
- V1 lock-target policy: on gesture start, record GestureTarget + lifecycle generation. On commit, revalidate through Track A guards.
- No gesture may target Tombstoned or Destroyed surfaces.
- No gesture may bypass `try_set_focus()` guards.
- Gesture commit that changes focus (tap on new target) must go through Track A focus validity.

## 9. Invariants

1. Every committed gesture must have a valid GestureTarget. If target becomes invalid mid-gesture, cancel.
2. Focus changes mid-gesture: lock initial target (V1 policy: lock, do not chase focus). Record target + lifecycle generation at start. Revalidate on commit.
3. V1 thresholds and timeouts are fixed deterministic constants in shell source code. No runtime configuration. Future adjustments deferred to accessibility/settings track (D).
4. Raw contacts never cross PD boundaries as unsafe pointers. Only NormalizedPointerEvent crosses sexinput→shell boundary. Apps receive only existing pointer/click path.
5. Gesture output must be shell actions, not framebuffer writes. sexdisplay receives intents, not pixels.
6. Gesture cancel must leave shell layout in valid state (no half-zoomed frames, no mid-switch Scene).
7. Pinch/zoom cannot create impossible frame geometry. Zoom flag toggle on valid frames only.
8. Swipe cannot switch to invalid Scene. Destination Scene must exist within WORKSPACE_COUNT.
9. Edge reveal may reveal shell chrome but must not change focus or deliver hidden actions. Edge reveal cannot steal focus from any surface unless explicit user gesture follows reveal.
10. Touchpad gestures must degrade safely to pointer movement/click if unsupported contact count received (4+ fingers).
11. Accessibility alternatives must remain possible. Gestures must not be the only way to invoke actions.
12. No gesture may bypass Track A lifecycle validity (focused surface must be focusable, target must not be tombstoned/destroyed).
13. A committed gesture must produce exactly one GestureProofEvent before dispatch.
14. Gesture proof events must include GestureType, TargetId, Result, and CancelReason (if cancelled).
15. Pre-commit gesture FSMs are orthogonal to InteractionState. Gesture may not mutate InteractionState until commit.
16. No gesture timeout may use std/time/sleep/thread/POSIX timers or new kernel timing ABI. If no deterministic shell tick source exists, V1 disables timed gestures.
17. V1 does not deliver scroll intents to apps. If no app scroll protocol exists, scroll V1 is proof-placeholder or shell-only.

## 10. STOP FIRST Gates

- Any USB/XHCI rewrite proposed by this track
- Any HID report parser rewrite beyond audit notes
- Any kernel/input ABI change
- Any PDX ABI change
- Any sexdisplay gesture policy — sexdisplay renders shell-provided state only
- Any app raw touch contact delivery in V1
- Any compositor lifecycle redesign driven by gesture needs
- Any shared memory/backing buffer redesign
- Any Linux evdev/libinput/POSIX assumptions
- Any gesture invoking app commands without explicit future capability protocol
- Any accessibility-hostile design with no keyboard/input alternative path
- Any gesture threshold that is not a deterministic constant
- Any focus-chase policy (V1 locks target at gesture start)
- Any gesture that modifies InteractionState before commit
- Any click dispatch during active drag/resize operation
- Any gesture timeout requiring std/time, sleep/thread, POSIX timers, or new kernel timing ABI
- Any new app scroll protocol or PDX ABI for scroll without explicit approval
- Any implementation before input audit confirms NormalizedPointerEvent delivery to shell

## 11. Proof Markers

```
[gesture.candidate.start] type=tap|scroll|pinch|swipe|edge_reveal target_id=N
[gesture.candidate.update] type=tap|scroll|pinch|swipe|edge_reveal state=tracking
[gesture.commit.tap] target_id=N x=X y=Y
[gesture.commit.scroll] target_id=N dx=D dy=D
[gesture.commit.pinch] target_id=N zoom=in|out
[gesture.commit.scene_swipe] from=N to=N direction=left|right
[gesture.commit.edge_reveal] edge=top|bottom|left|right
[gesture.cancel] reason=target_invalid|focus_changed|threshold_unmet|contact_lost|timeout
[gesture.reject] reason=unsupported_contact_count|invalid_target|no_tick_source
```

## 12. Negative Tests

| # | Scenario | Expected Result | Guard | Reject Marker |
|---|----------|----------------|-------|--------------|
| 1 | Single-finger tap below threshold, released within timeout | TapCommitted → click dispatch | movement<threshold, within_timeout | `[gesture.commit.tap]` |
| 2 | Tap candidate moves beyond threshold | TapCancelled → no click dispatch | movement>=threshold | `[gesture.cancel]` reason=threshold_unmet |
| 3 | Two-finger scroll above threshold | ScrollIntent logged (shell-only) | cumulative_movement>=threshold | `[gesture.commit.scroll]` |
| 4 | Pinch on frame without FRAME_FLAG_ZOOMED | No zoom action, cancel or no-op | zoom flag check | `[gesture.reject]` reason=invalid_target |
| 5 | Three-finger swipe to invalid Scene ID | Scene switch blocked, cancel | destination Scene exists within WORKSPACE_COUNT | `[gesture.cancel]` reason=invalid_target |
| 6 | Edge reveal over secure/locked surface | Visual-only or blocked, no focus steal | secure surface policy | `[gesture.commit.edge_reveal]` but no focus change |
| 7 | Target surface tombstoned mid-gesture | Cancel, layout unchanged | Track A lifecycle check | `[gesture.cancel]` reason=target_invalid |
| 8 | Focus changes mid-gesture | lock-target: continue on original target | lock_target policy | gesture continues, no cancel |
| 9 | 4+ finger contact | Degrade to pointer, no gesture FSM | unsupported contact count | `[gesture.reject]` reason=unsupported_contact_count |
| 10 | Tap during active drag/resize | No click dispatch | InteractionState::DragActive | tap not processed, gesture stays Idle |
| 11 | Scroll over app content with no app scroll protocol | No fake IPC, proof placeholder | no app scroll protocol | `[gesture.commit.scroll]` with note=shell_only |
| 12 | No deterministic tick source for tap timeout | Tap is movement-only, no timeout | no std/time/sleep | tap works, no `[gesture.cancel]` reason=timeout |

## 13. Minimal Phase Ladder

1. **C1_INPUT_AUDIT_V1** — Inspect current sexinput→shell pointer event route. Verify NormalizedPointerEvent exists or document gap. No code.
2. **C2_GESTURE_BOUNDARY_SPEC_V1** — Write `docs/handoff/GESTURE_BOUNDARY_SPEC_V1.md` defining sexinput→shell gesture boundary, NormalizedPointerEvent format, contact count semantics.
3. **C3_GESTURE_FSM_SPEC_V1** — Write `docs/handoff/GESTURE_FSM_SPEC_V1.md` with state tables, transition tables, thresholds, invariants for all 6 FSMs.
4. **C4_SHELL_GESTURE_MODEL_V1** — Add GestureState, GestureCandidate, GestureTarget tracking inside silk-shell. No behavior change beyond tracking and proof markers.
5. **C5_GESTURE_TARGET_VALIDITY_V1** — Wire GestureTarget validation against Track A lifecycle (focus validity, surface liveness, tombstone checks).
6. **C6_SCENE_ATLAS_GESTURE_INTENTS_V1** — Implement SceneSwitchIntent and AtlasOpenIntent dispatch through existing shell scene/atlas mechanisms.
7. **C7_GESTURE_PROOF_SCENARIOS_V1** — Run deterministic test sequences for all proof scenarios. Verify every allowed transition produces allow marker, every forbidden transition produces reject marker.

## 14. Scan 7 — Exceeded Hypothesis

Assume a rival shell beat Silk touchpad gestures across 10 dimensions:

| Rival Advantage | Why Silk Would Lose | SexOS-Native Fix | Invariant Preserved | Proof Gate |
|----------------|---------------------|------------------|-------------------|------------|
| Tap reliably clicks | Movement threshold may be wrong for small taps | Fixed TAP_MOVEMENT_THRESHOLD_PX=10 with timeout guard | §9.3: Thresholds are deterministic constants | C3 |
| Scroll tracks fingers perfectly | Contact tracking drift | ContactSlot with position+velocity+age tracking. Scroll uses cumulative movement, not instantaneous delta. | §9.1: Valid target required | C3+C4 |
| Pinch zoom is smooth and bounded | Zoom could create invalid frame geometry | Zoom flag toggle only on valid frames. geometry clamped after zoom. | §9.7: No impossible geometry | C5 |
| Swipe never switches to wrong scene | Swipe could target non-existent scene | Destination validated against WORKSPACE_COUNT before commit | §9.8: Valid Scene only | C6 |
| Edge reveal never steals focus | Reveal could focus hidden overlay | Edge reveal visual-only unless explicit user gesture follows | §9.9: No focus steal | C5 |
| Gestures work during drag | Drag+gesture conflict | Gesture FSMs suspended during DragActive. Resumes from Idle after drag. | §9.15: Orthogonal until commit | C4 |
| Dead surfaces never receive gestures | Gesture could target tombstoned surface | Target validated against Track A lifecycle at commit. Tombstoned → cancel. | §9.12: No lifecycle bypass | C5 |
| Cancel always leaves valid state | Mid-gesture cancel could leave half-zoomed frame | Cancel reverts Frame state. Gesture proof events always logged. | §9.6: Cancel leaves valid state | C7 |
| Proof markers make failures obvious | Gesture failure silently swallowed | Every commit/cancel produces proof marker with reason. | §9.13-14: Proof always emitted | C7 |
| Customization is rich but safe | Custom thresholds could break gestures | All thresholds are deterministic constants. Future customization validated by D track. | §9.3: Constants today | C3+D |

## 15. Scan 8 — Customization / User Policy Surface

Customization is shell-owned, validated, reversible, accessible, and unable to customize away gesture safety or bypass Track A lifecycle.

### Customizable (10 domains)

| Preference | Options | Constraint |
|-----------|---------|------------|
| Tap threshold | deterministic set (e.g. 5/10/15px) | Cannot disable tap entirely. Must remain ≥ minimum safe value. |
| Scroll sensitivity | low/medium/high (scales SCROLL_THRESHOLD) | Cannot bypass scroll safety, must have keyboard alternative. |
| Pinch zoom sensitivity | low/medium/high | Only operates on FRAME_FLAG_ZOOMED frames. |
| Swipe scene wrap | enabled/disabled | Destination must still validate against WORKSPACE_COUNT. |
| Edge reveal zone width | 5/10/15px from screen edge | Cannot disable edge reveal entirely (accessibility). Must remain ≥ minimum. |
| Tap timeout (if tick source exists) | bounded tick range | No std/time/sleep. Only if deterministic shell tick source exists. |
| Edge reveal action | silkbar/atlas/notification_center | Shell-owned actions only. Cannot reveal app content or bypass policy. |
| Gesture proof verbosity | minimum/normal/debug | Cannot suppress required safety markers. |
| Focus-follows-gesture (future) | enabled/disabled (after D audit) | Must not bypass Track A lifecycles or focus validity. |
| Keybindings (future) | scancode+modifiers | After D accessibility + shortcut audit. |

### Not Customizable (11 hard boundaries)

Capability-aware gesture validation (Track A). Gesture-target lifecycle checks (alive/focusable/non-tombstoned). InteractionState orthogonality rule (gestures suspended during DragActive). No app raw contact delivery in V1. sexdisplay policy ownership. No scroll IPC to apps without approval. No focus-chase policy (lock-target). No gesture timeout via std/time/sleep. No keyboard/accessibility alternative removal. No gesture invoking app commands without capability protocol. sexdisplay ownership boundary (renders shell state only).

### Customization Proof Scenarios

1. Valid tap threshold accepted → `[gesture.pref.accept]` threshold=N. Tap FSM uses new threshold.
2. Invalid threshold rejected → `[gesture.pref.reject]` reason=below_minimum, clamped to minimum safe value.
3. Reduced motion disables visual feedback but gesture FSMs unchanged → `[gesture.pref.apply]` motion=reduced. FSM states unchanged.
4. Swipe scene wrap=enabled still validates destination → `[gesture.commit.scene_swipe]` with valid Scene ID. Wrapping does not bypass WORKSPACE_COUNT.
5. Edge reveal action=notification_center restricted to shell-owned actions → shell reveals notification center, not arbitrary content.
6. Focus-follows-gesture cannot bypass Track A → `[gesture.cancel]` if target tombstoned/destroyed, even with preference enabled.
7. Proof verbosity=minimum still emits `[gesture.commit.tap]`, `[gesture.cancel]`, `[gesture.reject]` — required safety markers never suppressed.
8. Keybinding before audit rejected → `[gesture.pref.reject]` reason=no_audit. Planned-only until D accessibility gate.
9. Tap timeout disabled (no tick source) works as movement-only → tap committed on release below threshold, no timeout cancel.
10. Reset-to-safe-default restores canonical behavior → `[gesture.pref.reset]`. All preferences back to compiled defaults.

### Preference Lifecycle

1. **Load** → `[gesture.pref.load]`. 2. **Validate** → `[gesture.pref.validate.ok]` or `.reject`. 3. **Apply** → `[gesture.pref.apply]` (immediate for thresholds; policy prefs need guard re-validation). 4. **Persist** → blocked until E gates pass (memory-only in V1). 5. **Redact** → `[gesture.pref.redact]` per E8 policy. 6. **Reset** → `[gesture.pref.reset]`.

## 16. Handoff Files

- `docs/C_TOUCHPAD_GESTURES_PLAN_V1.md` — this document (overview)
- `docs/handoff/GESTURE_BOUNDARY_SPEC_V1.md` — sexinput→shell boundary, NormalizedPointerEvent format, contact semantics (C2)
- `docs/handoff/GESTURE_FSM_SPEC_V1.md` — state tables, transitions, thresholds, invariants for all 6 FSMs (C3)
- `docs/handoff/GESTURE_SHELL_MODEL_V1.md` — GestureState, GestureCandidate, GestureTarget tracking (C4)
- `docs/handoff/GESTURE_TARGET_VALIDITY_V1.md` — Track A lifecycle validation for gesture targets (C5)
- `docs/handoff/GESTURE_INTENT_DISPATCH_V1.md` — Scene/Atlas gesture intent dispatch (C6)

## 17. Final Safest Path

1. **C1 audit first** — Input pipeline audit must confirm NormalizedPointerEvent delivery. If gap exists, document before FSM design.
2. **Boundary before FSM** — C2 defines sexinput→shell boundary before C3 defines FSMs. FSMs designed against confirmed input format.
3. **FSM before model** — C3 specifies all FSMs before C4 implements tracking. Stable FSM targets needed before code.
4. **Target validity before dispatch** — C5 (Track A validation) must precede C6 (intent dispatch). Dispatch without validity is unsafe.
5. **Proof scenarios last** — C7 verifies all allowed/forbidden transitions produce correct markers. Must come after all preceding phases.
6. **No implementation before C1 or A4** — STOP FIRST for any gesture code before input audit completes and Track A A4 focus validity guards are in place.
7. **Accessibility review at C7 gate** — C7 proof scenarios must include keyboard alternative verification for every gesture action. Document accessibility gap if alternative does not exist.
