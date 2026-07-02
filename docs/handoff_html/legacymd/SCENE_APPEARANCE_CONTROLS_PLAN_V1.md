# SCENE_APPEARANCE_CONTROLS_PLAN_V1

## Status

Design (2026-05-04). Appearance control taxonomy and architecture for Silk Scene/Theme settings. Docs-only — no code changed.

---

## Verdict: APPEARANCE_MODEL_SAFE_TO_DESIGN ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Define appearance control taxonomy | ✅ | This document |
| Split user-facing intent from renderer-safe tokens | ✅ | Intent → Token → Pixel pipeline |
| Flat-color-only near-term path | ✅ | Constants → token substitution |
| Future true glass path designed | ✅ | Identifies renderer prerequisites |
| No sexdisplay changes in this phase | ✅ | Docs-only |
| No kernel/ABI changes | ✅ | Docs-only |
| Next phase named | ✅ | SCENE_RENDER_TOKENS_PLAN_V1 |

---

## Current State Audit

### Existing color constants (hardcoded, sexdisplay)

| Constant | Current Value | Used For |
|----------|--------------|----------|
| `FOCUS_SURFACE_COLOR` | `0x007AAFA4` | Focused window body fill |
| `FRAME_RIM_COLOR` | `0x00B8F2E8` | Neon rim (left/right/bottom); top rim in minimal mode |
| `FRAME_TOP_BAR_COLOR` | `0x0088C2B7` | Top bar background (default mode) |
| `TAB_ACTIVE_COLOR` | `FOCUS_SURFACE_COLOR` (cascaded) | Active tab block |
| `TAB_INACTIVE_COLOR` | `0x006080B0` | Inactive tab block |
| `FRAME_LIGHT_CLOSE_COLOR` | `0x00FF4444` | Red close light |
| `FRAME_LIGHT_MINIMIZE_COLOR` | `0x00FFCC44` | Yellow minimize light |
| `FRAME_LIGHT_ZOOM_COLOR` | `0x0044FF44` | Green zoom light |
| SilkBar panel | hardcoded: `0x00203058` / `0x00405880` | Launcher panel |
| SilkBar desktop | hardcoded: 4-band gradient `0x00081424` → `0x00281848` | Desktop backdrop |
| SilkBar chips | hardcoded: various `0x0038D6C8` / `0x00FFB84D` | Wifi/battery semantic colors |
| SilkBar selected option dots | hardcoded: red/green/yellow/cyan | CLOSE/ZOOM/MINIMIZE/MOVE dots |
| Cursor arrow | `0x00FFFFFF` | White cursor |

### Existing chrome mode communication

- `chrome_flags` bit in `0xFD` arg2 (bit 8 = top bar enabled)
- Shell → Display via `send_frame_tab_info()`
- No other appearance metadata communicated

### What does NOT exist (all gaps)

- No scene/theme storage model
- No per-scene settings
- No per-monitor settings
- No settings IPC protocol
- No settings app
- No render token model
- No alpha/blur pipeline
- No user-facing appearance controls
- No wallpaper/scene server
- No preference persistence

---

## Control Taxonomy

### 1. Color Controls

| Control | Type | Default (current) | Range | Near-Term | Future |
|---------|------|-------------------|-------|-----------|--------|
| accent_color | u32 ARGB | `0x007AAFA4` (teal) | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Full palette |
| glass_tint_color | u32 ARGB | `0x007AAFA4` | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Alpha-blended tint |
| topbar_color | u32 ARGB | `0x0088C2B7` | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Glass-frosted |
| rim_color | u32 ARGB | `0x00B8F2E8` | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Glow-blended |
| active_tab_color | u32 ARGB | accent_color (cascaded) | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Gradient/frost |
| inactive_tab_color | u32 ARGB | `0x006080B0` | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Dimmed frost |
| urgent_color | u32 ARGB | `0x00FF4444` (red) | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Animated glow |
| close_light_color | u32 ARGB | `0x00FF4444` | semantic red family | ✅ Flat constant swap | ✅ Semantic token |
| minimize_light_color | u32 ARGB | `0x00FFCC44` | semantic amber family | ✅ Flat constant swap | ✅ Semantic token |
| zoom_light_color | u32 ARGB | `0x0044FF44` | semantic green family | ✅ Flat constant swap | ✅ Semantic token |
| focus_indicator_color | u32 ARGB | rim_color | 0x00000000..0x00FFFFFF | ✅ Flat constant swap | ✅ Pulse animation |

**Near-term (flat ARGB only):**
All color controls are simple `u32` ARGB constants. Replacement is a constant swap in the render token table. No alpha compositing, no blending — fully opaque only.

**Future (true glass):**
Colors become base tints applied with alpha/blur by an effect engine. Each color has a `strength` parameter that controls how much of the effect is applied.

### 2. Glass/Effects Controls

| Control | Type | Default | Range | Near-Term | Future |
|---------|------|---------|-------|-----------|--------|
| transparency_enabled | bool | false | true/false | ❌ Forbidden | ✅ |
| opacity_level | u8 | 255 | 0..255 | ❌ Forbidden (flat only) | ✅ |
| blur_level | u8 | 0 | 0..10 | ❌ Forbidden | ✅ |
| frost_strength | u8 | 0 | 0..10 | ❌ Forbidden | ✅ |
| glow_intensity | u8 | 128 | 0..255 | ✅ Alpha-multiply rim | ✅ Full glow pass |
| edge_brightness | u8 | 200 | 0..255 | ✅ Rim color brightness | ✅ Rim effect pass |
| bevel_strength | u8 | 0 | 0..255 | ❌ Forbidden | ✅ |

**Safety rule:** Any control requiring alpha compositing against live scene content (transparency, blur, frost) is FORBIDDEN until the effect engine safety plan is complete. Only flat-ARGB-safe controls (glow_intensity, edge_brightness) may be implemented early, and only as tint adjustments (not alpha).

### 3. Chrome/Layout Controls

| Control | Type | Default | Range | Near-Term | Future |
|---------|------|---------|-------|-----------|--------|
| top_bar_enabled | bool | true | true/false | ✅ Implemented (toggle) | ✅ Per-scene override |
| minimal_rim_mode | bool | false | true/false | ✅ Same as !top_bar_enabled | ✅ Independent control |
| tab_strip_mode | enum | `always` | always / hover / hidden | ✅ Toggleable (static) | ✅ Hover-reveal |
| frame_lights_mode | enum | `always` | always / hover / hidden | ✅ Toggleable (static) | ✅ Hover-reveal |
| corner_radius | u8 | 0 | 0..8 | ❌ Requires renderer change | ✅ |
| rim_thickness | u8 | 4 | 1..8 | ✅ Constant swap | ✅ Per-scene |
| larger_hit_targets | bool | false | true/false | ✅ Constant swap (V2) | ✅ Accessibility |

**Near-term:** `top_bar_enabled` is already live via F4 toggle. `minimal_rim_mode` is the same flag. `tab_strip_mode` and `frame_lights_mode` are model-only — the hover-reveal behavior is not implemented. `rim_thickness` is a trivial constant change. `larger_hit_targets` would adjust `FRAME_LIGHT_GAP_PX` / `FRAME_LIGHT_SIZE_PX` — safe but V2.

### 4. Motion Controls

| Control | Type | Default | Range | Near-Term | Future |
|---------|------|---------|-------|-----------|--------|
| animations_enabled | bool | false | true/false | ❌ No animation system | ✅ |
| animation_speed | u8 | 100 | 0..200 (% of normal) | ❌ No animation system | ✅ |
| hover_reveal_delay | u16 | 0 | 0..1000 (ms) | ❌ No hover timing | ✅ |
| reduce_motion | bool | false | true/false | ❌ No motion to reduce | ✅ |

**All motion controls are deferred** until the system has a reliable scheduler cadence, yield timing, and animation loop. V1 has none of these.

### 5. Accessibility Controls

| Control | Type | Default | Range | Near-Term | Future |
|---------|------|---------|-------|-----------|--------|
| high_contrast | bool | false | true/false | ✅ Predefined high-contrast token set | ✅ Full palette |
| reduce_transparency | bool | true | true/false | ✅ Always true in V1 (flat only) | ✅ Disables glass effects |
| colorblind_safe | bool | false | true/false | ✅ Semantic token set (patterns+text) | ✅ Full support |
| stronger_focus_ring | bool | false | true/false | ✅ Double rim thickness | ✅ Dedicated ring |
| larger_targets | bool | false | true/false | ✅ Adjusted hit constants | ✅ Dedicated targets |

**Near-term:** `high_contrast` and `colorblind_safe` can be pre-defined alternative token sets (different hex values). `reduce_transparency` is implicitly always-on in V1. `stronger_focus_ring` is a rim thickness multiplier.

---

## Architecture: Three-Layer Model

```
┌──────────────────────────────────────────────────────────────────┐
│                    L1: APPEARANCE INTENT                         │
│  User-facing settings: accent="teal", glass="medium", etc       │
│  Owned by: future Settings app / Scene server                    │
│  Storage: Scene struct in silk-shell                             │
├──────────────────────────────────────────────────────────────────┤
│                    L2: RENDER TOKENS                              │
│  Clamped renderer-safe values: 0x007AAFA4, blur=3, etc          │
│  Owned by: silk-shell (token table)                              │
│  Validated: range-checked, no alpha-before-safety                │
├──────────────────────────────────────────────────────────────────┤
│                    L3: PIXEL EFFECT                               │
│  Actual pixel output: composite_pixel() passes                   │
│  Owned by: sexdisplay                                            │
│  Constrained: flat-ARGB-only in V1                               │
└──────────────────────────────────────────────────────────────────┘
```

### Pipeline

```
User setting → Intent (string/enum)
  → silk-shell resolves to Render Tokens (clamped u32/u8)
    → IPC to sexdisplay (OP_APPEARANCE_TOKENS or extended 0xFD)
      → sexdisplay substitutes tokens into composite_pixel()
```

### Near-term path (V1-V3)

1. **L3 tokens are hardcoded constants** in sexdisplay (current state)
2. **L2 tokens become a silk-shell-owned table** — color values stored in ShellScene struct
3. **L1 intent** is the initial boot config — no settings app yet
4. IPC: new opcode `OP_APPEARANCE_TOKENS` (or extension of 0xFD) to push token table to sexdisplay

### Future path (V4+)

1. **L1 gets a Settings app** — user picks colors/effects
2. **L2 gets per-scene overrides** — Scene struct has appearance token override table
3. **L3 gets effect engine** — alpha blending, blur, glow passes
4. IPC: per-token or batch updates

---

## Storage Model Candidates

### Candidate A: ShellScene struct (recommended for V1)

```rust
struct ShellScene {
    // ... existing fields ...
    
    // Appearance tokens (flat color, no alpha effects)
    appearance: SceneAppearance,
}

struct SceneAppearance {
    // Colors (flat ARGB, always fully opaque alpha=0xFF)
    accent_color: u32,
    topbar_color: u32,
    rim_color: u32,
    active_tab_color: u32,
    inactive_tab_color: u32,
    close_light_color: u32,
    minimize_light_color: u32,
    zoom_light_color: u32,
    urgent_color: u32,
    focus_indicator_color: u32,
    
    // Chrome settings
    top_bar_enabled: bool,
    rim_thickness: u8,       // 1..8
    tab_strip_mode: u8,      // 0=always, 1=hover, 2=hidden
    frame_lights_mode: u8,   // 0=always, 1=hover, 2=hidden
    
    // Accessibility
    high_contrast: bool,
    colorblind_safe: bool,
    stronger_focus_ring: bool,
    larger_targets: bool,
    
    // Reserved for future effect engine
    _reserved: [u8; 32],
}
```

**Pros:** Fixed-size, no allocation, trivially safe to copy, can be baked into boot image.
**Cons:** No per-monitor overrides without additional structs.

### Candidate B: Global static SceneAppearance table

```rust
static DEFAULT_APPEARANCE: SceneAppearance = SceneAppearance {
    accent_color: 0x007AAFA4,
    topbar_color: 0x0088C2B7,
    // ...
};
```

**Pros:** Compile-time default, zero runtime init cost, trivially safe.
**Cons:** Cannot be changed at runtime without `&mut` static (unsafe). But this is already the pattern used by all shell state (FRAMES, WINDOWS, etc.).

### Recommendation: Candidate B (static) for V1, Candidate A (struct field) for V2

V1: Global static token table. Tokens are pushed to sexdisplay at boot and on change via IPC.
V2: Scene struct gains appearance field when per-scene overrides are implemented.

---

## Render Token Model Candidates

### Candidate 1: Flat ARGB constants (V1 near-term)

```rust
/// Render-safe appearance tokens sent from silk-shell to sexdisplay.
/// All colors are fully opaque (alpha = 0xFF). No alpha/blur.
struct RenderTokens {
    focus_surface_color: u32,
    frame_rim_color: u32,
    frame_top_bar_color: u32,
    tab_active_color: u32,
    tab_inactive_color: u32,
    frame_light_close_color: u32,
    frame_light_minimize_color: u32,
    frame_light_zoom_color: u32,
}

// Clamping: alpha must be 0xFF (fully opaque).
fn clamp_token(color: u32) -> u32 {
    color | 0xFF000000  // Force alpha to 0xFF
}
```

**IPC:** Single opcode `OP_APPEARANCE_TOKENS` carries all 8 color tokens (8 × u32 = 32 bytes → 4 PDX args or batch).

### Candidate 2: Packed token payload (32 bytes)

```rust
// arg0: header (token_count | version<<8)
// arg1: token_0_1  = (token[0] << 32) | token[1]
// arg2: token_2_3  = (token[2] << 32) | token[3]
// arg3: token_4_5  = (token[4] << 32) | token[5]
// arg4: token_6_7  = (token[6] << 32) | token[7]
```

**Pros:** Single IPC call updates all colors atomically.
**Cons:** More complex encoding.

### Recommendation: Candidate 1 for V1

Simple per-token or small-batch IPC. Start with 8 color tokens + 1 byte for chrome/layout settings.

---

## Protocol Needs and STOP Conditions

### Required new opcode

```
OP_APPEARANCE_TOKENS = 0xFC  // or next available in display opcode space
```

### STOP Conditions

| Condition | Stop? | Mitigation |
|-----------|-------|------------|
| True alpha/blur required | ❌ STOP | Defer to effect engine safety plan (SCENE_EFFECT_ENGINE_SAFETY_V1) |
| Dynamic allocation needed | ❌ STOP | Fixed-size structs only |
| Per-pixel effect pass required | ❌ STOP | Defer to effect engine |
| Shell needs to read back pixels | ❌ STOP | Sexdisplay is sole writer |
| Per-monitor overrides in V1 | ❌ STOP | V1 = global only. Per-monitor deferred. |
| Settings app in V1 | ❌ STOP | V1 = keyboard shortcuts + boot config. Settings app deferred. |
| Multi-word IPC (sex-pdx ABI change) | ❌ STOP | Batch across multiple pdx_call() or use contiguous opcode region |
| sexdisplay needs heap | ❌ STOP | Fixed token table only, no dynamic storage |

---

## Ownership Split

| Layer | Owner | Responsibility |
|-------|-------|---------------|
| Appearance Intent Model | Future Settings app | User-facing preferences, defaults, per-scene overrides |
| Appearance State Storage | silk-shell | SceneAppearance struct, per-scene tables, boot defaults |
| Render Token Resolution | silk-shell | Intent → Token clamping, validation |
| Token IPC | silk-shell → sexdisplay | OP_APPEARANCE_TOKENS push |
| Token Storage | sexdisplay | Surface.render_tokens or global display_tokens |
| Pixel Application | sexdisplay | composite_pixel() substitution of token values |
| Compact Controls Display | SilkBar | Status indicators (current mode, accessibility state) |
| Effect Engine (future) | sexdisplay | Alpha blending, blur, glow passes (after safety plan) |

---

## Near-Term Path (Flat Color Only)

### Phase: SCENE_RENDER_TOKENS_PLAN_V1 (next)

Design the RenderTokens struct, clamping rules, and IPC protocol:

1. Define `RenderTokens` struct (8 color u32 + 1 layout byte + 1 accessibility byte = 34 bytes)
2. Define clamping: all colors forced to alpha=0xFF (opaque), range validation
3. Design IPC: `OP_APPEARANCE_TOKENS` opcode
4. Design sexdisplay storage: `render_tokens: RenderTokens` global or per-surface
5. Prototype: replace hardcoded color constants with token lookup
6. Verify: zero behavioral change when tokens match current defaults

### Phase: SCENE_APPEARANCE_TOKENS_IPC_V1

Implement the token IPC and sexdisplay storage:

1. Add `OP_APPEARANCE_TOKENS` to sex-pdx
2. Add `render_tokens` field/table to sexdisplay
3. Modify `composite_pixel()` to read from tokens instead of constants
4. Add silk-shell `push_appearance_tokens()` helper
5. Call at boot to set default tokens
6. Verify: colors unchanged, building passes

### Phase: SCENE_APPEARANCE_TUNE_V1 (future)

Replace FRAME_GLASS_TINT_TUNE_V1 scaffolding with proper token-driven colors:

1. Replace hardcoded constant changes with token set changes
2. Default tokens match current tuned values (`0x007AAFA4`, `0x00B8F2E8`, `0x0088C2B7`)
3. Verify: visual identical to glass tint, but through token system

---

## Future True Glass Path (Blocked)

The following are BLOCKED until `SCENE_EFFECT_ENGINE_SAFETY_V1` is complete:

| Feature | Requires |
|---------|----------|
| Transparency (alpha < 0xFF) | Alpha compositing safety plan |
| Background blur (frosted glass) | Multi-pass rendering, scene-background readback |
| Drop shadows | Full-frame post-processing |
| Wallpaper-aware adaptive contrast | Live wallpaper integration, scene-background access |
| True gradient fills (non-flat) | New fill_rect mode or gradient fragment shader |
| Pixel-level glow falloff | Frame-buffer post-processing |

### Prerequisites for effect engine safety

1. Stable scheduler yield (no spin-wait for compositing)
2. Partial redraw invalidation (not full-frame every time)
3. Scene-background readback without tearing
4. Frame-buffer bounds enforcement for multi-pass
5. Budgeted effect passes (no unbounded work)
6. MPK/PKU isolation for any shared framebuffer access

---

## Summary Roadmap

```
SCENE_APPEARANCE_CONTROLS_PLAN_V1     ← THIS DOCUMENT (design)
  ↓
SCENE_RENDER_TOKENS_PLAN_V1           ← NEXT: token design & IPC protocol
  ↓
SCENE_APPEARANCE_TOKENS_IPC_V1        → implement token push + storage
  ↓
SCENE_APPEARANCE_TUNE_V1              → replace glass tint scaffolding
  ↓
SCENE_EFFECT_ENGINE_SAFETY_PLAN_V1    → plan for true glass/alpha (future)
  ↓
... (blocked until safety plan)
```

## Pass Criteria

- [x] Docs-only: no code changes
- [x] Verdict APPEARANCE_MODEL_SAFE_TO_DESIGN
- [x] Control taxonomy defined (5 categories, ~30 controls)
- [x] Three-layer architecture defined (Intent → Token → Pixel)
- [x] Near-term flat-color path separated from future glass path
- [x] Storage model candidates evaluated
- [x] Render token model candidates evaluated
- [x] Protocol needs identified (OP_APPEARANCE_TOKENS)
- [x] STOP conditions documented
- [x] Ownership split defined
- [x] Next phase named: SCENE_RENDER_TOKENS_PLAN_V1
- [x] No kernel/ABI/sexdisplay code changes

---

## Files

| File | Role |
|------|------|
| `docs/handoff/SCENE_APPEARANCE_CONTROLS_PLAN_V1.md` | This document |
| `.claude/plans/splendid-brewing-starlight.md` | Updated roadmap |

### Existing docs referenced

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` | Chrome mode roadmap, per-scope definitions |
| `docs/handoff/FRAME_GLASS_TINT_TUNE_V1.md` | Current tint scaffolding, color constants audit |
| `docs/SILK_DE_GLASS_VISUAL_LANGUAGE.md` | Glass visual language, forbidden effects list |
| `docs/handoff/FRAME_TOP_BAR_TOGGLE_V1.md` | Current top bar toggle (the only user-facing chrome control) |
| `docs/handoff/FRAME_TOP_BAR_RENDER_V1.md` | chrome_flags communication, 0xFD extension pattern |
| `docs/handoff/FRAME_TOP_BAR_MODEL_V1.md` | Shell-side chrome model, flag constants |

---

## Next Phase

### SCENE_RENDER_TOKENS_PLAN_V1

Design the RenderTokens struct, clamping rules, and IPC protocol for communicating appearance tokens from silk-shell to sexdisplay:

1. Define `RenderTokens` struct layout (8 color u32 + layout byte + accessibility byte)
2. Define clamping/safety rules for each token
3. Design `OP_APPEARANCE_TOKENS` IPC (single opcode, argument packing)
4. Design sexdisplay token storage (global token table or per-surface)
5. Design substitution mechanism in `composite_pixel()` Pass 2
6. Verify zero behavioral change when tokens match current default colors
7. Identify sexdisplay initialization order (tokens must be set before first composite)
