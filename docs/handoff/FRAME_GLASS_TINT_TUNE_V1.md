# FRAME_GLASS_TINT_TUNE_V1

## Status

Implemented (2026-05-04). Tuned focused window/frame colors toward bottle-glass / blue-glass with greener teal bias. Visual-only, no architecture changes.

---

## New Color Palette

### Old → New constants

| Constant | Old (hex) | Old (RGB) | New (hex) | New (RGB) |
|----------|-----------|-----------|-----------|-----------|
| `FOCUS_SURFACE_COLOR` | `0x00A8E0FF` | R=168 G=224 B=255 | `0x007AAFA4` | R=122 G=175 B=164 |
| `FRAME_RIM_COLOR` | `0x00C0F0FF` | R=192 G=240 B=255 | `0x00B8F2E8` | R=184 G=242 B=232 |
| `FRAME_TOP_BAR_COLOR` | `0x00C0F0FF` (aliased to rim) | R=192 G=240 B=255 | `0x0088C2B7` | R=136 G=194 B=183 |

### Unchanged constants

| Constant | Value | Role |
|----------|-------|------|
| `TAB_ACTIVE_COLOR` | `FOCUS_SURFACE_COLOR` (now `0x007AAFA4`) | Active tab block (cascaded from focus color) |
| `TAB_INACTIVE_COLOR` | `0x006080B0` | Dimmed inactive tab |
| `FRAME_LIGHT_CLOSE_COLOR` | `0x00FF4444` | Red close light |
| `FRAME_LIGHT_MINIMIZE_COLOR` | `0x00FFCC44` | Yellow minimize light |
| `FRAME_LIGHT_ZOOM_COLOR` | `0x0044FF44` | Green zoom light |

### Visual impact

| Element | Before | After |
|---------|--------|-------|
| Focused window body | Bright blue-cyan (`0xA8E0FF`) | Teal bottle-glass (`0x7AAFA4`) |
| Neon rim (left/right/bottom) | Bright cyan (`0xC0F0FF`) | Teal-tinted cyan (`0xB8F2E8`) |
| Top bar background | Bright cyan (`0xC0F0FF`) | Medium teal (`0x88C2B7`) |
| Active tab block | Bright blue-cyan (`0xA8E0FF`) | Teal (`0x7AAFA4`) |
| Inactive tab block | Dimmed cyan (`0x6080B0`) | Unchanged |

---

## Rationale

The shift from bright blue-cyan (`0xA8E0FF` / `0xC0F0FF`) to teal-tinted greens (`0x7AAFA4` / `0xB8F2E8` / `0x88C2B7`) achieves:

1. **Bottle-glass appearance** — The greener teal base (`G=175` vs `B=164`) mimics the green-blue tint of thick glass panels
2. **Reduced eye strain** — Lower brightness and reduced blue dominance is easier on the eyes
3. **Futuristic aesthetic** — Teal/cyan palette evokes liquid-crystal and holographic displays
4. **Chrome hierarchy** — Three distinct layers: brightest rim (edge glow), medium top bar (glass pane), slightly darker body (content behind glass)

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Default build passes. Synthetic build passes. No new warning types.

---

## Verification

```bash
git diff servers/sexdisplay/src/main.rs | grep -E "^[+-].*= 0x"
```

Expected output:
```
-const FOCUS_SURFACE_COLOR: u32 = 0x00A8E0FF;
+const FOCUS_SURFACE_COLOR: u32 = 0x007AAFA4;
-const FRAME_RIM_COLOR: u32 = 0x00C0F0FF;
+const FRAME_RIM_COLOR: u32 = 0x00B8F2E8;
+const FRAME_TOP_BAR_COLOR: u32 = 0x0088C2B7;
```

Visual: focused window body and chrome render in teal bottle-glass palette.

---

## Limitations

- **Tint-only, not true glass:** This is a color shift, not a translucency/blur effect. The window does not show content behind it.
- **No opacity/alpha blending:** All colors are fully opaque. Future liquid-glass would require alpha compositing.
- **Tab active color contrast:** The active tab color (`0x007AAFA4`, cascaded from `FOCUS_SURFACE_COLOR`) is close to the top bar background (`0x0088C2B7`). The 14-point RGB difference provides sufficient contrast for visibility. If contrast proves insufficient in practice, a future tweak can lighten `TAB_ACTIVE_COLOR` independently.

---

## Next Phase

### FRAME_TOP_BAR_TOGGLE_PLAN_V1

Continue the planned roadmap: design a mechanism to toggle between default (top bar) and minimal (4px rim) chrome mode.
