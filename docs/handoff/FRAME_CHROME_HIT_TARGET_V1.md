# FRAME_CHROME_HIT_TARGET_V1

## Status

Implemented (2026-05-04). Hit-test results promoted to typed `HitTarget` enum. Surface hit semantics unchanged. `FrameChrome` variant modeled but not yet produced. All existing drag/focus proofs preserved.

---

## HitTarget Contract

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitTarget {
    None,
    Surface(u64),
    FrameChrome { frame_id: u32, kind: u32 },
}
```

| Variant | Produced when | Side effects | Future |
|---------|---------------|-------------|--------|
| `Surface(u64)` | Cursor is within an alive, focusable surface (focused first, then z-order) | Focus switch via `try_set_focus()`, drag start via `try_transition(Dragging)` | Same |
| `None` | Cursor is not on any surface or chrome | `[shell.click_focus.miss]` log, no focus/drag change | Same |
| `FrameChrome { frame_id, kind }` | Not yet produced. Reserved for clicks on tab strip, resize handle, close button, neon rim | TBD per chrome element | First chrome element insert determines `kind` values |

### SilkBar

SilkBar clicks are handled separately via `handle_silkbar_click()`. The return value `silkbar_handled: bool` is returned alongside `HitTarget` from `click_hit_test_and_focus()`. SilkBar is not merged into `HitTarget` because:
- SilkBar has side effects (panel toggle, workspace switch)
- SilkBar y < 50 is a distinct input region that preempts all surface interaction
- Future phase may add a `HitTarget::SilkBar` variant if callers need typed routing

---

## Changes

### servers/silk-shell/src/main.rs

**Added `HitTarget` enum** (after `PanelKind`, before `InteractionState`):

```rust
enum HitTarget {
    None,
    Surface(u64),
    FrameChrome { frame_id: u32, kind: u32 },
}
```

**Added `hit_test_at(x, y) -> HitTarget`** — pure hit-test function. Extracted from `click_hit_test_and_focus()`. Same priority order (focused → z-order → None), same dead-surface skip logic. No SilkBar check (handled separately). No side effects (no focus switch, no drag start).

**Added `hit_target_surface(target) -> Option<u64>`** — extractor for callers that need the surface_id from a hit result.

**Added `hit_target_label(target, silkbar_handled) -> (&'static str, u64)`** — classifies a hit into the `("app"|"chrome"|"none", surface_id)` diagnostic format used by `[shell.click.real.target]` budget markers.

**Changed `click_hit_test_and_focus()` return type** from `(u64, bool)` to `(HitTarget, bool)`:
- Returns `HitTarget::Surface` when a surface is hit (same behavior as before: focus switch, drag start)
- Returns `HitTarget::None` for background (same as `hit_id=0`)
- `FrameChrome` match arm exists but is a no-op (not yet produced)
- Internal hit-test logic replaced with `hit_test_at()` call

**Simplified both call sites** (USB path at line 913, EV_BTN path at line 1675):
- Removed `let focused = FOCUSED_SURFACE_ID;` and the 12-line if-else chain in each
- Replaced with `let (target, silkbar_handled) = click_hit_test_and_focus(...)`
- Budget markers use `hit_target_label(target, silkbar_handled)` instead of inline classification
- Net reduction: ~20 lines of duplicated classification code eliminated

### No other files changed

---

## Behavioral Equivalence

| Scenario | Before | After | Same? |
|----------|--------|-------|-------|
| Click on focused surface | hit_id=0, `[shell.click_focus.hit] id=N` | HitTarget::Surface(N), `[shell.click_focus.hit] id=N` | ✅ Same markers, same focus behavior (no change since hit_id == focused) |
| Click on different surface | hit_id=Z, try_set_focus(Z), `[shell.click_focus.hit/send.start/send.ok]` | HitTarget::Surface(Z), try_set_focus(Z), same markers | ✅ Same |
| Click on nothing | hit_id=0, `[shell.click_focus.miss]` | HitTarget::None, `[shell.click_focus.miss]` | ✅ Same |
| SilkBar handled | silkbar_handled=true, drag skipped | silkbar_handled=true, drag skipped | ✅ Same |
| Drag start | hit_id or focused surface, `[shell.drag.start]` | Same (uses FOCUSED_SURFACE_ID, unaffected by type change) | ✅ Same |
| Budget marker "chrome" | silkbar_handled → `("chrome", 0)` | hit_target_label → `("chrome", 0)` | ✅ Same |
| Budget marker "app" | hit_id!=0 or point_in_surface(focused) → `("app", id)` | hit_target_label → `("app", sid)` | ✅ Same |
| Budget marker "none" | else → `("none", 0)` | hit_target_label → `("none", 0)` | ✅ Same |

### Invariants preserved

1. **Focus only changes when a different alive surface is hit**: `click_hit_test_and_focus()` calls `try_set_focus()` only for `HitTarget::Surface(sid)` where `sid != FOCUSED_SURFACE_ID`. This is the same as before (hit_id was nonzero for z-order hits, zero for focused surface hit).

2. **Drag only starts on shell surfaces**: The `is_shell_surface(FOCUSED_SURFACE_ID)` guard at the drag start block is unchanged.

3. **Dead surfaces skipped**: `hit_test_at()` calls `surface_is_alive()` before `point_in_surface()`, same as the old inline code.

4. **SilkBar intercept preempts drag**: `handle_silkbar_click()` runs after hit-test, with drag gated on `!silkbar_handled`.

5. **All existing markers preserved**: `[shell.click_focus.down/hit/miss]`, `[shell.click.real.target]`, `[shell.drag.start/move/end]`, `[shell.hit_test.skip]` — all unchanged.

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic (optional verification)
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-chrome-hit-target-v1.log

for m in \
  shell.click_focus.hit \
  shell.focus.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-chrome-hit-target-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-chrome-hit-target-v1.log
```

Pass criteria:
- All existing drag/focus markers present with expected counts
- `shell.click_focus.hit` > 0, `shell.focus.set` > 0
- `shell.drag.start/move/end` > 0
- faults = 0

No new markers added — this phase changes the type contract only, not the diagnostic output.

---

## Deferred: FrameChrome Production

`HitTarget::FrameChrome { frame_id, kind }` is modeled but `hit_test_at()` never produces it. Production rules for future phases:

| Chrome element | frame_id | kind | Condition |
|---------------|----------|------|-----------|
| Tab strip | frame that owns the surface | 1 | y between bar_height and (bar_height + tab_strip_height), x within frame's tab strip region |
| Close button | frame that owns the surface | 2 | Same as tab strip, x within close button area |
| Resize handle | frame that owns the surface | 3 | x within edge threshold of frame's right/bottom edge |
| Neon rim | frame that owns the surface | 4 | Click on a frame that is NOT the focused frame (focus preempt) |

FrameChrome hits will be added incrementally as each chrome element is implemented. No sexdisplay rendering changes are needed for the hit-target model — the model only defines how hits are classified, not how chrome is painted.

---

## Remaining Risks

- **`FrameChrome` is dead code**: The variant exists in the enum but is never produced. It compiles away to zero runtime cost, but a future phase that forgets to add production logic may leave it dead. A `#[allow(dead_code)]` suppression or a reachability test in the Frame Chrome init phase would catch this.
- **Drag start uses `FOCUSED_SURFACE_ID` not `HitTarget`**: After `click_hit_test_and_focus()` returns, the drag start block at lines 621-624 still reads `FOCUSED_SURFACE_ID` instead of the returned `HitTarget`. This is correct for V1 because `click_hit_test_and_focus()` already set focus before returning. But when `FrameChrome` hits are produced and focus is NOT changed, the drag start would still check `FOCUSED_SURFACE_ID` and potentially start a drag where none is desired. The drag start gate needs to be updated when FrameChrome production is added.
- **`hit_target_surface()` is exported but unused**: The extractor helper is defined but not called in V1 (callers use `hit_target_label()` which destructures internally). It exists for external consumers (future chrome code paths). Dead code until consumed.

---

## Next Recommended Phase

**FRAME_HOVER_REVEAL_MODEL_V1** — Define the model for frame chrome reveal-on-hover (tab strip, close button, neon rim):

1. Define tab strip geometry relative to frame position (y = bar_height, height = tab_strip_height)
2. Add hover detection in silk-shell event loop (track which frame/tab the cursor is over without clicking)
3. Emit hover-state updates to sexdisplay via new or existing surface update path
4. Produce `HitTarget::FrameChrome` for clicks on chrome elements
5. Gate drag start on `!silkbar_handled && !is_frame_chrome(target)`

Or: **SELECTED_WINDOW_SILKBAR_OPTIONS_V1** — Add SilkBar actions for selected window (close button, focus indicator).

Recommended: **FRAME_HOVER_REVEAL_MODEL_V1** — hover detection and chrome element hit production are prerequisites for any interactive Frame Chrome.
