# SCENE_SETTINGS_PANEL_CONTROLS_PLAN_V1

## Status

Design (2026-05-04). Clickable control zone model for Scene Settings panel
(surface 0x96). Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_PANEL_CONTROLS_SAFE_SHELL_ONLY ✅

**Key finding**: Pointer hit-testing for surface 0x96 CAN be added shell-only
via a **parallel intercept** in the click dispatch that runs *before* the normal
hit-test when `SCENE_SETTINGS_ACTIVE` is true. This avoids ALL changes to:
- `point_in_surface()` / `hit_test_at()` / `get_surface_bounds()` / z-order
- Focus logic, drag logic, interaction states
- Any sexdisplay, protocol, kernel, or app crate

### Chosen Path: B (Add shell hit-test zones)

Option A (keyboard-only) is insufficient — users expect clickable controls.
Option C (renderer changes) is STOP FIRST. Option B is safe and minimal.

---

## Panel Coordinate Space

Panel surface 0x96 is positioned at screen coordinates (870, 60) with size
340w × 280h. All control zones use **absolute screen coordinates** (same as
SilkBar hit-testing in `handle_silkbar_click`).

```
(870,60)                    (1210,60)
   ┌────────────────────────────┐
   │  ┌──────────────────────┐  │ 60px  Cycle Preset (880,70,320,60)
   │  │    Cycle Preset      │  │
   │  └──────────────────────┘  │
   │  ┌──────────────────────┐  │ 50px  Cycle Tint (880,140,320,50)
   │  │    Cycle Tint        │  │
   │  └──────────────────────┘  │
   │  ┌──────────────────────┐  │ 40px  Toggle Top Bar (880,200,320,40)
   │  │   Toggle Top Bar     │  │
   │  └──────────────────────┘  │
   │  ┌──────────┐ ┌─────────┐ │ 40px  Reset (880,250,150,40)
   │  │  Reset   │ │  Close  │ │       Close (1050,250,150,40)
   │  └──────────┘ └─────────┘ │
   └────────────────────────────┘
(870,340)                  (1210,340)
```

---

## Control Zone Map

All coordinates are absolute screen-space (not panel-relative).
10px padding on all sides within the 340×280 panel.

| ID | Zone | Screen Rect | Command | Visual |
|----|------|------------|---------|--------|
| 1 | **Cycle Preset** | (880, 70, 320, 60) | `CMD_CYCLE_PRESET` | Colored bar showing current preset |
| 2 | **Cycle Tint** | (880, 140, 320, 50) | `CMD_CYCLE_TINT` | Tint swatch row |
| 3 | **Toggle Top Bar** | (880, 200, 320, 40) | `CMD_TOGGLE_TOP_BAR` | Toggle indicator rect |
| 4 | **Reset** | (880, 250, 150, 40) | `CMD_RESET_DEFAULTS` | Reset icon rect |
| 5 | **Close** | (1050, 250, 150, 40) | `toggle_scene_settings_panel()` | Close icon rect |

### Zone layout rationale

- **Vertical stacking**: Each zone spans full panel width (320px) except the
  bottom row which is split into Reset (left) and Close (right).
- **No overlap**: Zones are strictly non-overlapping. Hit-test order is top to
  bottom (1→5). First match wins.
- **Bottom row split**: Reset (150px) + 20px gap + Close (150px) = 320px.

---

## Click Behavior

### Entry point

New function `handle_settings_panel_click(px: i32, py: i32) -> bool` called
from the existing click dispatch, **before** the normal hit-test:

```rust
// In click dispatch (mouse button down), before normal hit-test:
if SCENE_SETTINGS_ACTIVE && handle_settings_panel_click(px, py) {
    // Panel consumed the click — skip normal focus/drag
    continue; // or equivalent skip
}
```

### Function

```rust
/// Hit-test the Scene Settings panel control zones.
/// Called before normal shell hit-test when panel is visible.
/// Returns true if a control zone consumed the click.
unsafe fn handle_settings_panel_click(px: i32, py: i32) -> bool {
    if !SCENE_SETTINGS_ACTIVE { return false; }

    static mut ZONE_BUDGET: u32 = 16;
    let b = &mut ZONE_BUDGET;

    // Zone 1: Cycle Preset
    if px >= 880 && px < 1200 && py >= 70 && py < 130 {
        handle_scene_settings_cmd(CMD_CYCLE_PRESET, 0, 0);
        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.click] zone=preset"); }
        return true;
    }
    // Zone 2: Cycle Tint
    if px >= 880 && px < 1200 && py >= 140 && py < 190 {
        handle_scene_settings_cmd(CMD_CYCLE_TINT, 0, 0);
        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.click] zone=tint"); }
        return true;
    }
    // Zone 3: Toggle Top Bar
    if px >= 880 && px < 1200 && py >= 200 && py < 240 {
        handle_scene_settings_cmd(CMD_TOGGLE_TOP_BAR, 0, 0);
        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.click] zone=topbar"); }
        return true;
    }
    // Zone 4: Reset
    if px >= 880 && px < 1030 && py >= 250 && py < 290 {
        handle_scene_settings_cmd(CMD_RESET_DEFAULTS, 0, 0);
        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.click] zone=reset"); }
        return true;
    }
    // Zone 5: Close
    if px >= 1050 && px < 1200 && py >= 250 && py < 290 {
        toggle_scene_settings_panel();
        if *b > 0 { *b -= 1; serial_println!("[shell.scene.settings.click] zone=close"); }
        return true;
    }

    false // click outside any zone — panel click-through allowed
}
```

### Click-through behavior

If `handle_settings_panel_click()` returns `false` (click outside any zone):
- Normal shell hit-test proceeds (click may focus a surface behind the panel).
- This is acceptable V1 behavior. The panel is a floating overlay.

---

## Interaction State Rules

| Current State | Panel Click → | New State |
|---------------|---------------|-----------|
| `Idle` | Any zone hit | `Idle` (command fires, no state change) |
| `PanelActive { Settings }` | Zone hit | `PanelActive { Settings }` (stays in panel) |
| `PanelActive { Settings }` | Close zone | `Idle` |
| `Drag { ... }` | Panel click | N/A (drag blocks panel consumption) |
| `ClickPending` | Panel click | N/A (ClickPending is transient) |

Panel clicks do NOT change focus, do NOT initiate drag, and do NOT change
`FOCUSED_SURFACE_ID`. Commands that mutate appearance (preset, tint, topbar)
fire `handle_scene_settings_cmd()` which handles all state internally.

---

## Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.scene.settings.click] zone=preset` | 16 | Click on Cycle Preset zone |
| `[shell.scene.settings.click] zone=tint` | 16 | Click on Cycle Tint zone |
| `[shell.scene.settings.click] zone=topbar` | 16 | Click on Toggle Top Bar zone |
| `[shell.scene.settings.click] zone=reset` | 16 | Click on Reset zone |
| `[shell.scene.settings.click] zone=close` | 16 | Click on Close zone |

Plus all existing markers from `handle_scene_settings_cmd()`:
`[shell.scene.settings.cmd] cmd=N ok=N` etc.

---

## Files for Implementation

### Modified

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Add `handle_settings_panel_click()` helper; add zone geometry constants; add call-site in click dispatch before normal hit-test; add marker budget. ~70 lines. |

### NOT modified

- `kernel/` — FORBIDDEN
- `servers/sexdisplay/` — no renderer changes
- `crates/sex-pdx/` — no ABI change
- `servers/sexinput/` — no input changes
- `servers/sexusb/` — unrelated
- `servers/sexstore/` — persistence path unchanged
- `servers/silkbar/` — unrelated
- `crates/silkbar-model/` — no model changes

---

## Proof Strategy

### Synthetic (env-var gated)

Add a `SEXOS_SETTINGS_CLICK_PROOF=1` gate in sexinput that sends mouse clicks
at each zone's center coordinates, using the existing click proof pattern:

```rust
// Stage 0: Move to (1040, 100), click — Cycle Preset zone center
// Stage 1: Move to (1040, 165), click — Cycle Tint zone center
// Stage 2: Move to (1040, 220), click — Toggle Top Bar zone center
// Stage 3: Move to (955, 270), click  — Reset zone center
// Stage 4: Move to (1125, 270), click — Close zone center
```

Centers computed from zone rects above.

### Runtime proof markers

```
[sexinput.settings.click_proof] stage=0
[shell.scene.settings.click] zone=preset
[shell.scene.settings.cmd] cmd=2 ok=1
...
[sexinput.settings.click_proof] stage=4
[shell.scene.settings.panel] visible=0  (panel closed)
```

All zones proven in one headless run. Zero faults.

---

## STOP Conditions

| Condition | Verdict |
|-----------|---------|
| Requires sexdisplay changes | STOP. Settings is shell-only. |
| Requires new protocol/opcode | Safe. Reuses existing `handle_scene_settings_cmd()`. |
| Requires sex-pdx ABI change | Safe. No public constants needed. |
| Requires kernel changes | STOP. No kernel edits. |
| Requires framebuffer writes | STOP. Shell does not write framebuffer. |
| Requires new app crate | Safe. All changes in silk-shell. |
| Requires broad hit-test rewrite | Safe. Parallel intercept, no existing code changed. |
| Click outside zones focuses panel | Safe. Click-through is V1-acceptable. |
| Panel click initiates drag | Safe. Panel click returns true → drag blocked. |
| Panel click changes focus | Safe. `handle_settings_panel_click` does not call `try_set_focus()`. |
| Zone rects overlap existing SilkBar hit zones | Safe. Panel is at y≥70, SilkBar is y<50. |
| Zone rects need per-frame update | Safe. All zones are static. |
| Close zone conflicts with `is_closeable_surface` | Safe. Close uses `toggle_scene_settings_panel()`, not `close_surface_from_frame_light()`. |
| Multiple zones triggered per click | Safe. First-match-wins; zones are non-overlapping. |
| Panel click during active drag | Safe. Drag state blocks new clicks via existing `InteractionState::Dragging` check. |

---

## Edge Cases

| Edge Case | Behavior |
|-----------|----------|
| Click on panel border (outside zones) | Click-through: normal hit-test proceeds |
| Panel open + click on background | Background click focuses linen/app as usual |
| Panel open + F7 | F7 closes panel (existing keyboard path) |
| Panel open + F4/F5/F6 | Key routing unchanged; commands fire normally |
| Click very fast (double-click) | Each click fires independently; tint cycle increments each click |
| Multiple zones overlap at pixel level | Impossible — zones are non-overlapping |
| Screen resolution < 1280×720 | Panel clipped; zones outside visible area unreachable |
| Focus on panel surface | Panel is OS-owned; `try_set_focus(0x96)` is not exposed to users |

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **No text labels** | User must infer zone purpose from colored rects |
| **No visual feedback** | No highlight/hover state on zone hover |
| **No drag slider** | Tint intensity / glow level cannot be adjusted |
| **Click-through on zone miss** | May cause unexpected focus changes behind panel |
| **Static zones only** | Layout cannot be reconfigured at runtime |
| **No scroll** | Only 5 zones fit; future expansion needs taller panel or scrolling |
| **Panel does not consume mouse-move** | Only click (button down) triggers zones |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PANEL_STATIC_V1.md` | Panel surface and F7 toggle |
| `docs/handoff/SCENE_SETTINGS_PANEL_KEYS_V1.md` | Existing keyboard controls (1/2/3/Esc) |
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_V1.md` | Command handler this reuses |
| `docs/handoff/SCENE_SETTINGS_PROTOCOL_SYNTH_PROOF_V1.md` | Protocol proof pattern |
| `docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` | Phase 2: pointer controls design |
| `servers/silk-shell/src/main.rs` | Target file for implementation |

## Next Recommended Phase

**SCENE_SETTINGS_PANEL_CONTROLS_V1** — Implement the 5-zone clickable controls
in silk-shell. Add `handle_settings_panel_click()` + call-site + constants +
markers. Add synthetic click proof in sexinput. Build + runtime verify.
