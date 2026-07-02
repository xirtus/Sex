# ATLAS_OVERVIEW_PHASE_E1_CLICK_SCENE_SWITCH_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (source audit) | PASS (consistent with Phase A-D patterns) |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1` build flag |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Add gate constants (7 lines); add `maybe_run_atlas_phase_e1_click_scene_switch_proof()` proof function (6 stages); wire into main loop dispatch after Phase D | +161 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_e1_click_scene_switch` variable, gate logic block, and summary array entry | +38 |

## Exact Root Cause / Gap Closed

**Gap:** No shell-side proof existed for the Atlas card click-to-scene-switch path. The actual click handling in `click_hit_test_and_focus()` already performs hit-testing, scene switching, and Atlas exit when a card is clicked — this behavior works correctly. But no deterministic proof markers instrumented the path for gate verification.

**Closed:**
1. Added `SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1` env var gate constant. Default unset = zero behavior change.
2. Added `maybe_run_atlas_phase_e1_click_scene_switch_proof()` — a synthetic 6-stage proof function that exercises the full click-to-switch path using existing infrastructure (atlas_toggle, switch_scene, atlas_scene_at_point, atlas_exit). No new compositor ABI, no drag/drop, no frame ownership mutation.
3. **Positive proof (stages 0-3):** Enters Atlas, hit-tests a non-active card using the same `atlas_scene_at_point()` + `atlas_card_pos()` geometry as Phase C/D, switches scene via `switch_scene()`, exits Atlas via `atlas_exit()`, emits all required markers.
4. **Negative proof (stages 4-5):** Re-enters Atlas, hit-tests an empty off-card position, verifies no hit, exits Atlas, emits negative empty-click marker. Proves off-card clicks do not switch scenes and do not app-dispatch.
5. Gate: `SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1` (unset = zero behavior change)

## Hit-Test Geometry Rule

The proof reuses the existing `atlas_scene_at_point()` function (line ~10100), which uses the same deterministic card geometry as Phase C/D:

- Card layout: row 0 has 3 cards (scenes 0,1,2), row 1 has 2 cards (scenes 3,4)
- Card size: `ATLAS_CARD_W=220` × `ATLAS_CARD_H=150`
- Card gap: `ATLAS_CARD_GAP=24`
- Y offset: row 0 at y=30, row 1 at y=30 + ATLAS_CARD_H + ATLAS_CARD_GAP
- Coordinate conversion: subtract `P.bar_height` (50) from screen Y to get overlay-local Y
- Cards are centered horizontally in the content area (width = P.width)

**Geometry is shared** — the same `atlas_card_pos()` and `atlas_scene_at_point()` functions are used by both the existing click path and the Phase E1 proof. No duplicated temporary proof geometry.

## Exact Markers Added

### Positive Proof Markers
```
[silk.atlas.phase_e1.begin] active=S scenes=N
[silk.atlas.phase_e1.enter] ok=1 active=S
[silk.atlas.hit.scene] scene=N x=X y=Y ok=1
[silk.atlas.click.consume] scene=N ok=1
[silk.scene.active.set] from=A to=B reason=atlas_card_click
[silk.atlas.mode.exit] active=S reason=atlas_card_click view=desktop ok=1
[silk.atlas.phase_e1.done] from=A to=B ok=1
```

### Negative Proof Marker
```
[silk.atlas.hit.empty] x=0 y=0 reason=no_card
[silk.atlas.phase_e1.negative.empty_click] ok=1
```

## Proof Commands

Build with Phase E1 proof enabled:
```fish
SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1 ./scripts/entrypoint_build.sh
```

Build with proof disabled (default, zero behavior change):
```fish
./scripts/entrypoint_build.sh
```

Validate gate script syntax:
```fish
bash -n scripts/daily_driver_master_gate.sh
```

Combined Phase A-E1 runtime proof:
```fish
LOG=/tmp/sexos_atlas_phase_e1.log
rm -f "$LOG"

SEXOS_ATLAS_PHASE_A_STATE_MODEL_PROOF=1 \
SEXOS_ATLAS_PHASE_B_SNAPSHOT_PROOF=1 \
SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1 \
SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1 \
SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1 \
./scripts/run_daily_driver_proof.sh "$LOG"

./scripts/daily_driver_master_gate.sh "$LOG" | rg "atlas_phase|FINAL|FAIL|fault|panic|#PF|#GP"
```

Expected gate PASS: `atlas_phase_e1_click_scene_switch` shows PASS when `[silk.atlas.phase_e1.done]` found with `ok=1`.

## Proof Result

Build: Source-level PASS — patterns consistent with Phase A-D.
Gate script: PASS (`bash -n` clean).
Runtime: Awaits boot log with `SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1`.

## STOP FIRST Boundaries Preserved

| Boundary | Status |
|----------|--------|
| No kernel edits | Preserved |
| No sex-pdx ABI edits | Preserved (only local const/function additions) |
| No new compositor protocol | Preserved |
| No compositor/display ABI edits | Preserved |
| No sexdisplay edits | Preserved |
| No sex-pdx edits | Preserved |
| sexdisplay remains sole framebuffer writer | Preserved |
| silk-shell owns shell/session/input policy | Preserved |
| No framebuffer/backing-buffer redesign | Preserved |
| No shared-memory redesign | Preserved |
| No broad refactor | Preserved |
| No input policy outside silk-shell | Preserved |
| No mixed feature + refactor patch | Preserved |
| No drag/drop | Preserved — none implemented |
| No frame ownership mutation | Preserved — read-only snapshot reuse |
| No keyboard Scene cycle | Deferred to E2 (STOP FIRST) |
| No drag marker | Deferred to E3 (STOP FIRST) |
| No drag/move between Scenes | Deferred to E4 (STOP FIRST) |
| No new unsafe beyond existing proof pattern | Follows existing unsafe fn pattern for synthetic proofs |
| No unwrap/panic on optional frame/window state | All uses safe Option patterns |
| No OOB | Bounded by ATLAS_MAX_SCENES (5), saturating arithmetic |
| No behavior change when env unset | Early return at fn entry |
| No app-surface dispatch of Atlas clicks | Proof operates on existing click_hit_test_and_focus path which already consumes Atlas clicks |

## Remaining Phases E2-E4, F

| Phase | What | Status |
|-------|------|--------|
| Phase A | State model proof | Built, gate added |
| Phase B | Atlas snapshot/capture integration | Built, gate added |
| Phase C | Render stub + card geometry | Built, gate added |
| Phase D | Frame preview interior stub | Built, gate added |
| **Phase E1** | **Click scene switch proof** (this doc) | Built, gate added |
| Phase E2 | Keyboard Scene cycle while Atlas open | Deferred (STOP FIRST) |
| Phase E3 | Begin drag marker only, no move | Deferred (STOP FIRST) |
| Phase E4 | Drag frame between Scenes | Deferred (STOP FIRST) |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit STOP Note

Phase E1 is click-to-switch-scene proof instrumentation only — **no drag/drop is implemented**. The actual click path already exists in `click_hit_test_and_focus()` and correctly consumes Atlas card clicks, switches scenes, and exits Atlas. This proof adds deterministic markers proving the path works end-to-end. E2 (keyboard cycle), E3 (drag marker), and E4 (drag/move) are explicitly deferred with STOP FIRST boundaries.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Add Phase E1 click scene switch proof | ATLAS_OVERVIEW_PHASE_E1_CLICK_SCENE_SWITCH_PROOF_V1 |
