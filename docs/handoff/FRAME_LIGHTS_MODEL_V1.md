# FRAME_LIGHTS_MODEL_V1

## Status

Implemented (2026-05-04). Shell-side model for Frame Lights (red close, yellow minimize, green zoom) added to `servers/silk-shell/src/main.rs`. Model-only — no actions, no renderer changes, no protocol changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Light kind constants exist | `FRAME_LIGHT_NONE=0`, `FRAME_LIGHT_CLOSE=1`, `FRAME_LIGHT_MINIMIZE=2`, `FRAME_LIGHT_ZOOM=3` | `[shell.frame.light.model]` |
| Light-to-option mapping works | `frame_light_to_option_mask()` maps CLOSE→1, MINIMIZE→4, ZOOM→2 | Called at boot |
| Light hover state tracks NONE | `HOVERED_FRAME_LIGHT` reset to `FRAME_LIGHT_NONE` on every hover change | `[shell.frame.light.hover]` (fires in future phases) |
| Frame Lights model does not affect focus | No focus change code in light model | No markers changed |
| Frame Lights model does not affect drag | No drag code in light model | No markers changed |
| Frame Lights model does not affect display | No sexdisplay/silkbar/silkbar-model changes | No protocol changes |
| Selected-window options unchanged | `OPTION_CLOSE/OPTION_ZOOM/OPTION_MINIMIZE/OPTION_MOVE` constants untouched | Existing markers unchanged |

---

## Changes

### `servers/silk-shell/src/main.rs`

#### 1. Frame Light constants (after `FRAME_TAB_STRIP_PX`, ~line 248)

```rust
const FRAME_LIGHT_NONE: u32 = 0;
const FRAME_LIGHT_CLOSE: u32 = 1;
const FRAME_LIGHT_MINIMIZE: u32 = 2;
const FRAME_LIGHT_ZOOM: u32 = 3;
```

#### 2. `HOVERED_FRAME_LIGHT` state (after `HOVER_KIND`, ~line 270)

```rust
static mut HOVERED_FRAME_LIGHT: u32 = FRAME_LIGHT_NONE;
```

#### 3. `frame_light_to_option_mask()` mapping helper (after `selected_window_options_mask()`, ~line 563)

```rust
fn frame_light_to_option_mask(light: u32) -> u32 {
    match light {
        FRAME_LIGHT_CLOSE => OPTION_CLOSE,
        FRAME_LIGHT_MINIMIZE => OPTION_MINIMIZE,
        FRAME_LIGHT_ZOOM => OPTION_ZOOM,
        _ => 0,
    }
}
```

#### 4. Light hover tracking in `update_frame_hover_at()` (~line 621)

Added reset of `HOVERED_FRAME_LIGHT` to `FRAME_LIGHT_NONE` within the hover change block. Guarded by `HOVERED_FRAME_LIGHT != FRAME_LIGHT_NONE` check — in V1 this never fires since light is always NONE, but future phases will set actual light kinds and the marker will fire on transition.

```rust
if HOVERED_FRAME_LIGHT != FRAME_LIGHT_NONE {
    HOVERED_FRAME_LIGHT = FRAME_LIGHT_NONE;
    // budgeted [shell.frame.light.hover] marker (max 8)
}
```

#### 5. Boot proof marker (~line 1106)

Budgeted marker (max 1) proving constants exist and mapping works:

```
[shell.frame.light.model] close=1 minimize=2 zoom=3 mask=0x2
```

---

### Files Modified

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +5 constants, +1 static, +1 function, +1 hover reset, +1 boot marker |

### Files NOT Modified

- `kernel/` — no ABI changes
- `crates/sex-pdx/` — no protocol changes
- `crates/silkbar-model/` — no model changes
- `servers/sexdisplay/` — no renderer changes
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.light.model] close=N minimize=N zoom=N mask=N` | 1 (boot only) | At boot, proves constants and mapping |
| `[shell.frame.light.hover] frame=N light=N` | 8 (state change) | Future phases when light kind actually changes |

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
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-lights-model-v1.log

for m in \
  shell.frame.light.model \
  shell.frame.light.hover \
  shell.selected.options.send \
  sexdisplay.selected.options.update \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-lights-model-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-lights-model-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.frame.light.model` | 1 | Model constants and mapping exist |
| `shell.frame.light.hover` | 0 (V1) | No light hover changes in model-only phase (future phases: ≥0) |
| `shell.selected.options.send` | ≥1 | Selected-window options display intact |
| `shell.drag.start/move/end` | ≥1 | Drag lifecycle intact |
| faults | 0 | Memory safety |

### Pass criteria

- `shell.frame.light.model` == 1 (model proved)
- `shell.selected.options.send` > 0 (options display undamaged)
- `shell.drag.start/move/end` > 0 (drag lifecycle intact)
- faults == 0
- No close/minimize/zoom behavior implemented
- No renderer/protocol changes

---

## Remaining Risks

- **No pixel detection**: `HOVERED_FRAME_LIGHT` always stays `FRAME_LIGHT_NONE`. The light kind is never set based on pointer position within frame chrome. This is by design for model-only V1.
- **No actions**: Close/minimize/zoom have no behavior. Clicking a light region would fall through to content behavior (or be captured as FrameChrome if hit-test is extended).
- **No rendering**: Lights are not visible. No protocol to communicate light state to sexdisplay.
- **Option constants duplicated in spirit**: `FRAME_LIGHT_CLOSE` (kind=1) maps to `OPTION_CLOSE` (bit=1) by coincidence; `FRAME_LIGHT_MINIMIZE` (kind=2) maps to `OPTION_MINIMIZE` (bit=4) — one is index-like, the other is bitfield. The mapping function handles translation.

---

## Next Recommended Phases

Two possible continuations:

1. **FRAME_LIGHTS_HIT_TARGET_V1**: Extend `hit_test_surface_chrome()` to detect which light region (close/minimize/zoom) the pointer is over within the frame chrome band. Would set `HOVERED_FRAME_LIGHT` to the appropriate kind and fire `[shell.frame.light.hover]` markers. Requires defining light pixel regions/positions.

2. **FRAME_LIGHTS_RENDER_PLAN_V1**: Design how sexdisplay should render the three colored dots (red/yellow/green) within the neon rim band when frame chrome is hover-revealed. No code, just design doc.

Recommended: **FRAME_LIGHTS_HIT_TARGET_V1** — activates the hover tracking model added in this phase and provides actual light kind detection for downstream rendering.
