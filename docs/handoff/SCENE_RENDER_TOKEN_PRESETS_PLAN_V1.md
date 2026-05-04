# SCENE_RENDER_TOKEN_PRESETS_PLAN_V1

## Status

Design (2026-05-04). Keyboard-driven RenderToken preset cycling via F5. Docs-only — no code changed.

---

## Verdict: SCENE_RENDER_TOKEN_PRESETS_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| New shortcut, no conflict | ✅ | F5 = scancode `0x3F` — free in `scancode_to_action()` |
| Reuse existing 0xFC path | ✅ | `push_token_preset()` sends same two `pdx_call` messages |
| No heap | ✅ | Fixed `[u32; 8]` arrays in `static` const table |
| No new opcode | ✅ | `OP_APPEARANCE_TOKENS = 0xFC` already defined |
| No sexdisplay changes | ✅ | 0xFC handler already accepts any preset values |
| No ABI/kernel changes | ✅ | silk-shell only |
| Semantic lights preserved in all presets | ✅ | close/minimize/zoom kept in red/amber/green families |
| Bounded preset index | ✅ | `(ACTIVE_PRESET_IDX + 1) % PRESET_COUNT` |
| Zero visual change at boot | ✅ | `ACTIVE_PRESET_IDX` starts at 0 = BottleGlass = current defaults |

---

## Scancode Audit

| Scancode | Key | Current Use |
|----------|-----|-------------|
| `0x3B` | F1 | `LegacyFocusToggle` |
| `0x3C` | F2 | `DestroyFocused` |
| `0x3D` | F3 | `RecreateFocused` |
| `0x3E` | F4 | `ToggleTopBar` |
| `0x3F` | F5 | **FREE** ← assign `CycleScenePreset` |
| `0x40` | F6 | free |
| `0x41` | F7 | free |
| `0x42` | F8 | free |
| `0x57` | F11 | free |
| `0x58` | F12 | free |

**Choice: F5 = `0x3F`**. Natural progression after F4 (ToggleTopBar). No conflict.

---

## Preset Definitions

**Type alias and count (silk-shell):**

```rust
const PRESET_COUNT: usize = 4;
// Fields in order: [focus_surface, frame_rim, frame_top_bar, active_tab,
//                   inactive_tab, close_light, minimize_light, zoom_light]
type TokenPreset = [u32; 8];
```

### Preset 0: BottleGlass (default teal — current defaults)

```rust
[
    0x007AAFA4,  // focus_surface_color
    0x00B8F2E8,  // frame_rim_color
    0x0088C2B7,  // frame_top_bar_color
    0x007AAFA4,  // active_tab_color   (= focus_surface)
    0x006080B0,  // inactive_tab_color
    0x00FF4444,  // close_light_color  (semantic red)
    0x00FFCC44,  // minimize_light_color (semantic amber)
    0x0044FF44,  // zoom_light_color   (semantic green)
]
```

Matches `DEFAULT_RENDER_TOKENS` exactly. Zero visual change from current state.

### Preset 1: VioletGlass (Silk canon purple)

```rust
[
    0x00503080,  // focus_surface_color (deep violet)
    0x00A060FF,  // frame_rim_color     (vivid violet rim)
    0x00604090,  // frame_top_bar_color (medium violet bar)
    0x00503080,  // active_tab_color    (= focus_surface)
    0x00302050,  // inactive_tab_color  (dim violet)
    0x00FF4080,  // close_light_color   (neon pink — red family)
    0x00FFAA00,  // minimize_light_color (amber — semantic)
    0x0040FF80,  // zoom_light_color    (bright green — semantic)
]
```

Deep purple/violet glass feel. Close light shifted toward pink but stays in semantic red family.

### Preset 2: GraphiteGlass (dark neutral)

```rust
[
    0x00282828,  // focus_surface_color (dark graphite)
    0x00808080,  // frame_rim_color     (medium gray)
    0x00404040,  // frame_top_bar_color (dark gray)
    0x00505050,  // active_tab_color    (slightly lighter)
    0x00303030,  // inactive_tab_color  (dim graphite)
    0x00CC4444,  // close_light_color   (muted red — semantic)
    0x00CCAA44,  // minimize_light_color (muted amber — semantic)
    0x0044CC44,  // zoom_light_color    (muted green — semantic)
]
```

All-neutral chrome. Semantic lights muted to match graphite palette. Good for low-distraction use.

### Preset 3: HighContrast (accessibility proof)

```rust
[
    0x00000000,  // focus_surface_color (black — clamped to 0xFF000000 on receive)
    0x00FFFFFF,  // frame_rim_color     (white rim — maximum contrast)
    0x00111111,  // frame_top_bar_color (near-black bar)
    0x00FFFF00,  // active_tab_color    (bright yellow — high-contrast)
    0x00555555,  // inactive_tab_color  (medium gray)
    0x00FF4444,  // close_light_color   (semantic red — unchanged)
    0x00FFDD00,  // minimize_light_color (vivid amber — semantic, slightly brighter)
    0x0000FF44,  // zoom_light_color    (vivid green — semantic)
]
```

Notes:
- `focus_surface_color = 0x00000000` → after `clamp_color_token()` in sexdisplay, stored as `0xFF000000`. Renderer ignores alpha byte → displays as pure black. Correct.
- `frame_rim_color = 0x00FFFFFF` → white. Maximum luminance contrast against black content area.
- Semantic close/minimize/zoom kept vivid — window action semantics must be legible.
- `active_tab_color = 0x00FFFF00` (bright yellow) is deliberate: accessibility high-contrast themes typically use yellow-on-black for selected/active elements.

---

## Preset Table (full)

```rust
static TOKEN_PRESETS: [TokenPreset; PRESET_COUNT] = [
    // 0: BottleGlass (default teal)
    [0x007AAFA4, 0x00B8F2E8, 0x0088C2B7, 0x007AAFA4, 0x006080B0, 0x00FF4444, 0x00FFCC44, 0x0044FF44],
    // 1: VioletGlass (Silk canon)
    [0x00503080, 0x00A060FF, 0x00604090, 0x00503080, 0x00302050, 0x00FF4080, 0x00FFAA00, 0x0040FF80],
    // 2: GraphiteGlass (dark neutral)
    [0x00282828, 0x00808080, 0x00404040, 0x00505050, 0x00303030, 0x00CC4444, 0x00CCAA44, 0x0044CC44],
    // 3: HighContrast (accessibility proof)
    [0x00000000, 0x00FFFFFF, 0x00111111, 0x00FFFF00, 0x00555555, 0x00FF4444, 0x00FFDD00, 0x0000FF44],
];

static mut ACTIVE_PRESET_IDX: usize = 0;
```

`static` (not `static mut`) for TOKEN_PRESETS — it's a read-only constant table. `ACTIVE_PRESET_IDX` is `static mut` (needs runtime mutation, single-threaded kernel).

---

## Action / Helper Names

### SurfaceAction variant

```rust
CycleScenePreset,  // add to SurfaceAction enum
```

### Scancode mapping

```rust
0x3F => Some(SurfaceAction::CycleScenePreset),  // F5
```

### Send helper (replaces/supplements send_scene_render_tokens)

```rust
/// Send a preset to sexdisplay via OP_APPEARANCE_TOKENS.
/// Two sequential pdx_call messages; sexdisplay state machine disambiguates.
unsafe fn push_token_preset(p: &TokenPreset) {
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(p[0], p[1]),
        pack_u32_pair(p[2], p[3]),
        pack_u32_pair(p[4], p[5]),
    );
    pdx_call(SLOT_DISPLAY, OP_APPEARANCE_TOKENS,
        pack_u32_pair(p[6], p[7]),
        0u64,
        0u64,
    );
}
```

### Cycle action handler

```rust
unsafe fn cycle_scene_preset() {
    ACTIVE_PRESET_IDX = (ACTIVE_PRESET_IDX + 1) % PRESET_COUNT;
    push_token_preset(&TOKEN_PRESETS[ACTIVE_PRESET_IDX]);
    unsafe {
        static mut CYCLE_BUDGET: u32 = 16;
        if CYCLE_BUDGET > 0 {
            CYCLE_BUDGET -= 1;
            serial_println!("[shell.appearance.tokens.cycle] preset={}", ACTIVE_PRESET_IDX);
        }
    }
}
```

Budget of 16: enough to cycle through all 4 presets multiple times during a test run.

### Boot call

The existing `send_scene_render_tokens()` boot call stays as-is. It seeds `[shell.appearance.tokens.send]` proof. Alternatively, replace with `push_token_preset(&TOKEN_PRESETS[0])` for unified path — either is valid in V1. Keeping `send_scene_render_tokens()` avoids touching the boot sequence. Recommended: keep it.

### Dispatch arm

```rust
SurfaceAction::CycleScenePreset => {
    unsafe { cycle_scene_preset(); }
}
```

Add to the keyboard dispatch match alongside `SurfaceAction::ToggleTopBar`.

---

## Implementation File List

### Modified

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `PRESET_COUNT`, `TokenPreset`, `TOKEN_PRESETS`, `ACTIVE_PRESET_IDX`; add `CycleScenePreset` to `SurfaceAction`; add `0x3F` in `scancode_to_action`; add `push_token_preset()` and `cycle_scene_preset()`; add dispatch arm |

### Created

| File | Role |
|------|------|
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_PLAN_V1.md` | This document |
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md` | Implementation handoff (next phase) |

### NOT modified

| File | Reason |
|------|--------|
| `servers/sexdisplay/src/main.rs` | 0xFC handler already accepts any valid preset values |
| `crates/sex-pdx/src/lib.rs` | `OP_APPEARANCE_TOKENS = 0xFC` already defined |
| `sexos_build_spec.toml` | No sex-pdx change → no ABI hash update needed |
| `kernel/` | Forbidden |
| `servers/silkbar/` | Independent theme system |
| `crates/silkbar-model/` | Independent |
| `servers/sexusb/` | Untouched |
| `servers/sexinput/` | Untouched |

---

## Proof Markers

| Marker | Source | Meaning |
|--------|--------|---------|
| `[shell.appearance.tokens.send] seq=2 sent` | silk-shell boot | Default tokens pushed at boot |
| `[sexdisplay.appearance.tokens] seq=0 buffered` | sexdisplay | Call 1 received |
| `[sexdisplay.appearance.tokens] seq=1 applied=N` | sexdisplay | Call 2 received, committed |
| `[shell.appearance.tokens.cycle] preset=N` | silk-shell | F5 pressed, preset N applied |

---

## Build / Proof Commands

```bash
# Default build
./scripts/entrypoint_build.sh

# Synthetic build (no USB hardware needed)
env SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh

# Run with display + keyboard
env SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>&1 | tee /tmp/scene-preset-v1.log
```

### Verification after run

```bash
# Shell sent both token calls at boot
grep -ac "\[shell.appearance.tokens.send\]" /tmp/scene-preset-v1.log

# Display applied tokens (seq=1 means both calls received)
grep -ac "\[sexdisplay.appearance.tokens\] seq=1" /tmp/scene-preset-v1.log

# F5 cycles triggered (press F5 several times during test run)
grep -ac "\[shell.appearance.tokens.cycle\]" /tmp/scene-preset-v1.log

# No panics or protection faults
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-preset-v1.log
```

### Pass criteria

- Default build passes
- Visual: boot shows BottleGlass (default teal, no change from V1 baseline)
- F5 cycles: BottleGlass → VioletGlass → GraphiteGlass → HighContrast → BottleGlass (wraps)
- F4 top bar toggle still works on any preset
- No panics, no kernel changes

---

## STOP Conditions

| Condition | Stop? | Mitigation |
|-----------|-------|------------|
| `0x3F` conflicts with existing scancode | ❌ STOP | Audit shows 0x3F free. Verified by grep of `scancode_to_action`. |
| Preset table requires heap | ❌ STOP | Fixed `[u32; 8]` arrays; `static` keyword; no Vec. |
| New opcode needed | ❌ STOP | Reuses `OP_APPEARANCE_TOKENS = 0xFC`. |
| sexdisplay changes needed | ❌ STOP | 0xFC handler is already general; clamping handles any valid u32 color. |
| ABI hash update needed | ❌ STOP (manual) | No sex-pdx edit → no recompute. If sex-pdx edited for any reason, run: `{ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } \| sha256sum` and update `sexos_build_spec.toml`. |
| Preset index OOB | ❌ STOP | `% PRESET_COUNT` guarantees 0..3. |
| HighContrast corrupts semantic lights | ❌ STOP | Preset 3 keeps close=red, minimize=amber, zoom=green families. |
| `TOKEN_PRESETS` marked `static mut` | Unnecessary | Table is read-only; use `static` (not mut). Indexing with `ACTIVE_PRESET_IDX` is in an unsafe block; static immutable table is safe to read. |

---

## Pass Criteria

- [x] Verdict: SCENE_RENDER_TOKEN_PRESETS_SAFE_NOW
- [x] F5 scancode `0x3F` confirmed free (no collision)
- [x] 4 presets defined with exact u32 token values
- [x] Preset 0 = BottleGlass = current defaults (zero visual change at boot)
- [x] Semantic lights (close/minimize/zoom) preserved in all 4 presets
- [x] HighContrast uses black+white+yellow for accessibility proof
- [x] `push_token_preset()` helper designed (reuses `pack_u32_pair`, `OP_APPEARANCE_TOKENS`)
- [x] `cycle_scene_preset()` designed with bounded index and budgeted marker
- [x] `CycleScenePreset` action named; dispatch arm designed
- [x] No sexdisplay changes needed
- [x] No sex-pdx changes needed
- [x] No ABI hash update needed
- [x] No kernel changes
- [x] Implementation file list complete (only silk-shell)
- [x] STOP conditions documented
- [x] Proof markers documented
- [x] Next phase named: SCENE_RENDER_TOKEN_PRESETS_V1

---

## Next Phase: SCENE_RENDER_TOKEN_PRESETS_V1

Implement:

1. In `servers/silk-shell/src/main.rs`:
   a. Add `PRESET_COUNT: usize = 4` and `TokenPreset = [u32; 8]` type alias
   b. Add `TOKEN_PRESETS: [TokenPreset; PRESET_COUNT]` static const table (4 presets above)
   c. Add `ACTIVE_PRESET_IDX: usize = 0` static mut
   d. Add `CycleScenePreset` to `SurfaceAction` enum
   e. Add `0x3F => Some(SurfaceAction::CycleScenePreset)` to `scancode_to_action()`
   f. Add `push_token_preset(p: &TokenPreset)` helper
   g. Add `cycle_scene_preset()` helper with budgeted `[shell.appearance.tokens.cycle]` marker
   h. Add `SurfaceAction::CycleScenePreset => { unsafe { cycle_scene_preset(); } }` dispatch arm

2. Build: `./scripts/entrypoint_build.sh`

3. Verify: boot shows teal (Preset 0). F5 → purple (Preset 1) → graphite (Preset 2) → high-contrast (Preset 3) → teal (Preset 0). F4 top-bar toggle still works.

4. Create `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md`.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_RENDER_TOKENS_PLAN_V1.md` | Token struct, IPC protocol, state machine |
| `docs/handoff/SCENE_RENDER_TOKENS_V1.md` | Implementation status, substitution sites |
| `docs/handoff/FRAME_TOP_BAR_TOGGLE_V1.md` | F4 / ToggleTopBar pattern (direct precedent) |
| `servers/silk-shell/src/main.rs` | `SurfaceAction`, `scancode_to_action`, `send_scene_render_tokens` |
| `servers/sexdisplay/src/main.rs` | `DISPLAY_TOKENS`, 0xFC handler |
| `crates/sex-pdx/src/lib.rs` | `OP_APPEARANCE_TOKENS = 0xFC` |
