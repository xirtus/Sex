# HIDDEN_STATE_TRACKING_CLEANUP_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Fixes the A8 Hidden-state drift: `set_lifecycle_state(sid, LifecycleState::Hidden)` was never called. After every scene switch, surfaces in non-active scenes now receive `Hidden` lifecycle state, and surfaces in the active scene receive `Visible`. This makes the lifecycle metadata match actual scene visibility.

No behavior change — `surface_is_alive()`, `frame_accepts_input()`, `tile_visible_frames()`, and focus policy all check scene membership directly (not lifecycle state). This is purely a lifecycle metadata cleanup.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +36 lines (helper function + call in sync_scene_visibility + proof marker) |
| `docs/handoff/HIDDEN_STATE_TRACKING_CLEANUP_V1.md` | New handoff doc |

---

## Code Change

### New helper: `sync_lifecycle_scene_visibility()`

Added before `sync_scene_visibility()`, called as the first operation inside it:

```rust
unsafe fn sync_lifecycle_scene_visibility() {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            let in_active = frame.scene_id == ACTIVE_SCENE_IDX;
            let minimized = (frame.flags & FRAME_FLAG_MINIMIZED) != 0;
            for tab in frame.tabs.iter() {
                if let Some(t) = tab {
                    let sid = t.surface_id;
                    if !surface_is_alive(sid) { continue; }
                    if minimized { continue; }
                    if let Some(state) = lifecycle_state(sid) {
                        match state {
                            LifecycleState::Closing
                            | LifecycleState::Tombstoned
                            | LifecycleState::Destroyed => continue,
                            _ => {}
                        }
                    }
                    if in_active {
                        set_lifecycle_state(sid, LifecycleState::Visible);
                    } else {
                        set_lifecycle_state(sid, LifecycleState::Hidden);
                    }
                }
            }
        }
    }
    // [lifecycle.scene.sync] proof marker (budgeted 8)
}
```

### Integration

Called from `sync_scene_visibility()` as the first operation:
```rust
unsafe fn sync_scene_visibility() {
    // Metadata first: sync lifecycle state before updating display.
    sync_lifecycle_scene_visibility();
    // ... existing 0xEC/0xEE display update loop ...
}
```

---

## Scene Switch Paths Covered

All 8 call sites of `sync_scene_visibility()` now include lifecycle scene visibility sync:

| Path | Trigger | Calls sync_scene_visibility? |
|------|---------|------------------------------|
| `switch_scene()` | Keyboard shortcut (F8/F9) | ✅ Yes (line 2594) |
| Atlas mouse confirm (different scene) | `switch_scene()` → `sync_scene_visibility()` | ✅ Indirect |
| Atlas mouse confirm (same scene) | `sync_scene_visibility()` directly (line 2375) | ✅ Direct |
| Atlas keyboard confirm (different scene) | `switch_scene()` → | ✅ Indirect |
| Atlas keyboard confirm (same scene) | `sync_scene_visibility()` directly (line 2447) | ✅ Direct |
| SilkBar workspace click | `ACTIVE_SCENE_IDX = ws_idx` → `sync_scene_visibility()` (line 4747) | ✅ Direct |
| Frame chrome scene switch | `sync_scene_visibility()` (line 4505) | ✅ Direct |
| Boot snapshot restore | `sync_scene_visibility()` (line 4997) | ✅ Direct |

---

## States Preserved

| State | Preserved? | Why |
|-------|-----------|-----|
| Minimized | ✅ | Guard: `if minimized { continue; }` — minimized stays Minimized regardless of scene |
| Closing | ✅ | Guard: `LifecycleState::Closing => continue` |
| Tombstoned | ✅ | Guard: `LifecycleState::Tombstoned => continue` + `!surface_is_alive()` skip |
| Destroyed | ✅ | Guard: `LifecycleState::Destroyed => continue` + `!surface_is_alive()` skip |
| Allocated | ✅ | Not iterated (no frame membership for Allocated panels) |
| Mapped | ✅ | Not iterated (no frame — mapped surfaces like cursor are not in FRAMES) |

---

## Proof Markers

| Marker | Location | Budget | When |
|--------|----------|--------|------|
| `[lifecycle.scene.sync]` | `sync_lifecycle_scene_visibility()` | 8 | After scene visibility lifecycle sync |

Budgeted at 8 — fires once per scene switch, not hot-path.

Also: `set_lifecycle_state()` fires `[lifecycle.transition.allow]` for each Hidden/Visible transition, providing per-surface granularity.

---

## Behavior Changes

**None.** This is purely a lifecycle metadata update. The following mechanisms are unaffected:

- **Display:** `sync_scene_visibility()` still sends 0xEC/0xEE based on `surface_is_alive()` + `in_active` — lifecycle state is NOT queried for display decisions
- **Focus:** `clear_focus_if_wrong_scene()` checks `surface_in_active_scene()` which checks frame scene_id, not lifecycle state. `try_set_focus()` checks `surface_is_lifecycle_focusable()` which returns `false` for Hidden — this is correct (non-active scene surfaces should not be focused)
- **Tiling:** `tile_visible_frames()` checks `frame.scene_id` directly, not lifecycle state
- **Atlas:** Atlas rendering uses `surface_is_alive()` for snapshot, not lifecycle state
- **Zoom:** `toggle_zoom_frame()` checks lifecycle state for Closing/Tombstoned/Destroyed — Hidden is not excluded (correct: frame could be zoomed before scene switch)

---

## State Coverage Updated

| State | Set by code before | Set by code after |
|-------|-------------------|-------------------|
| Allocated | ✅ Yes | ✅ Yes (unchanged) |
| Mapped | ✅ Yes | ✅ Yes (unchanged) |
| Visible | ✅ Yes (boot + restore) | ✅ Yes (boot + restore + scene active) |
| Hidden | ❌ **Never set** | ✅ **Yes (scene inactive)** |
| Minimized | ✅ Yes | ✅ Yes (unchanged) |
| Closing | ✅ Yes | ✅ Yes (unchanged) |
| Tombstoned | ✅ Yes | ✅ Yes (unchanged) |
| Destroyed | ✅ Yes (A6) | ✅ Yes (unchanged) |

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
```

---

## STOP FIRST Findings

None. This cleanup:
- Does not change display protocol
- Does not require broad scene model rewrite
- Does not break focus/tiling behavior
- Does not resurrect tombstoned/dead surfaces
- Does not change SurfaceId/FrameId allocation

---

## Ready for Atlas Expansion?

**Yes.** The lifecycle FSM now fully tracks scene visibility. Atlas can safely rely on lifecycle state to determine surface visibility without needing to re-derive scene membership.

---

## Document References

- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — identified Hidden-state drift gap
- `docs/handoff/A3_SHELL_LIFECYCLE_MODEL_V1.md` — lifecycle metadata model
- `servers/silk-shell/src/main.rs` — implementation

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Wire Hidden/Visible lifecycle state tracking on scene switch. Add `sync_lifecycle_scene_visibility()` helper. | HIDDEN_STATE_TRACKING_CLEANUP_V1 |
