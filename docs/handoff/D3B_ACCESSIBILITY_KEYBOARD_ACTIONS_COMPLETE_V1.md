# D3B_ACCESSIBILITY_KEYBOARD_ACTIONS_COMPLETE_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Completes the remaining shell-owned accessibility keyboard actions deferred from
D3. Adds close (F11), zoom toggle (Esc), and scene next/prev dispatch helpers
(bindings deferred). Atlas settings parity is already handled by the existing
`handle_atlas_keyboard()` function ('A' for cycle accent, 'P' for toggle pin).

**D3B is partial by design** — scene switch bindings are deferred due to lack
of safe single-key scancodes (all F-keys used, number/letter keys conflict with
future editor input).

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +100 lines (4 SurfaceAction variants, 2 scancode bindings, 4 dispatch handlers, 1 match arm group) |
| `docs/handoff/D3B_ACCESSIBILITY_KEYBOARD_ACTIONS_COMPLETE_V1.md` | New handoff doc |

---

## D3B Implemented

### Keybinding Map

| Key | Scancode | Action | Dispatch |
|-----|----------|--------|----------|
| F11 | 0x57 | `AccessClose` | `access_handle_keyboard_action()` → `close_surface_from_frame_light()` |
| Esc | 0x01 | `AccessZoomToggle` | `access_handle_keyboard_action()` → `toggle_zoom_frame()` |
| — | — | `AccessSceneNext` | `next_scene()` — **unbound**, dispatch helper only |
| — | — | `AccessScenePrev` | `prev_scene()` — **unbound**, dispatch helper only |

### Close Dispatch

```
access_handle_keyboard_action(AccessClose):
  1. Validate FOCUSED_SURFACE_ID != 0
  2. Validate surface_is_alive(sid) && !is_tombstoned(sid)
  3. Call close_surface_from_frame_light(sid)
     - Full lifecycle FSM: Clearing → Closing → Tombstoned → Destroyed
     - Clears focus, clears drag, re-tiles, captures snapshot
  4. [access.action.close] on success
  5. [access.action.reject] on failure (no focus, dead, close failed)
```

### Zoom Toggle Dispatch

```
access_handle_keyboard_action(AccessZoomToggle):
  1. Validate FOCUSED_SURFACE_ID != 0
  2. Validate surface_is_alive(sid) && !is_tombstoned(sid)
  3. Find frame via frame_for_surface(sid)
  4. Call toggle_zoom_frame(frame_id)
     - If zoomed: unzoom_frame() (restores saved normal geometry)
     - If not zoomed: zoom_frame() (saves normal geometry, maximizes)
     - Rejects dead lifecycle states
  5. [access.action.zoom] on success
  6. [access.action.reject] on failure
```

### Scene Next/Prev Dispatch

```
access_handle_keyboard_action(AccessSceneNext):
  1. Call next_scene() → switch_scene()
     - Advances workspace, wraps at max
     - sync_scene_visibility(), clears focus/drag/hover for wrong scene
     - Re-tiles, captures snapshot
  2. [access.action.scene_switch] dir=next

access_handle_keyboard_action(AccessScenePrev):
  Same but dir=prev via prev_scene()
```

---

## D3B Deferred

| Feature | Reason |
|---------|--------|
| SceneNext/ScenePrev bindings | No safe single-key scancode available. All F-keys used. Number keys conflict with Focus100-Focus200 bindings. Letter keys would conflict with future app/editor text input. Requires modifier tracking (Ctrl+Tab, Alt+Tab) which does not exist in the scancode-only keyboard model. |
| Atlas settings cycle accent | Already handled by `handle_atlas_keyboard()` via 'A' (0x1E) when Atlas is open. No D3B change needed. |
| Atlas settings toggle pin | Already handled by `handle_atlas_keyboard()` via 'P' (0x19) when Atlas is open. No D3B change needed. |

---

## Esc Binding Invariant

```
Esc binding invariant:
Atlas mode consumes Esc before normal shell dispatch.
Normal-mode Esc toggles zoom only when Atlas is not open.
If future app/editor input receives Esc, this binding must move
behind a shell modifier or mode gate.
```

**Rationale:** In Atlas mode, `handle_atlas_keyboard()` intercepts Esc as
"cancel and exit Atlas" (line 3081). This intercept fires before the
`scancode_to_action()` dispatch (line 6273-6275), so Esc always exits Atlas
when Atlas is open. In normal mode, Esc is unmapped and falls through to
`_ => None` in `scancode_to_action()`, so binding it to `AccessZoomToggle`
is safe. If Quil or any future editor receives Esc keystrokes, this binding
must be gated behind a shell modifier or mode (e.g., Alt+Esc) to avoid
conflict.

---

## Lifecycle Safety Verification

| Action | Path | Lifecycle guard |
|--------|------|----------------|
| Close | `close_surface_from_frame_light(sid)` | Checks alive, not closing/tombstoned, clears focus, drag, full FSM |
| Zoom toggle | `toggle_zoom_frame(frame_id)` → `zoom_frame()`/`unzoom_frame()` | Rejects dead lifecycle states, checks zoomed/minimized flags |
| Scene next/prev | `next_scene()`/`prev_scene()` → `switch_scene()` | Full scene sync, clears focus/drag/hover for wrong scene |

**No direct state mutation.** All paths go through existing lifecycle-safe
helpers.

---

## Proof Markers Added

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[access.action.close]` | 8 | `access_handle_keyboard_action()` :: AccessClose | Close dispatched successfully |
| `[access.action.zoom]` | 8 | Same :: AccessZoomToggle | Zoom toggle dispatched successfully |
| `[access.action.scene_switch]` | 8 | Same :: AccessSceneNext/AccessScenePrev | Scene switch dispatched |
| `[access.action.reject]` | 8 | Same (reused from D3) | All reject cases: no_focus, dead, failed |

---

## Behavior Changes

- **F11** now closes the focused surface (was unmapped)
- **Esc** now toggles zoom on the focused frame in normal mode (was unmapped; in Atlas mode, Esc continues to exit Atlas)
- **SceneNext/ScenePrev**: dispatch helpers exist but are **unbound** — no keyboard behavior change

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Close path cannot be called safely from semantic target | ✅ `close_surface_from_frame_light()` is lifecycle-safe |
| Zoom path bypasses lifecycle/tiling guards | ✅ `toggle_zoom_frame()` is lifecycle-safe |
| Scene switch alternative conflicts with nav keys | ✅ SceneNext/ScenePrev bindings deferred — no conflict |
| Atlas settings parity requires new UI/state model | ✅ Already implemented in `handle_atlas_keyboard()` — no D3B change |
| Safe key bindings are unclear | ✅ Close→F11 (function key), ZoomToggle→Esc (shell key). SceneNext/Prev deferred. |
| Bypasses lifecycle-safe paths | ✅ No — all paths validated |
| Adds app/editor input | ✅ Not added |
| Requires heap/String/broad refactor | ✅ Not needed |
| Requires kernel/ABI change | ✅ Not needed |
| Requires persistence/storage | ✅ Not needed |

**No STOP FIRST conditions triggered.**

---

## Ready for D4

**Yes.** All shell-owned accessibility keyboard actions are implemented
(close, zoom toggle, activate/minimize/restore, focus traversal). Scene
next/prev bindings are deferred to when modifier tracking exists. Atlas
settings parity is already handled by the existing Atlas keyboard model.

---

## References

- `docs/handoff/D3_ACCESSIBILITY_KEYBOARD_ACTIONS_V1.md` — D3 partial
- `docs/handoff/D2_ACCESSIBILITY_SEMANTIC_NODE_EMITTER_V1.md` — D2 node model
- `docs/handoff/D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1.md` — D1 audit
- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — Track D plan
- `servers/silk-shell/src/main.rs` — implementation (~100 lines added)
