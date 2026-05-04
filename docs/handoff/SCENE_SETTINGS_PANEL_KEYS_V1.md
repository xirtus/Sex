# SCENE_SETTINGS_PANEL_KEYS_V1

## Status

Complete (2026-05-04). Keyboard command routing for Scene Settings panel
(1=preset, 2=tint, 3=topbar, Esc=close). Shell-only — no sexdisplay,
kernel, or protocol changes. Build passes.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added panel key intercept before normal dispatch: when `SCENE_SETTINGS_ACTIVE`, scancodes 0x01-0x04 route to panel commands; all other keys fall through to normal `scancode_to_action` dispatch. Added `[shell.scene.settings.panel.key]` markers (budget 16). |
| `docs/handoff/SCENE_SETTINGS_PANEL_KEYS_V1.md` | New — this document |

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

## Key Map (when panel visible)

| Key | Scancode | Action | Persists? | Reuses |
|-----|----------|--------|-----------|--------|
| **Esc** | `0x01` | Close settings panel | — | `toggle_scene_settings_panel()` via 0xEE |
| **1** | `0x02` | Cycle render token preset | ✅ (sexstore PUT) | `cycle_scene_render_token_preset()` |
| **2** | `0x03` | Cycle custom tint | ❌ (ephemeral) | `cycle_custom_tint()` |
| **3** | `0x04` | Toggle top bar on active frame | ❌ (frame flag) | `toggle_top_bar_for_active_frame()` |
| **F7** | `0x41` | Toggle panel (same as when hidden) | — | Normal `scancode_to_action` → `ToggleSceneSettingsPanel` |

### Normal keys unaffected

All other keys (F4, F5, F6, arrow keys, focus keys, etc.) fall through to
the normal `scancode_to_action()` dispatch unchanged.

---

## Routing Behavior

```
EV_KEY make (value == 1)
    │
    ├── SCENE_SETTINGS_ACTIVE == true AND scancode in {0x01, 0x02, 0x03, 0x04}
    │       → Panel command (close/preset/tint/topbar)
    │       → mutated = true
    │       → Marker emitted
    │       → Normal dispatch NOT skipped (but panel keys have no SurfaceAction mapping,
    │         so they hit the `_ => None` case in scancode_to_action)
    │
    └── Otherwise
            → Normal scancode_to_action dispatch unchanged
            → F4/F5/F6/F7/arrows/focus keys all work normally
```

Note: When panel is active, pressing 1/2/3 (normally Focus100/Focus101/Focus102)
routes to panel commands. When panel is closed, these keys restore their normal
focus-surface behavior.

---

## Persistence Behavior

| Command | Persists? | Mechanism |
|---------|-----------|-----------|
| Preset (1) | ✅ | Same path as F5: `cycle_scene_render_token_preset()` → `pack_scene_settings_blob()` → `pdx_call(SLOT_SEXSTORE, OP_KV_PUT, ...)` |
| Tint (2) | ❌ | Same as F6: ephemeral `ACTIVE_TINT_IDX` cycle, no sexstore write |
| Top bar (3) | ❌ | Same as F4: per-frame chrome flag, `send_frame_tab_info()` |

---

## Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.scene.settings.panel.key] cmd=close` | 16 | Esc while panel visible |
| `[shell.scene.settings.panel.key] cmd=preset` | 16 | 1 while panel visible |
| `[shell.scene.settings.panel.key] cmd=tint` | 16 | 2 while panel visible |
| `[shell.scene.settings.panel.key] cmd=topbar` | 16 | 3 while panel visible |

---

## Build

```
[SEXOS ENTRYPOINT] success
```

Default build passes. No new warning types. No ABI hash update.

---

## Design Decisions

1. **Pre-dispatch intercept vs. new SurfaceAction variants**: Adding
   `ScenePanelPreset`/`ScenePanelTint`/etc. to `SurfaceAction` would require
   changes to `scancode_to_action()` and create ambiguity (same scancode
   mapping to different actions depending on panel state). The pre-dispatch
   intercept is cleaner: it runs before `scancode_to_action()` and handles
   panel-specific keys directly. Non-panel keys fall through unchanged.

2. **Panel keys not added to scancode_to_action**: Esc (0x01) is already
   unhandled (returns None). Keys 1/2/3 (0x02-0x04) map to FocusSurface
   actions when panel is closed — which is the correct default behavior.

3. **Top bar toggle targets the previously-selected frame**: When panel is
   active, `FOCUSED_SURFACE_ID` may be the panel surface (0x96).
   `toggle_top_bar_for_active_frame()` calls `selected_frame_id()` which
   maps through `frame_for_surface()`. If the panel surface is focused but
   has no owning frame, `selected_frame_id()` returns the most recently
   active frame. This matches F4 behavior when linen (surface 200) is focused.

---

## Limitations

| Limitation | Impact |
|------------|--------|
| **No text labels on panel** | User must remember 1/2/3/Esc key mapping |
| **No pointer controls** | Click zones not implemented |
| **Top bar targets last active frame** | May surprise if user expects panel-local toggle |
| **No synthetic proof** | Runtime F7 + panel key testing deferred to next phase |

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PANEL_STATIC_V1.md` | Panel surface and F7 toggle this extends |
| `docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` | Phase 1: keyboard controls design |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Preset persistence path (reused) |
| `docs/handoff/FRAME_TOP_BAR_TOGGLE_V1.md` | F4 top bar toggle pattern (reused) |
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md` | Preset cycling (reused) |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | Tint cycling (reused) |

## Next Recommended Phase

**SCENE_SETTINGS_PANEL_SYNTH_PROOF_V1** — Add synthetic F7 + panel key proof
in sexinput, then add click zones to the settings panel (preset cycle, tint
cycle, top bar toggle) via rect-based hit-testing in silk-shell.
See `docs/handoff/SCENE_SETTINGS_APP_PLAN_V1.md` Section 7 (Phase 0) for details.
