# ATLAS_THEME_PRESETS_KEYBOARD_V1

Date: 2026-05-14
Scope:
- `servers/silk-shell/src/main.rs`

## Problem
Atlas had accent-per-scene keyboard cycling (A/Z keys) but no way to cycle
the global render token preset (Default/Warm/Cool/HighContrast) from within
Atlas. Users had to use F5 (CycleRenderTokenPreset) outside Atlas.

## Fix Implemented

### 1. Proof Gate
- `SEXOS_ATLAS_THEME_PRESETS_PROOF=1` build-time gate
- `ATLAS_THEME_PRESETS_PROOF_ENABLED` / `ATLAS_THEME_PRESETS_PROOF_DONE`
- Follows existing Atlas proof pattern (single-call, not staged)

### 2. PRESET_NAMES Constant
```rust
static PRESET_NAMES: [&str; PRESET_COUNT] = [
    "Default",      // 0: BottleGlass (default teal)
    "Warm",         // 1: VioletGlass (purple)
    "Cool",         // 2: GraphiteGlass (dark neutral)
    "HighContrast", // 3: HighContrast (accessibility)
];
```
Helper: `get_preset_name(idx: u8) -> &'static str`

### 3. Preset Cycling (Atlas Keyboard Handler)
- **'S' key (0x1F)**: next preset — calls `cycle_scene_render_token_preset()`
  - Marker: `[atlas.preset.nav] old=N new=N name=NAME`
- **'W' key (0x11)**: prev preset — calls new `cycle_prev_scene_render_token_preset()`
  - Wraps backward: 0 → PRESET_COUNT-1
  - Marker: `[atlas.preset.nav] old=N new=N name=NAME`
- Both persist to sexstore (fire-and-forget PUT)
- Both re-render Atlas cards via `atlas_render_stub()`

### 4. Apply Marker
- Pressing Enter (0x1C) from Atlas already applies the scene + accent
- Added: `[atlas.preset.apply] idx=N name=NAME ok=1`

### 5. Proof Function
`maybe_run_atlas_theme_presets_proof()` exercises the full cycle:
1. Opens Atlas
2. S key → next preset (0→1, Default→Warm)
3. S key → next preset (1→2, Warm→Cool)
4. W key → prev preset (2→1, Cool→Warm)
5. Enter → apply (scene + preset)
6. Reopen Atlas, verify preset persisted
7. Close Atlas

### 6. Preserved Constraints
- No kernel edits
- No ABI/sex-pdx edits
- No USB/display edits
- No renderer redesign
- silk-shell only + docs
- Zero behavior change when `SEXOS_ATLAS_THEME_PRESETS_PROOF` is unset

## Markers
| Marker | Meaning |
|--------|---------|
| `[atlas.preset.nav] old=N new=N name=NAME` | Preset cycle navigation (S/W keys) |
| `[atlas.preset.apply] idx=N name=NAME ok=1` | Preset committed on Enter/1-5 |
| `[atlas.preset.before] preset=N name=NAME` | State snapshot before proof cycle |
| `[atlas.preset.proof] stage=N action=... ok=...` | Per-stage proof gate output |
| `[atlas.preset.proof.done] ok=1` | Proof complete with ok=1 |

## Build
```
SEXOS_ATLAS_THEME_PRESETS_PROOF=1 ./scripts/entrypoint_build.sh
./scripts/entrypoint_build.sh                          # baseline (zero change)
```

## Runtime
```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_atlas_theme_presets_keyboard_v1.log \
  -display none -no-reboot -no-shutdown || true
```

## Grep
```
grep -E "atlas.preset|atlas.theme|atlas.scene|fault.kill|#PF|#GP|panic|KERNEL PANIC" \
  /tmp/sexos_atlas_theme_presets_keyboard_v1.log | tail -2200
```

## Pass Criteria
- `[atlas.preset.nav]` entries for next (S) and prev (W) keys
- `[atlas.preset.apply]` after Enter
- `[atlas.preset.proof.done] ok=1`
- faults=0 (no fault.kill, #PF, #GP, panic, KERNEL PANIC)
