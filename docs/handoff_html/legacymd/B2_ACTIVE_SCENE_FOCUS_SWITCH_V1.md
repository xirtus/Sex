# B2: Active-Scene Focus + Switch Proof

**Status:** Approved
**Commit:** `3b7db18`
**Build:** Passed (ISO produced)
**Behavior:** No visible change — focus hardening only.

## Purpose

Harden and prove active-scene focus behavior over the B1 Scene/Frame/Tab model.
Prevents focus from landing on surfaces in inactive scenes, even if
`try_set_focus()` is called directly (not just through `clear_focus_if_wrong_scene()`).

## Changes to `servers/silk-shell/src/main.rs`

### 1. `surface_scene_id()` helper (new)
- Scans `FRAMES`/tabs to find which scene a surface belongs to
- Returns `Option<u8>` — `None` for panels, cursor, and non-frame surfaces
- Used by `try_set_focus()` for the active-scene guard

### 2. `try_set_focus()` — added scene guard (guard 8)
New guard added after lifecycle (guard 6) and generation (guard 7):

```rust
// B2: Reject focus if surface belongs to a frame in a non-active scene.
// Panels, cursor, and non-frame surfaces have no scene association.
if let Some(scene) = surface_scene_id(sid) {
    if scene != ACTIVE_SCENE_IDX {
        serial_println!("[scene.focus.reject.inactive] id={} ...", sid);
        return false;
    }
}
```

**Full guard order:**
| # | Guard | Reject marker |
|---|-------|---------------|
| 1 | `sid == 0` | clear focus |
| 2 | `!is_focusable_surface` | `[shell.focus.reject.nonfocusable]` |
| 3 | `!surface_is_alive` | `[shell.focus.reject.dead]` |
| 4 | `is_tombstoned` | `[lifecycle.tombstone.reject_focus]` |
| 5 | `!surface_is_lifecycle_focusable` | `[focus.lifecycle.reject]` |
| 6 | `!focus_ref_is_current` | `[focus.generation.reject]` |
| 7 | `scene != ACTIVE_SCENE_IDX` | `[scene.focus.reject.inactive]` |
| 8 | **Commit** `FOCUSED_SURFACE_ID` | `[focus.ref.commit]` |

### 3. `switch_scene()` — old scene flags
- Added `scene_update_flags(prev)` for the previous scene before switching
- Both old and new scene flags are updated on switch

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[scene.focus.reject.inactive]` | try_set_focus() | Surface in inactive scene |
| `[scene.focus.reject.inactive]` | clear_focus_if_wrong_scene() | Focused surface not in active scene |
| `[scene.switch]` | switch_scene() | Scene transition complete |

## Invariants

1. `try_set_focus()` never commits focus for surfaces in inactive scenes
2. `clear_focus_if_wrong_scene()` delegates to `try_set_focus()` which enforces all guards
3. `surface_scene_id()` returns `None` for panel/cursor surfaces — always eligible
4. Scene switch always updates flags for both old and new scene
5. No new IPC, no sexdisplay changes, no kernel/ABI edits

## Deferred

- B3: Deterministic tiling over Scene/Frame/Tab
- B4: Tab strip + hover/frame-light behavior
- C1: Atlas (only after B3/B4 stable)

## Dependencies

- **Requires:** B1 (Scene/Frame/Tab core model), A4 (focus guards), A7 (opcode audit)
- **Blocks:** B3 (deterministic tiling), C1 (Atlas)
