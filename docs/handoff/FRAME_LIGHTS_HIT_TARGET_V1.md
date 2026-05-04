# FRAME_LIGHTS_HIT_TARGET_V1

## Status

Implemented (2026-05-04). Shell-side hit detection for Frame Lights (close/minimize/zoom) within the top rim band of frame-owned surfaces. Model-only — no actions, no renderer changes, no protocol changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Light geometry defined | `FRAME_LIGHT_SIZE_PX=4`, `FRAME_LIGHT_GAP_PX=2` within 4px rim | Constants exist |
| Light hover detection | `frame_light_at()` resolves light kind from pointer position within top rim band | `[shell.frame.light.hover]` on state change |
| Light hover tracks correctly | `HOVERED_FRAME_LIGHT` updated on every hover change, reset to NONE when pointer leaves light region | Budgeted marker fires per transition |
| Three lights detected | CLOSE at x=gap..gap+size, MINIMIZE at gap+size+gap.., ZOOM at gap+size+gap+size+gap.. | `[shell.frame.light.model]` at boot |
| Frame Lights don't affect focus | No focus change code in light detection path | Focus markers unchanged |
| Frame Lights don't affect drag | Rim drag still resolved in `click_hit_test_and_focus()` independent of light hover | `[shell.drag.start]` unchanged |
| Frame Lights don't affect display | No sexdisplay/silkbar/silkbar-model/protocol changes | No new IPC |

---

## Changes

### `servers/silk-shell/src/main.rs`

#### 1. Geometry constants (after `FRAME_LIGHT_ZOOM`, ~line 257)

```rust
const FRAME_LIGHT_SIZE_PX: i32 = 4;
const FRAME_LIGHT_GAP_PX: i32 = 2;
```

Lights are 4×4px squares within the 4px top rim band. Positions computed relative to surface left edge: CLOSE at gap, MINIMIZE at gap+size+gap, ZOOM at gap+size+gap+size+gap.

#### 2. `frame_light_at()` mapping function (after `frame_light_to_option_mask()`, ~line 577)

```rust
unsafe fn frame_light_at(frame_id: u32, x: i32, y: i32) -> u32
```

Resolves active surface for the frame via `active_surface_for_frame()`, gets bounds via `get_surface_bounds()`, then checks y within top rim band and x within each light's horizontal range. Returns `FRAME_LIGHT_CLOSE`, `FRAME_LIGHT_MINIMIZE`, `FRAME_LIGHT_ZOOM`, or `FRAME_LIGHT_NONE`.

#### 3. `update_frame_hover_at()` — light wiring (previously always-reset-to-NONE)

- Added `new_light: u32` variable alongside `new_frame_id`/`new_kind`
- Each match arm now calls `frame_light_at()` for frame-related hits, `FRAME_LIGHT_NONE` otherwise
- Light state tracked with `HOVERED_FRAME_LIGHT` via `light_changed` gate
- Budgeted marker `[shell.frame.light.hover]` fires only on actual light changes (max 8)
- Return value updated to `changed || light_changed` (caller doesn't use it)

---

### File Changes

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +2 constants, +1 function (30 lines), ~25 lines modified in hover function |

### Files NOT Modified

- `kernel/` — no ABI changes
- `crates/sex-pdx/` — no protocol changes
- `crates/silkbar-model/` — no model changes
- `servers/sexdisplay/` — no renderer changes
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

### Light Geometry (top rim band of frame-owned surface)

```
                    FRAME_LIGHT_GAP_PX=2
                    ↓
sx + ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬...
     │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
sy   │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │← FRAME_RIM_PX=4
     │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
     │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
     └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──...
     ←4px→  ←2→  ←4px→  ←2→  ←4px→
      CLOSE  gap  MINIMIZE gap ZOOM
```

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.light.hover] frame=N light=N` | 8 | On every light state change (CLOSE/MINIMIZE/ZOOM/NONE transitions) |
| `[shell.frame.light.model] close=N minimize=N zoom=N` | 1 | Boot proof |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass. No new warnings in `silk-shell`.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-lights-hit-target-v1.log

for m in \
  shell.frame.light.hover \
  shell.frame.light.model \
  shell.frame.hover.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.selected.options.send \
  sexdisplay.selected.options.update
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-lights-hit-target-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-lights-hit-target-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.frame.light.model` | 1 | Model constants exist |
| `shell.frame.light.hover` | ≥0 (depends on pointer path) | Light hit detection works when pointer traverses light regions |
| `shell.frame.hover.set` | ≥0 | Existing hover tracking intact |
| `shell.drag.start/move/end` | ≥1 | Drag lifecycle intact |
| `shell.selected.options.send` | ≥1 | Options display intact |
| faults | 0 | Memory safety |

### Pass criteria

- `shell.frame.light.model` == 1 (model proved)
- `shell.drag.start/move/end` > 0 (drag lifecycle intact)
- faults == 0
- No close/minimize/zoom action behavior
- No renderer/protocol changes

---

## Remaining Risks

- **No tab strip**: Lights are positioned within the 4px rim band because `FRAME_TAB_STRIP_PX = 0`. When a tab strip is enabled, lights should relocate to the tab strip region and the rim light detection should be removed.
- **Tiny light targets**: Each light is 4×4 pixels with 2px gaps — small but functional for hit detection. Future phases may increase to e.g. 8×8 if rendering supports larger chrome regions (tab strip).
- **No visual feedback**: Hovering over a light updates `HOVERED_FRAME_LIGHT` state but no rendering reflects it. Light hover is invisible until the render phase.
- **No action on click**: Clicking a light region currently falls through to the FrameChrome rim match arm, which starts a rim drag or gets captured. No action behavior is implemented.
- **frame_light_at() called for Surface hits too**: For frame-owned surfaces in the body area, `frame_light_at()` is called but immediately returns NONE (pointer y below top rim band). This is harmless but adds a bounds lookup.
- **Only one frame**: In V1 only frame 1 (surface 100) exists. Light hit detection works for any frame that has an active surface with bounds.

---

## Next Recommended Phases

Two possible continuations:

1. **FRAME_LIGHTS_RENDER_PLAN_V1**: Design how sexdisplay renders the three colored dots (red/yellow/green) within the neon rim band when a frame light is hovered and/or the frame chrome is hover-revealed. No code, just design doc.

2. **FRAME_LIGHTS_ACTION_CLOSE_V1**: Implement close-frame behavior when the CLOSE light is clicked. Requires safety guards (surface destruction protocol, confirmation for dirty state).

Recommended: **FRAME_LIGHTS_RENDER_PLAN_V1** — designing how lights appear provides the complete picture before committing to action behavior.
