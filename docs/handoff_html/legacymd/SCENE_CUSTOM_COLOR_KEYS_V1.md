# SCENE_CUSTOM_COLOR_KEYS_V1

## Status

Complete (2026-05-04). F6 keyboard cycling of 5 tint bundles over active preset. `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `TINT_COUNT`, `TintBundle`, `CUSTOM_TINT_BUNDLES`, `ACTIVE_TINT_IDX`; added `apply_custom_tint_bundle()`, `cycle_custom_tint()`; added `CycleCustomTint` to `SurfaceAction`; added `0x40` scancode mapping; added dispatch arm; updated `cycle_scene_render_token_preset()` to reset `ACTIVE_TINT_IDX = 0` |

### NOT modified

- `servers/sexdisplay/src/main.rs` — 0xFC handler already clamps any u32
- `crates/sex-pdx/src/lib.rs` — no new opcode
- `sexos_build_spec.toml` — no ABI hash change
- `kernel/` — forbidden

---

## Shortcuts

| Key | Scancode | Action |
|-----|----------|--------|
| F5 | `0x3F` | `CycleRenderTokenPreset` → cycle preset, reset tint to 0 |
| F6 | `0x40` | `CycleCustomTint` → cycle tint over active preset |

---

## Tint Bundle Table

```rust
const TINT_COUNT: usize = 5;
type TintBundle = [u32; 8];

// Slot order: [focus_surface, frame_rim, frame_top_bar, active_tab, inactive_tab,
//              close_light, minimize_light, zoom_light]
static CUSTOM_TINT_BUNDLES: [TintBundle; TINT_COUNT] = [
    // 0: Clear
    [0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 1: WarmTint — amber/copper rim + topbar
    [0x00000000, 0x00D4822A, 0x00B86420, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 2: CoolTint — icy blue rim + topbar
    [0x00000000, 0x0080C8FF, 0x004488CC, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 3: CoralTint — coral focus_surface + pink rim
    [0x00CC5566, 0x00FF8090, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
    // 4: GoldTint — gold rim + active tab
    [0x00000000, 0x00DDBB00, 0x00000000, 0x00DDBB00, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
];

static mut ACTIVE_TINT_IDX: u8 = 0;
```

| Index | Name | Slots overridden | Visual effect |
|-------|------|-----------------|---------------|
| 0 | Clear | none | Reverts to active preset (no override) |
| 1 | WarmTint | rim + topbar | Amber copper rim and bar over any preset |
| 2 | CoolTint | rim + topbar | Icy blue rim and bar over any preset |
| 3 | CoralTint | focus_surface + rim | Coral/pink surface and vivid rim |
| 4 | GoldTint | rim + active_tab | Gold rim and active tab highlight |

Semantic lights (close/minimize/zoom = slots 5/6/7) are zero in all tints → always kept from preset.

---

## State Transitions

```
Boot:   preset=0, tint=0, use_custom_colors=0 → BottleGlass (no tint)

F5:     preset = (preset + 1) % 4
        use_custom_colors = 0
        ACTIVE_TINT_IDX = 0          ← reset tint on preset cycle
        → resolve + push

F6:     ACTIVE_TINT_IDX = (ACTIVE_TINT_IDX + 1) % 5
        if ACTIVE_TINT_IDX == 0:
            use_custom_colors = 0
        else:
            custom_colors = CUSTOM_TINT_BUNDLES[ACTIVE_TINT_IDX]
            use_custom_colors = 1
        → resolve + push
```

Resolve behavior (unchanged): nonzero custom_colors slots override preset; zero slots keep preset value.

---

## Helpers

```rust
unsafe fn apply_custom_tint_bundle(idx: usize) {
    if idx == 0 {
        SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    } else {
        let bundle = &CUSTOM_TINT_BUNDLES[idx];
        for i in 0..8 {
            SCENE_APPEARANCE_STATE.custom_colors[i] = bundle[i];
        }
        SCENE_APPEARANCE_STATE.use_custom_colors = 1;
    }
}

unsafe fn cycle_custom_tint() {
    ACTIVE_TINT_IDX = (ACTIVE_TINT_IDX + 1) % TINT_COUNT as u8;
    apply_custom_tint_bundle(ACTIVE_TINT_IDX as usize);
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    // [shell.appearance.custom] mode=tint tint=N (budget 32)
}
```

---

## Markers

| Marker | When | Budget |
|--------|------|--------|
| `[shell.appearance.tokens.send] seq=2 sent` | Boot | 4 |
| `[shell.appearance.state] preset=N custom=N chrome=N access=N` | Boot | 1 |
| `[shell.appearance.preset] idx=N` | F5 pressed | 16 |
| `[shell.appearance.custom] mode=tint tint=N` | F6 pressed | 32 |
| `[sexdisplay.appearance.tokens] seq=1 applied=N` | Display committed | 4 |

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

No ABI hash update required (sex-pdx unchanged).

### Verification (if run)

```bash
grep -ac "\[shell.appearance.custom\]" /tmp/scene-custom-color-v1.log
grep -ac "\[shell.appearance.preset\]" /tmp/scene-custom-color-v1.log
grep -ac "\[sexdisplay.appearance.tokens\].*applied" /tmp/scene-custom-color-v1.log
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-custom-color-v1.log
```

---

## Limitations

- **No persistence** — `ACTIVE_TINT_IDX` resets to 0 (Clear) on reboot
- **No per-channel editing** — tints are fixed bundles; individual color adjustment deferred to settings app
- **Forward only** — F6 cycles 0→1→2→3→4→0; no reverse
- **5 tints only** — table is fixed at compile time

---

## Next Recommended Phase

Settings app design (`SCENE_SETTINGS_APP_PLAN_V1`) or sexstore K/V API (`SEXSTORE_KV_API_PLAN_V1`) for persistent scene appearance.

Prerequisite for persistence: sexstore must gain a K/V read/write PDX API (see `SCENE_SETTINGS_STORAGE_PLAN_V1.md` Phase 2).
