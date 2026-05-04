# SCENE_SETTINGS_PANEL_STATIC_V1

## Status

Complete (2026-05-04). Static Scene Settings quick panel surface (0x96)
implemented in silk-shell, toggled via F7. Shell-only — no sexdisplay,
kernel, or protocol changes. Build passes.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `SURFACE_ID_SCENE_SETTINGS = 0x96`, `SCENE_SETTINGS_ACTIVE` flag, panel geometry constants, `ToggleSceneSettingsPanel` SurfaceAction, `Settings` PanelKind variant, F7 scancode (0x41) mapping, `surface_is_alive` arm, `is_closeable_surface` arm, `toggle_scene_settings_panel()` helper, dispatch arm with marker |
| `docs/handoff/SCENE_SETTINGS_PANEL_STATIC_V1.md` | New — this document |

### NOT modified

- `kernel/` — no kernel changes
- `servers/sexdisplay/` — no renderer changes
- `crates/sex-pdx/` — no ABI hash change
- `servers/sexinput/` — no input changes
- `servers/sexusb/` — unrelated
- `servers/sexstore/` — persistence path unchanged
- `servers/silkbar/` — unrelated
- `crates/silkbar-model/` — no model changes

---

## Keyboard Shortcut

| Key | Scancode | Action |
|-----|----------|--------|
| **F7** | `0x41` | Toggle Scene Settings panel visible/hidden |

## Surface

| Property | Value |
|----------|-------|
| **Surface ID** | `0x96` (146) |
| **Position** | (870, 60) — right side, below SilkBar |
| **Size** | 340w × 280h (within 1280×720 safe area) |

## Behavior

### Show (F7 when hidden)

1. `pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_SCENE_SETTINGS, ...)` — creates/upserts surface on sexdisplay
2. `SCENE_SETTINGS_ACTIVE = true`
3. `try_transition(InteractionState::PanelActive { panel: PanelKind::Settings })`
4. Marker: `[shell.scene.settings.panel] visible=1`

### Hide (F7 when visible)

1. `pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0)` — destroys surface
2. `SCENE_SETTINGS_ACTIVE = false`
3. `try_transition(InteractionState::Idle)`
4. Marker: `[shell.scene.settings.panel] visible=0`

### Surface lifecycle integration

- `surface_is_alive(0x96)` → reads `SCENE_SETTINGS_ACTIVE`
- `is_closeable_surface(0x96)` → returns `false` (OS-owned, prevents accidental close via DestroyFocused)

---

## Helper: `toggle_scene_settings_panel()`

```rust
unsafe fn toggle_scene_settings_panel() {
    static mut PANEL_BUDGET: u32 = 16;
    // ...
    if !SCENE_SETTINGS_ACTIVE {
        pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_SCENE_SETTINGS,
            (SCENE_SETTINGS_PANEL_Y as u64) << 32 | SCENE_SETTINGS_PANEL_X as u64,
            (SCENE_SETTINGS_PANEL_H as u64) << 32 | SCENE_SETTINGS_PANEL_W as u64);
        SCENE_SETTINGS_ACTIVE = true;
        try_transition(InteractionState::PanelActive { panel: PanelKind::Settings });
    } else {
        pdx_call(SLOT_DISPLAY, 0xEE, SURFACE_ID_SCENE_SETTINGS, 0, 0);
        SCENE_SETTINGS_ACTIVE = false;
        try_transition(InteractionState::Idle);
    }
}
```

Reuses the same `0xEC`/`0xEE` show/hide pattern as the existing
`toggle_os_panel()` helper (used for launcher, status, clock, bell panels).

---

## Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.scene.settings.panel] visible=N` | 16 | Every F7 toggle |
| `[shell.scene.settings.panel.open.start]` | unbudgeted | On show open |
| `[shell.scene.settings.panel.open.ok]` | unbudgeted | On show success |
| `[shell.scene.settings.panel.close.start]` | unbudgeted | On hide close |
| `[shell.scene.settings.panel.close.ok]` | unbudgeted | On hide success |

---

## Build

```
[SEXOS ENTRYPOINT] success
```

Default build passes. No new warning types. No ABI hash update.

---

## Design Decisions

1. **Dedicated toggle function** (not reusing `toggle_os_panel` directly):
   The existing `toggle_os_panel()` takes a `&mut bool` active flag and
   `PanelKind`, but the Settings panel may later need click-zone handling.
   A dedicated function keeps the extension path clean. For now behavior
   is identical to the panel toggle pattern.

2. **Panel geometry** (870, 60, 340, 280): Placed on the right side of
   the screen, below the SilkBar (y=50), within the 1280×720 safe area.
   No overlap with existing panels (launcher at 80×55, status at 860×55,
   clock at 1000×55, bell at 600×55).

3. **No click zones in V1**: The panel is a static colored rectangle.
   Click zones for preset cycling, tint cycling, and top bar toggle are
   deferred to the next phase. This keeps the diff minimal and the STOP
   conditions respected.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **Static panel only** | No clickable controls yet — just a colored rectangle |
| **No text labels** | Shape/color affordances deferred (font pipeline blocked) |
| **No pointer controls** | Click zones not implemented |
| **No F7 synthetic proof** | sexinput proof out of scope; will add in next phase |
| **No settings app** | This is a quick panel, not a full settings application |
| **No persistence changes** | Panel visibility resets on boot; sexstore path unchanged |

---

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Run (interactive, F7 to toggle)
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/scene-settings-panel-static-v1.log

# Check markers
grep -c "\[shell.scene.settings.panel\]" /tmp/scene-settings-panel-static-v1.log

# Zero faults
grep -cE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-settings-panel-static-v1.log
```

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` | Phase 0 design this implements |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState ownership |
| `docs/handoff/FRAME_TOP_BAR_TOGGLE_V1.md` | Existing F4 toggle pattern (model for F7) |
| `docs/handoff/PANEL_TOGGLE_CONSOLIDATION_V1.md` | OS panel toggle pattern |
| `docs/handoff/STABLE_BASELINE_20260503.md` | Surface ID registry, locked invariants |

## Next Recommended Phase

**SCENE_SETTINGS_PANEL_SYNTH_PROOF_V1** — Add synthetic F7 proof in sexinput,
then add click zones to the settings panel (preset cycle, tint cycle, top bar
toggle) via rect-based hit-testing in silk-shell. See
`docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` Section 7 (Phase 0) for details.
