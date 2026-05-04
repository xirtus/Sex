# SEPARATE TRACKS AFTER 12 PROMPTS

## 1. Overview

This document defines 9 major tracks that follow the rapid 12-prompt core plan for SexOS / Silk DE. Each track is intentionally separated because:

- Each requires its own design space that would bloat a unified prompt.
- Each has unique proof/audit/review gates that must not be shortcut.
- Each builds on proven baseline invariants without redesigning the kernel, PDX ABI, framebuffer ownership, or shell/display split.

These tracks are **not** speculative — they are the next concrete layers of a capability-native, deterministic, inspectable, reversible desktop operating system. Every track preserves:

- **no_std Rust Sex Microkernel constraints** — no std, libc, threads, POSIX, heap unless explicit
- **MPK/PKU/PKEY isolation** — cross-domain pointers are invalid
- **PDX IPC only** — capabilities, not syscall soup
- **sexdisplay sole framebuffer writer** — shell owns policy, display owns pixels
- **shell owns policy** — compositor lifecycle, focus, gestures, accessibility
- **renderer stays dumb/bounded/deterministic** — no policy in the pixel pusher

Each track prompt is copy-pasteable as a bash heredoc for use with any LLM agent (Codex, Claude, Gemini).

---

## 2. Priority Order and Dependency Graph

```
                    ┌──────────────────────┐
                    │ A: COMPOSITOR        │
                    │ LIFECYCLE (FOUNDATION)│
                    └──────────┬───────────┘
                               │
                    ┌──────────▼───────────┐
                    │ B: APP LAUNCH        │
                    │ SESSION RESTORE      │
                    └──────────┬───────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐  ┌──────────────┐  ┌──────────────────┐
│ C: TOUCHPAD      │  │ D: ACCESSIB- │  │ E: PERSISTENT    │
│ GESTURES         │  │ ILITY STACK  │  │ STORAGE MATURITY │
└──────────────────┘  └──────────────┘  └────────┬─────────┘
          │                                       │
          │                              ┌────────▼─────────┐
          │                              │ F: LINEN DOCUMENT│
          │                              │ LIFECYCLE        │
          │                              └──────────────────┘
          │                                       │
┌─────────▼──────────────────┐  ┌─────────────────▼──────┐
│ G: PACKAGE TRUST/UPDATE UX │  │ H: CRASH LOG VIEWER    │
│ (sexshop, Collar, Mesh)    │  │ (sexcrash, Bell, Mesh) │
└────────────────────────────┘  └───────────┬────────────┘
                                            │
                                  ┌─────────▼────────────┐
                                  │ I: FULL DEV COCKPIT  │
                                  │ (Quil first, Denim?) │
                                  └──────────────────────┘
```

**Strict sequencing:**
- A → B → (C, D, E can begin after A, but E must finish before F)
- F → (G, H can begin after F or in parallel)
- I depends on H + F + G (needs crash logs, document system, package view)

**Parallelism note:** C and D share no dependencies with E/F — they can be staffed independently after A is stable. G and H can be worked in parallel after their prerequisites are met.

---

## 3. MASTER PROMPT TEMPLATE (Reusable)

This template is the **skeleton for every track prompt below**. Each track prompt is a populated instance of this template.

```bash
cat > /tmp/<track_name>.prompt <<'EOF_TRACK'
MISSION: <TRACK_NAME> — <ONE_LINE_MISSION>.

CONTEXT:
- SexOS / Silk DE capability-native microkernel
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- See: docs/SEPARATE_TRACKS_AFTER_12_PROMPTS.md for full constraints
- See: docs/STABLE_BASELINE_20260503.md for locked invariants
- See: docs/AGENT_README_FIRST.md for token discipline

WHY THIS TRACK IS SEPARATE:
<2-3 sentences explaining why this track deserves its own plan>.

INNOVATION GOAL:
<What makes this uniquely SexOS/Silk — not a Linux clone>.

STRICT BOUNDARIES:
- <boundary 1>
- <boundary 2>
- <boundary 3>

FILES/DOCS TO INSPECT FIRST (use rg, read only relevant snippets):
- <file path>: <what to look for>
- <file path>: <what to look for>

EXPECTED DESIGN OUTPUT SECTIONS:
1. <section>
2. <section>
3. <section>

STOP FIRST CONDITIONS:
- <condition that must trigger a halt and explain>
- <condition that must trigger a halt and explain>

PROOF MARKERS / TESTS / SCENARIOS:
- <proof marker 1>
- <proof marker 2>
- <proof marker 3>

MINIMAL IMPLEMENTATION PHASE LADDER:
1. Phase 1: <minimal first step>
2. Phase 2: <next step>
3. Phase 3: <next step>

HANDOFF NOTES TO SAVE:
- <what to record in docs/handoff/>
- <what to record in docs/handoff/>

BACKUP BEFORE CHANGES.
READ HANDOUTS FIRST.
EOF_TRACK
```

---

## 4. TRACK PROMPTS

---

### A_COMPOSITOR_LIFECYCLE

```bash
cat > /tmp/A_COMPOSITOR_LIFECYCLE.prompt <<'EOF_TRACK'
MISSION: A_COMPOSITOR_LIFECYCLE_FSM_PLAN_V1 — Design the real shell-owned Surface/Tab/Frame lifecycle FSM for Silk DE. Docs/plan only. No implementation.

CONTEXT:
- SexOS / Silk DE capability-native microkernel
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- Audit current SurfaceId allocation/guard behavior before claiming gaps
- Shell currently tracks active surfaces but has no minimize/close/tombstone semantics
- sexdisplay renders pixels only — all lifecycle policy stays in shell
- See: docs/STABLE_BASELINE_20260503.md for locked invariants
- See: docs/SILK_DE_EXECUTION_PLAN.md for current compositor state
- See: docs/AGENT_README_FIRST.md for token discipline
- See: docs/handoff/SURFACE_ID_LIFETIME_PATCH_V1.md for current surface ID guards
- See: docs/handoff/SURFACE_LIFETIME_GUARD_V1.md for current lifetime guards
- See: docs/handoff/FRAME_LIGHTS_* for frame light semantics context

WHY THIS TRACK IS SEPARATE:
Compositor lifecycle is the foundation for all window management behavior. It must be designed as a complete finite state machine before any app lifecycle, gestures, or session restore can be built. Rushing this would create inconsistent surface states and broken focus recovery. This track produces the spec/plan only — implementation is delegated to eight sub-prompts.

INNOVATION GOAL:
A capability-native lifecycle system where every visible desktop object has a provable owner, state, and legal transition. Silk does not copy X11/Wayland window-manager ambiguity. It makes lifecycle inspectable: the user, Mesh, Bell, and Quil can all understand why a surface is focused, minimized, dead, restored, or destroyed.

OBJECT MODEL — Surface vs Tab vs Frame vs Scene vs Atlas:
- **Surface:** app content unit known to shell/display. Valid/liveness/renderability FSM.
- **Tab:** shell-owned wrapper around one surface/session. Selected/minimized/closing/restorable FSM.
- **Frame:** tiled container holding one or more tabs. Tiled/zoomed/minimized/empty/tombstoned FSM.
- **Scene:** workspace/layout collection of frames. Owns layout membership.
- **Atlas:** overview of all Scenes. Navigation layer.

These objects must not be collapsed into one FSM. Each has its own state machine and invariants.

STRICT BOUNDARIES:
- **No sexdisplay policy ownership** — sexdisplay renders pixels for whatever surfaces shell tells it to; all lifecycle logic is in shell. sexdisplay may render shell-provided visual state but must not decide lifecycle meaning.
- **No kernel ABI edits** — surface lifecycle is a shell/display concern, not kernel
- **Assume no PDX ABI changes.** If any lifecycle transition cannot be represented with current shell/display messages, STOP FIRST and document the smallest ABI gap without editing it.
- **No shared-memory/backing-buffer redesign** — existing sexdisplay buffer management stays
- **No app-side compositor protocol** — apps do not negotiate lifecycle; shell decides
- **No POSIX process model** — surfaces are not processes; PD death and surface death are separate concerns
- **No new abstraction crates** unless existing model crate proves insufficient
- **Detect stale/dead ownership only through existing observable events/proofs.** If PD liveness query is needed and absent, STOP FIRST.

FILES/DOCS TO INSPECT FIRST (use rg, read only relevant snippets):
- servers/silk-shell/src/main.rs: current surface tracking, focus management, toggle_os_panel
- servers/sexdisplay/src/main.rs: surface create/destroy/update opcode handling, composite_pixel
- crates/silkbar-model/src/lib.rs: model state, surface ID constants
- docs/handoff/SURFACE_ID_LIFETIME_PATCH_V1.md: current stale surface ID semantics
- docs/handoff/SURFACE_LIFETIME_GUARD_V1.md: current lifetime guard proof
- docs/handoff/FRAME_LIGHTS_MODEL_V1.md: frame light semantics
- docs/handoff/FRAME_LIGHTS_ACTION_PLAN_V1.md: frame light action plan
- docs/handoff/FRAME_CLOSE_LIGHT_ACTION_V1.md: close action via lights
- docs/handoff/FRAME_MINIMIZE_MODEL_PLAN_V1.md: minimize model
- docs/PDX_QUICKMAP.md: opcode reference
- docs/IPCPKU_MAP.md: domain isolation boundaries

EXPECTED DESIGN OUTPUT SECTIONS:
1. Mission: docs/plan only — no implementation in this prompt
2. Object model: Surface vs Tab vs Frame vs Scene vs Atlas with ownership and nesting rules
3. Surface lifecycle FSM states: Allocated, Mapped, Visible, Focused, Hidden, Minimized, Closing, Tombstoned, Destroyed — with legal transitions and invariants per state
4. Tab FSM: focusable, selected, minimized, closing, restorable
5. Frame FSM: tiled, zoomed, minimized, empty, tombstoned
6. Legal transition table — matrix of all state pairs: allowed, denied, or STOP FIRST gap
7. Invariants (see below)
8. Ownership boundaries: shell vs display vs app for each object and transition
9. Focus as separate shell selection state (not a lifecycle state) — `focused_surface: Option<SurfaceId>` with validity guards
10. Stale SurfaceId invalidation protocol
11. Dead focused surface recovery fallback policy
12. Drag/resize cancellation FSM
13. Frame Light semantics as user actions (red=close, yellow=minimize, green=zoom) — not health LEDs
14. Proof scenarios (see below)
15. Minimal implementation phase ladder (see below)

INVARIANTS:
- A destroyed SurfaceId is never reused in the same boot/session unless current allocator already guarantees otherwise. If not guaranteed, STOP FIRST.
- Focus must always point to `None` or a currently focusable surface.
- Drag/resize must be cancelled before target enters Closing/Tombstoned/Destroyed.
- Minimized surfaces cannot receive pointer focus until restored.
- Tombstoned surfaces are visible only as shell/debug/session artifacts, not live app content.
- sexdisplay never decides close/minimize/focus semantics.
- Apps cannot force themselves focused.
- Unknown SurfaceId messages are rejected/no-op with proof marker.
- Close is idempotent.
- Destroy is terminal.
- No lifecycle transition may require cross-PD raw pointer access.

STOP FIRST CONDITIONS:
- Any proposed change to sexdisplay's composite_pixel or framebuffer write path
- Any proposed change to PDX ABI or opcode semantics
- Any proposal to add POSIX signal/Wayland-style configure negotiation
- Any proposal to make apps responsible for lifecycle decisions
- Any proposal that requires kernel scheduler or MPK domain changes
- Surface lifecycle FSM that has undefined states or transitions
- PD liveness query needed and not currently supported by existing infrastructure
- SurfaceId reuse after Destroy that current allocator cannot prevent

PROOF SCENARIOS:
1. `create → map → visible → focus → close → tombstone → destroy`
2. `create → map → visible → focus → minimize → restore → focus`
3. `focused surface dies → focus cleared → fallback chosen → tombstone retained`
4. `drag active → close requested → drag cancelled → close proceeds`
5. `resize active → minimize requested → resize cancelled → minimized`
6. `unknown SurfaceId focus request → rejected`
7. `destroyed SurfaceId reused → rejected or STOP FIRST gap documented`
8. `close already closing/tombstoned surface → idempotent no-op`
9. `sexdisplay receives stale visual update → bounded no-op/render-safe behavior`
10. `Frame with last tab closed → frame empty/tombstoned/collapsed according to spec`

MINIMAL IMPLEMENTATION PHASE LADDER (separate sub-prompts):
1. **Audit-only phase** — Find current SurfaceId, focus, drag, frame light, and display update paths. Produce mismatch report. No code.
2. **FSM spec phase** — Write `docs/handoff/COMPOSITOR_LIFECYCLE_FSM_V1.md` with state tables, transition tables, invariants, and STOP FIRST gaps.
3. **Shell-only model phase** — Add lifecycle state tracking inside `silk-shell` only. No behavior change beyond guards/log markers.
4. **Focus validity phase** — Prevent focus/drag/resize against invalid, closing, tombstoned, destroyed, or hidden surfaces. Add recovery fallback.
5. **Frame Light action phase** — Wire red/yellow/green as user actions: red=close, yellow=minimize, green=zoom/unzoom. Do not redefine them as status lights.
6. **Tombstone/debug phase** — Preserve tombstone records for crash recovery, Bell events, Mesh visibility, and Quil proof console.
7. **Display conformance phase** — sexdisplay renders only shell-provided bounded visual state. No lifecycle policy, no framebuffer path changes.
8. **Deterministic scenario proof phase** — Run scripted transitions and negative cases. Save proof output.

FUTURE SUB-PROMPT NAMES:
- `A1_COMPOSITOR_LIFECYCLE_AUDIT_V1`
- `A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1`
- `A3_SHELL_LIFECYCLE_MODEL_V1`
- `A4_FOCUS_VALIDITY_GUARDS_V1`
- `A5_FRAME_LIGHT_ACTIONS_V1`
- `A6_TOMBSTONE_DEBUG_EVENTS_V1`
- `A7_DISPLAY_CONFORMANCE_V1`
- `A8_LIFECYCLE_PROOF_SCENARIOS_V1`

HANDOFF NOTES TO SAVE:
- docs/handoff/COMPOSITOR_LIFECYCLE_FSM_V1.md: full state machine definition
- docs/handoff/COMPOSITOR_LIFECYCLE_PROOF_V1.md: proof scenarios and results
- docs/handoff/DEAD_PD_SURFACE_CLEANUP_V1.md: dead PD detection and surface cleanup protocol
- Optionally propose baseline invariant updates in handoff; do not edit stable baseline unless explicitly requested

CROSS-TRACK DEPENDENCY:
B_APP_LAUNCH_SESSION_RESTORE must not begin implementation until A_COMPOSITOR_LIFECYCLE has at least A1–A4 complete: audit, FSM spec, shell lifecycle model, and focus validity guards.

BACKUP BEFORE CHANGES.
READ HANDOUTS FIRST.
EOF_TRACK
```

---

### B_APP_LAUNCH_SESSION_RESTORE

```bash
cat > /tmp/B_APP_LAUNCH_SESSION_RESTORE.prompt <<'EOF_TRACK'
MISSION: B_APP_LAUNCH_SESSION_RESTORE_PLAN_V1 — Design launch manifests, app identity, Scene restore journal, and crash-safe restart for Silk DE. Docs/plan only. No implementation.

CONTEXT:
- SexOS / Silk DE capability-native microkernel
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- No POSIX process model; no Linux .desktop compatibility
- App identity is capability-native, not filename/path identity
- Shell owns Scene restore policy; sexdisplay does not restore apps or sessions
- This track depends on Track A (COMPOSITOR_LIFECYCLE). Must not begin implementation until A1–A4 complete: audit, FSM spec, shell lifecycle model, and focus validity guards
- See: docs/STABLE_BASELINE_20260503.md for locked invariants
- See: docs/AGENT_README_FIRST.md for token discipline
- See: docs/handoff/SURFACE_ID_LIFETIME_PATCH_V1.md for surface ID lifetime semantics

WHY THIS TRACK IS SEPARATE:
Launch and session restore require a stable compositor lifecycle FSM (Track A) as foundation. Rushing restore would create fragile state that resurrects invalid capabilities, surfaces, and trust. This track designs the identity/manifest/journal layer independently from implementation.

INNOVATION GOAL:
SexOS launch/session restore should feel like restoring a living capability graph, not reopening processes. Every restored app, document, surface, Scene, and grant must explain why it is allowed to exist again.

OBJECT MODEL:
- **AppIdentity:** capability-native app identifier. Not a filename or path. Survives path/name changes if trust identity is unchanged.
- **LaunchManifest:** describes authority, surfaces, expected services, and restore hints for one app.
- **LaunchIntent:** a recorded request to launch an app with specific capability/document refs.
- **SceneRestoreJournal:** ordered journal of active Scenes, frames, tabs, surfaces, and their app identities at session end.
- **RestoreEntry:** one entry in the journal — app identity + capability snapshot + surface/tab/frame placement + document refs.
- **RestartPolicy:** rules governing when and how to relaunch a crashed or missing app.
- **CapabilityGrantSnapshot:** the set of Collar grants an app held at journal time, stored as capability refs, not raw tokens.
- **DocumentRestoreRef:** Linen document reference that survives path/identity changes.
- **SurfaceRestoreRef:** reference to a surface identity from the journal, validated against Track A lifecycle before restore.

EXPECTED DESIGN OUTPUT SECTIONS:
1. Mission: docs/plan only — no implementation in this prompt
2. Dependency on Track A (A1–A4 must be complete before B implementation begins)
3. Object model definitions for all 9 objects above
4. Launch manifest schema: fields, types, required vs optional, extension rules
5. Scene restore journal schema: journal format, entry lifecycle, validation rules
6. Crash-safe restart policy: conditions, limits, user intent preservation, crash-loop bounding
7. Ownership boundaries: shell vs launcher vs sexdisplay vs Linen vs Collar vs Mesh vs Bell vs Quil
8. App identity model: how identity is established, verified, survived across versions/paths
9. CapabilityGrantSnapshot design: what gets saved, what does not, how trust is re-validated
10. SurfaceRestoreRef validation against Track A lifecycle states
11. DocumentRestoreRef resolution through Linen
12. Proof scenarios (see below)
13. Minimal implementation phase ladder
14. Future sub-prompt names B1–B7

INVARIANTS:
- A restore entry cannot create authority it did not previously have.
- A restore entry cannot bypass Collar trust.
- A restore entry cannot resurrect destroyed SurfaceIds.
- A crashed app cannot mark itself trusted on restart.
- Scene restore must tolerate missing apps/documents/devices.
- Restore must be partial, explainable, and reversible.
- App identity must survive path/name changes if trust identity is unchanged.
- Unknown or stale manifest fields are ignored or rejected deterministically.
- sexdisplay only renders restored surfaces after shell validates them.
- User documents are restored through Linen refs, not raw paths.
- Restart loops must be bounded.
- Failed restore must produce Bell/Mesh/Quil-visible proof events.

STOP FIRST CONDITIONS:
- Any proposed change to kernel scheduler or MPK domain switching
- Any proposed change to PDX ABI or opcode semantics
- Any POSIX process/session model assumption
- Linux .desktop compatibility as foundation for launch manifests
- Raw path-based trust for app identity or document location
- Restoring raw pointers or cross-PD memory references
- sexdisplay owning session restore policy
- Linen owning app lifecycle decisions
- Unbounded restart loops without limit/backoff
- Trusting crashed app-provided state without validation
- Any proposal that assumes Track A lifecycle exists before A1–A4 are proven

FILES/DOCS TO INSPECT FIRST (use rg, read only relevant snippets):
- servers/silk-shell/src/main.rs: current surface tracking, panel toggle, any existing launch/restore logic
- servers/sexdisplay/src/main.rs: surface create/destroy opcodes
- crates/silkbar-model/src/lib.rs: model state, surface ID constants
- docs/handoff/SURFACE_ID_LIFETIME_PATCH_V1.md: stale surface semantics
- docs/handoff/COMPOSITOR_LIFECYCLE_FSM_V1.md: lifecycle states (once written by Track A)
- docs/PDX_QUICKMAP.md: opcode reference
- docs/IPCPKU_MAP.md: domain isolation boundaries
- docs/SILK_DE_EXECUTION_PLAN.md: current architecture state

PROOF SCENARIOS:
1. Clean boot launches app from manifest with correct capabilities.
2. App opens document through Linen ref; document survives restore.
3. Scene journal records app + doc + frame/tab placement; full restore succeeds.
4. System restarts and restores Scene partially — some apps missing, rest succeed with safe placeholders.
5. App crashes; shell tombstones surface via Track A; restart policy decides whether to relaunch based on restart count and user preference.
6. Missing document restore produces safe placeholder, not panic.
7. Missing app restore produces tombstone/session card, not crash.
8. Revoked Collar grant blocks restore of previously granted capability.
9. Package identity changed (update); restore is rejected pending trust review through Collar.
10. Restart loop exceeds limit; Bell reports event; Mesh shows failed node.
11. sexdisplay receives no live surface until shell lifecycle validation from Track A passes.
12. User chooses "do not restore this app"; journal is updated/revoked for that entry.

MINIMAL IMPLEMENTATION PHASE LADDER (separate sub-prompts):
1. **App identity audit** — Find current app path/identity assumptions. Produce mismatch report. No code.
2. **Launch manifest spec** — Write `docs/handoff/LAUNCH_MANIFEST_SPEC_V1.md` with schema, field semantics, extension rules.
3. **Scene restore journal spec** — Write `docs/handoff/SCENE_RESTORE_JOURNAL_SPEC_V1.md` with journal format, entry lifecycle, validation rules.
4. **Crash-safe restart policy spec** — Write `docs/handoff/CRASH_SAFE_RESTART_POLICY_V1.md` with restart conditions, loop bounding, user intent preservation.
5. **Session restore validation** — Design validation gates that every RestoreEntry must pass before shell accepts it.
6. **Launcher boundary map** — Document the split between shell (policy) vs future launcher service (execution). No implementation.
7. **Restore proof scenarios** — Design deterministic test sequences for all 12 proof scenarios above.

FUTURE SUB-PROMPT NAMES:
- `B1_APP_IDENTITY_AUDIT_V1`
- `B2_LAUNCH_MANIFEST_SPEC_V1`
- `B3_SCENE_RESTORE_JOURNAL_SPEC_V1`
- `B4_CRASH_SAFE_RESTART_POLICY_V1`
- `B5_SESSION_RESTORE_VALIDATION_V1`
- `B6_LAUNCHER_BOUNDARY_MAP_V1`
- `B7_RESTORE_PROOF_SCENARIOS_V1`

HANDOFF NOTES TO SAVE:
- docs/handoff/APP_IDENTITY_MODEL_V1.md: app identity model
- docs/handoff/LAUNCH_MANIFEST_SPEC_V1.md: launch manifest schema
- docs/handoff/SCENE_RESTORE_JOURNAL_SPEC_V1.md: journal format and rules
- docs/handoff/CRASH_SAFE_RESTART_POLICY_V1.md: restart policy
- docs/handoff/LAUNCHER_BOUNDARY_MAP_V1.md: shell vs launcher service split
- Optionally propose baseline invariant updates in handoff; do not edit stable baseline unless explicitly requested

CROSS-TRACK DEPENDENCIES:
- A_COMPOSITOR_LIFECYCLE (A1–A4 must be complete before B implementation begins)
- E_PERSISTENT_STORAGE_MATURITY (journal storage for Scene restore)
- G_PACKAGE_TRUST_UPDATE_UX (app identity verification, trust roots)

BACKUP BEFORE CHANGES.
READ HANDOUTS FIRST.
EOF_TRACK
```

---

### C_TOUCHPAD_GESTURES

```bash
cat > /tmp/C_TOUCHPAD_GESTURES.prompt <<'EOF_TRACK'
MISSION: C_TOUCHPAD_GESTURES_PLAN_V1 — Design shell-owned touchpad gesture policy after USB HID pointer producer exists. Docs/plan only. No implementation.

CONTEXT:
- SexOS / Silk DE capability-native microkernel
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- USB HID pointer producer must exist before gesture implementation (not yet verified — audit before claiming)
- Track A (COMPOSITOR_LIFECYCLE) A4 focus validity guards must be complete before gesture targets are validated
- USB/XHCI/HID stack is not rewritten by this track
- Compositor lifecycle is not modified by this track
- sexdisplay does not own gesture policy
- Gesture recognition belongs in shell after normalized input
- See: docs/STABLE_BASELINE_20260503.md for locked invariants
- See: docs/AGENT_README_FIRST.md for token discipline
- See: docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md for current USB input state

DEPENDENCY GATES:
1. USB HID pointer producer exists and delivers normalized pointer events to shell (audit current sexinput→shell route before claiming)
2. Track A A4 focus validity guards are complete (surface lifecycle, focus validity, dead-surface recovery)
3. If USB HID pointer producer does not yet deliver normalized events, STOP FIRST — this track plans the gesture policy layer only, not the raw input pipeline

WHY THIS TRACK IS SEPARATE:
Touchpad gestures are a pure shell-policy concern that must not be coupled to USB/HID implementation or compositor lifecycle. They deserve independent design because gesture FSMs, target validation, and safety properties (cancel, lock, degrade) are complex enough to warrant their own spec, proof, and audit. Separating them from the rapid 12-prompt plan prevents gesture-driven scope creep in compositor or input work.

INNOVATION GOAL:
Silk gestures should be capability-aware spatial commands, not laptop-driver magic. Raw HID stays below; Silk interprets intentional movement against Scene/Frame/Tab state with deterministic guards, visible proof, and reversible actions.

INPUT PIPELINE BOUNDARY:
```
Raw HID device -> sexinput -> NormalizedPointerEvent -> shell -> GestureCandidate -> GestureIntent -> shell Action -> sexdisplay render
[USB stack]    [HID parser] [normalized coords/buttons]  [gesture FSM]  [validated target] [shell policy] [bounded pixels]
```
- USB/HID normalizes device reports only (sexinput domain)
- Shell owns gesture policy (silk-shell domain)
- sexdisplay only renders shell-provided visual state (sexdisplay domain)
- Apps never receive raw touchpad contacts in V1
- Gestures operate on focused/hovered/valid shell targets only
- No gesture may bypass Track A lifecycle validity
- No hidden app command execution from gestures

OBJECT MODEL:
- **PointerSample:** raw timestamped coordinates + button state from HID (sexinput domain)
- **NormalizedPointerEvent:** device-independent coordinates, button mask, contact count (cross-domain safe)
- **ContactSlot:** tracked finger/contact with position, velocity, age, slot_id
- **GestureCandidate:** a detected pattern of contact movement that may become a gesture (shell domain)
- **GestureState:** current FSM state of a candidate gesture (Idle, Tracking, Triggered, Committed, Cancelled)
- **GestureIntent:** validated gesture that maps to a shell policy Action (e.g. GestureIntent::ScrollFrame, GestureIntent::SwitchScene)
- **GestureTarget:** the shell object a gesture targets (Surface, Frame, Tab, Scene, Atlas, EdgeReveal)
- **GestureCommit:** a committed intent that is dispatched as a shell action
- **GestureCancelReason:** why a gesture was cancelled (target_invalid, focus_changed, threshold_unmet, contact_lost, timeout)
- **GestureProofEvent:** logged proof event with gesture type, target, state, result

GESTURE FSMs:

### Tap/click FSM
```
Idle -> (contact_count==1 && movement<threshold) -> TapTracking -> (contact_released && within_timeout) -> TapCommitted
                                                              -> (movement>=threshold) -> TapCancelled
                                                              -> (contact_released && outside_timeout) -> TapCancelled
```
Output: Click action dispatched through existing shell click-dispatch path (same as mouse click).
- If GestureTarget is chrome (frame light, tab strip, rim): dispatch chrome click action (close, minimize, zoom, drag).
- If GestureTarget is app content area: dispatch focus + click through existing surface click-handling path.
- V1 policy: tap on app content is equivalent to current mouse click — no new gesture-specific click protocol.
- Threshold constants: `TAP_MOVEMENT_THRESHOLD_PX: u32 = 10`, `TAP_TIMEOUT_TICKS: u64 = <fixed shell/input tick count>`.
  **WARNING:** Do not use std/time/sleep/thread APIs. Timeout source must be an existing deterministic shell/input tick/counter. If no such source exists, STOP FIRST and make V1 tap movement-only (no timeout).

### Two-finger scroll FSM
```
Idle -> (contact_count==2) -> ScrollTracking -> (cumulative_movement>=scroll_threshold) -> ScrollIntent
                                             -> (contact_count<2) -> ScrollCancelled
```
Output: ScrollIntent for shell UI only (V1). If no app scroll protocol exists, V1 may use scroll for shell/Atlas/Scene UI only or log proof placeholder. Do not invent app scroll IPC.
Threshold constant: `SCROLL_THRESHOLD_PX: u32 = 20`.

### Pinch zoom FSM
```
Idle -> (contact_count==2 && contact_distance_changes) -> PinchTracking -> (distance_delta>=zoom_threshold) -> ZoomIntent
                                                                      -> (contact_count<2) -> ZoomCancelled
```
Output: Shell Frame zoom/unzoom (Frame flag toggle) or Atlas scale if shell-owned. V1 pinch maps only to shell-owned Frame or Atlas zoom — it does not zoom app content. Only valid on frames with FRAME_FLAG_ZOOMED support.

### Three-finger Scene/Atlas swipe FSM
```
Idle -> (contact_count==3) -> SwipeTracking -> (horizontal_movement>=scene_switch_threshold) -> SceneSwitchIntent
                                           -> (vertical_up_movement>=atlas_threshold) -> AtlasOpenIntent
                                           -> (contact_count<3 || movement_wrong_direction) -> SwipeCancelled
```
Output: Scene switch or Atlas open intent. Validated against valid Scene IDs.

### Edge reveal FSM
```
Idle -> (cursor enters screen edge proximity zone && contact_count>=1) -> EdgeRevealPending -> (movement_away_from_edge >= reveal_threshold) -> RevealIntent
                                                                          -> (contact_lost || movement_back_toward_edge) -> RevealCancelled
```
Output: SilkBar/overlay reveal intent. Shell decides which overlay.
**Edge zone definition:** screen-edge proximity region (configurable constant, e.g. 10px from any screen edge). The gesture triggers when cursor is in this zone AND contacts indicate an inward swipe. Touchpad edge gestures are detected by cursor position in edge zone at gesture start, not by touchpad hardware zone.

### Cancel/recovery FSM
```
AnyGestureState -> (target_invalid) -> CancelWithReason(target_invalid)
AnyGestureState -> (focus_changed && policy==lock_target) -> ContinueLocked or CancelWithReason(focus_changed)
AnyGestureState -> (contact_lost) -> CancelWithReason(contact_lost)
AnyGestureState -> (timeout_exceeded) -> CancelWithReason(timeout)
```
Cancel policy: V1 uses lock-target-at-start; if locked target becomes invalid, cancel with proof.

### Gesture states vs existing InteractionState

The shell already has `InteractionState` (Idle, DragActive, HoverTarget, etc.) for window management. Gesture FSMs are **orthogonal** to InteractionState until gesture commit:

- Pre-commit: gesture FSMs run in parallel with InteractionState. A two-finger scroll in progress does not change InteractionState.
- On commit: `GestureIntent` maps to a shell `Action`, which may transition InteractionState (e.g., a commit that maps to rim drag will set `InteractionState::DragActive`).
- On cancel: InteractionState is unchanged.
- Conflict rule: if `InteractionState::DragActive` is set (user is in a drag operation), gesture FSMs are suspended until drag completes. Gesture processing resumes from Idle.
- Exception: tap/click may commit during InteractionState::Idle only. No click dispatch during active drag.

SHELL OWNERSHIP BOUNDARIES:
- **sexinput owns:** raw HID parsing, NormalizedPointerEvent production, device-level filtering
- **silk-shell owns:** Contact tracking, GestureCandidate FSM, GestureIntent validation, GestureTarget lookup, GestureCommit dispatch, GestureCancel, GestureProofEvent logging
- **sexdisplay owns:** rendering shell-provided visual feedback (scroll indicators, zoom bounds, edge glow) — only after shell sends render intents
- **Apps own:** nothing in V1 — gesture targets are shell objects (Frame, Scene, Atlas), not app content
- **Future:** app scroll protocol may deliver scroll intents to focused app

INVARIANTS:
1. Every committed gesture must have a valid GestureTarget — if target becomes invalid mid-gesture, cancel.
2. If focus changes mid-gesture, lock initial target (V1 policy: lock, do not chase focus). On gesture start, shell records GestureTarget plus target generation/lifecycle state if available. On commit, target is revalidated through Track A guards. If mismatch or invalid, cancel.
3. V1 thresholds and timeout values are fixed deterministic constants in shell source code. Future adjustments may be shell-owned and persisted only after deterministic validation and accessibility policy exist (see D_ACCESSIBILITY_STACK).
4. Raw contacts must not cross PD boundaries as unsafe pointers — only NormalizedPointerEvent crosses the sexinput->shell boundary. Raw multi-contact data is consumed by shell gesture recognizer and not forwarded to apps in V1. Apps receive only existing pointer/click path unless future capability protocol is approved.
5. Gesture output must be shell actions, not framebuffer writes — sexdisplay receives intents, not pixels.
6. Gesture cancel must leave shell layout in valid state (no half-zoomed frames, no mid-switch Scene).
7. Pinch/zoom cannot create impossible frame geometry — zoom flag toggle on valid frames only.
8. Swipe cannot switch to invalid Scene — destination Scene must exist within WORKSPACE_COUNT.
9. Edge reveal may reveal shell chrome visually, but must not change focus or deliver hidden actions while secure/locked/private surface policy is unresolved. Edge reveal cannot steal focus from any surface unless explicit user gesture follows reveal.
10. Touchpad gestures must degrade safely to pointer movement/click if unsupported contact count received.
11. Accessibility alternatives must remain possible later — gestures must not be the only way to invoke actions.
12. No gesture may bypass Track A lifecycle validity (focused surface must be focusable, target must not be tombstoned/destroyed).
13. A committed gesture must produce exactly one GestureProofEvent before dispatch.
14. Gesture proof events must include GestureType, TargetId, Result, and CancelReason (if cancelled).
15. Pre-commit gesture FSMs are orthogonal to InteractionState; gesture may not mutate InteractionState until commit.

STOP FIRST CONDITIONS:
- Any USB/XHCI rewrite proposed by this track
- Any HID report parser rewrite beyond needed audit notes
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
- Any gesture that modifies InteractionState before commit (gesture FSMs are orthogonal until commit)
- Any click dispatch during active drag/resize operation
- Any gesture timeout requiring std/time, sleep/thread, POSIX timers, or new kernel timing ABI
- Any new app scroll protocol or PDX ABI for scroll without explicit approval

FILES/DOCS TO INSPECT FIRST (use rg, read only relevant snippets):
- servers/sexinput/src/main.rs: current HID event production, pointer event delivery to shell
- servers/silk-shell/src/main.rs: current pointer event handling, interaction state machine, focus, drag
- docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md: USB input backend audit
- docs/handoff/INPUT_DELIVERY_TRACE_V1.md: input delivery path
- docs/handoff/INPUT_SOLVE_PLAN_V1.md: input solve plan
- docs/handoff/SEXINPUT_TO_SHELL_ROUTE_AUDIT_V1.md: sexinput->shell routing
- docs/handoff/SHELL_INTERACTION_STATE_V1.md: shell interaction FSM
- docs/handoff/FOCUS_CONTRACT_V1.md: focus contract
- docs/handoff/HIT_TEST_PRIORITY_V1.md: hit test priority
- docs/INPUT_USB_NEXT.md: USB input next steps
- docs/PDX_QUICKMAP.md: opcode reference
- docs/IPCPKU_MAP.md: domain isolation boundaries

PROOF SCENARIOS:
1. One-finger pointer movement remains unchanged — gesture FSM stays in Idle for contact_count < 2.
2. Tap candidate below movement threshold -> contact released within timeout -> TapCommitted -> click dispatch.
3. Tap candidate moves beyond threshold -> TapCancelled -> no click dispatch.
4. Two-finger movement above scroll threshold -> ScrollIntent -> shell logs gesture and stores intent (no app dispatch in V1).
5. Pinch distance delta above zoom threshold -> ZoomIntent on valid frame -> frame zoom flag toggled -> tile_visible_frames() called.
6. Three-finger horizontal swipe above threshold -> SceneSwitchIntent -> ACTIVE_SCENE_IDX changes -> tile_visible_frames() called.
7. Three-finger upward swipe above threshold -> AtlasOpenIntent -> shell policy action (future Atlas).
8. Edge reveal movement away from edge -> RevealIntent -> shell reveals overlay.
9. Target Surface becomes invalid (tombstoned/destroyed) mid-gesture -> CancelWithReason(target_invalid) -> shell layout unchanged.
10. Focus changes mid-gesture -> lock_target policy applied -> gesture continues on original target.
11. Invalid/unknown surface as gesture target -> GestureIntent validation rejects -> GestureCommit cancelled.
12. Unsupported contact count (4+ fingers) -> degrade safely -> no gesture FSM transition.
13. Gesture cancelled mid-pinch -> frame zoom flag unchanged -> tile_visible_frames() not called.
14. Accessibility keyboard alternative exists for each gesture action (document in C8 gate).
15. No deterministic tick source exists for tap timeout → tap timeout disabled, V1 tap is movement-only, or STOP FIRST gap documented.
16. Two-finger scroll over app content with no app scroll protocol → no fake app IPC; logs placeholder or shell-only intent.
17. Edge reveal over secure/locked surface → visual-only reveal or blocked according to policy; no focus steal.
18. Threshold settings requested by user (future) → deferred to accessibility/settings track; V1 constants remain deterministic.

PROOF MARKERS:
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

MINIMAL IMPLEMENTATION PHASE LADDER (separate sub-prompts):
1. **Input audit** — Inspect current sexinput->shell pointer event route. Verify NormalizedPointerEvent exists or document gap. No code.
2. **Gesture boundary spec** — Write `docs/handoff/GESTURE_BOUNDARY_SPEC_V1.md` defining the exact sexinput->shell gesture boundary, NormalizedPointerEvent format, and contact count semantics.
3. **Gesture FSM spec** — Write `docs/handoff/GESTURE_FSM_SPEC_V1.md` with state tables, transition tables, thresholds, and invariants for all 6 FSMs.
4. **Shell gesture model** — Add GestureState, GestureCandidate, GestureTarget tracking inside silk-shell. No behavior change beyond tracking and proof markers.
5. **Gesture target validity** — Wire GestureTarget validation against Track A lifecycle (focus validity, surface liveness, tombstone checks).
6. **Scene/Atlas gesture intents** — Implement SceneSwitchIntent and AtlasOpenIntent dispatch through existing shell scene/atlas mechanisms.
7. **Gesture proof scenarios** — Run deterministic test sequences for all 14 proof scenarios above.
8. **Accessibility alternatives gate** — Document keyboard/input alternative for every gesture action. Verify no action is gesture-only.

FUTURE SUB-PROMPT NAMES:
- `C1_TOUCHPAD_INPUT_AUDIT_V1`
- `C2_GESTURE_BOUNDARY_SPEC_V1`
- `C3_GESTURE_FSM_SPEC_V1`
- `C4_SHELL_GESTURE_MODEL_V1`
- `C5_GESTURE_TARGET_VALIDITY_V1`
- `C6_SCENE_ATLAS_GESTURE_INTENTS_V1`
- `C7_GESTURE_PROOF_SCENARIOS_V1`
- `C8_ACCESSIBILITY_ALTERNATIVES_GATE_V1`

HANDOFF NOTES TO SAVE:
- docs/handoff/GESTURE_BOUNDARY_SPEC_V1.md: sexinput->shell gesture boundary
- docs/handoff/GESTURE_FSM_SPEC_V1.md: gesture FSM definitions
- docs/handoff/GESTURE_PROOF_V1.md: proof scenarios and results
- docs/handoff/GESTURE_ACCESSIBILITY_GATE_V1.md: accessibility alternatives
- Optionally propose baseline invariant updates in handoff; do not edit stable baseline unless explicitly requested

CROSS-TRACK DEPENDENCIES:
- A_COMPOSITOR_LIFECYCLE (A4 focus validity guards must be complete before gesture targets validated)
- D_ACCESSIBILITY_STACK (gesture alternatives must be accessible through keyboard/input)
- USB HID pointer producer (must exist before gesture implementation)

ACCESSIBILITY NOTE:
D_ACCESSIBILITY_STACK must review gesture alternatives before gesture settings become user-configurable. Track C V1 must not block keyboard/pointer alternatives. Every gesture action must have a documented keyboard/input alternative path (verified in C8 gate).

BACKUP BEFORE CHANGES.
READ HANDOUTS FIRST.
EOF_TRACK
```

---

### D_ACCESSIBILITY_STACK

```bash
cat > /tmp/D_ACCESSIBILITY_STACK.prompt <<'EOF_TRACK'
MISSION: D_ACCESSIBILITY_STACK_PLAN_V1 — Design a capability-safe accessibility stack for Silk DE: semantic roles, keyboard navigation, focus narration, and input alternatives. Docs/plan only. No implementation.

CONTEXT:
- SexOS / Silk DE capability-native microkernel
- no_std Rust, MPK/PKU, PDX IPC only
- sexdisplay sole framebuffer writer, shell owns policy
- No POSIX accessibility assumptions (no AT-SPI, no DBus, no Linux accessibility bus)
- No audio/speech engine implementation in V1 — narration is event-log/proof-first, not speech-first
- No app memory scraping — apps expose intentional semantic surfaces only through explicit future capability protocol
- Track A (COMPOSITOR_LIFECYCLE) A4 focus validity guards must be complete before keyboard navigation validates targets
- Track C (TOUCHPAD_GESTURES) C8 gate must verify gesture alternatives exist before gesture settings become user-configurable
- Bell may surface accessibility/security events but does not own navigation policy (Bell integration is future — V1 narration is proof markers only)
- Quil may inspect proof logs but does not own runtime policy (Quil integration is future — V1 does not require Quil infrastructure)
- Collar may mediate grants for sensitive semantic exposure (Collar integration is future)
- Mesh may visualize semantic/capability graph but does not decide policy (Mesh integration is future)
- sexdisplay does not infer semantics from pixels — no OCR/screen scraping
- See: docs/STABLE_BASELINE_20260503.md for locked invariants
- See: docs/AGENT_README_FIRST.md for token discipline
- See: docs/handoff/FOCUS_CONTRACT_V1.md for current focus model
- See: docs/handoff/SHELL_INTERACTION_STATE_V1.md for interaction FSM

DEPENDENCY GATES:
1. Track A A4 focus validity guards must be complete before keyboard navigation validates focus targets
2. Track C C8 accessibility alternatives gate must verify gesture alternatives exist before gesture settings become user-configurable
3. Audio/speech engine is NOT required for V1 — narration is proof/event-log only
4. App semantic protocol is NOT required for V1 — V1 covers shell chrome semantics only (SilkBar, Frame Lights, Scene/Frame/Tab selection)

WHY THIS TRACK IS SEPARATE:
Accessibility is not a feature — it is a cross-cutting semantic layer that must be designed independently from compositor, input, and app protocol work. It requires its own object model (SemanticNode, SemanticTree, FocusPath), its own policy (what gets narrated, what is private), and its own proof layer (what was announced, skipped, denied). Coupling it to the rapid 12-prompt plan would force accessibility through a pixel/focus-only lens without semantic intent.

INNOVATION GOAL:
SexOS accessibility should be a capability-scoped semantic layer, not an afterthought screen scraper. Apps expose intentional semantic surfaces; Silk validates focus/navigation; Bell/Quil can prove what was announced, selected, skipped, or denied.

OBJECT MODEL:
- **SemanticNode:** a shell-accessible element with role, label, state, children. Owned by shell for chrome; by app for app content (future). V1 SemanticNode children are flat (no deep tree hierarchy — tree nesting reserved for future app content merging).
- **SemanticTree:** ordered list of navigable SemanticNodes for the current Scene. V1 SemanticTree is a flat list — tree hierarchy (parent/child containers) is reserved for future app content semantic merging.
- **SemanticRole:** the kind of UI element (Frame, Tab, SceneToggle, FrameLight, Button, Slider, TextOutput, etc.). Fixed enum in V1.
- **SemanticLabel:** human-readable text for a SemanticNode. For shell chrome: deterministic from model state. For app content: future capability protocol.
- **FocusPath:** current navigation path through the SemanticTree — which node is focused, what navigation direction was taken, what was skipped.
- **AccessibleAction:** an action that can be performed on a SemanticNode (Activate, Close, Minimize, Zoom, SwitchToTab, SwitchScene, OpenAtlas, etc.). Maps to shell intents or app capabilities.
- **NavigationIntent:** a user navigation request (next, previous, first, last, activate, escape). Filtered through AccessibilityPolicy.
- **NarrationEvent:** a structured log of what was narrated/focused/denied. Contains timestamp, FocusPath, SemanticRole, SemanticLabel, AccessibleAction, result (narrated/skipped/denied).
- **InputAlternative:** a non-gesture input path for an action (keyboard shortcut, switch device, future speech). V1 covers keyboard alternatives only.
- **AccessibilityPolicy:** deterministic rules for what is narratable, what is private, what navigation directions are allowed, what input alternatives exist. V1 policy is hardcoded in shell source code — no runtime configuration.
- **AccessibilityProofEvent:** logged event with role, label, target_id, action, result. Used by Bell (attention) and Quil (proof console).

SEMANTIC ROLE MODEL (V1 shell chrome):
```
SemanticRole ::=
    | SilkBar            // top bar clock/chip/panel area
    | SceneToggle        // workspace toggle (chips)
    | Panel              // launcher, status, clock, bell panels
    | FrameLight         // close/minimize/zoom lights
    | FrameBody          // app content area (opaque role — no app content scraping)
    | Tab                // tab strip slot
    | TabStrip           // tab strip container
    | FrameRim           // resize/move rim
    | AtlasOverview      // Scene overview (future)
    | SettingsPanel      // scene/chrome settings UI
    | Unknown            // safe fallback
```
V1: shell chrome only. App content roles are reserved for future capability protocol.

KEYBOARD NAVIGATION MODEL (V1):
```
Navigation flow: FocusPath follows SemanticTree order within active Scene.
- Tab / Shift+Tab: cycle forward/backward through navigable SemanticNodes
- Arrow keys: directional navigation within a container (e.g., tabs within TabStrip, lights within FrameLight cluster)
- Enter/Space: activate AccessibleAction for focused node
- Escape: close panel/dialog, or return to Scene-level navigation
- Ctrl+W: close focused frame/tab (equivalent to red light)
- Ctrl+M: minimize focused frame (equivalent to yellow light)
- Ctrl+Shift+Enter: zoom/unzoom focused frame (equivalent to green light)
- Ctrl+Tab: cycle focused tab within active frame
- Ctrl+Shift+Tab: cycle tabs backward
- Alt+[1-9]: switch to Scene N
- Alt+Up: open Atlas overview
```
**WARNING:** These key bindings are **speculative** — audit current keyboard dispatch in silk-shell before implementing. Some bindings may already exist (e.g., F5 for scene settings). Conflicts must be documented and resolved before V1 keyboard navigation is wired.
Policy: keyboard navigation cannot focus destroyed/tombstoned/minimized surfaces. Navigation skips hidden/focus-ineligible nodes.

FOCUS NARRATION/EVENT MODEL:
```
Narration is event-log-first, not speech-first.
On each focus change (or poll on shell state in V1 if focus events do not yet exist):
1. Shell resolves focused SemanticNode (role + label)
2. Shell logs NarrationEvent with:
   - timestamp, FocusPath, SemanticRole, SemanticLabel
   - AccessibleAction list for the node (derived on-demand from node type + state, not stored per node)
   - result: narrated | skipped | denied
3. If label missing: fallback to role name + node ID ("Frame Light Close, frame 2")
4. If node is private/secure: log [accessibility.narrate.denied] reason=private, do NOT expose semantics
5. V1 NarrationEvents are proof markers (serial_println! format). Structured event logs for Bell/Quil are future — do not require Bell/Quil infrastructure for V1.
```

INPUT ALTERNATIVES MODEL:
```
Every gesture action in Track C must have an input alternative in V1:
| Gesture              | Alternative                    |
|----------------------|--------------------------------|
| One-finger tap/click | Click (unchanged)              |
| Two-finger scroll    | Arrow keys / scroll wheel      |
| Pinch zoom           | Ctrl+Shift+Enter (zoom toggle) |
| Three-finger swipe   | Alt+[1-9] / Alt+Up             |
| Edge reveal          | Ctrl+` / dedicated key         |
```
V1 input alternatives are keyboard-only. Future may add switch devices, speech, or other alternatives (see D_ACCESSIBILITY_STACK future phases).

OWNERSHIP BOUNDARIES:
- **shell owns:** SemanticTree construction for chrome, NavigationIntent filtering, FocusPath traversal, NarrationEvent production (V1: proof markers), AccessibilityPolicy enforcement
- **sexdisplay owns:** nothing — no semantic inference from pixels, no narration rendering
- **silkbar owns:** nothing — silkbar chips/chrome may be semantic sources, but shell constructs the tree
- **apps own (future):** app content SemanticNode production through explicit capability protocol; shell validates and merges into tree
- **Bell owns (future):** surfacing accessibility/security events from NarrationEvent stream — V1 does not require Bell infrastructure
- **Quil owns (future):** inspecting AccessibilityProofEvent logs for debug/audit — V1 proof markers are serial_println! only
- **Collar owns (future):** mediating grants for sensitive app semantic exposure
- **Mesh owns (future):** visualizing semantic/capability graph — does not decide policy

INVARIANTS:
1. Focus narration must correspond to a shell-valid focus target — never a destroyed/tombstoned/invalid surface.
2. Semantic nodes cannot grant authority — they describe UI state, not control access.
3. Semantic labels are untrusted until validated by capability/policy — app-provided labels are assertions, not facts.
4. Hidden/private/secure surfaces do not expose semantics unless explicit policy allows — denied narration is logged.
5. Keyboard navigation cannot focus destroyed/tombstoned/invalid surfaces — same guards as Track A A4.
6. Every AccessibleAction maps to an explicit shell intent or app capability — no hidden action execution.
7. Input alternatives must not bypass lifecycle/focus guards — keyboard navigation respects same target validation as pointer.
8. AccessibilityPolicy must be deterministic and reversible — settings changes rebuild the tree, no hidden state.
9. No NarrationEvent may leak private document/app content across PD boundaries — role+label are shell-validated.
10. sexdisplay never derives semantics from framebuffer pixels — no OCR, no scraping, no pixel-reading for accessibility.
11. Missing semantic label must degrade safely to role-based fallback identification.
12. Accessibility must provide keyboard alternatives for touchpad gestures where possible (Track C C8 gate).
13. V1 narration is event-log-only — no speech, no audio, no TTS engine dependency.
14. V1 semantics are shell chrome only — app content semantics require future capability protocol.

STOP FIRST CONDITIONS:
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

FILES/DOCS TO INSPECT FIRST (use rg, read only relevant snippets):
- servers/silk-shell/src/main.rs: current focus model, keyboard handling, interaction state, frame/tab model
- servers/silk-shell/src/main.rs: FrameLight actions, scene switch, panel toggle (keyboard equivalents)
- docs/handoff/FOCUS_CONTRACT_V1.md: current focus contract
- docs/handoff/SHELL_INTERACTION_STATE_V1.md: interaction FSM
- docs/handoff/HIT_TEST_PRIORITY_V1.md: hit test model (for understanding target resolution)
- docs/PDX_QUICKMAP.md: opcode reference
- docs/IPCPKU_MAP.md: domain isolation boundaries

PROOF SCENARIOS:
1. Keyboard Tab cycles focus across valid visible frames only — skips destroyed/tombstoned/hidden.
2. Keyboard navigation skips minimized frames unless explicit restore action.
3. Ctrl+W triggers close intent (red light equivalent) on focused frame — dispatches through existing close path.
4. Ctrl+M triggers minimize intent (yellow light equivalent) on focused frame.
5. Ctrl+Shift+Enter triggers zoom/unzoom intent (green light equivalent) on focused frame.
6. Alt+[1-9] switches to Scene N — same effect as workspace chip click or three-finger swipe.
7. Alt+Up opens Atlas overview — same effect as three-finger upward swipe.
8. Focus on frame light narrates role+label ("Close light, frame 2") — NarrationEvent logged.
9. Focus on surface with no semantic label falls back to role+ID ("Frame body, frame 3").
10. Hidden/private surface focus request — narration denied, logged as [accessibility.narrate.denied].
11. Destroyed/tombstoned surface cannot be narrated — guard rejects before narration.
12. App without semantic protocol — shell chrome navigation still works; app content area has opaque role.
13. Keyboard navigation during active gesture — suspended until gesture completes (same as InteractionState rule).
14. Missing narratable target in Scene — keyboard navigation reports empty Scene fallback.
15. Input alternative for every gesture action verified — Track C C8 gate cross-check.

PROOF MARKERS:
```
[accessibility.narrate] role=FrameLight label="Close light, frame 2" target=N result=narrated
[accessibility.narrate.denied] role=FrameBody target=N reason=private|secure|invalid
[accessibility.narrate.fallback] target=N fallback=role+id role=FrameBody
[accessibility.navigate] direction=next|prev|first|last|activate|escape from=N to=N
[accessibility.navigate.skipped] target=N reason=minimized|tombstoned|hidden|invalid
[accessibility.action] action=close|minimize|zoom|scene_switch|atlas_open target=N result=dispatched|denied
[accessibility.action.denied] action=close|minimize|zoom|scene_switch target=N reason=target_invalid|no_capability
```

MINIMAL IMPLEMENTATION PHASE LADDER (separate sub-prompts):
1. **Accessibility audit** — Inspect current keyboard handling, focus model, frame/tab actions. Identify gaps for semantic roles, narration, keyboard equivalents. No code.
2. **Semantic role spec** — Write `docs/handoff/SEMANTIC_ROLE_SPEC_V1.md` defining SemanticRole enum, SemanticNode structure, SemanticTree construction rules for shell chrome.
3. **Keyboard navigation model** — Design FocusPath traversal through SemanticTree. Add keyboard navigation constants (direction, skip rules, fallback). Shell model only.
4. **Focus narration event log** — Implement NarrationEvent production on focus change. Log role+label per focus. No speech/audio.
5. **Input alternatives model** — Wire keyboard equivalents for gesture actions (Scene switch, Atlas open, zoom, minimize, close). Cross-check against Track C gestures.
6. **Accessibility capability policy** — Define AccessibilityPolicy rules for private/secure surfaces, denied narration, capability-gated semantic exposure.
7. **Shell chrome SemanticTree capture** — Build SemanticTree from current shell model (Frames, Tabs, Scene, FrameLights). No behavior change — tree is constructed and logged but not yet used for navigation.
8. **Narration + keyboard navigation wire** — Wire NarrationEvent log to focus changes. Wire keyboard navigation through FocusPath traversing SemanticTree. Wire key bindings through existing shell keyboard dispatch.
9. **Accessibility proof scenarios** — Run deterministic test sequences for all 15 proof scenarios above.

FUTURE SUB-PROMPT NAMES:
- `D1_ACCESSIBILITY_AUDIT_V1`
- `D2_SEMANTIC_ROLE_SPEC_V1`
- `D3_KEYBOARD_NAVIGATION_MODEL_V1`
- `D4_FOCUS_NARRATION_EVENT_LOG_V1`
- `D5_INPUT_ALTERNATIVES_MODEL_V1`
- `D6_ACCESSIBILITY_CAPABILITY_POLICY_V1`
- `D7_SHELL_CHROME_SEMANTIC_TREE_V1`
- `D8_NAVIGATION_NARRATION_WIRE_V1`
- `D9_ACCESSIBILITY_PROOF_SCENARIOS_V1`

HANDOFF NOTES TO SAVE:
- docs/handoff/SEMANTIC_ROLE_SPEC_V1.md: semantic role definitions
- docs/handoff/KEYBOARD_NAVIGATION_MODEL_V1.md: keyboard navigation spec
- docs/handoff/FOCUS_NARRATION_EVENT_LOG_V1.md: narration event format
- docs/handoff/INPUT_ALTERNATIVES_V1.md: input alternatives mapping
- docs/handoff/ACCESSIBILITY_CAPABILITY_POLICY_V1.md: capability-gated semantic policy
- docs/handoff/ACCESSIBILITY_PROOF_V1.md: proof scenarios and results
- Optionally propose baseline invariant updates in handoff; do not edit stable baseline unless explicitly requested

CROSS-TRACK DEPENDENCIES:
- A_COMPOSITOR_LIFECYCLE (A4 focus validity guards must be complete before keyboard navigation validates targets)
- C_TOUCHPAD_GESTURES (input alternatives must cover all gesture actions — verified in C8 gate)
- E_PERSISTENT_STORAGE_MATURITY (future accessibility settings persistence)
- G_PACKAGE_TRUST_UPDATE_UX (future app semantic protocol capability grants)

BACKUP BEFORE CHANGES.
READ HANDOUTS FIRST.
EOF_TRACK
```
