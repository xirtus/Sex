# Phase CHROME_TEMPLATE_V1: Data-Driven Chrome + Animation

## Revolutionary Vision
Every OS compositor hardcodes chrome and animation in compiled code. KWin's animation curves are in C++. Mutter's glass parameters are in C. COSMIC's layout constants are in Rust. To change how minimize looks, you edit source, rebuild, and restart.

SexOS does not recompile to change a style.

**ChromeTemplate is a data structure that defines how windows look and move.** Glass alpha, animation curves, layout dimensions, colors — all live in a fixed-size struct, resolved at runtime, hot-swappable without rebuild or restart.

The template IS the "scripting language" — without an interpreter, without Lua, without FFI. Just a 60-byte struct in a ring buffer.

## What This Means Practically

**Hot-swappable chrome:** Push a new ChromeTemplate via PDX → glass alpha, colors, and layout update on the next frame. No rebuild. No restart. No reboot.

**Per-workspace profiles:** Workspace A uses subtle glass (alpha=0x40) with slow ease. Workspace B uses bold chrome (alpha=0xC0) with snappy ease. Same binary, different data.

**Per-app chrome:** An image viewer could request minimal chrome (transparent top bar, no lights). A recording app could request persistent chrome (opaque, high-contrast). Sexdisplay decides whether to allow based on app capabilities.

**User-created templates:** ChromeTemplates stored as Linen Objects. Create, edit, pin, share — like a document, but it controls your desktop's look and feel.

**Versioning:** Linen tracks template history. "What did my desktop look like last week?" — replay the template from the temporal object graph.

## Ownership
- **silk-shell** (exclusive): ChromeTemplate storage, resolution, hot-swap dispatch, animation engine
- **sexdisplay** (consumer): glass alpha + colors received via 0xFC (already supported), no template awareness needed for V1
- **Linen** (future): template persistence as Objects (Phase 04)
- **Collar** (future): "change chrome template" as a grantable capability (Phase 06)

## What Already Exists
- `OP_APPEARANCE_TOKENS` (0xFC) sends 8 colors + 2 flags from silk-shell to sexdisplay
- 4 static presets in `TOKEN_PRESETS` (hardcoded arrays)
- `cycle_scene_render_token_preset()` switches presets at runtime
- `FrameAnimation` concept from prior design — frame-based interpolation
- 0xEC handles geometry updates (resize + reposition)
- `FRAME_FLAG_MINIMIZED` and `FRAME_FLAG_ZOOMED` track window state
- Event loop with sys_yield() — can host tick_animations()

## ChromeTemplate Struct

```rust
/// Complete chrome style + animation profile.
/// Fixed-size, repr(C), no pointers, no heap.
#[derive(Clone, Copy)]
#[repr(C)]
struct ChromeTemplate {
    /// Human-readable name (for Linen + debug)
    name: [u8; 32],

    // ── Animation Parameters ──
    /// Duration of minimize animation in milliseconds. 0 = instant.
    minimize_duration_ms: u16,
    /// Ease function for minimize. See EaseId.
    minimize_ease_id: u8,

    /// Duration of zoom/unzoom animation in milliseconds. 0 = instant.
    zoom_duration_ms: u16,
    /// Ease function for zoom.
    zoom_ease_id: u8,

    /// Cursor lag for drag follow in milliseconds. 0 = instant snap.
    drag_follow_ms: u16,

    // ── Layout Parameters ──
    /// Top bar height in pixels (matches current 16px default).
    top_bar_height_px: u8,
    /// Light indicator size in pixels.
    light_size_px: u8,

    // ── Glass Alpha Overrides ──
    /// These alpha values are applied ON TOP of the color's own alpha.
    /// 0xFF means "use the color's alpha as-is."
    /// 0x40 means "force top bar to 25% opacity regardless of color."
    top_bar_alpha: u8,
    rim_alpha: u8,
    tab_active_alpha: u8,
    tab_inactive_alpha: u8,

    // ── Colors (same 8-slot layout as current TokenPreset) ──
    colors: [u32; 8],
    // Slot order: [focus_surface, frame_rim, frame_top_bar, active_tab,
    //              inactive_tab, close_light, minimize_light, zoom_light]
    // Alpha bytes are used directly (no clamp to 0xFF, requires GLASS_V1).
    // Alpha overrides (above) can modify these at render time.
}
```

### EaseId enum

```rust
#[repr(u8)]
enum EaseId {
    /// Linear interpolation — no easing.
    Linear = 0,
    /// Cubic ease-out — fast start, smooth deceleration. Good for minimize/zoom.
    EaseOutCubic = 1,
    /// Ease-out with overshoot — settles past target then bounces back. Ping.
    EaseOutBack = 2,
    /// Elastic ease-out — bounces at end. Playful.
    EaseOutElastic = 3,
}
```

### Static presets (replaces hardcoded TOKEN_PRESETS)

```rust
const CHROME_TEMPLATE_COUNT: usize = 4;

static CHROME_TEMPLATES: [ChromeTemplate; CHROME_TEMPLATE_COUNT] = [
    // 0: BottleGlass — subtle glass, smooth animations
    ChromeTemplate {
        name: *b"BottleGlass\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        minimize_duration_ms: 200, minimize_ease_id: EaseId::EaseOutCubic as u8,
        zoom_duration_ms:     150, zoom_ease_id:     EaseId::EaseOutCubic as u8,
        drag_follow_ms: 50,
        top_bar_height_px: 16, light_size_px: 8,
        top_bar_alpha: 0x60, rim_alpha: 0x80, tab_active_alpha: 0x80, tab_inactive_alpha: 0x40,
        colors: [0xFF7AAFA4, 0x80B8F2E8, 0x6088C2B7, 0x807AAFA4,
                 0x406080B0, 0xFFFF4444, 0xFFFFCC44, 0xFF44FF44],
    },
    // 1: VioletGlass — slower, more pronounced animations
    ChromeTemplate {
        name: *b"VioletGlass\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        minimize_duration_ms: 300, minimize_ease_id: EaseId::EaseOutBack as u8,
        zoom_duration_ms:     200, zoom_ease_id:     EaseId::EaseOutBack as u8,
        drag_follow_ms: 80,
        top_bar_height_px: 16, light_size_px: 8,
        top_bar_alpha: 0x50, rim_alpha: 0x70, tab_active_alpha: 0x70, tab_inactive_alpha: 0x30,
        colors: [0xFF503080, 0x80A060FF, 0x60604090, 0x80503080,
                 0x40302050, 0xFFFF4080, 0xFFFFAA00, 0xFF40FF80],
    },
    // 2: GraphiteGlass — snappy, efficient
    ChromeTemplate {
        name: *b"GraphiteGlass\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        minimize_duration_ms: 100, minimize_ease_id: EaseId::Linear as u8,
        zoom_duration_ms:     100, zoom_ease_id:     EaseId::Linear as u8,
        drag_follow_ms: 0,
        top_bar_height_px: 12, light_size_px: 6,
        top_bar_alpha: 0x40, rim_alpha: 0x60, tab_active_alpha: 0x60, tab_inactive_alpha: 0x20,
        colors: [0xFF282828, 0x80808080, 0x60404040, 0x80505050,
                 0x40303030, 0xFFCC4444, 0xFFCCAA44, 0xFF44CC44],
    },
    // 3: HighContrast — no glass, instant, max visibility
    ChromeTemplate {
        name: *b"HighContrast\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        minimize_duration_ms: 0,   minimize_ease_id: EaseId::Linear as u8,
        zoom_duration_ms:     0,   zoom_ease_id:     EaseId::Linear as u8,
        drag_follow_ms: 0,
        top_bar_height_px: 20, light_size_px: 10,
        top_bar_alpha: 0xFF, rim_alpha: 0xFF, tab_active_alpha: 0xFF, tab_inactive_alpha: 0xFF,
        colors: [0xFF000000, 0xFFFFFFFF, 0xFF111111, 0xFFFFFF00,
                 0xFF555555, 0xFFFF4444, 0xFFFFDD00, 0xFF00FF44],
    },
];

static mut ACTIVE_CHROME_IDX: u8 = 0;
```

## The Animation Engine (replaces hardcoded silk-shell transitions)

### Generic interpolator driven by template data

```rust
/// A single active animation. Duration and ease read from ACTIVE_CHROME.
struct ActiveTween {
    active: bool,
    kind: AnimationKind,  // Minimize, Zoom, Restore, Drag
    surface_id: u64,
    start_x: i32, start_y: i32, start_w: u32, start_h: u32,
    end_x: i32, end_y: i32, end_w: u32, end_h: u32,
    elapsed_ms: u16,
    duration_ms: u16,
    ease_id: u8,
}
```

### tick_animations() — called once per event loop iteration

```rust
unsafe fn tick_animations() {
    for tween in ACTIVE_TWEENS.iter_mut() {
        if !tween.active { continue; }
        tween.elapsed_ms += TICK_MS;  // approximate, fine for V1
        if tween.elapsed_ms >= tween.duration_ms || tween.duration_ms == 0 {
            // Snap to end state
            tween.active = false;
            apply_geometry(tween.surface_id, tween.end_x, tween.end_y, tween.end_w, tween.end_h);
            on_animation_complete(tween.kind, tween.surface_id);
        } else {
            let t = (tween.elapsed_ms as f32) / (tween.duration_ms as f32);
            let eased = apply_ease(tween.ease_id, t);
            let x = lerp(tween.start_x, tween.end_x, eased);
            let y = lerp(tween.start_y, tween.end_y, eased);
            let w = lerp(tween.start_w as i32, tween.end_w as i32, eased) as u32;
            let h = lerp(tween.start_h as i32, tween.end_h as i32, eased) as u32;
            apply_geometry(tween.surface_id, x, y, w, h);
        }
    }
}
```

### apply_ease() dispatches based on EaseId from template

```rust
fn apply_ease(ease_id: u8, t: f32) -> f32 {
    match ease_id {
        0 => t,                                            // Linear
        1 => 1.0 - (1.0 - t).powi(3),                     // EaseOutCubic
        2 => {                                             // EaseOutBack
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
        },
        3 => {                                             // EaseOutElastic
            if t == 0.0 || t == 1.0 { return t; }
            let c4 = (2.0 * core::f32::consts::PI) / 3.0;
            2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
        },
        _ => t, // fallback to linear
    }
}
```

### Minimize now reads from template

```rust
unsafe fn minimize_frame(frame_id: u32) {
    let template = &CHROME_TEMPLATES[ACTIVE_CHROME_IDX as usize];
    let surface_id = get_active_tab(frame_id);
    let (x, y, w, h) = get_surface_bounds(surface_id);
    if template.minimize_duration_ms == 0 {
        // Instant minimize (HighContrast or user preference)
        // ...existing instant path...
    } else {
        // Animated minimize using template params
        start_tween(AnimationKind::Minimize, surface_id,
                    x, y, w, h, x, y, 0, 0,
                    template.minimize_duration_ms, template.minimize_ease_id);
    }
}
```

### Hot-swap: cycle template

```rust
unsafe fn cycle_chrome_template() {
    ACTIVE_CHROME_IDX = ((ACTIVE_CHROME_IDX as usize + 1) % CHROME_TEMPLATE_COUNT) as u8;
    let template = &CHROME_TEMPLATES[ACTIVE_CHROME_IDX as usize];

    // Push colors (with alpha) to sexdisplay via 0xFC
    push_token_preset(&template.colors);

    // Apply glass alpha overrides to sexdisplay — new opcode or extend 0xFC
    push_glass_alphas(
        template.top_bar_alpha,
        template.rim_alpha,
        template.tab_active_alpha,
        template.tab_inactive_alpha,
    );

    // Apply layout params to shell state
    set_top_bar_height(template.top_bar_height_px);

    // Emit marker
    let name = core::str::from_utf8(&template.name).unwrap_or("?");
    serial_println!("[shell.chrome.swap] template={}", name);
}
```

## Glass Alpha Override Protocol

The glass alpha overrides (`top_bar_alpha`, `rim_alpha`, etc.) need to reach sexdisplay. Options:

**Option A: Extend 0xFC third call** — current 0xFC uses two calls (6 colors + 2 colors). Add a third call with alpha overrides:

```
0xFC call 1: arg0=color0+color1, arg1=color2+color3, arg2=color4+color5
0xFC call 2: arg0=color6+color7, arg1=appearance_flags|(effect_levels<<8), arg2=0
0xFC call 3 (NEW): arg0=alphas(top|rim|active|inactive), arg1=layout(height|light|pad|pad), arg2=0
```

**Option B: New opcode 0xFB** for chrome template push — sexdisplay stores a full template:

```
0xFB: arg0=alphas_packed, arg1=animation_packed, arg2=layout_packed
```

Option A is simpler (extends existing protocol), but Option B is cleaner (separate concern). For V1, use Option A since it adds no new opcode dispatch.

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| ChromeTemplate struct | Fixed-size, repr(C), ~60 bytes, 4 static presets | 1h | HIGH |
| ACTIVE_CHROME static | Store active template index, resolve on use | 0.5h | HIGH |
| Generic animation engine | tick_animations(), ActiveTween, apply_ease(), lerp() | 4h | HIGH |
| Wire minimize to template | Read duration + ease from ACTIVE_CHROME | 1h | HIGH |
| Wire zoom/unzoom to template | Same pattern as minimize | 1h | HIGH |
| Wire restore to template | Same pattern | 1h | HIGH |
| Smooth drag follow | Spring-like cursor lag, read drag_follow_ms | 2h | Medium |
| Extend 0xFC protocol (3rd call) | Push alpha overrides + layout to sexdisplay | 2h | HIGH |
| cycle_chrome_template() | Hot-swap: push colors + alphas + layout to sexdisplay | 1h | HIGH |
| sexdisplay alpha override storage | Store alpha overrides, apply in blend_chrome() | 2h | HIGH |
| 4 default templates | BottleGlass (subtle), VioletGlass (expressive), GraphiteGlass (snappy), HighContrast (accessible) | 1h | Medium |
| Per-template marker | `[shell.chrome.swap] template=Name` on hot-swap | 0.5h | Medium |

Total: ~17h

## GLASS_V1 as prerequisite

CHROME_TEMPLATE_V1 depends on GLASS_V1 for alpha blending infrastructure. The two phases are:

**GLASS_V1** (~6h, sexdisplay only)
- alpha_blend(), blend_chrome(), remove alpha clamp
- Pure infrastructure — sexdisplay supports semi-transparent pixels

**CHROME_TEMPLATE_V1** (~17h, silk-shell + sexdisplay)
- Data-driven chrome/animation on top of that infrastructure
- The revolutionary part: chrome is a struct, not code

## Smallest First Step
Define ChromeTemplate struct and 4 static presets in silk-shell. Add `cycle_chrome_template()` called by F5 (replaces `cycle_scene_render_token_preset()`). It pushes the colors (with alpha) to sexdisplay via existing 0xFC path. No animation yet — just prove that swapping a template changes both colors AND glass alpha on the next frame.

## Dependencies
- **GLASS_V1 must come first** (alpha blending infrastructure in sexdisplay)
- **No dependency on Linen, Quil, Mesh, Collar, Bell, USB, or any other phase**
- Animation engine depends on Phase 02 frame model (FRAME_FLAG_MINIMIZED etc.)

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Template hot-swap desyncs sexdisplay (colors pushed but alpha overrides lost) | Medium | Medium | Extend 0xFC with 3rd call in the same sequence. sexdisplay state machine waits for all 3 calls before applying. |
| Animation engine runs at inconsistent framerate (event loop yields) | Medium | High | When animation is active, skip sys_yield() and spin-loop. This keeps animation at CPU speed (~1000fps) rather than message rate. |
| ChromeTemplate struct size grows past fixed limit | Low | Low | V1 struct is ~60 bytes. Reserve 64 bytes max. Any field beyond that goes in V2 template. |
| Glass alpha overrides contradict color alphas (both set alpha) | Low | Low | Override wins: top_bar_alpha substitutes the alpha byte of the top bar color at render time. Color's own alpha is used only if override is 0xFF. |
| Ease function uses f32 math (no_std concern) | Low | Low | f32 is available in no_std for basic arithmetic. No transcendental dependency (sin is only in EaseOutElastic, which is optional). sin is available in core::intrinsics or approximated. |

## Exit Criteria
- [ ] ChromeTemplate struct defined (fixed-size, repr(C), ~60 bytes)
- [ ] 4 static templates with distinct glass + animation profiles
- [ ] ACTIVE_CHROME static, cycle_chrome_template() dispatches new template
- [ ] Generic animation engine: ActiveTween, tick_animations(), apply_ease(), lerp()
- [ ] tick_animations() called in main event loop
- [ ] Minimize reads duration + ease from active template
- [ ] Zoom/unzoom reads from active template
- [ ] Restore reads from active template
- [ ] Drag follow reads drag_follow_ms from active template (0 = instant snap)
- [ ] 0xFC protocol extended with 3rd call for alpha overrides + layout
- [ ] sexdisplay stores alpha overrides, applies in blend_chrome() during composite_pixel()
- [ ] Hot-swap: F5 cycles templates → sexdisplay updates glass + colors on next frame
- [ ] Hot-swap: next minimize/zoom after swap uses new animation params
- [ ] HighContrast template: minimize_duration_ms=0 (instant), all alphas=0xFF (opaque)
- [ ] Build passes. Boot passes. No panic.

## Testing Strategy
- **Hot-swap glass**: Boot with BottleGlass. Press F5. Verify sexdisplay switches to VioletGlass colors + alpha on next frame. Press F5 again, verify GraphiteGlass.
- **Hot-swap animation**: On BottleGlass, minimize. Verify 200ms cubic ease-out. Switch to GraphiteGlass. Minimize. Verify 100ms linear.
- **HighContrast**: Switch to HighContrast. Verify minimize is instant (no animation). Verify all chrome is fully opaque.
- **Drag follow**: Set drag_follow_ms=50. Drag window. Verify cursor lags by ~50ms. Set drag_follow_ms=0. Drag window. Verify instant snap.
- **Per-template markers**: Verify `[shell.chrome.swap] template=BottleGlass` fires on each swap.
- **Regression**: All existing markers fire. F5 still cycles presets (now cycles templates instead).

## Efficiency Opportunity
**The animation engine is ~100 lines of generic code that replaces ~50 lines of per-operation hardcoded transitions.** The generic engine handles minimize, zoom, restore, and drag with the same interpolate-apply loop. Each operation is ~5 lines of "start tween with template params" instead of ~20 lines of hardcoded animation. Less code, more flexibility.

## Completeness Gain
Visual polish: **50–60% → 90–95%** (glass animations + data-driven hot-swap). The OS now looks, feels, AND adapts like a polished desktop — without recompilation.

## Files Changed
- `servers/silk-shell/src/main.rs` (+ChromeTemplate struct, +4 static presets, +ACTIVE_CHROME, +ActiveTween struct, +tick_animations(), +apply_ease(), +lerp(), +cycle_chrome_template(), modify minimize/zoom/restore/drag to read from template, modify F5 handler to cycle templates)
- `servers/sexdisplay/src/main.rs` (+alpha override storage, +extend 0xFC state machine for 3rd call, +apply alpha overrides in blend_chrome())

## Next Phase
CHROME_TEMPLATE_V1 is the final visual phase. After it, begin the revolutionary trinity:
PHASE_04_LINEN_FILE_OBJECT_BROWSER.md → PHASE_05_QUIL_LANGUAGE_WORKSTATION.md → PHASE_06_MESH_CAPABILITY_GRAPH.md

## Future: Linen Integration (V2, Phase 04+)

ChromeTemplates become Linen Objects:
```rust
Object {
    id: ..., kind: ObjectKind::ChromeTemplate,
    name: *b"BottleGlass",
    source_pd: PD_ID_SILK_SHELL,
    data: ChromeTemplate_bytes,  // capability-gated read
    // ...
}
```

Users create custom templates by editing ChromeTemplate fields in Quil.
Templates can be pinned to workspaces, apps, or sessions.
Collar gates who can modify chrome templates.
