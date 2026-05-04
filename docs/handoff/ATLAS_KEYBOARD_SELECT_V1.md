# ATLAS_KEYBOARD_SELECT_V1

**Status:** Active  
**Purpose:** Add keyboard navigation to Atlas: arrow keys to move selection, Enter to confirm, Esc to cancel, number keys 1-5 for direct select.  
**Scope:** `servers/silk-shell/src/main.rs` only.  
**Prerequisites:** ATLAS_SCENE_SELECT_V1 (ad8aa78)

---

## 1. Key Map

| Key | Scancode | Action | Notes |
|-----|----------|--------|-------|
| Left arrow | `0x4B` | `AtlasMoveLeft` | Navigate to previous card in row (wraps) |
| Right arrow | `0x4D` | `AtlasMoveRight` | Navigate to next card in row (wraps) |
| Up arrow | `0x48` | `AtlasMoveUp` | Move from row 1 to row 0 (same column) |
| Down arrow | `0x50` | `AtlasMoveDown` | Move from row 0 to row 1 (same or nearest column) |
| Enter | `0x1C` | `AtlasConfirm` | Switch to selected scene and exit Atlas |
| Escape | `0x01` | `AtlasCancel` | Exit Atlas without switching |
| Keys 1-5 | `0x02..=0x06` | Direct select | Switch to scene N-1 directly and exit Atlas |
| F10 | `0x44` | `ToggleAtlas` | Falls through to normal dispatch — exits Atlas |

All keys captured by Atlas intercept when `ATLAS_MODE_ENABLED` is true, EXCEPT F10 which falls through to the normal `ToggleAtlas` handler.

---

## 2. Selection State

```rust
/// Index of the currently selected scene in Atlas mode (0..4).
/// Reset to active_scene_id when entering Atlas. Updated by arrow key navigation.
static mut ATLAS_SELECTED_SCENE: u8 = 0;
```

### Initialization

Set to `ACTIVE_SCENE_IDX` when entering Atlas (in `atlas_toggle()` entering branch).

### Navigation layout

```
Row 0: [Scene 0] [Scene 1] [Scene 2]
Row 1: [Scene 3] [Scene 4]
```

| Direction | Mappings |
|-----------|----------|
| **Left** | 0→2, 1→0, 2→1, 3→4, 4→3 |
| **Right** | 0→1, 1→2, 2→0, 3→4, 4→3 |
| **Up** | 3→0, 4→1; row 0 stays at current selection |
| **Down** | 0→3, 1→4, 2→4; row 1 stays at current selection |

Navigation within a row wraps (left from scene 0 goes to scene 2). Up/down moves between rows at the same column where possible; scene 2 (col 2) maps to scene 4 (row 1's rightmost card) on down.

---

## 3. Selection Visual

When a card is selected, a 2-pixel bright cyan border (`ATLAS_COLOR_SELECT: 0x0080e0ff`) is drawn around the card using four 0xEF fill-rect calls per card (top, bottom, left, right edges).

The border is drawn in `atlas_render_stub()` within the card drawing loop, after the frame indicator blocks. When the user navigates with arrow keys, `atlas_render_stub()` is called again, which redraws the entire overlay with the new selection position.

### Number of 0xEF calls per Atlas redraw

| Element | Calls |
|---------|-------|
| Background fill | 1 |
| Cards (5 × top area) | 5 |
| Frame blocks (up to 20) | ≤20 |
| Selection border (4 per selected card) | 4 |
| **Total worst case** | ~30 |

30 fill-rect calls per redraw is negligible for keyboard-paced interaction.

---

## 4. `handle_atlas_keyboard()` Function

```rust
unsafe fn handle_atlas_keyboard(scancode: u8) -> bool
```

### Flow

```
handle_atlas_keyboard(scancode)
├── 0x02..0x06 → direct scene select:
│   ├── scene_idx = scancode - 0x02
│   ├── destroy overlay (0xEE)
│   ├── if scene != active: switch_scene(scene_idx)
│   │   else: sync + tile + snap (restore)
│   ├── ATLAS_MODE_ENABLED = false
│   └── [shell.atlas.confirm] id=N
│
├── 0x4B → left arrow:
│   ├── update ATLAS_SELECTED_SCENE (wrap within row)
│   ├── atlas_render_stub() — redraw with new selection
│   └── [shell.atlas.key] dir=left sel=N
│
├── 0x4D → right arrow:
│   └── same pattern as left
│
├── 0x48 → up arrow:
│   ├── update selection if in row 1
│   ├── atlas_render_stub() if changed
│   └── [shell.atlas.key] dir=up sel=N
│
├── 0x50 → down arrow:
│   └── same pattern as up
│
├── 0x1C → Enter (confirm):
│   ├── destroy overlay (0xEE)
│   ├── if selected != active: switch_scene(selected)
│   │   else: sync + tile + snap (restore)
│   ├── ATLAS_MODE_ENABLED = false
│   └── [shell.atlas.confirm] id=N
│
├── 0x01 → Escape (cancel):
│   ├── atlas_clear_stub() — destroy overlay + restore
│   ├── ATLAS_MODE_ENABLED = false
│   └── [shell.atlas.cancel]
│
└── _ → all other keys:
    └── [shell.atlas.key] scancode=XX noop (consumed)
```

### Return value

Always returns `true` (key was consumed).

---

## 5. Atlas Keyboard Intercept

Added in the EV_KEY handler (`OP_HID_EVENT`), between the Scene Settings panel intercept and the normal action dispatch:

```rust
// ── Atlas keyboard intercept: consume non-F10 keys when Atlas active ──
if ATLAS_MODE_ENABLED && scancode != 0x44 /* F10 falls through to ToggleAtlas */ {
    handle_atlas_keyboard(scancode);
    mutated = true;
} else if let Some(action) = scancode_to_action(scancode) {
    match action { ... }
}
```

### Priority order in EV_KEY handler

1. Scene Settings panel intercept (if `SCENE_SETTINGS_ACTIVE`)
2. **Atlas keyboard intercept** (if `ATLAS_MODE_ENABLED` and not F10)
3. Normal `SurfaceAction` dispatch via `scancode_to_action()`
4. F10 falls through to `ToggleAtlas` handler (exits Atlas)

---

## 6. Markers

| Marker | Budget | When |
|--------|--------|------|
| `[shell.atlas.key]` | 4 | Arrow navigation, any other consumed key |
| `[shell.atlas.confirm]` | 4 | Enter or number key select |
| `[shell.atlas.cancel]` | 4 | Escape exit |

---

## 7. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Enter on active scene | Exits Atlas, restores current scene without switch | ✅ |
| Enter after arrow navigation | Switches to moved selection | ✅ |
| Esc after arrow moves | Exits Atlas without switching | ✅ |
| Esc without any moves | Exits Atlas without switching | ✅ |
| Left arrow at left edge (scene 0) | Wraps to scene 2 (rightmost in row) | ✅ |
| Right arrow at right edge (scene 2) | Wraps to scene 0 (leftmost in row) | ✅ |
| Up arrow in row 0 (scenes 0-2) | Stays in row 0 (no-op) | ✅ |
| Down arrow in row 1 (scenes 3-4) | Stays in row 1 (no-op) | ✅ |
| Number key 1-5 | Directly selects scene (N-1) and exits | ✅ |
| Number key out of range | `if scene_idx < ATLAS_MAX_SCENES` guard → falls through to match's `_` arm | ✅ |
| F10 while Atlas enabled | Falls through to normal ToggleAtlas → exits Atlas | ✅ |
| F8/F9 while Atlas enabled | Consumed by Atlas intercept → `[shell.atlas.key] scancode=XX noop` | ✅ |
| Atlas not enabled | No intercept, normal key dispatch unchanged | ✅ |
| Concurrent with Scene Settings panel | Panel intercept runs first (higher priority) | ✅ |
| Arrow keys while panel active | Panel intercept runs first, normal handling unchanged | ✅ |

---

## 8. Files Changed

- `servers/silk-shell/src/main.rs` — +143 lines (1 constant, 1 static, 1 init line, 4 selection borders, `handle_atlas_keyboard()`, Atlas keyboard intercept)
- `docs/handoff/ATLAS_KEYBOARD_SELECT_V1.md` — new handoff doc

---

## 9. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new meaningful warnings.
```

Warning count: 231 (increase from pre-existing 212 is from unnecessary-unsafe-block warnings in nested code).

---

## 10. Deferred for Later Phases

- Atlas card hover highlighting on keyboard selection
- Atlas card text labels (requires sexdisplay text protocol)
- Frame thumbnails inside Atlas cards
- Atlas card reorder via drag
- Mouse hover to select (auto-follow pointer)

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add ATLAS_SELECTED_SCENE state, handle_atlas_keyboard() with arrows/enter/esc/number select, visual selection border, keyboard intercept in EV_KEY handler | ATLAS_KEYBOARD_SELECT_V1 |
