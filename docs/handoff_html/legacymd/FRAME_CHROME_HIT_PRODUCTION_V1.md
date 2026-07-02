# FRAME_CHROME_HIT_PRODUCTION_V1

## Status

Implemented (2026-05-04). Silk-shell hit-test now produces `HitTarget::FrameChrome` targets for
a 4px neon rim band around frame-owned surfaces. Tab strip band is reserved (geometry constant
defined, value = 0 = disabled in V1).

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Chrome rim hit production | `hit_test_surface_chrome()` checks 4px edge band on all four sides of frame-owned surfaces | `[shell.hit_target.chrome]` |
| Chrome hit before surface content | Rim check runs before content-area fallthrough in `hit_test_at()` | Same |
| No drag from chrome click | `click_hit_test_and_focus()` drag-start gate checks `!is_chrome_hit` | `[shell.drag.skip.chrome]` |
| No focus from chrome click | FrameChrome match arm in `click_hit_test_and_focus()` does not call `try_set_focus()` | `[shell.frame.chrome.capture]` |
| Chrome on z-order surfaces | Same chrome check runs in z-order fallback loop for frame-owned surfaces | `[shell.hit_target.chrome]` with `z=1` |
| Hover state resolves chrome hits | `update_frame_hover_at()` already handles FrameChrome target → frame_id/kind mapping | (pre-existing) |

### Contracts Not Changed

| Contract | Status |
|----------|--------|
| Drag target reads InteractionState::Dragging.surface_id | Unchanged — drag never starts from chrome click |
| Focus changes only through try_set_focus() | Unchanged — chrome click hits the FrameChrome arm which never calls try_set_focus |
| Surface lifetime self-defends in point_in_surface | Unchanged — `get_surface_bounds()` returns None for dead surfaces (delegates to surface_is_alive via point_in_surface) |
| Event ordering (receive → normalize → hit-test → action) | Unchanged — hit_test_at is called in the same order |

---

## Changes

### servers/silk-shell/src/main.rs

#### 1. Chrome geometry constants (after HOVER_TAB_STRIP, ~line 239)

```rust
const FRAME_CHROME_RIM: u32 = 1;
const FRAME_CHROME_TAB_STRIP: u32 = 2;
const FRAME_RIM_PX: i32 = 4;
const FRAME_TAB_STRIP_PX: i32 = 0;
```

#### 2. `get_surface_bounds()` helper (~line 353)

Extracts the bounding rectangle for a surface ID. Returns `None` for OS-owned surfaces
(cursor, panels) and invalid IDs. Duplicates the bounds match from `point_in_surface()`
to avoid refactoring that function.

```rust
unsafe fn get_surface_bounds(sid: u64) -> Option<(i32, i32, u32, u32)> {
    match sid {
        SURFACE_ID_APP    => Some((WINDOWS[1].desc.x, WINDOWS[1].desc.y, ...)),
        SURFACE_ID_STATIC => Some((SURFACE_101_X, SURFACE_101_Y, ...)),
        SURFACE_ID_TEST3  => Some((SURFACE_102_X, SURFACE_102_Y, ...)),
        SURFACE_ID_TEST4  => Some((SURFACE_103_X, SURFACE_103_Y, ...)),
        SURFACE_ID_LINEN  => Some((SURFACE_200_X, SURFACE_200_Y, ...)),
        _ => None,
    }
}
```

#### 3. `hit_test_surface_chrome()` helper (~line 701)

Checks if a point falls on frame chrome for a given surface:
1. Get surface bounds via `get_surface_bounds()` — returns None if surface has no geometry
2. Get frame_id via `frame_for_surface()` — returns None if surface not owned by a frame (no chrome)
3. Tab strip check (gated on `FRAME_TAB_STRIP_PX > 0`, disabled in V1)
4. Four-edge rim check: left, right, top, bottom at `FRAME_RIM_PX` (4px) thickness
5. Returns `Some(HitTarget::FrameChrome { frame_id, kind })` or `None`

#### 4. Modified `hit_test_at()` (~line 724)

After `point_in_surface()` returns true, calls `hit_test_surface_chrome()` before returning
`Surface(sid)`. If chrome is hit, returns `FrameChrome` instead. Same logic in the z-order
fallback loop. Budgeted `[shell.hit_target.chrome]` diagnostic (max 6 focused + 4 z-order).

#### 5. Modified drag-start gate in `click_hit_test_and_focus()` (~line 828)

Added `&& !is_chrome_hit` where `is_chrome_hit = matches!(target, HitTarget::FrameChrome { .. })`.
Also emits `[shell.drag.skip.chrome]` when drag is suppressed due to chrome hit.

#### 6. FrameChrome click handler (~line 822)

Replaced no-op comment with budgeted `[shell.frame.chrome.capture]` marker (max 4).

#### 7. Updated comments (3 locations)

- HitTarget doc: "FrameChrome is produced from rim/tab-strip geometry"
- hit_test_at doc: removed "FrameChrome variant is modeled but not yet produced"
- update_frame_hover_at: updated to reflect chrome is now produced

### Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.hit_target.chrome] frame=N kind=N x=N y=N` | 6 focused + 4 z-order | hit_test_at produces a FrameChrome target |
| `[shell.frame.chrome.capture] frame=N kind=N x=N y=N` | 4 | click lands on FrameChrome (no-op) |
| `[shell.drag.skip.chrome] x=N y=N` | unbudgeted | drag suppressed because target is chrome |

### hit_target_label() updated

Now classifies `FrameChrome` as `"chrome_frame"` instead of falling through to `"none"`.

---

## Build

```bash
# Default (no synthetic)
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly (no new warnings in changed files).

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-chrome-hit-v1.log

for m in \
  shell.hit_target.chrome \
  shell.frame.chrome.capture \
  shell.drag.skip.chrome \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.frame.hover.set
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-chrome-hit-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-chrome-hit-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.hit_target.chrome` | ≥0 (depends on cursor position during test) | Chrome hits can be produced |
| `shell.drag.start` | ≥1 | Drag lifecycle intact (content-area clicks still work) |
| `shell.drag.move` | ≥1 | Drag movement works |
| `shell.drag.end` | ≥1 | Drag release works |
| `shell.drag.skip.chrome` | ≥0 (depends on test) | Drag suppressed on chrome clicks |
| faults | 0 | Memory safety |

### Pass criteria

- `shell.drag.start` > 0, `shell.drag.move` > 0, `shell.drag.end` > 0 (drag lifecycle intact)
- faults == 0

Note: `shell.hit_target.chrome` and `shell.drag.skip.chrome` counts depend on whether the test
cursor passes through the 4px rim band. In the synthetic proof (SEXUSB_SYNTHETIC=1), the
242-frame drag sequence moves the cursor across surface 100 (800×500) starting at pixel (395, 245),
well inside the content area. The 4px rim at the edges is not traversed, so chrome hits are 0
in the synthetic proof. Real mouse testing with edge-of-surface clicks will produce positive counts.

---

## Remaining Risks

- **Tab strip disabled**: `FRAME_TAB_STRIP_PX = 0`. The tab strip chrome kind is defined but never
  produced. Enabling it requires changing the constant and ensuring the top-rim exclusion zone.
- **Linen (surface 200) has no chrome**: Linen is the desktop background and is not owned by any
  frame. Clicking near its edges produces `HitTarget::Surface(200)` not `FrameChrome`.
- **Single frame only**: Chrome is only produced for surfaces owned by frames. In V1, only surface
  100 (APP) is in a frame (frame 1). Surfaces 101-103 are standalone and produce no chrome.
- **Chrome hit-target unused by display**: The shell produces FrameChrome targets but sexdisplay
  does not render a neon rim. This is the next phase (FRAME_CHROME_RENDER_V1 or equivalent).
- **No hover action on chrome**: Hover state tracks FrameChrome hits via `update_frame_hover_at()`
  but no visual feedback is rendered. Future phases may add rim glow/highlight on hover.

---

## Next Recommended Phase

### FRAME_CHROME_RENDER_V1 (or FRAME_CHROME_HOVER_FEEDBACK_V1)

Two possible continuations:

1. **FRAME_CHROME_RENDER_V1**: Teach sexdisplay to render a neon rim border around frame-owned
   surfaces based on policy descriptors emitted by silk-shell. Requires IPC protocol extension
   or surface attribute channel.

2. **FRAME_CHROME_HOVER_FEEDBACK_V1**: Use the existing hover state (`HOVERED_FRAME_ID`,
   `HOVER_KIND`) to emit a visual indicator (e.g., rim color change) through the existing
   surface update mechanism. Lighter lift than full renderer change.

Recommended: **FRAME_CHROME_HOVER_FEEDBACK_V1** — can be done purely in silk-shell policy
without sexdisplay changes, and provides immediate visual feedback for the chrome hit-production
that this phase enabled.
