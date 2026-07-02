# SILK_SHELL_INTERACTION_CONTRACT_V1

## Status: **LOCKED** ✅

Shell-side interaction contract for Scene/Frame/Tab focus safety is complete
and verified. All validation guards and proof markers are active.

## Changes

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `clear_hover_if_dead()` + wired at 8 cleanup sites |

## Minimal Model (existing, verified)

| Concept | Type | Location |
|---------|------|----------|
| Scene | `struct Scene` with flags, label, accent, pinned | line ~3719 |
| Frame | `struct ShellFrame` with frame_id, scene_id, flags, tabs, active_tab | line ~2864 |
| Tab | `struct ShellTab` (inside `ShellFrame.tabs[]`) | type-safe `TabIndex(u8)` wrapper at line ~2900 |
| Focus target | `FOCUSED_SURFACE_ID: u64`, backed by `FocusRef { surface_id, generation }` | lines 3759, 1890 |
| Hover target | `HOVERED_FRAME_ID: u32`, `HOVER_KIND`, `HOVERED_FRAME_LIGHT` | lines 3755-3757 |
| Active scene | `ACTIVE_SCENE_IDX: u8` | line 3761 |

## Validation Guards (verified active)

| Guard | Function | Marker |
|-------|----------|--------|
| No focus on dead surface | `try_set_focus()` → `surface_is_alive()` | `[shell.focus.reject.dead]` |
| No focus on non-focusable | `try_set_focus()` → `is_focusable_surface()` | `[shell.focus.reject.nonfocusable]` |
| No focus on tombstoned | `try_set_focus()` → `is_tombstoned()` | `[shell.focus.reject.tombstoned]` |
| No focus on stale generation | `try_set_focus()` → `focus_ref_is_current()` | `[focus.generation.reject]` |
| No focus on inactive scene | `try_set_focus()` → `surface_scene_id() == ACTIVE_SCENE_IDX` | `[scene.focus.reject.inactive]` |
| Clear focus on dead surface | `clear_focus_if_dead()` → fallback to first alive | `[shell.focus.clear_dead]`, `[focus.ref.clear]` |
| Clear focus on scene switch | `clear_focus_if_wrong_scene()` | `[shell.scene.focus.clear.wrong-scene]` |
| Clear drag on dead surface | `clear_drag_if_dead()` | `[shell.drag.clear_dead]` |
| Clear drag on wrong scene | `clear_drag_if_wrong_scene()` | `[shell.scene.drag.clear.wrong-scene]` |
| Clear hover on dead surface | **NEW** `clear_hover_if_dead()` | `[shell.hover.clear.dead]` |
| Clear hover on wrong scene | `clear_hover_if_wrong_scene()` | `[shell.frame.hover.clear.wrong-scene]` |
| Bounded hit-test order | `hit_test_at()` via fixed `z_order` array with dead-skip | `[shell.hit_test.skip]` for dead |
| Overlay/bar captures first | SilkBar intercept before `click_hit_test_and_focus()` | `[shell.click.real.target] kind=chrome` |

## Proof Markers (runtime verified)

```
Focus change:
  [focus.ref.commit] id=N        — FocusRef synced
  [shell.focus.set] id=N         — focus committed
  [shell.interact.focus] sid=N   — interaction focus delivered

Invalid target rejection:
  [shell.focus.reject.dead] id=N
  [shell.focus.reject.nonfocusable] id=N
  [shell.focus.reject.tombstoned] id=N
  [scene.focus.reject.inactive] sid=N active=X
  [focus.generation.reject] id=N

Click path:
  [shell.click_focus.down] x=N y=N buttons=0xN
  [shell.click_focus.hit] id=N
  [shell.click_focus.miss]
  [shell.click.real.target] x=N y=N target=N kind=N

Hover:
  [shell.hover.clear.dead] frame=N surface=N reason=dead    — NEW
  [shell.frame.hover.clear.wrong-scene]
  [shell.frame.hover.set] frame=N kind=N
  [shell.interact.hover] frame=N kind=N light=N x=N y=N

Frame lights:
  [shell.frame.light.close] frame=N surface=N
  [shell.frame.light.close.reject] frame=N surface=N reason=N
  [shell.frame.minimize.reject] frame=N reason=N
  [shell.frame.zoom.reject] frame=N reason=N
  [frame.light.reject.inactive] frame=N reason=N
```

## Build & Runtime Result

- Build: PASS
- `gate_render.sh`: ALL 6 CHECKS PASSED
- `master_runtime_gate`: GREEN_MASTER
- Faults: 0

## Recurring Issue

Hover cleanup was only handled for scene switches (`clear_hover_if_wrong_scene`),
but not for surface death specifically (`clear_hover_if_dead`). The old function
incidentally caught dead surfaces via `frame_accepts_input`, but the naming was
misleading and there was no dedicated marker. Now both paths are explicit.
