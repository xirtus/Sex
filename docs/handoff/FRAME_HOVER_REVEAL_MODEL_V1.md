# FRAME_HOVER_REVEAL_MODEL_V1

## Status

Implemented (2026-05-04). Minimal shell-side hover reveal state for Silk Frame Chrome. No renderer or protocol changes. All existing drag/focus proofs preserved.

---

## Hover State Model

### Constants

```rust
const HOVER_NONE: u32 = 0;
const HOVER_FRAME_BODY: u32 = 1;    // app content area
const HOVER_FRAME_RIM: u32 = 2;     // future: neon rim
const HOVER_TAB_STRIP: u32 = 3;     // future: tab strip
```

### State variables

```rust
static mut HOVERED_FRAME_ID: u32 = 0;   // 0 = no frame hovered
static mut HOVER_KIND: u32 = HOVER_NONE;  // HOVER_NONE..HOVER_TAB_STRIP
```

### Update function

```rust
unsafe fn update_frame_hover_at(x: i32, y: i32) -> bool;
```

| Condition | Result |
|-----------|--------|
| Active drag (`InteractionState::Dragging`) | Skip (return false). Pointer captured by drag action. |
| `y < P.bar_height` (SilkBar strip) | Clear hover: `(0, HOVER_NONE)` |
| `hit_test_at(x, y)` returns `Surface(sid)` | Map via `frame_for_surface(sid)`: `(frame_id, HOVER_FRAME_BODY)` if found, else `(0, HOVER_NONE)` |
| `hit_test_at(x, y)` returns `FrameChrome { frame_id, kind }` | `(frame_id, kind)` — not yet produced, model ready for future chrome geometry |
| `hit_test_at(x, y)` returns `None` | `(0, HOVER_NONE)` |

### Reveal rules (single-tab vs multi-tab)

| Frame type | HOVER_FRAME_BODY reveals | HOVER_FRAME_RIM reveals | HOVER_TAB_STRIP reveals |
|-----------|-------------------------|------------------------|------------------------|
| Single-tab (V1) | Tab label on hover reveal (future: render tab title near bottom-right of frame) | Neon rim glow (future: 1px neon edge highlight) | Not applicable |
| Multi-tab (future) | Persistent tab strip on any tab hover | Neon rim glow on frame | Tab strip always visible, active tab highlighted |

In V1, only `HOVER_FRAME_BODY` is produced. `HOVER_FRAME_RIM` and `HOVER_TAB_STRIP` are reserved constants ready for future chrome geometry.

---

## Changes

### servers/silk-shell/src/main.rs

**Added** (net ~55 lines):
- 4 hover kind constants: `HOVER_NONE`, `HOVER_FRAME_BODY`, `HOVER_FRAME_RIM`, `HOVER_TAB_STRIP` (line 233)
- 2 static hover state variables: `HOVERED_FRAME_ID`, `HOVER_KIND` (line 237)
- `update_frame_hover_at(x, y) -> bool` function (line 490): hit-tests current pointer position, maps Surface hits to frame_id via `frame_for_surface()`, updates hover state on change. Skips during active drag. Budgeted marker `[shell.frame.hover.set]` / `[shell.frame.hover.clear]` (max 6 state changes).
- Call to `update_frame_hover_at(POINTER_X, POINTER_Y)` in event loop after state updates (line 1775).

### Invariants preserved

1. **Hover does not affect focus**: No call to `try_set_focus()`, `FOCUSED_SURFACE_ID` is never read or written by hover logic (except transitively through `hit_test_at()` which read-only inspects it).
2. **Hover does not affect drag**: During `InteractionState::Dragging`, `update_frame_hover_at()` returns `false` immediately.
3. **Hover does not affect click behavior**: `update_frame_hover_at()` is called AFTER `click_hit_test_and_focus()` in the event loop — all click/focus/drag side effects have already occurred.
4. **No state leak between events**: Hover state is fully determined by current `POINTER_X`/`POINTER_Y` and the Frame model. No accumulated or deferred state.
5. **All existing markers preserved**: `[shell.click_focus.*]`, `[shell.drag.*]`, `[shell.focus.*]`, `[shell.cursor_surface.*]` — all unchanged.

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic (optional)
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-hover-reveal-model-v1.log

for m in \
  shell.frame.hover.set \
  shell.frame.hover.clear \
  shell.click_focus.hit \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-hover-reveal-model-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-hover-reveal-model-v1.log
```

Expected counts:
- `shell.frame.hover.set` >= 1 (first pointer move into surface 100 → frame 1)
- `shell.frame.hover.clear` >= 1 (if pointer moves to SilkBar area or out of all surfaces, else 0)
- Existing drag/focus markers unchanged
- faults = 0

Note: In the synthetic proof, the cursor starts at center (640, 400) which is inside surface 100. On the first EV_REL event, hover state transitions from (0, NONE) to (1, HOVER_FRAME_BODY). Hover stays at (1, HOVER_FRAME_BODY) for the rest of the proof because the cursor never exits surface 100's bounds. During drag phases (3-4), hover update is skipped (active drag guard). So the proof produces exactly 1 `[shell.frame.hover.set]` marker.

---

## Remaining Risks

- **Hover state only updates after POINTER_X/Y changes**: The `update_frame_hover_at()` call in the event loop reads the current POINTER_X/Y. If the pointer hasn't moved since the last event (e.g., a keyboard-only event), the hit-test runs unnecessarily but the state-change gate prevents marker spam. The hit-test is cheap (5 surface checks).
- **No boot-time hover init**: At boot, POINTER_X/Y = (0, 0) and HOVERED_FRAME_ID = 0, HOVER_KIND = HOVER_NONE. The hover state is not initialized from the boot cursor position (P.width/2, P.height/2) until the first USB or HID event arrives. This means there's a brief window at boot where hover is unset. Harmless because no rendering depends on hover state yet.
- **`HOVER_FRAME_RIM` and `HOVER_TAB_STRIP` are dead constants**: These are defined but never produced. They exist as reserved kind values for future chrome geometry detection. A future phase that adds rim or tab-strip hit production can use these constants.
- **Hover state not consumed**: No code path reads `HOVERED_FRAME_ID` or `HOVER_KIND` for rendering or policy decisions. This is intentional for V1 — the state is seeded for a future renderer protocol phase that will map hover state to visual chrome updates.

---

## Next Recommended Phase

**FRAME_CHROME_HIT_PRODUCTION_V1** — Produce `HitTarget::FrameChrome` hits from defined geometry:

1. Define tab strip geometry: rectangle at `y = P.bar_height`, height = `tab_strip_height` (e.g., 24px), width = frame width
2. Define neon rim geometry: 2px border around frame content area
3. Update `hit_test_at()` to check chrome geometry before/after surface check
4. Produce `HitTarget::FrameChrome { frame_id, kind }` for clicks on chrome areas
5. Update `click_hit_test_and_focus()` and `update_frame_hover_at()` to handle FrameChrome hits
6. Gate drag start on `!is_frame_chrome(target)`

Or: **SELECTED_WINDOW_SILKBAR_OPTIONS_V1** — Add SilkBar actions for selected window.

Recommended: **FRAME_CHROME_HIT_PRODUCTION_V1** — chrome hit production is the next step toward interactive frame chrome.
