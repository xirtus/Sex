# A5_FRAME_LIGHTS_FSM_V1

**Status:** Frame light actions wired through lifecycle FSM.
**Date:** 2026-05-04
**Purpose:** Wire red/yellow/green frame light dispatch through A2-A4 lifecycle FSM. Add drag-before-close guard, lifecycle validation for zoom, and proof markers for all frame light transitions.

---

## 1. Changes

- `servers/silk-shell/src/main.rs` — 4 edit points
- `docs/handoff/A5_FRAME_LIGHTS_FSM_V1.md` — this file

No kernel/ABI/sex-pdx/sexdisplay changes. No WINDOWS Vec migration. No 0xEE collision fix.

## 2. Close/Red Frame Light — FSM Order

`close_surface_from_frame_light()` now executes in this order:

1. Check `surface_is_alive()` → return false if dead
2. **Drag guard:** if `InteractionState::Dragging` on target surface → cancel drag → `[frame.light.close.reject.drag]`
3. **Clear focus first** if this surface was focused
4. Set `SURFACE_N_ALIVE = false`
5. `set_lifecycle_state(Closing)` — bumps generation
6. `tombstone_surface()`
7. `set_lifecycle_state(Tombstoned)` — bumps generation
8. `[frame.light.close.fsm]` proof marker
9. `pdx_call(0xEE)` — sexdisplay destroy
10. `clear_focus_if_dead()` — fallback focus
11. `clear_drag_if_dead()` — cleanup
12. `clear_hover_if_wrong_scene()`
13. `tile_visible_frames()` + `snap_capture_layout()`

## 3. Minimize/Yellow Frame Light — FSM Order

`minimize_frame()` continues existing flow with proof marker:

1. Check not already minimized → return false
2. Get surface_id, check alive
3. `set_frame_minimized(true)`
4. `set_lifecycle_state(Minimized)`
5. `pdx_call(0xEE)` — sexdisplay hide
6. `clear_drag_if_dead()`
7. Clear hover
8. `clear_focus_if_dead()`
9. `[frame.light.minimize.fsm]`
10. `snap_capture_layout()`

## 4. Restore — FSM Order

`restore_minimized_frame()` continues existing flow:

1. Check is minimized → return false
2. Get surface_id, check alive
3. `set_frame_minimized(false)`
4. `set_lifecycle_state(Visible)`
5. 0xEC upsert to sexdisplay
6. `try_set_focus(surface_id)`
7. `[frame.light.restore.fsm]`
8. `tile_visible_frames()` + `snap_capture_layout()`

## 5. Zoom/Green Frame Light — Lifecycle Validation

`toggle_zoom_frame()` now validates lifecycle state:

1. Check surface is not in Closing/Tombstoned/Destroyed → `[frame.light.zoom.fsm.reject]` if invalid
2. Toggle zoom/unzoom
3. `[frame.light.zoom.fsm]` on success

## 6. Drag-Before-Close Handling

**Decision:** Cancel drag on target surface before close transition.

- If `InteractionState::Dragging { surface_id }` matches the target surface → cancel drag via `try_transition(Idle)`, emit `[frame.light.close.reject.drag]`, then proceed with close
- If drag is on a different surface → close proceeds without cancellation
- This matches the A2 spec: "Drag must cancel before lifecycle transition"

## 7. Proof Markers Added

| Marker | When |
|--------|------|
| `[frame.light.close.reject.drag]` | Drag cancelled before close on target surface |
| `[frame.light.close.fsm]` | Close FSM transition complete |
| `[frame.light.minimize.fsm]` | Minimize FSM transition complete |
| `[frame.light.restore.fsm]` | Restore FSM transition complete |
| `[frame.light.zoom.fsm]` | Zoom toggle successful |
| `[frame.light.zoom.fsm.reject]` | Zoom rejected (invalid lifecycle state) |

## 8. Build Result

**Build:** Passed (ISO produced successfully)
**Code-specific errors:** Zero

## 9. Behavior Changes

- **Drag guard:** Close now cancels active drag on target surface before transition (was: after)
- **Focus clear:** Close now clears focus on target surface before lifecycle transition (was: after)
- **Zoom validation:** Zoom rejected for Closing/Tombstoned/Destroyed surfaces
- **Proof markers:** All frame light actions now emit FSM markers

## 10. Behavior Intentionally Unchanged

- ❌ No 0xEE opcode collision fix (still collides for close vs minimize)
- ❌ No WINDOWS Vec migration
- ❌ No proof marker renaming to `[comp.*]` convention
- ❌ No DestroyFocused keyboard handler refactor (inline close path bypasses close_surface_from_frame_light)
- ❌ No caller identity validation

## Document References

- `docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md`
- `docs/handoff/A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1.md`
- `docs/handoff/A3_SHELL_LIFECYCLE_MODEL_V1.md`
- `servers/silk-shell/src/main.rs`
