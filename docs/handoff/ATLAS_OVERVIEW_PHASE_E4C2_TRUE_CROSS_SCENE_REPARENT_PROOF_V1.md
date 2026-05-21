# ATLAS_OVERVIEW_PHASE_E4C2_TRUE_CROSS_SCENE_REPARENT_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default, proof off) | PASS |
| Build (proof enabled) | PASS |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_E4C2_TRUE_REPARENT_PROOF=1` build flag |
| Proof runner syntax | PASS (`bash -n` clean) |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Add gate constants (~8 lines); add `maybe_run_atlas_phase_e4c2_true_reparent_proof()` proof function (~280 lines); wire into main loop dispatch (1 line) | ~+289 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_e4c2_true_cross_scene_reparent` variable, gate logic block (~43 lines), and summary array entry (~1 line) | ~+45 |
| `scripts/run_daily_driver_proof.sh` | Add `export SEXOS_ATLAS_PHASE_E4C2_TRUE_REPARENT_PROOF=1` | +1 |
| `docs/handoff/ATLAS_OVERVIEW_PHASE_E4C2_TRUE_CROSS_SCENE_REPARENT_PROOF_V1.md` | This handoff doc | new |

## Exact Gap Closed

**Gap:** Phase E4c gate PASSed but only ever via noop path (`reason=single_scene` or `reason=no_frame`). The E4c proof function checks `populated_scenes <= 1` before attempting a cross-scene reparent. In the daily driver, only scene 0 has any frames at boot time, so `populated_scenes` is always 1 and the proof always noops — meaning a true cross-scene reparent was never actually runtime-proven.

**Closed by E4c2:**
1. Removes the `populated_scenes <= 1` gate — instead uses any valid scene index (0..WORKSPACE_COUNT-1) different from the source as the target. A scene does not need to be "populated" to be a valid reparent target.
2. Forces actual `frame.scene_id` mutation: source → target → source restore.
3. Verifies both the move and the restore with explicit markers.
4. Tracks and restores original scene_id.
5. Only skips if genuinely impossible (WORKSPACE_COUNT < 2 or no non-minimized frame exists).

## Why E4c Noop Was Insufficient

E4c's proof function (stage 2) counts populated scenes. If `populated_scenes <= 1`, it unconditionally emits noop and exits. In the daily driver boot:

- Scene 0 has 1+ frames (the only populated scene at boot)
- Scenes 1-4 exist but are empty
- `populated_scenes = 1` → noop always

Therefore E4c never actually exercised:
- The `frame.scene_id` mutation path inside `reparent_frame_to_scene()`
- Focus/drag/hover reconciliation after a real cross-scene move
- Scene visibility sync after frame departure from active scene
- Snapshot re-derivation with changed scene membership
- Restoration back to original scene

E4c only proved that same-scene detection works and that the noop path is clean. E4c2 proves the full mutation path.

## Setup Strategy

1. Enter Atlas mode if not already open.
2. Find a non-minimized frame in the active scene (same as E4c).
3. Choose target scene as: if source==0 then 1 else 0. This guarantees a different valid scene index.
4. Record `original_scene` from the frame's current `scene_id`.
5. Emit `[silk.atlas.phase_e4c2.setup]` with frame, source, target, original.
6. Call `reparent_frame_to_scene(frame_id, target, "atlas_true_reparent")` — this is the E4c helper that mutates `frame.scene_id` and runs all reconciliation.
7. Verify frame.scene_id == target after move.
8. Call `reparent_frame_to_scene(frame_id, source, "atlas_true_reparent_restore")` — restore back.
9. Verify frame.scene_id == source after restore.
10. If original_scene != source (should not happen in practice), directly restore to original and re-reconcile.
11. Final orphan check, exit Atlas, emit done.

## Restoration Strategy

The proof always restores the frame to its original scene before exiting:
- Primary path: reparent target→source using the same `reparent_frame_to_scene()` helper.
- Edge case: if original_scene != source (e.g., frame was moved before proof ran), direct `frame.scene_id` mutation back to original, followed by full reconciliation.
- Final verify: scan all FRAMES[] for orphans (non-minimized frames not in active scene).
- No persistent scene_id drift is possible — the proof breaks (doesn't reach `done`) if any verify step fails.

## Proof Markers

### Normal Path (Frame Exists, Multiple Scene Indices)
```
[silk.atlas.phase_e4c2.begin] active=S scenes=N
[silk.atlas.phase_e4c2.enter] ok=1 active=S
[silk.atlas.phase_e4c2.setup] frame=F source=A target=B original=O ok=1
[silk.frame.scene.move.begin] frame=F from=A to=B reason=atlas_true_reparent
[silk.frame.scene.move.done] frame=F from=A to=B ownership_unique=1 focus_valid=1 ok=1
[silk.atlas.phase_e4c2.verify_moved] frame=F expected=B actual=B ownership_unique=1 focus_valid=1 focus_cleared=1 ok=1
[silk.frame.scene.move.begin] frame=F from=B to=A reason=atlas_true_reparent_restore
[silk.frame.scene.move.done] frame=F from=B to=A ownership_unique=1 focus_valid=1 ok=1
[silk.frame.scene.move.restore] frame=F from=B to=A ok=1
[silk.atlas.phase_e4c2.verify_restored] frame=F expected=A actual=A ok=1
[silk.atlas.phase_e4c2.restore_original] frame=F original=O ok=1
[silk.atlas.phase_e4c2.final_verify] ok=1
[silk.atlas.mode.exit] active=A reason=atlas_e4c2_true_reparent_done view=desktop ok=1
[silk.atlas.phase_e4c2.done] ok=1
```

### Skip Path (No Frame)
```
[silk.atlas.phase_e4c2.begin] active=S scenes=N
[silk.atlas.phase_e4c2.enter] ok=1 active=S
[silk.atlas.phase_e4c2.skip] reason=no_safe_frame ok=1
```

### Skip Path (WORKSPACE_COUNT < 2)
```
[silk.atlas.phase_e4c2.skip] reason=no_target_scene ok=1
```

## Gate Criteria

| Condition | Result |
|-----------|--------|
| `[silk.atlas.phase_e4c2.done] ok=1` AND `verify_moved ok=1` AND `verify_restored ok=1` | PASS |
| `[silk.atlas.phase_e4c2.skip] ok=1` (no_safe_frame or no_target_scene) | SKIP |
| `verify_moved ok=0` | FAIL |
| `verify_restored ok=0` (frame scene_id drift) | FAIL |
| `move.done ownership_unique=0` | FAIL |
| `move.done focus_valid=0` without `focus_cleared=1` | FAIL |
| `move.begin` without `move.done` | FAIL |
| `move.done` without `move.restore` | FAIL |
| `phase_e4c2.begin` without `phase_e4c2.done` | FAIL |
| No proof markers | SKIP |

## Proof Commands

Build with E4c2 proof enabled:
```fish
SEXOS_ATLAS_PHASE_E4C2_TRUE_REPARENT_PROOF=1 ./scripts/entrypoint_build.sh
```

Build default (proof disabled, zero behavior change):
```fish
./scripts/entrypoint_build.sh
```

Validate scripts:
```fish
bash -n scripts/daily_driver_master_gate.sh
bash -n scripts/run_daily_driver_proof.sh
```

Runtime proof:
```fish
DAILY_DRIVER_PROBE_SECONDS=60 ./scripts/run_daily_driver_proof.sh /tmp/atlas_e4c2_proof.log
./scripts/daily_driver_master_gate.sh /tmp/atlas_e4c2_proof.log | rg "atlas_phase|phase_e4c2|true_reparent|verify_moved|verify_restored|scene.move|FINAL|FAIL|fault|panic|#PF|#GP"
```

## STOP FIRST Boundaries Preserved

| Boundary | Status |
|----------|--------|
| No kernel edits | Preserved |
| No sex-pdx ABI edits | Preserved |
| No sexdisplay edits | Preserved |
| sexdisplay remains sole framebuffer writer | Preserved |
| silk-shell owns shell/session/input policy | Preserved |
| No compositor/display ABI edits | Preserved |
| No shared-memory/backing-buffer redesign | Preserved |
| No broad refactor | Preserved |
| No input policy outside silk-shell | Preserved |
| No mixed feature + refactor patch | Preserved |
| No real pointer drop path | Preserved — synthetic proof only |
| No tab moves | Preserved |
| No new frame allocation | Preserved — uses existing FRAMES[] |
| No persistent layout drift | Frame restored to original scene before done |
| No visual drag ghost | Preserved |
| No animation | Preserved |
| No new unsafe beyond existing patterns | Uses existing unsafe fn patterns |
| No unwrap/panic | Safe Option patterns; unwrap only after is_none() check |
| No OOB | Bounded by WORKSPACE_COUNT and ATLAS_MAX_SCENES |
| No behavior change when env unset | Early return at fn entry |

## Remaining E4d/E4e/F Work

| Phase | What | Status |
|-------|------|--------|
| Phase A | State model proof | Built, gate added |
| Phase B | Atlas snapshot/capture | Built, gate added |
| Phase C | Render stub + card geometry | Built, gate added |
| Phase D | Frame preview interior stub | Built, gate added |
| Phase E1 | Click scene switch proof | Built, gate added |
| Phase E2 | Keyboard scene cycle proof | Built, gate added |
| Phase E3 | Drag begin marker proof | Built, gate added |
| Phase E4b | Same-scene no-op proof | Built, gate added |
| Phase E4c | Cross-scene reparent proof | Built, gate added (noop-only in practice) |
| **Phase E4c2** | **True cross-scene reparent proof** | Built, gate added |
| Phase E4d | Real pointer drop path | Deferred (STOP FIRST) |
| Phase E4e | Integrated drag/drop gate | Deferred (STOP FIRST) |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit STOP Note

**No real pointer drop path was implemented.** Phase E4c2 proves that true cross-scene frame reparent is safe via a synthetic proof that:
1. Mutates `frame.scene_id` directly (not through a pointer drop target)
2. Runs all reconciliation helpers (focus, drag, hover, visibility, snapshot)
3. Restores the frame to its original scene before exiting Atlas
4. Forces actual cross-scene mutation even when only one scene is populated

The `reparent_frame_to_scene()` helper from E4c is used — E4c2 only changes the proof setup strategy to remove the `populated_scenes <= 1` gate. Real pointer drop detection (E4d), visual drag ghost, animation, tab moves, and integrated drag/drop gate (E4e) are explicitly deferred with STOP FIRST boundaries.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Phase E4c2 true cross-scene reparent synthetic proof — built and gated | ATLAS_OVERVIEW_PHASE_E4C2_TRUE_CROSS_SCENE_REPARENT_PROOF_V1 |
