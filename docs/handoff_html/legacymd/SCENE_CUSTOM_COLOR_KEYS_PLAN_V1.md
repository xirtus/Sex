# SCENE_CUSTOM_COLOR_KEYS_PLAN_V1

## Status

Design (2026-05-04). Keyboard-driven custom color tint overlays for dev/proof of `SceneAppearanceState.custom_colors`. Docs-only — no code changed.

---

## Verdict: SCENE_CUSTOM_COLOR_KEYS_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| New shortcut, no conflict | ✅ | F6 = scancode `0x40` — free in `scancode_to_action()` |
| Reuse 0xFC path | ✅ | `push_token_preset()` + `resolve_scene_render_tokens()` unchanged |
| Exercise `custom_colors` / `use_custom_colors` | ✅ | Tint cycle writes to `SCENE_APPEARANCE_STATE` |
| No heap | ✅ | Fixed `[u32; 8]` tint table, no Vec |
| No new opcode | ✅ | `OP_APPEARANCE_TOKENS = 0xFC` reused |
| No sexdisplay changes | ✅ | 0xFC handler already clamps any u32 |
| No kernel/ABI changes | ✅ | silk-shell only |
| Tint 0 = clear (preset restored) | ✅ | Tint 0 = all zeros → `use_custom_colors = 0` |
| F5 preset cycle clears tint state | ✅ | F5 resets `ACTIVE_TINT_IDX = 0` and `use_custom_colors = 0` |

---

## Control Evaluation

| Option | Description | Decision |
|--------|-------------|----------|
| A — One key cycles accent+rim+topbar bundle | F6 cycles full color bundles | ✅ **Chosen** |
| B — Toggle custom on/off | F6 toggles `use_custom_colors` | ❌ Not useful without pre-set custom_colors |
| C — F5 preset + F6 custom tint | F5 and F6 are adjacent, clear semantics | ✅ **Chosen** (same as A) |
| D — Per-channel RGB editing | F-key + shift/ctrl adjusts individual channels | ❌ Too complex, defer to settings app |

**Chosen:** F6 (`0x40`) cycles through a small tint bundle table. Each bundle is a partial `[u32; 8]` override — zeros mean "use preset value". F5 still clears tint (existing behavior extended to also reset `ACTIVE_TINT_IDX`).

---

## Scancode Audit

| Scancode | Key | Current Use |
|----------|-----|-------------|
| `0x3E` | F4 | `ToggleTopBar` |
| `0x3F` | F5 | `CycleRenderTokenPreset` |
| `0x40` | F6 | **FREE** ← assign `CycleCustomTint` |
| `0x41` | F7 | free |
| `0x42` | F8 | free |
| `0x57` | F11 | free |
| `0x58` | F12 | free |

---

## Tint Bundle Design

**Type:** `type TintBundle = [u32; 8]` — same layout as `TokenPreset`.

Slot order: `[focus_surface, frame_rim, frame_top_bar, active_tab, inactive_tab, close_light, minimize_light, zoom_light]`

Zero in any slot = "keep preset value" (handled by `resolve_scene_render_tokens()`).
Nonzero = override that slot. Semantic lights (close/minimize/zoom) untouched in all tints.

### Tint table

```rust
const TINT_COUNT: usize = 5;
type TintBundle = [u32; 8];

static CUSTOM_TINT_BUNDLES: [TintBundle; TINT_COUNT] = [
    // 0: Clear — all zeros → use_custom_colors = 0 (clean preset)
    [0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],

    // 1: WarmTint — amber/copper rim + topbar; rest from preset
    [0x00000000, 0x00D4822A, 0x00B86420, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],

    // 2: CoolTint — icy blue rim + topbar; rest from preset
    [0x00000000, 0x0080C8FF, 0x004488CC, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],

    // 3: CoralTint — coral focus_surface + pink rim; rest from preset
    [0x00CC5566, 0x00FF8090, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000],

    // 4: GoldTint — gold rim + active tab; rest from preset
    [0x00000000, 0x00DDBB00, 0x00000000, 0x00DDBB00, 0x00000000, 0x00000000, 0x00000000, 0x00000000],
];
```

### Tint descriptions

| Index | Name | Slots overridden | Visual effect |
|-------|------|-----------------|---------------|
| 0 | Clear | none | Reverts to active preset (no override) |
| 1 | WarmTint | rim + topbar | Amber copper rim and bar over any preset |
| 2 | CoolTint | rim + topbar | Icy blue rim and bar over any preset |
| 3 | CoralTint | focus_surface + rim | Coral/pink surface and vivid rim |
| 4 | GoldTint | rim + active_tab | Gold rim and active tab highlight |

Semantic lights (close/minimize/zoom) = all zero in every tint → always kept from preset.

---

## State

```rust
static mut ACTIVE_TINT_IDX: u8 = 0; // 0 = clear, 1..4 = active tint
```

---

## State Transitions

```
Boot:
  SCENE_APPEARANCE_STATE.preset_idx = 0
  SCENE_APPEARANCE_STATE.use_custom_colors = 0
  ACTIVE_TINT_IDX = 0
  → send BottleGlass (no tint)

F5 (CycleRenderTokenPreset):
  SCENE_APPEARANCE_STATE.preset_idx = (idx + 1) % 4
  SCENE_APPEARANCE_STATE.use_custom_colors = 0   ← already clears custom
  ACTIVE_TINT_IDX = 0                            ← NEW: also reset tint index
  → resolve_scene_render_tokens() → push

F6 (CycleCustomTint):
  ACTIVE_TINT_IDX = (idx + 1) % TINT_COUNT
  if ACTIVE_TINT_IDX == 0:
    SCENE_APPEARANCE_STATE.use_custom_colors = 0
    (custom_colors left in place — will be overwritten on next tint)
  else:
    SCENE_APPEARANCE_STATE.custom_colors = CUSTOM_TINT_BUNDLES[ACTIVE_TINT_IDX]
    SCENE_APPEARANCE_STATE.use_custom_colors = 1
  → resolve_scene_render_tokens() → push
```

### Example session

```
Boot:       BottleGlass, tint=0 (clear)
F5:         VioletGlass, tint=0 (clear, tint reset)
F6:         VioletGlass + WarmTint (amber rim/bar over violet preset)
F6:         VioletGlass + CoolTint (icy blue rim/bar over violet preset)
F6:         VioletGlass + CoralTint (coral surface/rim over violet preset)
F6:         VioletGlass + GoldTint (gold rim/tab over violet preset)
F6:         VioletGlass + Clear (tint=0, preset restored)
F5:         GraphiteGlass, tint=0 (tint reset again)
F6:         GraphiteGlass + WarmTint
```

Tints compose with any preset. F5 always returns to clean preset.

---

## Helpers / Action Names

### New static

```rust
static mut ACTIVE_TINT_IDX: u8 = 0;
```

### New helper: apply_custom_tint_bundle

```rust
unsafe fn apply_custom_tint_bundle(idx: usize) {
    if idx == 0 {
        SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    } else {
        let bundle = &CUSTOM_TINT_BUNDLES[idx];
        // copy_from_slice is bounded; CUSTOM_TINT_BUNDLES[idx] is [u32; 8]
        for i in 0..8 {
            SCENE_APPEARANCE_STATE.custom_colors[i] = bundle[i];
        }
        SCENE_APPEARANCE_STATE.use_custom_colors = 1;
    }
}
```

Note: `copy_from_slice` requires `Clone` on elements and slice lengths to match. Since `u32: Copy` and both slices are `[u32; 8]`, a manual `for i in 0..8` loop is clearest and avoids any `Copy`/`Clone` machinery in `no_std`. Either works.

### New helper: cycle_custom_tint

```rust
unsafe fn cycle_custom_tint() {
    ACTIVE_TINT_IDX = (ACTIVE_TINT_IDX + 1) % TINT_COUNT as u8;
    apply_custom_tint_bundle(ACTIVE_TINT_IDX as usize);
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    unsafe {
        static mut TINT_BUDGET: u32 = 32;
        if TINT_BUDGET > 0 {
            TINT_BUDGET -= 1;
            serial_println!("[shell.appearance.custom] mode=tint tint={}", ACTIVE_TINT_IDX);
        }
    }
}
```

Budget 32: enough to cycle through all 5 tints × 4 presets × multiple times.

### Updated: cycle_scene_render_token_preset (F5)

Add one line to reset tint index:

```rust
unsafe fn cycle_scene_render_token_preset() {
    SCENE_APPEARANCE_STATE.preset_idx =
        (SCENE_APPEARANCE_STATE.preset_idx + 1) % PRESET_COUNT as u8;
    SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    ACTIVE_TINT_IDX = 0;  // ← new: reset tint on preset cycle
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);
    // existing marker unchanged
}
```

### New SurfaceAction variant

```rust
CycleCustomTint,  // F6
```

### New scancode mapping

```rust
0x40 => Some(SurfaceAction::CycleCustomTint),  // F6
```

### New dispatch arm

```rust
SurfaceAction::CycleCustomTint => {
    unsafe { cycle_custom_tint(); }
}
```

---

## Implementation File List

### Modified

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `TINT_COUNT`, `TintBundle`, `CUSTOM_TINT_BUNDLES`, `ACTIVE_TINT_IDX`; add `apply_custom_tint_bundle()`, `cycle_custom_tint()`; add `CycleCustomTint` to `SurfaceAction`; add `0x40` to `scancode_to_action`; add dispatch arm; update `cycle_scene_render_token_preset()` to reset `ACTIVE_TINT_IDX` |

### Created

| File | Role |
|------|------|
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_PLAN_V1.md` | This document |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | Implementation handoff (next phase) |

### NOT modified

| File | Reason |
|------|--------|
| `servers/sexdisplay/src/main.rs` | 0xFC handler already clamps any u32; no change needed |
| `crates/sex-pdx/src/lib.rs` | No new opcode |
| `sexos_build_spec.toml` | No ABI hash change (sex-pdx unchanged) |
| `kernel/` | Forbidden |
| All other listed forbidden files | Unchanged |

---

## Proof Markers

| Marker | When | Budget |
|--------|------|--------|
| `[shell.appearance.tokens.send] seq=2 sent` | Boot | 4 |
| `[shell.appearance.state] preset=N custom=N chrome=N access=N` | Boot | 1 |
| `[shell.appearance.preset] idx=N` | F5 pressed | 16 |
| `[shell.appearance.custom] mode=tint tint=N` | F6 pressed | 32 |
| `[sexdisplay.appearance.tokens] seq=1 applied=N` | Display committed | 4 |

---

## Build / Proof Commands

```bash
# Default build
./scripts/entrypoint_build.sh

# Synthetic build
env SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh

# Run with keyboard
env SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>&1 | tee /tmp/scene-custom-color-v1.log
```

### Verification (if run)

```bash
# F6 tint cycles triggered
grep -ac "\[shell.appearance.custom\]" /tmp/scene-custom-color-v1.log

# F5 preset cycles still work
grep -ac "\[shell.appearance.preset\]" /tmp/scene-custom-color-v1.log

# Display applied tokens
grep -ac "\[sexdisplay.appearance.tokens\].*applied" /tmp/scene-custom-color-v1.log

# No panics
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-custom-color-v1.log
```

### Pass criteria

- Default build passes
- Boot: BottleGlass (preset 0, tint 0)
- F5: cycles BottleGlass → VioletGlass → GraphiteGlass → HighContrast → BottleGlass; tint resets to 0 each time
- F6: cycles Clear → WarmTint → CoolTint → CoralTint → GoldTint → Clear; composites with active preset
- F5 after F6: returns to clean next preset (tint index reset)
- No panics, no geometry/focus/tab/topbar changes

---

## STOP Conditions

| Condition | Stop? | Mitigation |
|-----------|-------|------------|
| `0x40` conflicts with existing scancode | ❌ STOP | Audit shows 0x40 free. Verified by grep. |
| Tint table requires heap | ❌ STOP | Fixed `[u32; 8]` arrays, `static` keyword. |
| New opcode needed | ❌ STOP | Reuses `OP_APPEARANCE_TOKENS = 0xFC`. |
| sexdisplay changes needed | ❌ STOP | 0xFC handler clamps any u32; already general. |
| `resolve_scene_render_tokens()` needs refactor | ❌ None | Tint bundles use same zero-means-keep logic already implemented. |
| `copy_from_slice` not available in no_std | Mitigated | Use manual `for i in 0..8` loop instead. |
| Tint index OOB | ❌ STOP | `% TINT_COUNT` guarantees 0..4. |
| Semantic lights corrupted | ❌ None | All tints have zeros for slots 5/6/7 (close/minimize/zoom). |

---

## Pass Criteria

- [x] Verdict: SCENE_CUSTOM_COLOR_KEYS_SAFE_NOW
- [x] F6 scancode `0x40` confirmed free
- [x] Tint table designed (5 entries: 0=clear + 4 tints with exact u32 values)
- [x] Zero-slot semantics correct (resolve already treats zero as "keep preset")
- [x] Tint 0 = clear → `use_custom_colors = 0` (no override)
- [x] Semantic lights untouched in all tints (slots 5/6/7 = 0)
- [x] F5 resets `ACTIVE_TINT_IDX = 0` (tint cleared on preset cycle)
- [x] State transition table complete
- [x] All helpers named and pseudocoded
- [x] Implementation file list: silk-shell only
- [x] No sexdisplay, sex-pdx, or kernel changes
- [x] Proof markers documented
- [x] STOP conditions documented
- [x] Next phase: SCENE_CUSTOM_COLOR_KEYS_V1

---

## Next Phase: SCENE_CUSTOM_COLOR_KEYS_V1

Implement:

1. `servers/silk-shell/src/main.rs`:
   a. Add `TINT_COUNT: usize = 5` and `TintBundle = [u32; 8]` type alias
   b. Add `CUSTOM_TINT_BUNDLES: [TintBundle; TINT_COUNT]` static const table (5 entries above)
   c. Add `ACTIVE_TINT_IDX: u8 = 0` static mut
   d. Add `apply_custom_tint_bundle(idx: usize)` helper (manual copy loop, no copy_from_slice)
   e. Add `cycle_custom_tint()` helper with `[shell.appearance.custom]` marker (budget 32)
   f. Add `CycleCustomTint` to `SurfaceAction` enum
   g. Add `0x40 => Some(SurfaceAction::CycleCustomTint)` to `scancode_to_action()`
   h. Add `SurfaceAction::CycleCustomTint => { unsafe { cycle_custom_tint(); } }` dispatch arm
   i. Update `cycle_scene_render_token_preset()`: add `ACTIVE_TINT_IDX = 0;` after clearing `use_custom_colors`

2. Build: `./scripts/entrypoint_build.sh`

3. Verify: boot=teal, F5 cycles presets (tint resets), F6 applies tints over active preset, F5 after F6 returns to clean next preset.

4. Create `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md`.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | `SceneAppearanceState`, `resolve_scene_render_tokens()`, `custom_colors` |
| `docs/handoff/SCENE_RENDER_TOKEN_PRESETS_V1.md` | `TOKEN_PRESETS`, `PRESET_COUNT`, `push_token_preset()` |
| `docs/handoff/SCENE_RENDER_TOKENS_V1.md` | `OP_APPEARANCE_TOKENS = 0xFC`, two-call IPC |
| `docs/handoff/SCENE_SETTINGS_STORAGE_PLAN_V1.md` | Model split, custom override design intent |
| `servers/silk-shell/src/main.rs` | Current `SCENE_APPEARANCE_STATE`, `cycle_scene_render_token_preset()` |
