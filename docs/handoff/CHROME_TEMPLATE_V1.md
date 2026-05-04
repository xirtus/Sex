# CHROME_TEMPLATE_V1

## Status

Complete (2026-05-04). Data-driven `ChromeTemplate` model centralizes all
shell chrome geometry constants. No visual behavior change. Build passes.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `Rect` struct, `ChromeTemplate` struct, `SILK_CHROME_TEMPLATE_DEFAULT` const; replaced 10 `FRAME_*` geometry constants and 4 `SCENE_SETTINGS_PANEL_*` constants with template references. +213/-180 lines. |
| `docs/handoff/CHROME_TEMPLATE_V1.md` | New — this document |

### NOT modified

- `kernel/` — no kernel changes
- `servers/sexdisplay/` — no renderer changes
- `crates/sex-pdx/` — no ABI change
- `servers/sexinput/` — no input changes
- `servers/sexusb/` — unrelated
- `servers/sexstore/` — unrelated
- `servers/silkbar/` — unrelated

---

## ChromeTemplate Fields

```rust
struct ChromeTemplate {
    rim_px: i32,                          // 4
    top_bar_height_px: i32,               // 16
    light_size_px: i32,                   // 4
    light_gap_px: i32,                    // 2
    top_bar_light_size_px: i32,           // 8
    top_bar_light_gap_px: i32,            // 4
    top_bar_light_exclusion_px: i32,      // 40
    tab_light_exclusion_px: i32,          // 20
    tab_min_width_px: i32,                // 12
    tab_strip_px: i32,                    // 4
    settings_panel_x: u32,                // 870
    settings_panel_y: u32,                // 60
    settings_panel_w: u32,                // 340
    settings_panel_h: u32,                // 280
    control_preset_up: Rect,              // (0,0,0,0) reserved
    control_preset_down: Rect,            // (0,0,0,0) reserved
    control_reset: Rect,                  // (0,0,0,0) reserved
    control_close: Rect,                  // (0,0,0,0) reserved
    control_topbar_toggle: Rect,          // (0,0,0,0) reserved
}
```

Default values in parentheses match the pre-existing hardcoded constants
exactly. No visual behavior change.

---

## Centralized Constants

### Replaced FRAME_* constants

| Old (hardcoded) | New (template) |
|-----------------|----------------|
| `FRAME_RIM_PX = 4` | `SILK_CHROME_TEMPLATE_DEFAULT.rim_px` |
| `FRAME_TAB_STRIP_PX = 4` | `SILK_CHROME_TEMPLATE_DEFAULT.tab_strip_px` |
| `FRAME_TAB_LIGHT_EXCLUSION_PX = 20` | `SILK_CHROME_TEMPLATE_DEFAULT.tab_light_exclusion_px` |
| `FRAME_TAB_MIN_WIDTH_PX = 12` | `SILK_CHROME_TEMPLATE_DEFAULT.tab_min_width_px` |
| `FRAME_LIGHT_SIZE_PX = 4` | `SILK_CHROME_TEMPLATE_DEFAULT.light_size_px` |
| `FRAME_LIGHT_GAP_PX = 2` | `SILK_CHROME_TEMPLATE_DEFAULT.light_gap_px` |
| `FRAME_TOP_BAR_HEIGHT_PX = 16` | `SILK_CHROME_TEMPLATE_DEFAULT.top_bar_height_px` |
| `FRAME_TOP_BAR_LIGHT_SIZE_PX = 8` | `SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_size_px` |
| `FRAME_TOP_BAR_LIGHT_GAP_PX = 4` | `SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_gap_px` |
| `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX = 40` | `SILK_CHROME_TEMPLATE_DEFAULT.top_bar_light_exclusion_px` |

### Replaced SCENE_SETTINGS_PANEL_* constants

| Old (hardcoded) | New (template) |
|-----------------|----------------|
| `SCENE_SETTINGS_PANEL_X = 870` | `SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_x` |
| `SCENE_SETTINGS_PANEL_Y = 60` | `SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_y` |
| `SCENE_SETTINGS_PANEL_W = 340` | `SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_w` |
| `SCENE_SETTINGS_PANEL_H = 280` | `SILK_CHROME_TEMPLATE_DEFAULT.settings_panel_h` |

### Preserved (not templated)

- `FRAME_CHROME_RIM` / `FRAME_CHROME_TAB_STRIP` — hit-target kind tags, not geometry
- `FRAME_LIGHT_NONE/CLOSE/MINIMIZE/ZOOM` — light kind identifiers
- `FRAME_FLAG_*` — frame flag bit positions
- `OPTION_*` — window option bits
- Flag/shift constants — not geometry

---

## Rect Helper

```rust
struct Rect { x: i32, y: i32, w: u32, h: u32 }
impl Rect {
    const fn new(x, y, w, h) -> Self;
    fn contains(&self, px: i32, py: i32) -> bool;
}
```

The `Rect` type and `contains()` method are available for future control zone
hit-testing (settings panel click zones, frame chrome element dispatch).

---

## Build

```
[SEXOS ENTRYPOINT] success
```

Default build passes. No new warning types.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Template is compile-time const** | No runtime reconfiguration; requires rebuild to change chrome geometry |
| **Control rects reserved as zero** | Panel click zones not yet populated; deferred to SCENE_SETTINGS_PANEL_CONTROLS_V1 |
| **No alpha / glass fields yet** | Future GLASS_CHROME_PLAN_V1 will add effect parameters |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PANEL_STATIC_V1.md` | Panel geometry centralized |
| `docs/handoff/FRAME_TOP_BAR_RENDER_PLAN_V1.md` | Top bar height centralized |
| `docs/handoff/SCENE_SETTINGS_PANEL_CONTROLS_PLAN_V1.md` | Future control zone rects |

## Next Recommended Phase

**SCENE_SETTINGS_PANEL_CONTROLS_V1** — Implement clickable control zones on
the Scene Settings panel using the `ChromeTemplate` model and `Rect` helper.
Then **GLASS_CHROME_PLAN_V1** for alpha/glass effect parameters.
