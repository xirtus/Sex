# Input Core V1 — Patch Summary

## Date
2026-05-04

## Context
Redirected from Scene Persistence (postponed) to fix practical shell input bugs after Scene/Frame/Tab/Tiling work. Focus: ensure pointer/keyboard input only targets active-scene, non-minimized, non-tombstoned frames.

## Bugs Found

### B1: No scene guard in hit-test
`hit_test_at()` returned `HitTarget::Surface(sid)` for any alive surface regardless of which scene the surface's frame belonged to.

### B2: No scene guard in chrome hit-test
`hit_test_surface_chrome()` returned frame chrome hits (rim, tab strip) for frames in non-active scenes.

### B3: No scene guard in tab hit-test
`frame_tab_at()` returned tab indices for frames in non-active or minimized frames.

### B4: `try_set_focus` could focus wrong-scene surface
No `surface_in_active_scene()` check in `try_set_focus()`. A surface in a non-active scene could receive focus via click or keyboard shortcut.

### B5: Stale hover after scene switch
`handle_silkbar_click()` called `clear_focus_if_wrong_scene()` and `clear_drag_if_dead()` but did NOT clear hover state (`HOVERED_FRAME_ID`, `HOVER_KIND`, `HOVERED_FRAME_LIGHT`).

### B6: Stale hover after minimize
`minimize_frame()` cleared focus and drag but did NOT clear hover if the minimized frame was hovered.

### B7: Hit-test didn't skip minimized/tombstoned frames
`hit_test_at()` only checked `surface_is_alive()` but did not check frame minimized flag or tombstoned status for content-area hits.

## Minimal Patch Summary

### 7 changes to `servers/silk-shell/src/main.rs`:

1. **`frame_accepts_input(frame_id)`** (new, line 1379) — Returns true only if frame is in active scene, non-minimized, and active tab surface is alive and not tombstoned.

2. **`clear_hover_if_wrong_scene()`** (new, line 1398) — Clears `HOVERED_FRAME_ID`, `HOVER_KIND`, `HOVERED_FRAME_LIGHT` if the hovered frame no longer accepts input.

3. **`hit_test_surface_chrome()`** (line 2283) — Added `!frame_accepts_input(frame_id)` guard: chrome is invisible for non-input frames.

4. **`hit_test_at()`** (lines 2350-2358, 2385-2395) — Added `frame_accepts_input` checks for both focused-surface and z-order content-area hits. Non-frame surfaces (linen) always pass.

5. **`frame_tab_at()`** (line 1839) — Added `!frame_accepts_input(frame_id)` guard at top.

6. **`try_set_focus()`** (line 2145) — Added `!surface_in_active_scene(sid)` reject guard.

7. **`handle_silkbar_click()`** (line 2650) — Added `clear_hover_if_wrong_scene()` call after scene switch.

8. **`minimize_frame()`** (lines 1457-1462) — Clear hover if `HOVERED_FRAME_ID == frame_id`.

## Files Changed

- `servers/silk-shell/src/main.rs`

## Build Result

```
[SEXOS ENTRYPOINT] success
```
silk-shell compiled with 0 errors (all warnings are pre-existing).

## Edge Cases Covered

- Linen surface (no frame): always passes frame_accepts_input checks (returns `HitTarget::Surface`)
- All surfaces in wrong scene/layer: `HitTarget::None` → miss, focus unchanged
- Empty scene (no surfaces): `HitTarget::None`, focus cleared to 0
- Minimized + zoomed: minimize check fires first, rejects input
- Drag start: guarded by `try_set_focus` rejecting wrong-scene surfaces + scene switch clearing focus
- Keyboard focus: `try_set_focus` guard catches all paths (click + keyboard shortcut)
- Tab strip click: guarded by `frame_accepts_input` in both `hit_test_surface_chrome` (caller) and `frame_tab_at` (defense-in-depth)

## Deferred / Out of Scope

- Scene Persistence (postponed per redirect)
- Proof expansion
- USB/XHCI changes
- Kernel edits
- sex-pdx ABI changes
- Shared-memory/backing-buffer redesign
