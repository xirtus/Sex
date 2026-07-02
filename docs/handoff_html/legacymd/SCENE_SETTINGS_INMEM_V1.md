# SCENE_SETTINGS_INMEM_V1

## Status

Complete (2026-05-04). In-memory `SceneAppearanceState` struct replaces raw `ACTIVE_PRESET_IDX`. Build passes: `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Replaced `ACTIVE_PRESET_IDX: u8` with `SceneAppearanceState` struct + `DEFAULT_SCENE_APPEARANCE` const + `SCENE_APPEARANCE_STATE` static; added `resolve_scene_render_tokens()`; updated `send_scene_render_tokens()` and `cycle_scene_render_token_preset()` to resolve through state |

### NOT modified

- `servers/sexdisplay/src/main.rs` — 0xFC handler unchanged
- `crates/sex-pdx/src/lib.rs` — no change
- `sexos_build_spec.toml` — no ABI hash change
- `kernel/` — forbidden

---

## State Struct Fields / Defaults

```rust
#[derive(Clone, Copy)]
struct SceneAppearanceState {
    preset_idx: u8,           // 0 = BottleGlass (default)
    use_custom_colors: u8,    // 0 = use preset; nonzero = substitute custom_colors
    chrome_flags: u8,         // 0 (all reserved in V1)
    accessibility_flags: u8,  // 0 (bit 0=high_contrast, bit 1=colorblind_safe, ...)
    custom_colors: [u32; 8],  // [0; 8] — override slots; only nonzero values substitute
}

static mut SCENE_APPEARANCE_STATE: SceneAppearanceState = DEFAULT_SCENE_APPEARANCE;
// DEFAULT_SCENE_APPEARANCE: preset_idx=0, all flags=0, custom_colors=[0;8]
```

Size: 4 × u8 + 8 × u32 = 36 bytes. No heap. `static mut`.

---

## Resolve Behavior

`resolve_scene_render_tokens() -> TokenPreset`:

1. `idx = SCENE_APPEARANCE_STATE.preset_idx % PRESET_COUNT`
2. `base = TOKEN_PRESETS[idx]`
3. If `use_custom_colors == 0` → return `base` directly
4. Otherwise: copy `base`, then for each of 8 slots, if `custom_colors[i] != 0`, substitute `base[i]` with `custom_colors[i]`
5. Return result (stack-allocated `[u32; 8]`, no heap)

Zero custom color slots are treated as "no override" — the preset value is preserved. This allows partial custom overrides (e.g. only rim color changed) without storing the entire preset redundantly.

---

## Preset Cycling Behavior

F5 → `cycle_scene_render_token_preset()`:

1. `SCENE_APPEARANCE_STATE.preset_idx = (idx + 1) % 4`
2. `SCENE_APPEARANCE_STATE.use_custom_colors = 0` — clears custom override (reverts to preset)
3. Calls `resolve_scene_render_tokens()` → `push_token_preset()`
4. Emits `[shell.appearance.preset] idx=N` (budget 16)

Cycle always clears custom overrides. Future custom color editing (from a settings app) sets `use_custom_colors = 1` and fills `custom_colors`; F5 then resets to the next clean preset.

---

## Markers

| Marker | When | Budget |
|--------|------|--------|
| `[shell.appearance.tokens.send] seq=2 sent` | Boot token send | 4 |
| `[shell.appearance.state] preset=N custom=N chrome=N access=N` | Boot state dump | 1 |
| `[shell.appearance.preset] idx=N` | F5 pressed | 16 |
| `[sexdisplay.appearance.tokens] seq=0 buffered` | Display received Call 1 | 4 |
| `[sexdisplay.appearance.tokens] seq=1 applied=N` | Display committed tokens | 4 |

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

No ABI hash update required (sex-pdx unchanged).

### Verification (if run)

```bash
grep -ac "\[shell.appearance.preset\]" /tmp/scene-settings-inmem-v1.log
grep -ac "\[shell.appearance.tokens.send\]" /tmp/scene-settings-inmem-v1.log
grep -ac "\[sexdisplay.appearance.tokens\].*applied=1" /tmp/scene-settings-inmem-v1.log
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-settings-inmem-v1.log
```

---

## Limitations

- **No persistence** — `SCENE_APPEARANCE_STATE` resets to `DEFAULT_SCENE_APPEARANCE` (BottleGlass) on reboot
- **No custom color input** — `use_custom_colors` and `custom_colors` are initialized to zero; only settable by future code
- **No settings app** — only F5 preset cycling and boot defaults
- **No real alpha/blur** — `effect_levels` zero; no alpha blending pipeline
- **chrome_flags unused** — all bits reserved; top bar still controlled via 0xFD per-frame

---

## Next Recommended Phase: SCENE_CUSTOM_COLOR_KEYS_PLAN_V1

Design keyboard shortcuts for adjusting individual token colors at runtime (e.g. Shift+F5 = cycle rim color through a small palette; Shift+F6 = cycle accent color). This exercises the `custom_colors` and `use_custom_colors` fields added in this phase without requiring a full settings app.

Prerequisite: none — `SceneAppearanceState.custom_colors` already exists and `resolve_scene_render_tokens()` already handles it.
