# SILK_CHROME_CANON_SPEC_V1

## Verdict: PASS REVIEW ONLY — Docs-only canon specification. No source changes.

## 1. Canonical Terms

| Term | Definition |
|------|-----------|
| **Silk** | The visual design system: crystalline glass, neon rims, dark blue-violet depth. Owns look, not policy. |
| **Scene** | A workspace container holding 1-N Frames. Atlas manages scene navigation. |
| **Atlas** | The scene overview/map. Shows all scenes as thumbnail cards. Keyboard-navigable. |
| **Frame** | A window container holding 1-N Tabs. Has Chrome, Lights, Rims. |
| **Tab** | A document/app surface inside a Frame. Has hover-revealed controls. |
| **Frame Chrome** | The structural border of a Frame: title bar, control strip, glass edge. |
| **Frame Lights** | Status/action indicators on Frame Chrome: close, minimize, zoom, focus glow. |
| **Rim** | The 2-4px neon edge around a Frame. Thin, sharp, crystalline bevel. |
| **Hover Tab** | Tab controls revealed on hover: close, drag, info. Deferred until pointer stability. |
| **Focus Glow** | Subtle neon accent on focused Frame rim. Color shifts per active app. |
| **Minimized Card** | Collapsed Frame representation in taskbar/dock area. Title + icon. |

## 2. Ownership Table

| Component | Owner | Role |
|-----------|-------|------|
| Scene/Frame/Tab/focus | silk-shell | Window/session/input policy |
| Bounded rendering | sexdisplay | Pixels only, no policy |
| Global strip/status | SilkBar | Clock, active app, palette state |
| Files/objects/projects | Linen | Object browser, not shell policy |
| Notifications | Bell | Event stream (future) |
| Authority/trust grants | Collar | Capability graph (future) |
| Capability visualization | Mesh | Graph render (future) |

## 3. Visual Canon

### Core Principles
- **Crystalline glass**: dark blue-violet (#1E1E2E base, #302E56 deep, #454075 highlight)
- **Thin neon edges**: 2-4px rims, #89B4FA (blue) / #A6E3A1 (green) / #F9E2AF (yellow)
- **Sharp cut-glass bevels**: small radius (2-4px), not bubbly
- **Hover-revealed controls**: hidden until pointer nearby (deferred to Phase 6)
- **Tabbed tiled glass**: Frames tile inside Scene, Tabs tile inside Frame
- **Workspace Atlas**: overview map of all Scenes

### Catppuccin Mocha Palette (baseline)
| Token | Hex | Use |
|-------|-----|-----|
| Base | #1E1E2E | Glass background |
| Surface0 | #313244 | Frame fill |
| Surface1 | #45475A | Tab strip |
| Blue | #89B4FA | Focus rim, accent |
| Green | #A6E3A1 | Success/ready indicator |
| Yellow | #F9E2AF | Warning/attention |
| Lavender | #B4BEFE | SilkBar accent |
| Text | #CDD6F4 | Primary text |
| Subtext | #A6ADC8 | Secondary text |

## 4. V1 Safe Implementation Ladder

| Phase | Name | Type | Requires | Deliverable |
|-------|------|------|----------|-------------|
| 0 | Docs/Spec | docs | Nothing | This document |
| 1 | Color-only glass | source | Runtime smoke PASS | Frame glass fill + rim rects in Catppuccin colors |
| 2 | Frame/rim state markers | source | Phase 1 | `[silk.chrome.canon]` + `[silk.frame.rim]` markers |
| 3 | Noninteractive visual rim proof | source | Phase 2 | Static rim render proven via QEMU markers |
| 4 | Keyboard frame workflow | source | Phase 3 | Focus/minimize/restore/zoom bound to keyboard |
| 5 | Frame Lights status/help | source | Phase 4 | Close/minimize/zoom lights + `[silk.frame.lights]` markers |
| 6 | Hover reveal | source | Pointer stability proven | Tab controls on hover (deferred until pointer) |
| 7 | Atlas scene view | source | Scene model proven | `[silk.atlas.scene]` scene overview markers |

## 5. Forbidden Now

- ❌ Alpha/translucency (ARGB alpha channel)
- ❌ Blur/shadows (Gaussian, box, drop-shadow)
- ❌ Full-frame effect passes (gradient overlays, glassmorphism shaders)
- ❌ Renderer-owned policy (sexdisplay renders, silk-shell decides)
- ❌ Unsafe close of non-disposable surfaces
- ❌ Pointer/hover dependency (pointer stability not yet proven)
- ❌ Shared framebuffer/backing-buffer redesign
- ❌ Broad renderer/architectural refactor

## 6. Future Proof Marker Names

```
[silk.chrome.canon] phase=N name=NAME ok=N reason=...
[silk.chrome.phase] phase=N status=NAME
[silk.frame.model] frame=N scene=N tabs=N
[silk.frame.rim] frame=N color=HEX width=N
[silk.frame.lights] frame=N close=N minimize=N zoom=N
[silk.atlas.scene] scene=N name=NAME frames=N
```

## 7. Recommended Next 8 Prompts

1. **SILK_CHROME_PHASE1_COLOR_GLASS_V1** — First source: frame glass fill + rim rects
2. **SILK_CHROME_PHASE2_RIM_MARKERS_V1** — Canon + rim state markers
3. **SILK_CHROME_PHASE3_RIM_PROOF_V1** — Noninteractive rim render proof
4. **SILK_CHROME_PHASE4_WINDOW_KEYS_V1** — Keyboard frame workflow integration
5. **SILK_CHROME_PHASE5_FRAME_LIGHTS_V1** — Frame Lights (close/minimize/zoom)
6. **SILK_CHROME_PHASE6_ATLAS_SPEC_V1** — Atlas scene overview spec (docs first)
7. **RUNTIME_SMOKE_75_POST_CHROME_V1** — Smoke test after Phase 1-5
8. **SILK_CHROME_PHASE7_ATLAS_VIEW_V1** — Atlas scene overview implementation

## 8. Safety
- Docs-only: no source, kernel, pdx, ABI, USB, input, pointer changes.
- Single file: `docs/handoff/SILK_CHROME_CANON_SPEC_V1.md`
