# A_COMPOSITOR_LIFECYCLE_PLAN_V1

**Status:** Plan only. No implementation.

**Core principle:** Shell-owned Surface/Tab/Frame lifecycle FSM for Silk DE. Compositor lifecycle is the foundation for all window management — focus, gestures, tiling, session restore, accessibility. Sexdisplay renders shell-provided visual state only and never decides lifecycle policy.

## 1. Mission

Design the real shell-owned Surface/Tab/Frame lifecycle FSM for Silk DE. Define states, transitions, focus validity, frame light actions, sexdisplay conformance, proof markers, and safety boundaries. Docs/plan only. No implementation.

## 2. Context / References

- **`rapid/PHASE_02_SHELL_SURFACE_OWNERSHIP_SCENE_FRAME_TAB.md`** — existing ~70% complete shell state (SurfaceAction, ShellInteractionState, Frame model, tabs, lights, focus)
- **`docs/phase25-compositor.md`** — historical PDX + compositor contract (reference only; not build authority)
- **`docs/SEPARATE_TRACKS_AFTER_12_PROMPTS.md`** — track separation defining A as foundation for all other window management tracks

Current state: SurfaceAction enum, ShellInteractionState FSM, ShellFrame with tabs, close/minimize/zoom lights, chrome hit-testing, top bar, focus management (`try_set_focus`, `clear_focus_if_dead`), rim drag FSM exist. Missing: unified lifecycle FSM, tombstone slot rotation, scene model integration, formal focus validity guards.

## 3. Ownership Boundaries

- **silk-shell** (exclusive): all lifecycle policy, focus decisions, tab/frame/scene management, hit-test dispatch, input routing policy.
- **sexdisplay** (read-only consumer): renders surface chrome and content per shell's visual state commands. Never decides lifecycle state, focus, or policy.
- **App PDs**: may request focus through PDX messages but shell validates all lifecycle guards. Apps cannot force focus, bypass close guards, or set lifecycle state.

## 4. Object Model

- **Surface:** a renderable entity managed by shell. Has SurfaceId, geometry, frame membership, lifecycle state, z-order.
- **SurfaceId:** unique u64 identifier for each surface. Sequential allocation (100=APP, 101=STATIC, 200=Linen, 201=Quil…). Never reused without generation safety.
- **Tab:** shell-side representation of an open surface within a Frame. Has title_id, surface_id reference, active/pinned state.
- **Frame:** a ShellFrame struct with tabs array (max 4), flags (MINIMIZED, ZOOMED, TOP_BAR), normal geometry, active tab index. Frames are the visual container for surfaces.
- **Scene:** a workspace context with its own set of frames. Active scene tracked by `ACTIVE_SCENE_IDX`. Focus belongs to a scene.
- **Atlas:** a launcher/app overview view showing all active surfaces. Does not own lifecycle — displays shell-provided state.
- **FocusRef:** a (SurfaceId, LifecycleGeneration) pair used to validate that a focus target is still valid. Rejected if surface has progressed past the generation.
- **LifecycleGeneration:** monotonic u64 counter incremented on each Tombstoned/Destroyed transition. Used to detect stale references.
- **TombstoneRecord:** contains SurfaceId, destroy timestamp, generation at destroy, caller identity. Used to prevent premature ID reuse and to display tombstoned state.
- **LifecycleProofEvent:** proof marker for any lifecycle transition. Contains sequence_id, operation, SurfaceId, old_state, new_state, focus_impact.

## 5. Lifecycle FSM

8 surface states + 1 separate focus state:

```
                     ┌──────────────────────────────────┐
                     │                                  │
                     v                                  │
  Allocated ──→ Mapped ──→ Visible ──→ Hidden          │
                    │         │            │            │
                    │         v            │            │
                    │    Minimized ←───────┘            │
                    │         │                         │
                    v         v                         │
                 Closing ←─── (all live states)         │
                    │                                   │
                    v                                   │
              Tombstoned                                │
                    │                                   │
                    v                                   │
              Destroyed (terminal) ─────────────────────┘
```

- **Allocated:** SurfaceId reserved but no frame mapped. No display state. Transitions: → Mapped (on frame attach).
- **Mapped:** Surface attached to a Frame. Has geometry but may not be visible (behind other surfaces, off-screen). Transitions: → Visible, → Closing.
- **Visible:** Surface in active scene, frame not minimized, z-order includes it. Receives input events if focused. Transitions: → Hidden (scene switch), → Minimized, → Closing.
- **Hidden:** Surface's frame is in a non-active scene. Geometry exists but no input routing. Transitions: → Visible (scene switch), → Closing.
- **Minimized:** Frame collapsed to Atlas/SilkBar. Surface hidden. No pointer focus. Transitions: → Visible (restore), → Closing.
- **Closing:** Close requested. Guards check closeable, alive, non-tombstoned. Transitions: → Tombstoned.
- **Tombstoned:** Surface dead but record exists. Cannot receive focus. Transition: → Destroyed (after reclamation timeout).
- **Destroyed:** Terminal. SurfaceId eligible for reuse only after generation safety guarantee.

**Focus is separate from lifecycle state.** `focused_surface: Option<(SurfaceId, LifecycleGeneration)>` is a shell variable that must point to a Visible or Mapped (if no Visible surface in scene) surface. Focus never targets Tombstoned, Closing, or Destroyed surfaces.

## 6. Allowed Transitions

| From | To | Trigger | Guard | Proof Marker | Failure |
|------|----|---------|-------|-------------|---------|
| Allocated | Mapped | Frame attach | SurfaceId valid, frame slot available | `[comp.surface.map]` | No slot → `[comp.surface.map.reject]` |
| Mapped | Visible | Scene activation, un-minimize | Frame in active scene, not minimized | `[comp.surface.visible]` | Scene inactive → Hidden |
| Mapped | Closing | Close request | Surface closeable, alive | `[comp.surface.close]` | Not closeable → `[comp.surface.close.reject]` |
| Visible | Hidden | Scene switch | New scene active, surface's scene inactive | `[comp.surface.hide]` | N/A |
| Visible | Minimized | Minimize action | Frame not already minimized | `[comp.surface.minimize]` | Already minimized → no-op |
| Visible | Closing | Close request | Same as Mapped→Closing | `[comp.surface.close]` | Same |
| Hidden | Visible | Scene switch back | Surface's scene becomes active | `[comp.surface.visible]` | Scene never active → Hidden |
| Hidden | Closing | Close request | Same | `[comp.surface.close]` | Same |
| Minimized | Visible | Restore | Frame un-minimized | `[comp.surface.visible]` | Surface destroyed → `[comp.surface.open.reject]` |
| Minimized | Closing | Close request | Same | `[comp.surface.close]` | Same |
| Closing | Tombstoned | Tombstone record created | Generation incremented | `[comp.surface.tombstone]` | Drag in progress → cancel first |
| Tombstoned | Destroyed | Reclamation timeout | Generation safety period elapsed | `[comp.surface.destroy]` | Generation collision → delay reclamation |

## 7. Forbidden Transitions

| From | To | Why |
|------|----|-----|
| Allocated | Visible/ Hidden/ Minimized | Must map to a frame first |
| Allocated | Tombstoned/ Destroyed | Surface has no content or authority to destroy |
| Mapped | Minimized | Must be Visible first — minimize is a visibility transition |
| Tombstoned | Visible/ Mapped/ Hidden/ Minimized | Cannot resurrect — must allocate new surface |
| Destroyed | Any | Terminal state |
| Closing | Visible/ Hidden/ Minimized | Close is irreversible — must complete to Tombstoned |
| Any (with active drag) | Closing/ Tombstoned/ Destroyed | Drag must cancel before lifecycle transition |

## 8. Focus Validity Rules

1. `focused_surface` must reference a Visible surface (or Mapped if no Visible surface in active scene).
2. `try_set_focus()` validates focus by verifying lifecycle state, visibility/focusability, active Scene membership, caller/event authority where available, and stale-generation rejection if generation exists.
3. On Tombstoned/Destroyed of focused surface → `clear_focus_if_dead()` selects next valid surface from z-order.
4. Apps cannot force focus — `try_set_focus()` rejects focus requests from non-shell callers without proper SurfaceAction routing.
5. Focus never targets Minimized surfaces — `frame_accepts_input()` returns false for minimized frames.
6. On scene switch → focus transferred to the top Visible surface in the new scene. If none → focus cleared.
7. Drag/resize in progress → focus pinned to drag target until drag completes or cancels.

## 9. Frame Light Action Mapping

| Light | Action | Behavior | Guard | Proof Marker |
|-------|--------|----------|-------|-------------|
| Red (close) | Close active tab/surface | Surface → Closing → Tombstoned | `is_closeable_surface()` | `[comp.surface.close]` or `.close.reject` |
| Yellow (minimize) | Minimize frame | Visible → Minimized | `frame_accepts_input()` (false after minimize) | `[comp.surface.minimize]` |
| Green (zoom) | Toggle frame zoom | Toggle FRAME_FLAG_ZOOMED, save/restore geometry | Frame accepts zoom | `[comp.surface.zoom]` |

Frame lights are hit-testable regions in the chrome bar. Shell dispatches light actions via `frame_light_action()`. Lights are visual only — sexdisplay renders the light state but shell decides the action.

## 10. Sexdisplay Conformance Rules

1. sexdisplay renders shell-provided visual state: surface chrome, lights, top bar, tab strip, geometry. Never infers state.
2. sexdisplay never decides lifecycle transitions, focus changes, tab order, or scene membership.
3. All surface geometry from shell is bounds-checked via `clamp_position()` / `clamp_surface_size()` before sexdisplay receives it.
4. sexdisplay receives standard geometry/focus/create/destroy opcodes — no lifecycle-specific protocol. Exact opcode numbers are not plan canon; audit in A1 before use. Any lifecycle-related opcode numbers or protocol constants must be audited in A1 before implementation.
5. sexdisplay framebuffer writes are bounded — shell never sends out-of-bounds geometry.
6. sexdisplay has no authority to refuse or modify shell state commands — it renders what it receives.

## 11. Invariants

1. Destroyed is terminal — no transition out.
2. Destroyed IDs are never reused unless allocator guarantees generation safety (LifecycleGeneration monotonic).
3. Focus target must be live, visibility-valid (Visible/Mapped in scene), and lifecycle-valid (non-Tombstoned, non-Destroyed).
4. Minimized cannot receive pointer focus — `frame_accepts_input()` returns false.
5. Tombstoned is not live content — cannot be focused, rendered, or interacted with.
6. Close is idempotent — subsequent close on Closing/Tombstoned returns false.
7. Destroy is terminal — no undo from Destroyed.
8. Unknown SurfaceId rejects or no-ops deterministically — never panics.
9. Drag/resize cancels before close/tombstone/destroy — no lifecycle transition during drag.
10. Apps cannot force focus — `try_set_focus()` validates caller identity.
11. sexdisplay never decides lifecycle meaning, focus policy, tab/frame/scene policy.
12. sexdisplay renders only bounded pixels from shell-provided model.
13. No raw cross-PD pointers for surface state — all communication via PDX.
14. No shared backing-buffer redesign — existing PDX-only IPC.
15. No kernel/ABI/sex-pdx edits unless STOP FIRST.
16. Frame tabs are display-only — tab membership does not grant capability or authority.
17. LifecycleGeneration is monotonic — never decrements, wrap-around detected.
18. Surface allocations respect the reserved ID ranges (100=APP, 200=Linen, 201=Quil…).
19. All lifecycle transitions produce a proof marker — no silent state changes.

## 12. STOP FIRST Gates

- Any proposal allowing apps to force focus
- Any sexdisplay lifecycle or focus policy ownership
- Any Destroyed surface resurrection
- Any SurfaceId reuse without generation safety
- Any focus on Tombstoned or Minimized surface
- Any lifecycle transition during active drag/resize
- Any kernel/ABI/sex-pdx edit without explicit handoff
- Any raw shared buffer or backing-buffer redesign
- Any framebuffer bounds removal for surface geometry
- Any dynamic allocation for FSM state (static arrays only)
- Any animation system in lifecycle FSM
- Any tab-as-authority or frame-as-capability model
- Any close that bypasses Tombstoned state (must go through Closing→Tombstoned)
- Any sexdisplay protocol extension for lifecycle semantics
- Any implementation before A1 audit is complete

## 13. Proof Markers

```
[comp.audit.start]
[comp.surface.map]         [comp.surface.map.reject]
[comp.surface.visible]     [comp.surface.hide]
[comp.surface.minimize]    [comp.surface.minimize.reject]
[comp.surface.zoom]        [comp.surface.zoom.reject]
[comp.surface.close]       [comp.surface.close.reject]
[comp.surface.tombstone]
[comp.surface.destroy]
[comp.surface.focus.set]   [comp.surface.focus.reject]
[comp.surface.focus.clear]
[comp.scene.switch]
[comp.surface.geometry.update]
[comp.surface.cancel.drag]
[comp.error]
```

## 14. Negative Tests

| # | Scenario | Expected Result | Guard | Reject Marker |
|---|----------|----------------|-------|--------------|
| 1 | Focus Destroyed surface | `try_set_focus()`=false | generation check | `[comp.surface.focus.reject]` reason=destroyed |
| 2 | Focus Tombstoned surface | `try_set_focus()`=false | `is_tombstoned()` | `[comp.surface.focus.reject]` reason=tombstoned |
| 3 | Focus Minimized surface | `try_set_focus()`=false | `frame_accepts_input()`=false | `[comp.surface.focus.reject]` reason=minimized |
| 4 | Close already Closing surface | `close_surface()`=false | idempotent close guard | `[comp.surface.close.reject]` reason=already_closing |
| 5 | Close Tombstoned surface | `close_surface()`=false | `surface_is_alive()`=false | `[comp.surface.close.reject]` reason=already_dead |
| 6 | Destroy during drag | `destroy_surface()` blocked | drag FSM active | `[comp.surface.cancel.drag]` then retry |
| 7 | Unknown SurfaceId | no-op, no panic, return false | valid ID range check | `[comp.error]` reason=unknown_surface |
| 8 | Minimize already-minimized | no-op, stays minimized | `FRAME_FLAG_MINIMIZED` check | `[comp.surface.minimize.reject]` reason=already_minimized |
| 9 | App PD sends focus request | validated against caller; may reject if not shell | caller identity check | `[comp.surface.focus.reject]` reason=not_shell_caller |
| 10 | Scene switch with no Visible surface | focus cleared, no crash | `clear_focus_if_dead()` fallback | `[comp.surface.focus.clear]` reason=no_surface_in_scene |

## 15. Minimal Phase Ladder

1. **A1_COMPOSITOR_LIFECYCLE_AUDIT_V1** — Audit current shell lifecycle: SurfaceAction states, ShellInteractionState, frame/tab model, focus guards, tombstone existence, scene model gaps. Document current invariants and missing pieces. No code.

2. **A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1** — Define 8-state FSM: state definitions, allowed/forbidden transitions, guards, proof markers per transition. Handoff doc.

3. **A3_SHELL_LIFECYCLE_MODEL_V1** — Integrate FSM into shell: state tracking per SurfaceId, transition dispatch, LifecycleGeneration monotonic counter, generation-checked FocusRef.

4. **A4_FOCUS_VALIDITY_GUARDS_V1** — Formalize 5-guard `try_set_focus()`, `clear_focus_if_dead()` for Tombstoned/Destroyed focus targets, scene-switch focus transfer, caller identity validation.

5. **A5_FRAME_LIGHT_ACTIONS_V1** — Wire red/yellow/green frame light dispatch through FSM: close→Tombstoned path, minimize→Minimized state, zoom toggle. Drag cancellation before lifecycle transitions.

6. **A6_TOMBSTONE_DEBUG_EVENTS_V1** — TombstoneRecord with Generation, reclamation timeout, debug display in shell state. SurfaceId reuse safety.

7. **A7_DISPLAY_CONFORMANCE_V1** — Verify sexdisplay never receives lifecycle-invalid state: all geometry bounds-checked, no Tombstoned/Destroyed surfaces in display state. Add conformance proof markers.

8. **A8_LIFECYCLE_PROOF_SCENARIOS_V1** — Define all proof scenarios, negative tests, cross-reference with LifecycleProofEvent. Verify all allowed/forbidden transitions produce correct proof markers.

## 16. Scan 7 — Exceeded Hypothesis

Assume a rival shell beat Silk lifecycle across 10 dimensions:

| Rival Advantage | Why Silk Would Lose | SexOS-Native Fix | Invariant Preserved | Proof Gate |
|----------------|---------------------|------------------|-------------------|------------|
| Focus never lands on dead surface | Silk focus could target Tombstoned surface after clear_focus_if_dead() race | 5-guard try_set_focus() with generation check. FocusRef validated against LifecycleGeneration. | §11.3: Focus target must be live and lifecycle-valid | A4 |
| Close/minimize/zoom always predictable | Frame light dispatch might not match visual state | Lights dispatch through FSM: red→Closing→Tombstoned; yellow→Minimized; green→zoom toggle. All idempotent. | §11.6: Close is idempotent; §11.4: Minimized no focus | A5 |
| Tabs never desync from frames | Tab index and frame state could drift | Tab stack is shell-owned per Frame. Active tab tracked, tab switch updates frame visual state atomically. | §11.16: Tabs are display-only, not authority | A3 |
| Destroyed IDs never resurrect | SurfaceId reused without generation check | LifecycleGeneration monotonic counter. Destroyed IDs deferred until generation safety period elapses. | §11.2: ID reuse requires generation safety | A6 |
| Tombstones are visible/explainable | Dead surface just disappears — user confused | TombstoneRecord with reason, caller identity, timestamp. Shell surfaces tombstones in Atlas/debug view. | §11.5: Tombstoned not live but record exists | A6 |
| Renderer never owns policy | sexdisplay might infer tab order or focus | sexdisplay receives visual state only. Shell sends all lifecycle decisions via standard opcodes (0xEC/0xEE/0xED/0xEB). | §11.11: sexdisplay never decides lifecycle or focus | A7 |
| Crash/fault does not freeze shell | Panic in lifecycle transition could hang shell | All transitions produce proof markers. On fault: surface→Tombstoned (not Destroyed), focus cleared. Shell continues. | §11.8: Unknown/rejected ops deterministically no-op | A8 |
| Proof markers make failures obvious | Transition failure silently swallowed | Every allowed/forbidden transition has allow+reject proof marker. Failure includes reason string. | §11.19: All transitions produce proof marker | A8 |
| Customization is rich but safe | Shell customization could bypass lifecycle guards | All customization (§17) is shell-owned, validated, cannot customize away lifecycle invariants. | §11.1-19: Invariants non-negotiable | A8+Scan8 |
| Session restore recovers exact state | Restore creates surface in wrong lifecycle state | Restore flows through FSM: Allocated→Mapped→Visible. Never bypasses to Visible directly. | §11.1-3: Destroyed terminal, focus valid | A3+A4 |

## 17. Scan 8 — Customization / User Policy Surface

Customization is shell-owned, validated, reversible, accessible, and unable to customize away lifecycle safety.

### Customizable (10 domains)

| Preference | Options | Constraint |
|-----------|---------|------------|
| Close behavior (multi-tab) | close_active_tab, prompt, close_frame_later | Cannot close Destroyed/Tombstoned surface |
| Tombstone visibility | hidden, show_in_atlas, show_in_debug | Cannot make tombstoned surfaces live |
| Minimize destination | atlas_card, silkbar_shelf, hidden_list | Cannot focus minimized surface |
| Zoom behavior | frame_zoom, scene_zoom (bounded) | Geometry bounds always enforced |
| Tab reveal delay | ticks (deterministic range) | Cannot suppress tab reveal entirely |
| Rim/accent/theme token | bounded compiled token set | No raw RGBA; no identity/authority claim |
| Animation | enabled, disabled, reduced_motion | Cannot affect lifecycle transitions |
| Focus-follows-hover (future) | enabled/disabled (after audit) | Must pass liveness check — never bypasses §11.3 |
| Keybindings (future) | scancode+modifiers (after D audit) | Must pass shortcut conflict + accessibility audit |
| Proof verbosity | minimum, normal, debug | Cannot suppress required lifecycle markers |

### Not Customizable (11 hard boundaries)

Lifecycle FSM rules; Destroyed terminal; focus/liveness validation (5-guard); SurfaceId uniqueness + generation safety; sexdisplay ownership boundary; framebuffer bounds checks; PDX capability checks; PDX ABI/opcodes; required proof markers (safety markers always fire); app ability to force focus; tombstone/destroy semantics; drag/resize cancellation safety.

### Customization Proof Scenarios

1. Valid rim token accepted → `[comp.pref.accept]` with token name.
2. Invalid token rejected → `[comp.pref.reject]` reason=invalid_token, clamped to default.
3. Reduced motion disables animations but never lifecycle transitions → `[comp.pref.apply]` motion=reduced. FSM states unchanged.
4. Proof verbosity=minimum still fires `[comp.surface.close]`, `[comp.surface.tombstone]`, `[comp.surface.focus.reject]` — required safety markers never suppressed.
5. Close policy=close_frame_later cannot close Destroyed surface → `[comp.surface.close.reject]` reason=already_dead.
6. Minimize destination=hidden_list cannot focus minimized surface → `[comp.surface.focus.reject]` reason=minimized.
7. Keybinding before audit rejected → `[comp.pref.reject]` reason=no_audit. Planned-only until D accessibility gate.
8. Focus-follows-hover cannot bypass liveness → `[comp.surface.focus.reject]` if hover target is Tombstoned/Minimized/Destroyed.
9. Reset-to-safe-default restores canonical behavior → `[comp.pref.reset]`. All preferences back to compiled defaults.

### Preference Lifecycle

1. **Load** → `[comp.pref.load]`. 2. **Validate** → `[comp.pref.validate.ok]` or `.reject`. 3. **Apply** → `[comp.pref.apply]` (UI prefs immediate; policy prefs need guard re-validation). 4. **Persist** → blocked until E gates pass (memory-only in V1). 5. **Redact** → `[comp.pref.redact]` per E8 policy. 6. **Reset** → `[comp.pref.reset]`.

## 18. Handoff Files

- `docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md` — this document (overview)
- `docs/handoff/COMPOSITOR_LIFECYCLE_FSM_V1.md` — 8-state FSM, transitions, guards (A2)
- `docs/handoff/COMPOSITOR_SHELL_LIFECYCLE_MODEL_V1.md` — state tracking, LifecycleGeneration, FocusRef (A3)
- `docs/handoff/COMPOSITOR_FOCUS_VALIDITY_V1.md` — 5-guard focus, clear_focus_if_dead, scene-switch transfer (A4)
- `docs/handoff/COMPOSITOR_FRAME_LIGHT_ACTIONS_V1.md` — close/minimize/zoom dispatch, drag cancellation (A5)
- `docs/handoff/COMPOSITOR_TOMBSTONE_EVENTS_V1.md` — TombstoneRecord, reclamation, generation safety (A6)
- `docs/handoff/COMPOSITOR_DISPLAY_CONFORMANCE_V1.md` — sexdisplay state validation, bounds checks (A7)
- `docs/handoff/COMPOSITOR_LIFECYCLE_PROOF_V1.md` — proof scenarios, negative tests, markers (A8)

## 19. Final Safest Path

1. **A1 audit first** — Current shell lifecycle capabilities must be audited before FSM design. Skipping A1 means FSM targets are based on assumptions, not reality.
2. **FSM before integration** — A2 defines the pure FSM. A3-A7 integrate it into shell, focus, lights, tombstones, display. FSM must be stable before integration begins.
3. **Focus guards before frame lights** — A4 (focus validity) must precede A5 (frame lights) because light actions affect focus state.
4. **Tombstone before destroy reclamation** — A6 tombstone record must exist before any Destroyed surface ID is reclaimed or reused.
5. **Display conformance last** — A7 verifies sexdisplay never receives invalid state. Must come after all other integration phases.
6. **Proof scenarios validate all transitions** — A8 must verify every allowed transition produces allow marker and every forbidden transition produces reject marker.
7. **No implementation before A1** — STOP FIRST for any lifecycle code changes before audit completes.
