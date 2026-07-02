# ATLAS_OVERVIEW_PHASE_E4A_DRAG_MOVE_AUDIT_PLAN_V1

## Result: STOP FIRST — audit complete, safe implementation ladder ready, no code changes

## Status

| Field | Value |
|-------|-------|
| Audit scope | Frame ownership model, scene membership, drag/focus/hover state reconciliation |
| Files audited | `servers/silk-shell/src/main.rs` (full scan of FRAMES, SCENES, scene_id, switch_scene, drag, focus, hover, Atlas E1-E3 proof paths) |
| Implementation | None. Docs-only audit + plan. |
| E4b-E4e gate code | None written. Markers defined but not implemented. |
| Build | PASS (no code changed) |
| Gate script syntax | PASS (`bash -n` clean) |

## Files Changed

None. This is a docs-only audit + plan. No code, no gate entries.

## Root Cause / Why E4 Is Risky

Frame ownership is stored **on the frame** (`ShellFrame.scene_id`) but scene membership is only **derived** by scanning FRAMES[] at snapshot time. There is no reverse index (scene → frame list) that must stay consistent — which is good. However:

1. **No existing reparent helper exists.** Frame `scene_id` is set only at creation (in placeholder `attach.frame` code). There is no function that safely changes `frame.scene_id` after boot.

2. **Focus validity risk.** The active focus target (`FOCUSED_SURFACE_ID`) is not automatically re-validated when a frame's scene_id changes while its scene stays active. `clear_focus_if_wrong_scene()` exists but is called only inside `switch_scene()` — not after a frame reparent.

3. **Drag/hover state risk.** `clear_drag_if_wrong_scene()` and `clear_hover_if_wrong_scene()` also exist but are only called on scene-switch paths, not after frame reparent.

4. **Active-frame reference risk.** `selected_frame_id()` derives the active frame from `FOCUSED_SURFACE_ID`. Moving a frame out of the active scene without clearing focus would leave a stale active-frame reference pointing to a frame now in a different scene.

5. **Scene-count derivation.** Scene frame counts in `SceneDescriptor` are derived on-demand by `atlas_capture_snapshot()`. There is no incrementally-maintained count to corrupt — but snapshot must be called after every reparent.

6. **AtlasDragIntent is separate from real drag.** Phase E3 markers use a standalone `ATLAS_DRAG_INTENT` struct. E4 must bridge from `ATLAS_DRAG_INTENT` (synthetic marker) to real `InteractionState` drag paths — or add a new Atlas-specific drag path that does NOT leak into app pointer dispatch.

### Is E4 Fundamentally Unsafe?

**No.** The ownership model is unambiguous and well-scoped. Moving a frame requires changing exactly one field (`frame.scene_id`) plus a small reconciliation routine. No kernel, PDX, display, or compositor changes are needed. The risk is entirely in correctly sequencing focus/drag/hover reconciliation after the scene_id mutation.

## Frame/Scene Ownership Findings (Audit Answers)

### 1. Authoritative Frame Storage

```
static mut FRAMES: [Option<ShellFrame>; MAX_FRAMES] = [None; MAX_FRAMES];
```

`MAX_FRAMES = 9`. Each `ShellFrame` carries:
- `frame_id: u32`
- `scene_id: u8` ← **authoritative scene ownership**
- `active_tab: u8`, `tab_count: u8`
- `tabs: [Option<ShellTab>; MAX_TABS_PER_FRAME]`
- `flags: u32` (minimized, zoomed, etc.)

### 2. One Frame, One Scene

**Yes.** A frame's `scene_id` is a single `u8` field. A frame belongs to exactly one scene at all times. There is no multi-scene membership or shared ownership.

### 3. Scene Membership Storage

Scene membership is **derived, not stored.** The `SCENES[]` array (`[Scene; ATLAS_MAX_SCENES]`) stores per-scene metadata (flags, label, accent, pinned) but does NOT contain a frame list. Frame lists appear only in the derived `AtlasSnapshot.scenes[].frame_ids[]` which is rebuilt from scratch each time `atlas_capture_snapshot()` is called.

To find all frames in a scene: scan FRAMES[] for `frame.scene_id == target_scene`.

### 4. Does switch_scene() Assume Immutable Membership?

**No.** `switch_scene()` changes `ACTIVE_SCENE_IDX` and then calls:
- `sync_scene_visibility()` — iterates FRAMES[], hides/shows surfaces based on `frame.scene_id == ACTIVE_SCENE_IDX`
- `clear_focus_if_wrong_scene()` — iterates FRAMES[] by scene_id
- `clear_drag_if_wrong_scene()` — checks `surface_in_active_scene()`
- `clear_hover_if_wrong_scene()` — same
- `tile_active_scene_frames()`
- `atlas_capture_snapshot()` — rebuilds all SceneDescriptors

All of these dynamically scan FRAMES[] by current `frame.scene_id`. They do NOT cache frame-per-scene counts. They would automatically pick up a changed `frame.scene_id` on the next call.

**Conclusion:** `switch_scene()` is robust against frame reparenting because it re-derives everything from FRAMES[] at call time.

### 5. State That Must Update When Moving a Frame

| State | Source | Action Required |
|-------|--------|-----------------|
| `frame.scene_id` | ShellFrame | **Write** new scene_id |
| Active scene index | `ACTIVE_SCENE_IDX` | None — unchanged by reparent |
| Active frame ref | `selected_frame_id()` | Derived — repoints automatically if focus follows |
| Focused surface | `FOCUSED_SURFACE_ID` | Must clear+repoint if frame left active scene |
| Drag state | `INTERACTION` | Must cancel if dragged surface now in wrong scene |
| Hover state | `HOVERED_FRAME_ID` | Must clear if hovered frame left active scene |
| Minimized state | `frame.flags` | Unchanged — minimized is per-frame, not per-scene |
| Tab stack | `frame.tabs[]` | Unchanged — tabs move with the frame |
| Atlas snapshot | `ATLAS_SNAPSHOT` | Must re-derive via `atlas_capture_snapshot()` |
| Scene descriptors | `SceneDescriptor.frame_ids[]` | Derived automatically on snapshot |
| SilkBar workspace | `OP_SILKBAR_WORKSPACE_ACTIVE` | None — workspace unchanged |

### 6. Existing Safe Helper for Moving/Reparenting Frames?

**None exists.** Frame `scene_id` is set exactly once at creation in placeholder attach code:
```rust
frame.scene_id = ACTIVE_SCENE_IDX;  // in linen/quil/mesh/collar/spindle/bell/command_palette attach
```

No code path ever changes `frame.scene_id` after initial assignment.

### 7. Invariants Needing Proof Before Real Move

1. **Single ownership:** After move, exactly one frame has that frame_id, and its scene_id is the target.
2. **No duplicate:** No other frame has the same frame_id (structurally guaranteed by FRAMES[] array indexing).
3. **Focus validity:** If the moved frame contained the focused surface AND source == active scene, focus must repoint to a valid surface in the active scene.
4. **Drag safety:** If a drag was active on a surface in the moved frame AND source == active scene, drag must cancel.
5. **Hover safety:** If the hovered frame was the moved frame AND source == active scene, hover must clear.
6. **Scene emptiness:** If the frame was the last non-minimized frame in the source scene, the scene becomes empty (but not destroyed — scene object persists).
7. **Lifecycle correctness:** Surfaces in the moved frame must transition Visible→Hidden if source was active and target is not, or Hidden→Visible if target is active and source was not.
8. **Snapshot consistency:** Atlas snapshot must be re-derived after reparent.

### 8. Can E4 Be Synthetic First Without Real Pointer Path?

**Yes.** Follows the E1/E2/E3 pattern: synthetic proof functions that exercise the logic without real pointer input. E4b (same-scene no-op) and E4c (cross-scene reparent) can both be synthetic proofs that mutate `frame.scene_id` directly with full invariant checks, without requiring real pointer drop targets.

### 9. Negative Cases Required

| Case | Expected Behavior |
|------|-------------------|
| Move to same scene (source == target) | No-op. Emit `[silk.frame.scene.move.noop]` |
| No card/preview hit at drop point | No-op or reject. Emit reject marker. |
| Dead/missing frame (frame_id not in FRAMES[]) | Reject. Emit `[silk.frame.scene.move.reject] reason=dead_frame` |
| Invalid target scene (OOB) | Reject. Emit `[silk.frame.scene.move.reject] reason=invalid_scene` |
| Dragging minimized frame | Reject. Emit `[silk.frame.scene.move.reject] reason=minimized` |
| Moving last frame out of scene | Allow. Source scene becomes empty (SCENE_FLAG_EMPTY set). Scene object persists. |
| Drop point lands on app surface (not Atlas card) | Must NOT dispatch click to app. Drag-drop must be gated to Atlas mode. |

### 10. Does E4 Require Sexdisplay, PDX, Kernel, or ABI Changes?

**No.** Frame `scene_id` is purely shell-internal state maintained in `FRAMES[]`. Visibility sync is already performed by `sync_scene_visibility()` which scans FRAMES[] dynamically. No new IPC, no new syscalls, no new PDX opcodes, no display protocol changes.

## E4 Implementation Ladder

### E4a — Audit + Plan Doc Only (THIS PHASE)

| Item | Detail |
|------|--------|
| Files changed | None (docs-only: `docs/handoff/ATLAS_OVERVIEW_PHASE_E4A_DRAG_MOVE_AUDIT_PLAN_V1.md`) |
| Helpers defined | None. Markers defined in spec only (see Future Markers below). |
| STOP conditions | No E4b-E4e code written. No runtime behavior change. |
| Proof commands | `bash -n scripts/daily_driver_master_gate.sh`; `bash -n scripts/run_daily_driver_proof.sh` |
| Forbidden | Any code change to main.rs, gate script, or proof script |

### E4b — Synthetic Same-Scene No-Op Move Proof

**Scope:** Prove that same-scene reparent is detected as no-op.

| Item | Detail |
|------|--------|
| Files | `servers/silk-shell/src/main.rs` (+~80 lines gate + proof fn), `scripts/daily_driver_master_gate.sh` (+~30 gate), `scripts/run_daily_driver_proof.sh` (+1 export) |
| Gate constant | `SEXOS_ATLAS_PHASE_E4B_SAME_SCENE_NOOP_PROOF=1` |
| Proof function | `maybe_run_atlas_phase_e4b_same_scene_noop_proof()` — enters Atlas, selects a frame in active scene, calls synthetic reparent to same scene_id, verifies no-op, verifies invariants unchanged |
| Key markers | `[silk.atlas.phase_e4.begin]`, `[silk.frame.scene.move.noop]`, `[silk.atlas.phase_e4.done]` |
| STOP conditions | Do NOT implement cross-scene move. Do NOT mutate frame.scene_id to a different value. |
| Proof commands | `SEXOS_ATLAS_PHASE_E4B_SAME_SCENE_NOOP_PROOF=1 ./scripts/entrypoint_build.sh` |

### E4c — Synthetic Cross-Scene Reparent Proof

**Scope:** Prove that cross-scene frame reparent is safe with full invariant verification.

| Item | Detail |
|------|--------|
| Files | `servers/silk-shell/src/main.rs` (+~120 lines gate + proof fn + reparent helper), `scripts/daily_driver_master_gate.sh` (+~30 gate), `scripts/run_daily_driver_proof.sh` (+1 export) |
| Gate constant | `SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1` |
| New helper | `unsafe fn reparent_frame_to_scene(frame_id: u32, new_scene: u8) -> bool` — changes frame.scene_id, reconciles focus/drag/hover/visibility, re-derives snapshot. Returns false on reject. |
| Proof function | `maybe_run_atlas_phase_e4c_cross_scene_reparent_proof()` — synthetic: enters Atlas, selects frame, reparents to a different scene, verifies all invariants, optionally reparents back |
| Key markers | `[silk.frame.scene.move.begin]`, `[silk.frame.scene.move.done]`, `[silk.frame.scene.move.reject]` |
| STOP conditions | Do NOT wire to real pointer drop path. Do NOT add visual drag ghost. Do NOT add animation. |
| What is forbidden | Changing any code outside the proof function and the reparent helper. No changes to `switch_scene()`, sexdisplay, PDX, kernel. |
| Proof commands | `SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1 ./scripts/entrypoint_build.sh` |

### E4d — Real Pointer Drop Path Instrumentation

**Scope:** Wire Atlas drag drop detection into real pointer-up path, gated behind proof flag.

| Item | Detail |
|------|--------|
| Files | `servers/silk-shell/src/main.rs` (+~100 lines in pointer-up dispatch path) |
| Gate constant | `SEXOS_ATLAS_PHASE_E4D_POINTER_DROP_PROOF=1` |
| Behavior | On pointer-up in Atlas mode with active drag intent, call `atlas_scene_at_point()` to find target scene, call `reparent_frame_to_scene()` |
| STOP conditions | Do NOT remove existing proof function. Do NOT change app pointer dispatch. |
| Forbidden | Any change to non-Atlas pointer paths, any change to app surface click dispatch |

### E4e — Integrated Drag/Drop Gate

**Scope:** Run combined E4b+E4c+E4d in a single gate entry.

| Item | Detail |
|------|--------|
| Files | `scripts/daily_driver_master_gate.sh` (+~20 lines combined gate) |
| Gate variable | `gate_atlas_phase_e4_drag_move` |
| Criteria | All E4b, E4c, E4d markers present with ok=1, zero faults |

## Future Markers (Defined, NOT Implemented)

```
[silk.atlas.phase_e4.begin] from_scene=A to_scene=B frame=F
[silk.atlas.drag.drop.target] scene=B x=X y=Y ok=1
[silk.frame.scene.move.noop] frame=F scene=A reason=same_scene ok=1
[silk.frame.scene.move.begin] frame=F from=A to=B
[silk.frame.scene.move.done] frame=F from=A to=B ownership_unique=1 focus_valid=1 ok=1
[silk.frame.scene.move.reject] frame=F reason=...
[silk.atlas.phase_e4.done] ok=1
```

## Required Invariants (Future E4 Must Prove)

| # | Invariant | Verification Method |
|---|-----------|---------------------|
| 1 | One frame has exactly one scene owner | After move, scan FRAMES[] — frame appears in exactly one scene |
| 2 | Source scene count decrements or remains valid | Compare pre/post snapshot frame_count for source scene |
| 3 | Target scene count increments or remains valid | Compare pre/post snapshot frame_count for target scene |
| 4 | No duplicate frame in two scenes | Structurally guaranteed (single scene_id field) |
| 5 | Active focus valid after move | If moved frame contained focus AND source was active, focus repoints to active scene frame or clears to 0 |
| 6 | Dragging state clears after drop/cancel | `ATLAS_DRAG_INTENT.active` is false; `INTERACTION` not in Dragging state for moved surface |
| 7 | Same-scene drop is no-op | `reparent_frame_to_scene(f, s)` where `frame.scene_id == s` returns false, emits noop |
| 8 | Invalid target rejects | `reparent_frame_to_scene(f, s)` where `s >= WORKSPACE_COUNT` returns false |
| 9 | No app click leakage | Drop dispatch only active in Atlas mode; pointer-up in Atlas mode never reaches app hit-test |
| 10 | No #PF/#GP/panic/fault.kill | Gate script validates `faults_zero=PASS` |

## STOP Conditions (Any of These → Do Not Proceed to Implementation)

| Condition | Why |
|-----------|-----|
| Frame ownership model ambiguous | Already clear — not triggered |
| Moving frame requires broad refactor | Does not — single field mutation + reconciliation |
| Requires sexdisplay/compositor ABI change | Does not — all shell-internal state |
| Requires kernel/sex-pdx change | Does not — no new syscalls or opcodes |
| Focus validity cannot be proven | Must be proven in E4c before E4d |
| Duplicate ownership risk exists | Structurally impossible (single scene_id field) |
| Source/target scene counts not derivable | Derived on-demand by atlas_capture_snapshot() |
| App click leakage risk cannot be bounded | Bounded by Atlas-mode-only gate in E4d |

**Verdict:** No STOP condition is currently triggered. E4b can proceed. E4c requires focus proof first. E4d requires E4c passing first.

## Proof Commands Run

```fish
# Gate script syntax check
bash -n scripts/daily_driver_master_gate.sh  # PASS

# Proof runner syntax check
bash -n scripts/run_daily_driver_proof.sh    # PASS

# Build (no code changed, optional)
./scripts/entrypoint_build.sh                # PASS (unmodified build)
```

## Recommended Next Prompt

**E4b — Same-scene no-op synthetic proof.** Safe first step. No reparenting. No focus changes. No drag/hover state reconciliation. Just proves the detection path.

Exact next prompt:
```
Atlas Phase E4b: implement synthetic same-scene no-op drag/move proof.
Use the plan in docs/handoff/ATLAS_OVERVIEW_PHASE_E4A_DRAG_MOVE_AUDIT_PLAN_V1.md.
Define gate constant SEXOS_ATLAS_PHASE_E4B_SAME_SCENE_NOOP_PROOF=1.
Do not implement cross-scene reparent. Do not change frame.scene_id to a different scene.
Add gate entry to daily_driver_master_gate.sh.
```

## Commit Commands

```fish
# No code changes — docs-only commit
git add docs/handoff/ATLAS_OVERVIEW_PHASE_E4A_DRAG_MOVE_AUDIT_PLAN_V1.md
git commit -m "docs: Atlas Phase E4a drag/move audit and implementation plan"
```

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Phase E4a audit + plan — docs only, no code | ATLAS_OVERVIEW_PHASE_E4A_DRAG_MOVE_AUDIT_PLAN_V1 |
