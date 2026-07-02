# ATLAS_OVERVIEW_PHASE_E4C_SYNTHETIC_CROSS_SCENE_REPARENT_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default, proof off) | PASS |
| Build (proof enabled) | PASS |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1` build flag |
| Proof runner syntax | PASS (`bash -n` clean) |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Add gate constants (~10 lines); add `reparent_frame_to_scene()` helper (~110 lines); add `maybe_run_atlas_phase_e4c_cross_scene_reparent_proof()` proof function (~240 lines); wire into main loop dispatch (1 line) | ~+360 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_e4c_cross_scene_reparent` variable, gate logic block (~35 lines), and summary array entry (~1 line) | ~+36 |
| `scripts/run_daily_driver_proof.sh` | Add `export SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1` | +1 |
| `docs/handoff/ATLAS_OVERVIEW_PHASE_E4C_SYNTHETIC_CROSS_SCENE_REPARENT_PROOF_V1.md` | This handoff doc | new |

## Exact Root Cause / Gap Closed

**Gap:** No proof existed that a frame can be safely reparented from one Scene to another with full focus/drag/hover/visibility reconciliation. Phase E4c closes this gap by adding:

1. A `reparent_frame_to_scene()` helper that mutates `frame.scene_id`, then runs all reconciliation helpers.
2. A synthetic proof function that enters Atlas, finds a frame in the active scene, reparents it to a different scene, verifies invariants, reparents it back to restore original state, and exits Atlas.
3. All existing reconciliation helpers (`clear_focus_if_wrong_scene`, `clear_drag_if_wrong_scene`, `clear_hover_if_wrong_scene`, `sync_scene_visibility`, `tile_active_scene_frames`, `atlas_capture_snapshot`) are called after the move.

**Closed:**
1. Added `SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1` env var gate constant. Default unset = zero behavior change.
2. Added `reparent_frame_to_scene(frame_id, target_scene, reason) -> bool` helper — asserts frame validity, scene validity, handles same-scene noop via existing `atlas_same_scene_drop_noop()`, mutates `frame.scene_id` for cross-scene, reconciles focus/drag/hover/visibility/snapshot, emits move.begin/move.done/reject markers.
3. Added `maybe_run_atlas_phase_e4c_cross_scene_reparent_proof()` — 6-stage synthetic proof: begin → enter Atlas → find frame + target → reparent → verify → restore to source → exit Atlas → done.
4. Noop paths: single_scene (only one valid scene) or no_frame (no non-minimized frame in active scene) emit honest skip markers.
5. Gate: `SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1` (unset = zero behavior change)

## Reparent Helper

```rust
unsafe fn reparent_frame_to_scene(frame_id: u32, target_scene: u8, reason: &str) -> bool
```

**Validation Rules:**
- `frame_id` must map to an existing `FRAMES[]` entry
- `target_scene` must be < `ATLAS_MAX_SCENES` (5)
- Invalid frame → `[silk.frame.scene.move.reject] reason=no_frame ok=0`, return false
- Invalid target → `[silk.frame.scene.move.reject] reason=bad_target ok=0`, return false
- Source == target → same-scene noop via `atlas_same_scene_drop_noop()`, return true

**Cross-Scene Mutation Sequence:**
1. Emit `[silk.frame.scene.move.begin]` with frame, from, to, reason
2. Mutate `frame.scene_id = target_scene` (single field write)
3. Call `clear_focus_if_wrong_scene()` — existing helper, clears focus if focused surface is now in wrong scene
4. Call `clear_drag_if_wrong_scene()` — existing helper, clears drag if dragged surface is now in wrong scene
5. Call `clear_hover_if_wrong_scene()` — existing helper, clears hover if hovered frame is now in wrong scene
6. Call `sync_scene_visibility()` — existing helper, shows/hides surfaces based on new scene membership
7. Call `tile_active_scene_frames()` — existing helper, re-tiles active scene frames
8. Call `atlas_capture_snapshot()` — existing helper, re-derives Atlas snapshot
9. Clear `ATLAS_DRAG_INTENT.active` and emit `[silk.atlas.drag.clear]`
10. Verify focus state, emit `[silk.frame.scene.move.done]` with `ownership_unique=1` and `focus_valid`

## Reconciliation Helpers Used

| Helper | Purpose | Existing? |
|--------|---------|-----------|
| `clear_focus_if_wrong_scene()` | Clears focus if focused surface is no longer in active scene; tries to re-focus a surface in active scene | Yes — add-only in E4a audit, used by `switch_scene()` |
| `clear_drag_if_wrong_scene()` | Transitions INTERACTION to Idle if dragged surface is no longer in active scene | Yes — add-only in E4a audit |
| `clear_hover_if_wrong_scene()` | Clears HOVERED_FRAME_ID if hovered frame is not in active scene or invalid | Yes — add-only |
| `sync_scene_visibility()` | Shows (0xEC) surfaces in active scene, hides (0xEE) surfaces in inactive scenes | Yes — used by `switch_scene()` |
| `tile_active_scene_frames()` | Re-tiles all tileable frames in the active scene | Yes — used by `switch_scene()` |
| `atlas_capture_snapshot()` | Re-derives AtlasSnapshot with updated scene descriptors | Yes — used by `switch_scene()` |

All six helpers already exist and are proven safe by their use in `switch_scene()`. No new helpers were created for E4c beyond `reparent_frame_to_scene()` itself.

## Proof Sequence

### Normal Path (Frame Exists, Multiple Scenes)
```
[silk.atlas.phase_e4c.begin] active=S scenes=N
[silk.atlas.phase_e4c.enter] ok=1 active=S
[silk.frame.scene.move.begin] frame=F from=A to=B reason=atlas_synthetic_reparent
[silk.frame.scene.move.done] frame=F from=A to=B ownership_unique=1 focus_valid=1 ok=1
[silk.atlas.phase_e4c.verify] frame=F source=A target=B after=B ownership_unique=1 focus_valid=1 ok=1
[silk.frame.scene.move.begin] frame=F from=B to=A reason=atlas_synthetic_restore
[silk.frame.scene.move.done] frame=F from=B to=A ownership_unique=1 focus_valid=1 ok=1
[silk.frame.scene.move.restore] frame=F from=B to=A ok=1
[silk.atlas.phase_e4c.restore_verify] ok=1
[silk.atlas.mode.exit] active=A reason=atlas_cross_scene_reparent_done view=desktop ok=1
[silk.atlas.phase_e4c.done] ok=1
```

### Noop Path (Single Scene)
```
[silk.atlas.phase_e4c.begin] active=S scenes=1
[silk.atlas.phase_e4c.enter] ok=1 active=S
[silk.atlas.phase_e4c.noop] reason=single_scene ok=1
[silk.atlas.phase_e4c.restore_verify] ok=1
[silk.atlas.mode.exit] active=S reason=atlas_cross_scene_reparent_done view=desktop ok=1
[silk.atlas.phase_e4c.done] ok=1
```

### Noop Path (No Frame)
```
[silk.atlas.phase_e4c.begin] active=S scenes=N
[silk.atlas.phase_e4c.enter] ok=1 active=S
[silk.atlas.phase_e4c.noop] reason=no_frame ok=1
[silk.atlas.phase_e4c.restore_verify] ok=1
[silk.atlas.mode.exit] active=S reason=atlas_cross_scene_reparent_done view=desktop ok=1
[silk.atlas.phase_e4c.done] ok=1
```

## Gate Criteria

| Condition | Result |
|-----------|--------|
| `[silk.atlas.phase_e4c.done] ok=1` AND `[silk.atlas.phase_e4c.verify] ok=1` | PASS — proof complete |
| `[silk.atlas.phase_e4c.noop] ok=1` (single_scene or no_frame) | PASS — honest skip |
| `[silk.atlas.phase_e4c.verify] ok=0` with phase_e4c.done | FAIL — verify failed |
| `[silk.frame.scene.move.done] ownership_unique=0` | FAIL — invariant violated |
| `[silk.frame.scene.move.done] focus_valid=0` without `focus_cleared=1` | FAIL — focus invalid |
| `[silk.frame.scene.move.begin]` without `move.done` | FAIL — incomplete reparent |
| `[silk.frame.scene.move.done]` without `move.restore` | FAIL — restore missing |
| `[silk.frame.scene.move.reject]` during proof without noop/done | FAIL — unexpected reject |
| `[silk.atlas.phase_e4c.begin]` without `phase_e4c.done` | FAIL — incomplete proof |
| No proof markers | SKIP — proof not enabled |

## Proof Commands

Build with Phase E4c proof enabled:
```fish
SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1 ./scripts/entrypoint_build.sh
```

Build with proof disabled (default, zero behavior change):
```fish
./scripts/entrypoint_build.sh
```

Validate gate script syntax:
```fish
bash -n scripts/daily_driver_master_gate.sh
```

Validate proof runner syntax:
```fish
bash -n scripts/run_daily_driver_proof.sh
```

Runtime proof:
```fish
DAILY_DRIVER_PROBE_SECONDS=60 ./scripts/run_daily_driver_proof.sh /tmp/atlas_e4c_proof.log
./scripts/daily_driver_master_gate.sh /tmp/atlas_e4c_proof.log | rg "atlas_phase|phase_e4c|scene.move|ownership_unique|focus_valid|restore|FINAL|FAIL|fault|panic|#PF|#GP"
```

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
| No real pointer drop path | Preserved — synthetic proof only |
| No tab moves | Preserved |
| No frame create/delete | Preserved |
| No visual drag ghost | Preserved |
| No animation | Preserved |
| No new unsafe beyond existing patterns | Follows existing unsafe fn patterns |
| No unwrap/panic on optional state | Safe Option patterns; unwrap_or only on proved-some values |
| No OOB | Bounded by ATLAS_MAX_SCENES (5), WORKSPACE_COUNT, FRAMES bounds |
| No behavior change when env unset | Early return at fn entry |
| No persistent daily-driver layout drift | Frame restored to source scene before done |
| Restoration before completion | Synthetic proof restores frame to original scene |

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
| **Phase E4c** | **Cross-scene reparent proof** | Built, gate added |
| Phase E4d | Real pointer drop path | Deferred (STOP FIRST) |
| Phase E4e | Integrated drag/drop gate | Deferred (STOP FIRST) |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit STOP Note

**No real pointer drop path was implemented.** Phase E4c proves that cross-scene frame reparent is safe via a synthetic proof that:
1. Mutates `frame.scene_id` directly (not through a pointer drop target)
2. Runs all reconciliation helpers
3. Restores the frame to its original scene before exiting Atlas

The `reparent_frame_to_scene()` helper is ready to be called from a real pointer drop path (E4d), but is NOT wired to any pointer-up dispatch. Real pointer drop detection (E4d), visual drag ghost, animation, tab moves, and integrated drag/drop gate (E4e) are explicitly deferred with STOP FIRST boundaries.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Phase E4c cross-scene reparent synthetic proof — built and gated | ATLAS_OVERVIEW_PHASE_E4C_SYNTHETIC_CROSS_SCENE_REPARENT_PROOF_V1 |
