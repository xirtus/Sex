# FRAME_RIM_DRAG_POLICY_V1

## Status

Implemented (2026-05-04). Silk-shell now starts a drag operation on frame chrome rim clicks,
resolving the active surface from the hit frame. Content drag behavior is unchanged. Non-rim
chrome (tab strip, reserved) remains no-op capture without focus or drag.

---

## Policy

| Hit target | Focus change | Drag action | Marker |
|------------|-------------|-------------|--------|
| `Surface(sid)` | `try_set_focus(sid)` if different | Content drag on `FOCUSED_SURFACE_ID` | `[shell.drag.start]` |
| `FrameChrome{kind=RIM}` | **No focus change** | Rim drag on `active_surface_for_frame(frame_id)` | `[shell.frame.rim.drag.start]` |
| `FrameChrome{kind=TAB_STRIP}` | None | No-op capture | `[shell.frame.chrome.capture]` + `[shell.drag.skip.chrome]` |
| `None` | None | Content drag on focused surface (same as before) | `[shell.drag.start]` (if in bounds) |
| `FrameChrome{kind=other}` | None | No-op capture | `[shell.frame.chrome.capture]` |

### Focus policy for chrome capture

- Rim drag does **not** call `try_set_focus()`. Focus remains on the previously focused surface.
- This means focus can be on surface 100 while the user drags a different frame's rim
  (multi-frame V2+ use case).
- `InteractionState::Dragging.surface_id` stores the frame-resolved surface, keeping the drag
  target independent of `FOCUSED_SURFACE_ID` (same invariant proven in SHELL_INTERACTION_STATE_V1).

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Rim drag resolves frame → active surface | `active_surface_for_frame()` called with frame_id from HitTarget | `[shell.frame.rim.drag.start] surface=N` |
| Rim drag validates surface alive | `surface_is_alive()` check before drag start | `[shell.frame.rim.drag.reject] reason=dead` |
| Rim drag no focus change | No `try_set_focus()` call in rim path | Verified by absence of `[shell.focus.set]` between click rim and drag |
| Rim drag uses `InteractionState::Dragging.surface_id` | `try_transition(Dragging{surface_id})` stores frame-resolved surface | `[shell.interaction.transition] ... Dragging` |
| Content drag unchanged | `is_content_hit` flag preserves Surface/None drag behavior | `[shell.drag.start]` unchanged |
| Non-rim chrome no-op | TAB_STRIP and reserved kinds hit capture arm only | `[shell.frame.chrome.capture]`, no drag.start |

---

## Changes

### `servers/silk-shell/src/main.rs` — `click_hit_test_and_focus()` only

**Before:** FrameChrome match arm was a homogeneous no-op with budgeted capture marker.
**After:** FrameChrome match arm branches on `kind`:
- `kind == FRAME_CHROME_RIM` → resolve active surface, validate alive, start drag
- `kind != FRAME_CHROME_RIM` → existing no-op capture

**Before:** Content drag gate used `!is_chrome_hit` to exclude all chrome.
**After:** Content drag gate uses `is_content_hit = matches!(Surface | None)` which excludes
chrome but does not interfere with rim drag start in the match arm.

**Before:** `[shell.drag.skip.chrome]` fired for all chrome hits.
**After:** `[shell.drag.skip.chrome]` fires only for `FRAME_CHROME_TAB_STRIP` (rim drag is
already started, `shell.frame.rim.drag.start` replaces the skip diagnostic).

### New markers

| Marker | Budget | Where |
|--------|--------|-------|
| `[shell.frame.rim.drag.start] frame=N surface=N x=N y=N` | 8 | Rim drag started via try_transition |
| `[shell.frame.rim.drag.reject] frame=N reason=dead\|no_active_surface` | unbudgeted | Rim drag could not start (surface dead or missing) |

### Unchanged

- `[shell.click_focus.down]` — still first line of click_hit_test_and_focus
- `[shell.click_focus.hit/miss]` — Surface/None arms unchanged
- `[shell.drag.start]` — content drag gate preserved for Surface/None hits
- `[shell.frame.chrome.capture]` — non-rim chrome capture preserved
- `[shell.drag.skip.chrome]` — now tab-strip specific only
- `drag_move_focused()` — no changes; reads InteractionState::Dragging.surface_id as before
- All synthetic proof markers — content drag path unchanged

---

## Build

```bash
# Default (no synthetic)
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly. No new warnings.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-rim-drag-policy-v1.log

for m in \
  shell.frame.rim.drag.start \
  shell.frame.rim.drag.reject \
  shell.frame.chrome.capture \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.click_focus.hit \
  shell.cursor_surface.move.ok \
  shell.drag.skip.chrome
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-rim-drag-policy-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-rim-drag-policy-v1.log
```

### Expected counts with synthetic proof (SEXUSB_SYNTHETIC=1)

The 242-frame synthetic drag sequence clicks on surface 100 content area (not rim). Rim drag
markers `[shell.frame.rim.drag.start]` will be 0 in the synthetic proof because the cursor
travels at pixel (395, 245) well inside the 4px rim band. Rim drag requires a real mouse
click at the edge of a frame-owned surface.

| Marker | Synthetic expected | Real mouse (edge click) expected |
|--------|-------------------|----------------------------------|
| `shell.frame.rim.drag.start` | 0 | ≥1 (when rim clicked) |
| `shell.frame.rim.drag.reject` | 0 | 0 (frame has active surface) |
| `shell.drag.start` | ≥1 | ≥1 (content drags still work) |
| `shell.drag.move` | ≥1 | ≥1 |
| `shell.drag.end` | ≥1 | ≥1 |
| `shell.click_focus.hit` | ≥1 | ≥1 |
| faults | 0 | 0 |

### Pass criteria

- `shell.drag.start` > 0 (content drag lifecycle intact)
- faults == 0
- No new warnings in build
- Synthetic proof markers unchanged in meaning

---

## Remaining Risks

- **Rim drag tested only via real mouse**: The synthetic proof does not exercise the rim drag
  path because the 242-frame drag sequence stays in the content area. Rim drag must be verified
  manually by clicking within 4px of a frame-owned surface edge.
- **Tab strip (kind=2) not produced**: `FRAME_TAB_STRIP_PX = 0` so tab strip chrome hits are
  never produced. The `[shell.drag.skip.chrome]` diagnostic for tab strip is untestable until
  tab strip geometry is enabled.
- **Single frame only**: In V1, only frame 1 (surface 100) exists. Rim drag on surface 100
  resolves to surface 100 (same surface). In multi-frame V2, rim drag on frame 2 would start
  a drag on frame 2's active surface while focus remains on the previous frame — this is
  correctly modeled but untested.
- **No visual rim feedback**: The shell starts a rim drag but sexdisplay does not render a
  neon rim. The user sees the surface move without visual indication that the rim was captured.

---

## Next Recommended Phase

### FRAME_CHROME_RENDER_PLAN_V1

Two options:

1. **FRAME_CHROME_RENDER_PLAN_V1** (recommended): Design a protocol extension or surface
   attribute channel for sexdisplay to render neon rim borders. Silk-shell would emit
   rim descriptors (frame_id, color, thickness) that sexdisplay consumes for rendering.
   This is the first phase requiring sexdisplay changes.

2. **SELECTED_WINDOW_SILKBAR_OPTIONS_V1**: Add frame chrome model integration to SilkBar
   (close/minimize/maximize buttons rendered in SilkBar for the selected window). Does not
   require sexdisplay changes but depends on SELECTED_WINDOW model.

Recommended: **FRAME_CHROME_RENDER_PLAN_V1** — rim drag without visual feedback is
incomplete; the neon rim must be rendered for the user to know where the drag-able
region is. This is the natural next step after rim drag policy.
