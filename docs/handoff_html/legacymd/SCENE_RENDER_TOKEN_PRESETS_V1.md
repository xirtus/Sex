# SCENE_RENDER_TOKEN_PRESETS_V1

## Status

Complete (2026-05-04). F5 keyboard cycling of 4 built-in RenderToken presets. `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `TOKEN_PRESETS` table, `ACTIVE_PRESET_IDX`, `push_token_preset()`, refactored `send_scene_render_tokens()`, added `cycle_scene_render_token_preset()`, added `CycleRenderTokenPreset` to `SurfaceAction`, added F5 scancode mapping, added dispatch arm |

### NOT modified

- `servers/sexdisplay/src/main.rs` — 0xFC handler already accepts any valid preset
- `crates/sex-pdx/src/lib.rs` — `OP_APPEARANCE_TOKENS = 0xFC` unchanged
- `sexos_build_spec.toml` — no ABI hash change (no sex-pdx edit)
- `kernel/` — forbidden

---

## Shortcut

| Key | Scancode | Action |
|-----|----------|--------|
| F5 | `0x3F` | `CycleRenderTokenPreset` → `cycle_scene_render_token_preset()` |

No conflict with existing bindings (F4 = `0x3E` = `ToggleTopBar`).

---

## Preset List

| Index | Name | Colors |
|-------|------|--------|
| 0 | BottleGlass | Default teal — matches `DEFAULT_RENDER_TOKENS` in sexdisplay exactly |
| 1 | VioletGlass | Deep violet/purple Silk canon |
| 2 | GraphiteGlass | Dark neutral graphite |
| 3 | HighContrast | Black/white/yellow — accessibility proof |

### Exact token values (`servers/silk-shell/src/main.rs`)

```rust
// Fields: [focus_surface, frame_rim, frame_top_bar, active_tab,
//          inactive_tab, close_light, minimize_light, zoom_light]
static TOKEN_PRESETS: [TokenPreset; PRESET_COUNT] = [
    // 0: BottleGlass
    [0x007AAFA4, 0x00B8F2E8, 0x0088C2B7, 0x007AAFA4, 0x006080B0, 0x00FF4444, 0x00FFCC44, 0x0044FF44],
    // 1: VioletGlass
    [0x00503080, 0x00A060FF, 0x00604090, 0x00503080, 0x00302050, 0x00FF4080, 0x00FFAA00, 0x0040FF80],
    // 2: GraphiteGlass
    [0x00282828, 0x00808080, 0x00404040, 0x00505050, 0x00303030, 0x00CC4444, 0x00CCAA44, 0x0044CC44],
    // 3: HighContrast
    [0x00000000, 0x00FFFFFF, 0x00111111, 0x00FFFF00, 0x00555555, 0x00FF4444, 0x00FFDD00, 0x0000FF44],
];
```

Semantic light colors (close/minimize/zoom) preserved across all presets. HighContrast `focus_surface = 0x00000000` → clamped to `0xFF000000` (opaque black) by sexdisplay on receive; renderer ignores alpha byte.

---

## Helper / Action Names

| Symbol | Kind | Location |
|--------|------|----------|
| `PRESET_COUNT: usize` | const | silk-shell |
| `TokenPreset` | type alias `[u32; 8]` | silk-shell |
| `TOKEN_PRESETS` | static (read-only) | silk-shell |
| `ACTIVE_PRESET_IDX: u8` | static mut | silk-shell |
| `push_token_preset(p: &TokenPreset)` | unsafe fn | silk-shell |
| `send_scene_render_tokens()` | unsafe fn (boot only) | silk-shell — now calls `push_token_preset(&TOKEN_PRESETS[0])` |
| `cycle_scene_render_token_preset()` | unsafe fn | silk-shell |
| `SurfaceAction::CycleRenderTokenPreset` | enum variant | silk-shell |

---

## Markers

| Marker | When |
|--------|------|
| `[shell.appearance.tokens.send] seq=2 sent` | Boot — default (Preset 0) pushed |
| `[shell.appearance.preset] idx=N` | F5 pressed — Preset N now active (budget 16) |
| `[sexdisplay.appearance.tokens] seq=0 buffered` | Display received Call 1 |
| `[sexdisplay.appearance.tokens] seq=1 applied=N` | Display committed tokens, redrew |

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

All ABI/pipeline guards pass. No ABI hash update required (sex-pdx unchanged).

### Verification commands (if run)

```bash
grep -ac "\[shell.appearance.preset\]" /tmp/scene-render-token-presets-v1.log
grep -ac "\[shell.appearance.tokens.send\]" /tmp/scene-render-token-presets-v1.log
grep -ac "\[sexdisplay.appearance.tokens\].*applied=1" /tmp/scene-render-token-presets-v1.log
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-render-token-presets-v1.log
```

---

## Limitations

- **No persistence** — `ACTIVE_PRESET_IDX` resets to 0 (BottleGlass) on reboot
- **No settings app** — presets only reachable via F5
- **No real alpha/blur** — flat ARGB only; `effect_levels` zero in all presets
- **Forward only** — F5 cycles 0→1→2→3→0; no reverse direction
- **No per-surface presets** — global `DISPLAY_TOKENS` applies to all surfaces

---

## Next Recommended Phase: SCENE_SETTINGS_STORAGE_PLAN_V1

Design a minimal settings storage model:
- Small fixed-size settings block (active preset index + future fields)
- Persist via sexstore or similar service
- Load on boot, restore last preset
- Optionally expose to a future settings app

Prerequisite: sexstore or equivalent persistent K/V service must exist and be accessible via PDX.
