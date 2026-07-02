# ATLAS_OVERVIEW_PHASE_E3_DRAG_BEGIN_MARKER_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default, proof off) | PASS |
| Build (proof enabled) | PASS |
| Build (all Phase A-E3 proofs) | PASS |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1` build flag |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Add gate constants and AtlasDragIntent struct (~35 lines); add `maybe_run_atlas_phase_e3_drag_begin_marker_proof()` proof function (~130 lines); wire into main loop dispatch (1 line) | ~+166 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_e3_drag_begin_marker` variable, gate logic block, and summary array entry; bump version to V36 | +30 |
| `scripts/run_daily_driver_proof.sh` | Add `export SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1`; bump version to V35 | +2 |

## Exact Root Cause / Gap Closed

**Gap:** No proof existed for Atlas card drag-intent detection. Phase E3 closes this gap by adding a synthetic drag-begin marker proof that exercises the card hit-test geometry and emits begin/cancel markers — without any movement, frame ownership mutation, scene switch, or drop target.

**Closed:**
1. Added `SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1` env var gate constant. Default unset = zero behavior change.
2. Added `AtlasDragIntent` struct — minimal state tracking `active`, `scene_id`, `frame_id`, `start_x`, `start_y`. Does NOT drive actual frame movement. Used only for proof marker tracking.
3. Added `maybe_run_atlas_phase_e3_drag_begin_marker_proof()` — a 5-stage synthetic proof function that enters Atlas, computes a valid card center point using existing `atlas_card_pos()`, finds a frame in the active scene, emits drag-begin marker, immediately cancels (no movement), verifies invariants (scene unchanged, ownership not mutated), exits Atlas, and emits done.
4. Noop path: if the active scene has no non-minimized frames, emits `[silk.atlas.drag.noop] reason=no_card_or_preview ok=1` and still completes the proof with `phase_e3.done`.
5. Gate: `SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1` (unset = zero behavior change)

## Proof: Synthetic-Only

**Synthetic-only.** No real pointer-down path was instrumented. The proof function computes a hit point from `atlas_card_pos()` and uses it to emit begin/cancel markers. No actual pointer events are synthesized, no HID interaction occurs, and no compositor protocol changes are made. The existing drag infrastructure (window drag, tab reorder) is untouched.

## Exact Markers Added

### Normal Path (Frame Exists in Active Scene)
```
[silk.atlas.phase_e3.begin] active=S scenes=N
[silk.atlas.phase_e3.enter] ok=1 active=S
[silk.atlas.drag.begin] scene=S frame=F x=X y=Y ok=1 source=synthetic
[silk.atlas.drag.cancel] scene=S frame=F reason=proof_release ok=1
[silk.atlas.drag.invariant] scene_before=A scene_after=B ownership_mutated=0 ok=1
[silk.atlas.mode.exit] active=S reason=atlas_drag_begin_done view=desktop ok=1
[silk.atlas.phase_e3.done] ok=1
```

### Noop Path (No Frame in Active Scene)
```
[silk.atlas.phase_e3.begin] active=S scenes=N
[silk.atlas.phase_e3.enter] ok=1 active=S
[silk.atlas.drag.noop] reason=no_card_or_preview ok=1
[silk.atlas.drag.invariant] scene_before=A scene_after=B ownership_mutated=0 ok=1
[silk.atlas.mode.exit] active=S reason=atlas_drag_begin_done view=desktop ok=1
[silk.atlas.phase_e3.done] ok=1
```

## Gate Criteria

| Condition | Result |
|-----------|--------|
| `[silk.atlas.phase_e3.done] ok=1` found | PASS — proof complete |
| `[silk.atlas.drag.noop] ok=1` found | PASS — no card/preview (honest skip) |
| `[silk.atlas.drag.begin]` without `[silk.atlas.drag.cancel]` or `phase_e3.done` | FAIL — orphaned drag-begin |
| `[silk.atlas.drag.invariant] ownership_mutated=1` | FAIL — invariant violated |
| `[silk.atlas.phase_e3.begin]` without `phase_e3.done` | FAIL — incomplete proof |
| No proof markers | SKIP — proof not enabled |

## Proof Commands

Build with Phase E3 proof enabled:
```fish
SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1 ./scripts/entrypoint_build.sh
```

Build with proof disabled (default, zero behavior change):
```fish
./scripts/entrypoint_build.sh
```

Validate gate script syntax:
```fish
bash -n scripts/daily_driver_master_gate.sh
```

Combined Phase A-E3 runtime proof:
```fish
LOG=/tmp/atlas_e3_proof.log
rm -f "$LOG"

DAILY_DRIVER_PROBE_SECONDS=60 ./scripts/run_daily_driver_proof.sh "$LOG"

./scripts/daily_driver_master_gate.sh "$LOG" | rg "atlas_phase|phase_e3|drag.begin|drag.cancel|FINAL|FAIL|fault|panic|#PF|#GP"
```

Expected gate PASS: `atlas_phase_e3_drag_begin_marker` shows PASS when `[silk.atlas.phase_e3.done]` found with `ok=1`, or when `[silk.atlas.drag.noop]` found with `ok=1`.

## STOP FIRST Boundaries Preserved

| Boundary | Status |
|----------|--------|
| No kernel edits | Preserved |
| No sex-pdx ABI edits | Preserved |
| No sexdisplay edits | Preserved |
| No new compositor protocol | Preserved |
| No compositor/display ABI edits | Preserved |
| sexdisplay remains sole framebuffer writer | Preserved |
| silk-shell owns shell/session/input policy | Preserved |
| No framebuffer/backing-buffer redesign | Preserved |
| No shared-memory redesign | Preserved |
| No broad refactor | Preserved |
| No input policy outside silk-shell | Preserved |
| No mixed feature + refactor patch | Preserved |
| No frame move | Preserved — begin/cancel only |
| No frame ownership mutation | Preserved — invariant verified |
| No tab mutation | Preserved |
| No Scene switch | Preserved — active scene verified unchanged |
| No drop target | Preserved |
| No visual drag ghost | Preserved |
| No animation | Preserved |
| No new unsafe beyond existing patterns | Follows existing unsafe fn pattern |
| No unwrap/panic on optional state | Safe Option patterns |
| No OOB | Bounded by ATLAS_MAX_SCENES (5), FRAMES length |
| No behavior change when env unset | Early return at fn entry |
| No existing drag infrastructure mutation | AtlasDragIntent is separate from real drag state |
| No real pointer event synthesis | Synthetic-only proof |

## Remaining Phases E4, F

| Phase | What | Status |
|-------|------|--------|
| Phase A | State model proof | Built, gate added |
| Phase B | Atlas snapshot/capture | Built, gate added |
| Phase C | Render stub + card geometry | Built, gate added |
| Phase D | Frame preview interior stub | Built, gate added |
| Phase E1 | Click scene switch proof | Built, gate added |
| Phase E2 | Keyboard scene cycle proof | Built, gate added |
| **Phase E3** | **Drag begin marker proof** | Built, gate added |
| Phase E4 | Drag frame between Scenes | Deferred (STOP FIRST) |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit STOP Note

Phase E3 is drag-begin marker proof instrumentation only — **no frame movement, no drop target, no frame reparenting, no Scene mutation, no focus ownership changes, no compositor protocol changes, no visual drag ghost, no animation**. The proof emits begin/cancel markers using existing `atlas_card_pos()` geometry and immediately cancels. E4 (drag/move between Scenes) is explicitly deferred with STOP FIRST boundaries.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Add Phase E3 drag begin marker proof | ATLAS_OVERVIEW_PHASE_E3_DRAG_BEGIN_MARKER_PROOF_V1 |
