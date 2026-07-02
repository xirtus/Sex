# Atlas Theme Apply Visual Proof V1

## Status: PASS
Date: 2026-05-14
Attempts: 1

## Apply Path Root Cause

Atlas Enter/key-number apply emitted `[atlas.scene.apply] scene=N accent=N ok=1` but
the accent was only stored in `SCENES[].accent` — a local per-scene metadata field.
It was NEVER propagated to `ACTIVE_TINT_IDX` or the `SCENE_APPEARANCE_STATE` that
controls visible chrome rendering via `OP_APPEARANCE_TOKENS` to sexdisplay.

The accent value was used only in the Atlas overlay card rendering (`atlas_render_stub`)
to color card tiles. The actual shell chrome (frame top bars, tab colors, focus rings,
window decorations) was controlled by a completely separate system:
- `ACTIVE_TINT_IDX` (0=Clear, 1=Warm, 2=Cool, 3=Coral, 4=Gold)
- `CUSTOM_TINT_BUNDLES[ACTIVE_TINT_IDX]` → `SCENE_APPEARANCE_STATE.custom_colors`
- `push_token_preset()` → `OP_APPEARANCE_TOKENS` → sexdisplay

**Result**: `[atlas.scene.apply]` only recorded a selection — it did NOT change any
visible shell/chrome/theme state.

## Fix

Added `atlas_apply_scene_accent_to_chrome(scene_idx)` bridge function that propagates
the scene's accent token to the active chrome tint system:

1. Reads `SCENES[scene_idx].accent`
2. Sets `ACTIVE_TINT_IDX = accent`
3. Calls `apply_custom_tint_bundle(accent)` → updates `SCENE_APPEARANCE_STATE`
4. Calls `resolve_scene_render_tokens()` + `push_token_preset()` → sends `OP_APPEARANCE_TOKENS` to sexdisplay
5. Emits before/apply/after markers with `changed=N`

Integrated into:
- `switch_scene()` — called when Enter or number key selects a different scene
- Atlas Enter handler same-scene branch — accent may have changed via A/Z keys

The accent→tint mapping is 1:1:
- accent 0 = Clear (no custom colors, use preset)
- accent 1 = WarmTint (amber/copper rims)
- accent 2 = CoolTint (icy blue rims)
- accent 3 = CoralTint (pink rims)
- accent 4 = GoldTint (gold rims)

These match `CUSTOM_TINT_BUNDLES` indices exactly.

## Visual Theme Proof Table

| Stage | Action | ok | Reason |
|-------|--------|----|--------|
| 0 | open_focus (Atlas overlay) | 1 | ok |
| 1 | cycle_accent (A key → accent 0→1) | 1 | ok |
| 2 | apply_commit (Enter → accent→tint) | 1 | ok |
| 3 | verify_chrome_change (tint 0→1) | 1 | chrome_updated |
| 4 | close_back (Escape) | 1 | ok |

## Runtime Proof Counts

```
[atlas.theme.before]           scene=0 accent=0 tint=0 preset=0          (initial)
[atlas.theme.before]           scene=0 accent=1 tint=0 preset=0 custom=0 (after accent cycle)
[atlas.theme.apply]            old_accent=0 new_accent=1 ok=1 reason=applied
[atlas.theme.after]            scene=0 accent=1 tint=1 preset=0 custom=1 changed=1
[atlas.theme.visual.proof]     stage=0-4 all ok=1
[atlas.theme.visual.proof.done] ok=1
[atlas.scene.apply]            scene=0 accent=1 ok=1 reason=ok
```

- `atlas.theme.before`: 2
- `atlas.theme.apply`: 2 (bridge function + proof marker)
- `atlas.theme.after`: 2
- `atlas.theme.visual.proof` stages: 5 (0-4)
- `atlas.theme.visual.proof.done`: 1
- faults: 0

Key result: `changed=1` — tint went from 0 (Clear) to 1 (Warm), proving the accent
now propagates to visible chrome state.

## Files Changed

`servers/silk-shell/src/main.rs`
- Added `ATLAS_THEME_VISUAL_PROOF_ENABLED` const (gated on `SEXOS_ATLAS_THEME_VISUAL_PROOF`)
- Added `ATLAS_THEME_VISUAL_PROOF_DONE` static flag
- Added `atlas_apply_scene_accent_to_chrome(scene_idx)` bridge function (~25 lines)
- Integrated accent→tint call into `switch_scene()` (line ~6964)
- Integrated accent→tint call into Atlas Enter same-scene branch (line ~6593)
- Added `maybe_run_atlas_theme_visual_proof()` proof function (~80 lines)
- Added proof call site in main loop

## Build Results
```
SEXOS_ATLAS_THEME_VISUAL_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS
```

## Notes
- No sex-pdx/ABI edits. No kernel edits.
- No sexusb/sexinput/sexdisplay edits — accent propagation uses existing
  `OP_APPEARANCE_TOKENS` (0xFC) protocol already supported by sexdisplay.
- No Quil edits. No pointer work.
- No broad visual redesign — accent tokens map 1:1 to existing tint bundles.
- The accent→tint propagation happens on Enter (Atlas confirm) only, not on
  every A/Z keystroke. This avoids flooding sexdisplay with token updates
  during accent cycling.
- `ACCENT_DEFAULT` (0) sets `use_custom_colors = 0`, reverting to the preset;
  all other accents set `use_custom_colors = 1` with custom color overrides.
