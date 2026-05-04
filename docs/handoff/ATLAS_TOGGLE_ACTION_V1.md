# ATLAS_TOGGLE_ACTION_V1

**Status:** Active  
**Purpose:** Add shell-owned Atlas mode toggle action using the existing keyboard/action path. State-only — no rendering, no sexdisplay changes.  
**Scope:** `servers/silk-shell/src/main.rs` only.  
**Prerequisites:** ATLAS_MODEL_V1 (67f8ca4)

---

## 1. Key / Action Chosen

| Key | Scancode | Action | Notes |
|-----|----------|--------|-------|
| **F10** | `0x44` | `ToggleAtlas` | Unused in existing map (F8=Linen, F9=Quil, F7=SceneSettings) |

---

## 2. Atlas State

### New static

```rust
/// Atlas mode enabled: when true, the shell is in overview mode (no rendering yet in V1).
/// Toggled by F10 (ToggleAtlas). State-only — no visual behavior changes in V1.
static mut ATLAS_MODE_ENABLED: bool = false;
```

### New functions

| Function | Purpose |
|----------|---------|
| `atlas_toggle()` | Toggle mode, capture snapshot, clear hover/drag on enter. Markers: `[shell.atlas.enter]`, `[shell.atlas.exit]` |
| `atlas_is_enabled()` | Query current mode (future use) |
| `atlas_exit()` | Force-exit Atlas mode (future use) |

### Behavior on toggle

**Entering Atlas** (`ATLAS_MODE_ENABLED: false → true`):
1. Flip mode bit
2. `atlas_capture_snapshot()` — derive fresh SceneDescriptors from current shell state
3. `clear_hover_if_wrong_scene()` — prevent stale hover from bleeding into Atlas
4. `clear_drag_if_dead()` — prevent stale drag from interacting with Atlas
5. Budgeted `[shell.atlas.enter]` marker
6. `mutated = true` in dispatch (triggers SilkBar focus update)

**Exiting Atlas** (`ATLAS_MODE_ENABLED: true → false`):
1. Flip mode bit
2. `atlas_capture_snapshot()` — capture state as we leave Atlas
3. Budgeted `[shell.atlas.exit]` marker
4. `mutated = true` in dispatch

---

## 3. Patch Summary

### `SurfaceAction` enum — added `ToggleAtlas` variant

```rust
enum SurfaceAction {
    // ... existing variants ...
    ToggleAtlas,       // F10 — toggle Atlas overview mode
    ToggleSceneSettingsPanel,
    // ...
}
```

### `scancode_to_action()` — added F10 mapping

```rust
0x43 => Some(SurfaceAction::ToggleQuil),     // F9
0x44 => Some(SurfaceAction::ToggleAtlas),    // F10
0x47 => Some(SurfaceAction::SnapHome),
```

### Dispatch handler — added ToggleAtlas arm

```rust
SurfaceAction::ToggleAtlas => {
    unsafe { atlas_toggle(); }
    mutated = true;
}
```

---

## 4. Files Changed

- `servers/silk-shell/src/main.rs` — +55 lines (1 enum variant, 1 scancode mapping, 1 dispatch arm, 3 functions, 1 static, 2 #[allow(dead_code)])

## 5. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## 6. Future Phases

| Phase | What | Status |
|-------|------|--------|
| ATLAS_MODEL_V1 | AtlasSnapshot + SceneDescriptor | ✅ Done |
| **ATLAS_TOGGLE_ACTION_V1** | **F10 toggle + state** | ✅ **Done** |
| ATLAS_RENDER_STUB_V1 | Draw simple scene cards via 0xEF fill rect | Next |
| ATLAS_SCENE_SELECT_V1 | Click Atlas card → switch to that Scene | After render |
| ATLAS_FRAME_PREVIEW_PLAN_V1 | Show Frame mini-layouts inside Atlas cards | Later |

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add ToggleAtlas action, F10 binding, ATLAS_MODE_ENABLED state, atlas_toggle/exit/is_enabled | ATLAS_TOGGLE_ACTION_V1 |
