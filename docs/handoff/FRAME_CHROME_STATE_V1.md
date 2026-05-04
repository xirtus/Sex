# FRAME_CHROME_STATE_V1

**Status:** Active  
**Purpose:** Define and harden Silk frame chrome state: hover light consistency after tab switch, chrome mode toggle, and zoom state transitions.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** TILING_ENGINE_HARDENING_V1 (bdcb67b)

---

## 1. Frame Chrome State Invariants

```
┌─────────────────────────────────────────────────┐
│         Frame Chrome State Invariants            │
├─────────────────────────────────────────────────┤
│ I1: HOVERED_FRAME_LIGHT reflects the light at    │
│     the current pointer position                 │
│ I2: HOVERED_FRAME_LIGHT is cleared when the      │
│     surface or chrome geometry changes           │  ← NEW
│ I3: Hover state never survives wrong scene/      │
│     minimize/close/tombstone                     │
│ I4: Single-tab frame has no tab switching        │
│     affordance (tab_count <= 1 → focus_next/prev │
│     are no-ops)                                  │
│ I5: Multi-tab frame exposes tab selection via    │
│     frame_tab_at() / switch_to_tab()             │
│ I6: Red close light is gated on                  │
│     is_closeable_surface()                       │
│ I7: Yellow minimize preserves live surface       │
│     (0xEE hide, never 0xEC destroy)              │
│ I8: Green zoom excludes frame from tiling until  │
│     unzoomed (see TILING_ENGINE_HARDENING)       │
│ I9: switch_to_tab() validates                    │
│     frame_accepts_input() before proceeding      │  ← NEW
│ I10: Chrome mode toggle (top bar ↔ minimal)      │
│      clears stale hover light                    │  ← NEW
└─────────────────────────────────────────────────┘
```

### Invariants I2, I9, I10 (new)

- **I2**: After a tab switch, the new active surface may have different chrome geometry (different size, position, or chrome mode). `HOVERED_FRAME_LIGHT` from the old surface's chrome is invalid for the new surface. Cleared in `switch_to_tab()`.
- **I9**: `switch_to_tab()` must guard on `frame_accepts_input()`. The mouse-click path (via `frame_tab_at()`) checks this upstream, but keyboard paths (`focus_next_tab()`, `focus_prev_tab()`) call `switch_to_tab()` directly.
- **I10**: `toggle_top_bar_for_active_frame()` flips the chrome mode between top bar (16px band with 8px lights) and minimal (4px rim with 4px lights). All light positions shift. Hover light must be cleared.

---

## 2. Imperfections Found

| # | Issue | Location | Severity |
|---|-------|----------|----------|
| 1 | **`switch_to_tab()` does not clear `HOVERED_FRAME_LIGHT`** — After switching tabs, the new active surface may have different chrome geometry. `HOVERED_FRAME_LIGHT` still references a position on the old surface's chrome. While `update_frame_hover_at()` corrects this on the next event, explicit cleanup prevents stale light state between events. | `switch_to_tab()` | Low |
| 2 | **`switch_to_tab()` lacks `frame_accepts_input()` guard** — The mouse-click path (via `frame_tab_at()`) checks `frame_accepts_input()` upstream. But keyboard shortcut paths (`focus_next_tab()`, `focus_prev_tab()`) call `switch_to_tab()` directly without this check. | `switch_to_tab()` | Low (defense-in-depth) |
| 3 | **`toggle_top_bar_for_active_frame()` does not clear hover light** — Toggling chrome mode (top bar ↔ minimal) shifts all light positions (size, gap, exclusion zone change). `HOVERED_FRAME_LIGHT` from the old mode references a position that may not correspond to any light in the new mode. | `toggle_top_bar_for_active_frame()` | Low |
| 4 | **`zoom_frame()` does not clear `HOVERED_FRAME_LIGHT`** — Zoom changes surface geometry to full content area, completely invalidating light positions from the non-zoomed chrome. | `zoom_frame()` | Low |
| 5 | **`unzoom_frame()` does not clear `HOVERED_FRAME_LIGHT`** — Unzoom restores saved normal geometry. Light positions from the zoomed full-area chrome are invalid for the restored geometry. | `unzoom_frame()` | Low |

---

## 3. Patch Summary

### `switch_to_tab()` — added `frame_accepts_input()` guard (line ~2707)

```rust
// Guard: frame must accept input (active scene, non-minimized, alive, non-tombstoned).
if !frame_accepts_input(frame_id) {
    return false;
}
```

Inserted after function signature, before frame validation. Ensures keyboard-initiated tab switches (which bypass the `frame_tab_at()` → `frame_accepts_input()` check) are also guarded.

### `switch_to_tab()` — added hover light clear (after `clear_drag_if_dead()`)

```rust
// Clear stale hover light — the new active surface may have different
// chrome geometry, making the old light position invalid.
HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
```

### `toggle_top_bar_for_active_frame()` — added hover light clear (after `send_frame_tab_info()`)

```rust
// Chrome mode changed (top bar ↔ minimal) — all light positions have
// shifted. Clear hover light to prevent stale light from a different
// chrome geometry. Hover is re-evaluated on the next pointer event.
HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
```

### `zoom_frame()` — added hover light clear (after zoom flag set, before budget)

```rust
// Clear stale hover light — zoom changes surface geometry completely,
// invalidating any light position from the previous (non-zoomed) chrome.
HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
```

### `unzoom_frame()` — added hover light clear (after geometry restore, before budget)

```rust
// Clear stale hover light — unzoom restores normal geometry, which has
// different chrome than the zoomed full-content-area geometry.
HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
```

---

## 4. Frame Light Action Dispatch (model correctness)

Current frame light dispatch in `click_hit_test_and_focus()`:

| Light | Action | Guards | Status |
|-------|--------|--------|--------|
| RED (CLOSE) | `close_surface_from_frame_light()` | `is_closeable_surface()` pre-check | ✅ |
| YELLOW (MINIMIZE) | `minimize_frame()` | Internal: alive surface, not already minimized | ✅ |
| GREEN (ZOOM) | `toggle_zoom_frame()` | Internal: not already zoomed, not minimized | ✅ |
| Rim drag | `try_transition(Dragging)` | Surface alive, frame resolved | ✅ |

All three light actions have the correct dispatch guards. The renderer-side visual feedback (light color/shape) is a deferred concern.

---

## 5. Tab Strip Rules

| Rule | Implementation | Status |
|------|----------------|--------|
| Tabs only on frames accepting input | `frame_tab_at()` checks `frame_accepts_input()` | ✅ |
| Tab slot: equal-width fill | `available_width / tab_count` with `FRAME_TAB_MIN_WIDTH_PX` floor | ✅ |
| Exclusion zone: Frame Lights | `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX` / `FRAME_TAB_LIGHT_EXCLUSION_PX` | ✅ |
| Right rim exclusion | `right_rim_start = sx + sw - FRAME_RIM_PX` | ✅ |
| Keyboard next/prev tab | `focus_next_tab()` / `focus_prev_tab()` | ✅ |
| Tab switch clears drag | `clear_drag_if_dead()` after switch | ✅ (FRAME_LIFECYCLE) |
| Tab switch clears hover light | `HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE` | ✅ **NEW** |
| Tab switch guards frame_accepts_input | `frame_accepts_input()` check | ✅ **NEW** |

---

## 6. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Click tab strip on non-input frame | `frame_tab_at()` returns None | ✅ I1 |
| Tab switch via keyboard on non-input frame | `switch_to_tab()` returns false | ✅ **NEW I9** |
| Tab switch via keyboard on frame with 1 tab | `focus_next_tab()` returns early (tab_count <= 1) | ✅ I4 |
| Tab switch while frame is zoomed | Tab switch succeeds (tabs within zoomed frame are valid) | ✅ |
| Hover light after tab switch | Light is cleared, re-evaluated on next pointer event | ✅ **NEW I2** |
| Hover light after chrome mode toggle | Light is cleared, re-evaluated on next pointer event | ✅ **NEW I10** |
| Hover light after zoom | Light is cleared, re-evaluated on next pointer event | ✅ **NEW I2** |
| Hover light after unzoom | Light is cleared, re-evaluated on next pointer event | ✅ **NEW I2** |
| Close light on non-closeable surface | `is_closeable_surface()` returns false → reject logged | ✅ I6 |
| Close light on linen surface | `is_closeable_surface(SURFACE_ID_LINEN)` returns false | ✅ I6 |

---

## 7. Files Changed

- `servers/silk-shell/src/main.rs` — +18 lines across 4 functions (switch_to_tab, toggle_top_bar_for_active_frame, zoom_frame, unzoom_frame)

## 8. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## Deferred Renderer Work (not in scope)

- Frame light visual rendering (color, hover highlight, active state) — requires sexdisplay protocol changes
- Tab strip visual rendering (tab labels, active tab highlight) — requires sexdisplay protocol changes
- Selected frame indication (neon rim brightness, top bar highlight) — requires sexdisplay protocol changes
- Frame light non-interactive state (gray out close light on non-closeable surfaces) — requires sexdisplay protocol changes

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add frame_accepts_input guard to switch_to_tab, clear hover light on tab switch/chrome toggle/zoom toggle | FRAME_CHROME_STATE_V1 |
