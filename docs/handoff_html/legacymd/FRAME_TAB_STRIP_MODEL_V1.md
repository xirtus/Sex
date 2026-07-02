# FRAME_TAB_STRIP_MODEL_V1

## Status

Implemented (2026-05-04). Shell-side tab strip geometry and hit detection. No renderer changes. No IPC/protocol changes. Tab strip model enabled with light-exclusion zone and deterministic tab slot layout.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Tab strip model exists | `FRAME_TAB_STRIP_PX = 4` enables tab strip band in top rim | `[shell.frame.tab.model]` |
| Tab strip excludes Frame Lights | `frame_tab_at()` rejects x < `FRAME_TAB_LIGHT_EXCLUSION_PX` (20px) | Lights zone not overridden by tab strip |
| Tab strip excludes right rim | `frame_tab_at()` rejects x >= `right_rim_start` | Right rim preserved |
| Frame Lights still higher priority | Lights checked in `click_hit_test_and_focus()` before tab strip fallthrough | `[shell.frame.light.close/minimize/zoom]` |
| Tab strip click remains no-op | `FRAME_CHROME_TAB_STRIP` arm hits `[shell.drag.skip.chrome]` | No drag, no action |
| Rim drag preserved | Non-light, non-tab rim clicks start drag | `[shell.frame.rim.drag.start]` |
| Tab hints returned | `frame_tab_at()` returns `Some(tab_index)` for tab positions | N/A |
| Tab count/active index queryable | `frame_tab_count()` and `frame_active_tab_index()` helpers | N/A |

---

## Changes

### File: `servers/silk-shell/src/main.rs`

#### 1. Constants (formerly FRAME_TAB_STRIP_PX = 0, now enabled)

```rust
const FRAME_TAB_STRIP_PX: i32 = 4;                                 // was 0
const FRAME_TAB_LIGHT_EXCLUSION_PX: i32 = 20;                       // new
const FRAME_TAB_MIN_WIDTH_PX: i32 = 12;                             // new
```

`FRAME_TAB_LIGHT_EXCLUSION_PX = 20` covers:
```
gap(2) + close(4) + gap(2) + minimize(4) + gap(2) + zoom(4) + gap(2) = 20px
```

#### 2. Tab strip helpers (after `frame_light_at()`, ~line 963)

**`frame_tab_count(frame_id) -> u32`** — returns `tab_count` from ShellFrame.

**`frame_active_tab_index(frame_id) -> u32`** — returns `active_tab` from ShellFrame.

**`frame_tab_at(frame_id, x, y) -> Option<u32>`** — detects which tab the pointer is over:
1. Must be in top rim band (y within `FRAME_TAB_STRIP_PX`)
2. Must be outside light exclusion zone (`x >= sx + 20`)
3. Must not extend into right rim (`x < right_rim_start`)
4. Computes equal-width tab slots from `available_width / tab_count`
5. Returns `Some(tab_index)` or `None`

#### 3. `hit_test_surface_chrome()` tab strip check (line 1203)

Was:
```rust
if y >= strip_top && y < strip_bot && x >= sx && x < (sx + sw as i32) {
    return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
```

Now with exclusion zones:
```rust
let tab_strip_start = sx + FRAME_TAB_LIGHT_EXCLUSION_PX;
let right_rim_start = sx + sw as i32 - FRAME_RIM_PX;
if y >= strip_top && y < strip_bot
    && x >= tab_strip_start
    && x < right_rim_start
{
    if frame_tab_at(frame_id, x, y).is_some() {
        return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
    }
}
```

#### 4. Boot proof marker (after frame light model marker, ~line 1663)

```rust
serial_println!("[shell.frame.tab.model] tabs=N has_tab=N strip_px=4");
```

Proves `frame_tab_at()` works with boot geometry: surface 100 at `sx=100`, tab strip starts at `x=120`, single tab slot at the boot rect.

### File Changes

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +3 constants, +3 helpers (~50 lines), +hit test exclusion fix, +boot proof marker |

### Files NOT Modified

Sexdisplay, kernel, PDX ABI, silkbar, sexusb, sexinput — all untouched.

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.tab.model] tabs=N has_tab=N strip_px=N` | 1 | Boot proof that tab strip model is initialized |
| `[shell.frame.light.model]` | 1 | Pre-existing — lights still work |
| `[shell.hit_target.chrome]` | 6+4 | Chrome hits still produced (tab strip produces `FRAME_CHROME_TAB_STRIP`) |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass. No new warning types.

---

## Light Exclusion Zone

The tab strip hit zone is explicitly bounded to avoid overlapping Frame Lights:

```
Surface left edge (sx)
  │
  ├── 0-1:  empty (2px)
  ├── 2-5:  CLOSE light  (4px)
  ├── 6-7:  gap (2px)
  ├── 8-11: MINIMIZE light (4px)
  ├── 12-13: gap (2px)
  ├── 14-17: ZOOM light (4px)
  ├── 18-19: gap (2px)
  │
  ├── 20+:  TAB STRIP zone (after FRAME_TAB_LIGHT_EXCLUSION_PX = 20)
  │         tab blocks fill available width as equal slots
  │
  └── right rim (last 4px): excluded from tab strip
```

**Safety:** If the lights zone width ever changes (e.g., more lights added), `FRAME_TAB_LIGHT_EXCLUSION_PX` must be updated to match. The `frame_light_at()` function is the authority for light positions — `FRAME_TAB_LIGHT_EXCLUSION_PX` should always be >= the last light's right edge + gap.

---

## Tab Slot Layout

Tab blocks are sized as equal-width slots spanning the available width:

```
available_width = (right_rim_start - tab_strip_start)
slot_width = available_width / tab_count
tab_index = floor((x - tab_strip_start) / slot_width)
```

For V1 with 1 tab and surface 100 at boot geometry (sx=100, sw=800):
- `tab_strip_start = 100 + 20 = 120`
- `right_rim_start = 100 + 800 - 4 = 896`
- `available_width = 896 - 120 = 776px`
- `slot_width = 776 / 1 = 776px`
- Tab 0 covers x=120..896 (entire tab strip)

For multi-tab (future) with 4 tabs:
- `slot_width = 776 / 4 = 194px`
- Tab 0: x=120..314, Tab 1: x=314..508, etc.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-tab-strip-model-v1.log

for m in \
  shell.frame.tab.model \
  shell.frame.light.model \
  shell.hit_target.chrome \
  shell.frame.zoom \
  shell.frame.minimize \
  shell.drag.start
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-tab-strip-model-v1.log)"
done
```

### Pass criteria

- Default build passes
- Synthetic build passes
- `[shell.frame.tab.model]` fires with `tabs=1 has_tab=1 strip_px=4`
- `[shell.frame.light.model]` still fires
- Frame Lights clicks still work (close/minimize/zoom)
- Rim drag still works on non-light, non-tab rim clicks
- CLOSE/MINIMIZE/ZOOM still work via their respective lights
- No panic/#PF/#GP

---

## Remaining Risks

- **No visual tab strip**: Sexdisplay renders nothing in the tab strip zone — the rim color fills the entire top band. Tab strip hits exist in hit-test but have no visible representation. Users can click on invisible tab blocks with no feedback.
- **Tab-to-light proximity**: Tab strip starts at x=20 (relative to surface left edge). The ZOOM light ends at x=18. A user clicking at x=18-19 hits ZOOM light (visible green) or gap (rim color). At x=20, tab strip starts. No visible boundary between lights and tab strip.
- **Equal-width slot model**: Fixed equal-width slots mean tabs don't reflect actual content width. Future phases may need variable-width tabs based on title text.
- **FRAME_TAB_STRIP_PX = FRAME_RIM_PX**: Tab strip height equals rim height (4px). If the tab strip needs to be taller than the rim (e.g., for text labels), this constant must change independently of rim height. The current equality is not enforced by code.

---

## Next Recommended Phase

### FRAME_TAB_STRIP_IPC_PLAN_V1

Design the IPC protocol for communicating tab metadata (tab_count, active_tab) from silk-shell to sexdisplay. Required before colored tab blocks can be rendered in the top rim band. Includes:
- New opcode design (0xFD or similar)
- Sexdisplay data structure for tab info
- Tab block rendering in composite_pixel()
- Active vs inactive tab color scheme
- Tab strip update on tab switch (future)
