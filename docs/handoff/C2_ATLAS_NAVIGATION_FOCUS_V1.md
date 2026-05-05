# C2: Atlas Navigation + Focus Switching

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive proof markers only)

## Purpose

Add keyboard navigation and focus-switching proof markers to the Atlas
overview mode (C1). Atlas can now select a scene via arrow keys, activate
it via Enter or number keys, or cancel via Escape — with full proof marker
coverage for navigation, activation, and focus result.

## Changes to `servers/silk-shell/src/main.rs`

### 1. `atlas_toggle()` — Enter-select marker (Change 1)
- Added `[atlas.nav.enter.select]` proof marker when Atlas mode is entered
- Prints current `ATLAS_SELECTED_SCENE` value

### 2. `handle_atlas_keyboard()` — Navigation markers (Change 2)

**Directional navigation (arrow keys):**
- `[atlas.nav.move] dir=left/right/up/down from={} to={}` — emitted when
  ATLAS_SELECTED_SCENE changes in response to arrow key
- Only emitted when selection actually changes

**Number keys 1-5 (direct scene select):**
- `[atlas.nav.activate] scene={} keys=number` — before scene activation
- `[atlas.nav.focus.commit] scene={} sid={}` — after activation if focus exists
- `[atlas.nav.focus.empty] scene={}` — after activation if no focus

**Enter key (confirm):**
- `[atlas.nav.activate] scene={} keys=enter` — before scene activation
- `[atlas.nav.focus.commit] scene={} sid={}` — after activation if focus exists
- `[atlas.nav.focus.empty] scene={}` — after activation if no focus

**Escape key (cancel):**
- `[atlas.nav.cancel] scene={}` — on cancel, always uses ACTIVE_SCENE_IDX
- `[atlas.nav.focus.commit] scene={} sid={}` — if focus exists
- `[atlas.nav.focus.empty] scene={}` — if no focus

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[atlas.nav.enter.select]` | atlas_toggle() | Atlas mode entered |
| `[atlas.nav.move]` | handle_atlas_keyboard() | Arrow key changes selection |
| `[atlas.nav.activate]` | handle_atlas_keyboard() | Scene activated (Enter/number) |
| `[atlas.nav.focus.commit]` | handle_atlas_keyboard() | Focus exists after activation |
| `[atlas.nav.focus.empty]` | handle_atlas_keyboard() | No focus after activation |
| `[atlas.nav.cancel]` | handle_atlas_keyboard() | Atlas exited via Escape |

## Invariants

1. Focus after scene activation always goes through B2 scene guards
   (try_set_focus guard 7)
2. Cancel always reverts to ACTIVE_SCENE_IDX, never switches scene
3. Navigation markers only fire on actual selection change
4. All focus result markers fire regardless of scene switch vs no-op

## Deferred

- C3: Atlas visual polish tokens (accent colors, card shading)
- Thumbnails (requires renderer support)
- Mouse-based Atlas interaction

## Dependencies

- **Requires:** C1 (Atlas snapshot/view), B2 (active-scene focus guards)
- **Blocks:** C3 (visual polish)
