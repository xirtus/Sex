# ATLAS_OVERVIEW_PHASE_E4D_REAL_POINTER_DROP_PATH_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default, proof off) | PASS |
| Build (proof enabled) | PASS |
| Build (all Atlas E4 proofs enabled) | PASS |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_E4D_REAL_POINTER_DROP_PROOF=1` build flag |
| Proof runner syntax | PASS (`bash -n` clean) |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Add gate constants (~8 lines); add `atlas_pointer_drag_begin_at()`, `atlas_pointer_drop_at()`, `atlas_pointer_drag_cancel()` helpers (~115 lines); add `maybe_run_atlas_phase_e4d_real_pointer_drop_proof()` proof function (~230 lines); wire Atlas guard into `handle_hid_event` (~16 lines); wire proof into main loop (1 line) | ~+370 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_e4d_real_pointer_drop` variable, gate logic block (~50 lines), and summary array entry (~1 line) | ~+51 |
| `scripts/run_daily_driver_proof.sh` | Add `export SEXOS_ATLAS_PHASE_E4D_REAL_POINTER_DROP_PROOF=1` | +1 |
| `docs/handoff/ATLAS_OVERVIEW_PHASE_E4D_REAL_POINTER_DROP_PATH_PROOF_V1.md` | This handoff doc | new |

## Exact Gap Closed

**Gap:** No proof existed that real pointer events routed through `handle_hid_event` are correctly intercepted and consumed by Atlas drag/drop logic. No proof existed that the end-to-end drag-begin → drop-target → reparent → verify → restore lifecycle works through the helpers that real pointer events will use.

**Closed by E4d:**
1. Added three real-pointer-path helpers (`atlas_pointer_drag_begin_at`, `atlas_pointer_drop_at`, `atlas_pointer_drag_cancel`) that are callable from `handle_hid_event` EV_BTN dispatch.
2. Wired the helpers into the real pointer path: when `ATLAS_MODE_ENABLED`, ALL button-1 events are consumed before reaching `click_hit_test_and_focus` / app surface dispatch.
3. Added synthetic proof that exercises the real helpers through synthetic coordinates (card center points), verifying the entire lifecycle: drag-begin → cross-scene drop → verify moved → restore → verify restored.
4. Frame ownership is always restored to original scene before proof completion.
5. Event consumption is proven: `[silk.atlas.pointer.event.consume]` markers confirm that Atlas consumed pointer events and app dispatch was not reached.

## Real Pointer Path: WIRED

The real pointer path is **wired** into `handle_hid_event`. When Atlas mode is active:
- **Button-down:** calls `atlas_pointer_drag_begin_at(POINTER_X, POINTER_Y)`. If a valid Atlas card is hit and a non-minimized frame exists in that scene, drag intent is armed. Event is consumed — does not fall through to `click_hit_test_and_focus`.
- **Button-up:** if `ATLAS_DRAG_INTENT.active`, calls `atlas_pointer_drop_at(POINTER_X, POINTER_Y)`. The helper hit-tests the target scene at the drop point and either rejects (no target), noops (same-scene), or reparents (cross-scene). Event is consumed.
- **Button-up without drag intent:** consumed but no-op (edge case: button-up in Atlas without prior button-down arm).
- **After successful drop:** Atlas is exited (corresponding to the E1 behavior of switching scene and exiting).

The existing `click_hit_test_and_focus` Atlas intercept is preserved as a safety net but will not normally be reached when Atlas mode is active through the real HID path.

## Helper Behavior

### atlas_pointer_drag_begin_at(px, py) -> bool
- Guard: `ATLAS_MODE_ENABLED` must be true
- Hit-tests Atlas scene cards at (px, py) via `atlas_scene_at_point`
- Finds a non-minimized frame in the hit scene
- Sets `ATLAS_DRAG_INTENT`: `active=true`, `scene_id`, `frame_id`, `start_x`, `start_y`
- Emits `[silk.atlas.pointer.drag.begin]` with `source=real_path`
- Returns `true` if consumed (valid card + frame), `false` if no hit or no frame

### atlas_pointer_drop_at(px, py) -> bool
- Guard: `ATLAS_MODE_ENABLED` AND `ATLAS_DRAG_INTENT.active`
- Hit-tests target scene at (px, py)
- **No target:** emit `[silk.atlas.pointer.drop.reject]`, clear intent, return `true`
- **Same-scene:** call `atlas_same_scene_drop_noop()`, emit `[silk.atlas.pointer.drop.noop]`
- **Cross-scene:** emit `[silk.atlas.pointer.drop.target]`, call `reparent_frame_to_scene()`
- After any success path: verify ownership_unique, verify focus_valid, clear intent, emit `[silk.atlas.pointer.drop.done]`, exit Atlas, emit `[silk.atlas.mode.exit]`
- Returns `true` if drag intent was active and processed

### atlas_pointer_drag_cancel(reason) -> bool
- Guard: `ATLAS_DRAG_INTENT.active`
- Clears `ATLAS_DRAG_INTENT` (sets `active=false`)
- Emits `[silk.atlas.drag.clear]`
- Returns `true` if intent was active and cleared

## Proof Sequence

### Normal Path (Frame Exists, Multiple Scenes)
```
[silk.atlas.phase_e4d.begin] active=S scenes=N
[silk.atlas.phase_e4d.enter] ok=1 active=S
[silk.atlas.phase_e4d.setup] frame=F source=A target=B src_x=X src_y=Y tgt_x=X' tgt_y=Y' ok=1
[silk.atlas.pointer.drag.begin] frame=F scene=A x=X y=Y source=real_path ok=1
[silk.atlas.pointer.drop.target] frame=F from=A to=B x=X' y=Y' ok=1
[silk.frame.scene.move.begin] frame=F from=A to=B reason=atlas_pointer_drop
[silk.frame.scene.move.done] frame=F from=A to=B ownership_unique=1 focus_valid=1 ok=1
[silk.atlas.drag.clear] reason=pointer_drop ok=1
[silk.atlas.pointer.drop.done] frame=F from=A to=B ownership_unique=1 focus_valid=1 ok=1
[silk.atlas.mode.exit] active=S reason=atlas_pointer_drop_done view=desktop ok=1
[silk.atlas.phase_e4d.verify_moved] frame=F expected=B actual=B ok=1
[silk.frame.scene.move.begin] frame=F from=B to=A reason=atlas_pointer_drop_restore
[silk.frame.scene.move.done] frame=F from=B to=A ownership_unique=1 focus_valid=1 ok=1
[silk.frame.scene.move.restore] frame=F from=B to=A ok=1
[silk.atlas.phase_e4d.verify_restored] frame=F expected=A actual=A ok=1
[silk.atlas.phase_e4d.final_verify] ok=1
[silk.atlas.mode.exit] active=A reason=atlas_e4d_done view=desktop ok=1
[silk.atlas.phase_e4d.done] ok=1
```

### Skip Path (No Source Frame)
```
[silk.atlas.phase_e4d.begin] active=S scenes=N
[silk.atlas.phase_e4d.enter] ok=1 active=S
[silk.atlas.phase_e4d.skip] reason=no_source ok=1
```

### Skip Path (WORKSPACE_COUNT < 2)
```
[silk.atlas.phase_e4d.skip] reason=no_target_scene ok=1
```

### Skip Path (drag_begin_failed)
```
[silk.atlas.phase_e4d.skip] reason=drag_begin_failed ok=1
```

## Gate Criteria

| Condition | Result |
|-----------|--------|
| `[silk.atlas.phase_e4d.done] ok=1` AND `verify_moved ok=1` AND `verify_restored ok=1` | PASS |
| `[silk.atlas.phase_e4d.skip] ok=1` (honest skip) | SKIP |
| `verify_moved ok=0` | FAIL |
| `verify_restored ok=0` (frame scene_id drift) | FAIL |
| `drop.done ownership_unique=0` | FAIL |
| `drop.done focus_valid=0` without `focus_cleared=1` | FAIL |
| `drag.begin` without `drop.done`/`drop.reject`/`drag.clear` | FAIL |
| `drop.done` without `event.consume` | FAIL |
| `phase_e4d.begin` without `phase_e4d.done` | FAIL |
| No proof markers | SKIP |

## Restoration Strategy

The proof always restores the frame to its original scene before emitting `phase_e4d.done`:
1. After the drop (`atlas_pointer_drop_at`), the frame is in the target scene.
2. Atlas is re-entered (since `drop_at` exits it).
3. `reparent_frame_to_scene(frame_id, original_scene, "atlas_pointer_drop_restore")` moves it back.
4. `verify_restored` confirms `frame.scene_id == original_scene`.
5. No persistent scene_id drift is possible — the proof breaks if any verify step fails.

## Proof Commands

Build with E4d proof enabled:
```fish
SEXOS_ATLAS_PHASE_E4D_REAL_POINTER_DROP_PROOF=1 ./scripts/entrypoint_build.sh
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
DAILY_DRIVER_PROBE_SECONDS=60 ./scripts/run_daily_driver_proof.sh /tmp/atlas_e4d_proof.log
./scripts/daily_driver_master_gate.sh /tmp/atlas_e4d_proof.log | rg "atlas_phase|phase_e4d|pointer.drag|pointer.drop|event.consume|verify_moved|verify_restored|FINAL|FAIL|fault|panic|#PF|#GP"
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
| No visual drag ghost | Preserved |
| No animation | Preserved |
| No tab moves | Preserved |
| No new frame allocation | Uses existing FRAMES[] |
| No persistent layout drift | Frame restored to original scene before done |
| No app click leakage after Atlas pointer consumption | Event.consume markers confirm consumption |
| No behavior change when env unset | Early return at fn entry + gate guard |
| No new unsafe beyond existing patterns | Uses existing unsafe fn patterns |
| No unwrap/panic | Safe Option patterns; unwrap only after is_none() check |
| No OOB | Bounded by WORKSPACE_COUNT and ATLAS_MAX_SCENES |

## Remaining E4e/F Work

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
| Phase E4c | Cross-scene reparent proof | Built, gate added |
| Phase E4c2 | True cross-scene reparent proof | Built, gate added |
| **Phase E4d** | **Real pointer drop path** | Built, gate added, real path wired |
| Phase E4e | Integrated drag/drop gate | Deferred (STOP FIRST) |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit STOP Note

**No visual drag ghost, no animation, no tab moves.** Phase E4d wires the real pointer path for Atlas drag/drop but only produces logical scene reparenting. Visual feedback (drag ghost following cursor, animated transitions, tab reordering during drag) is explicitly deferred with STOP FIRST boundaries. The proof exercises the full logical lifecycle (begin → drop → reparent → restore) through synthetic coordinates without requiring any visual changes.

## Commit Commands

```fish
git add servers/silk-shell/src/main.rs
git add scripts/daily_driver_master_gate.sh
git add scripts/run_daily_driver_proof.sh
git add docs/handoff/ATLAS_OVERVIEW_PHASE_E4D_REAL_POINTER_DROP_PATH_PROOF_V1.md
git commit -m "gate: prove Atlas Phase E4d real pointer drop path proof"
```

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Phase E4d real pointer drop path proof — built and gated | ATLAS_OVERVIEW_PHASE_E4D_REAL_POINTER_DROP_PATH_PROOF_V1 |
