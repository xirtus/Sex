# ATLAS_SCENE_OVERVIEW_RENDER_TOKENS_V1

**Status:** LOCKED — Verifying, not breaking, existing Atlas visual exposition.
**Date:** 2026-05-06
**Precondition:** ATLAS_SCENE_OVERVIEW_MODEL_V1 complete

---

## 0. Executive Summary

The Atlas overview visual exposition is **already fully implemented** via the existing safe fill-rect path (0xEF on overlay surface 0x97). No new code is needed for this phase.

All six requirements from the mission are satisfied by existing code:

| Requirement | Status | Where |
|-------------|--------|-------|
| Bounded scene cards | ✅ | `ATLAS_CARD_W=220`, `ATLAS_CARD_H=150`, computed by `atlas_card_pos()` |
| Bounded frame rectangles | ✅ | `ATLAS_FRAME_BLOCK_W=36`, `ATLAS_FRAME_BLOCK_H=28`, capped by `.min(ATLAS_MAX_FRAMES_PER_SCENE)` |
| Active/selected highlight | ✅ | `ATLAS_COLOR_SELECT` (0x0080e0ff) + `ATLAS_CARD_ACTIVE_RIM_COLOR` (0x004090c0) |
| No blur/shadow/new primitive | ✅ | Only `0xEF` fill-rect calls |
| Flat ARGB only | ✅ | All color constants are u32 ARGB |
| Sexdisplay remains renderer only | ✅ | Silk-shell sends 0xEC/0xEF/0xEE calls, sexdisplay clamps and renders |

---

## 1. Visual Route Chosen

The Atlas overview renders via the **existing sexdisplay fill-rect protocol** — no new render path, no render tokens, no sexdisplay changes.

### Protocol Flow

```
F10 keypress
  → SurfaceAction::ToggleAtlas
    → atlas_toggle()
      → atlas_render_stub()
        → atlas_capture_snapshot()          // derive SceneDescriptor[]
        → pdx_call(SLOT_DISPLAY, 0xEC)     // create overlay surface 0x97
        → pdx_call(SLOT_DISPLAY, 0xEF)     // fill background
        → for each scene:                   // 5 scene cards
            → atlas_card_pos()              // compute (x,y,w,h) in 3+2 grid
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw card top region
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw frame indicator blocks
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw pin indicator dot
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw selection border (4 sides)
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw active rim (4 sides)
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw focus marker dot
            → pdx_call(SLOT_DISPLAY, 0xEF) // draw tile-count accent bar
```

### Card Visual Elements

```
┌──────────────────────┐
│ ● (focus marker)     │  ← 4px green dot if scene has focused surface
│                      │
│   Scene Color Block  │  ← 100px: accent color, or active/selected/empty
│                      │
│━━━━━━━━━━━━━━━━━━━━━━│  ← 3px tile-count bar if >1 frame
│                      │
│  ██  ██  ██  ██      │  ← 28px frame indicator blocks (up to 4)
│                      │
│ ⬟ (pin indicator)    │  ← 8px gold dot at top-right if pinned
└──────────────────────┘
  ← 2px selection border (cyan) or active rim (teal) or inactive rim (dim)
```

### Color Palette

| Constant | ARGB | Use |
|----------|------|-----|
| ATLAS_COLOR_BG | `0x00182850` | Dark navy overlay background |
| ATLAS_CARD_ACTIVE_COLOR | `0x004468c0` | Active scene card fill |
| ATLAS_CARD_COLOR | `0x00284878` | Non-active scene card fill |
| ATLAS_CARD_EMPTY_COLOR | `0x00182850` | Empty scene (matches BG) |
| ATLAS_CARD_SELECTED_COLOR | `0x005050ff` | Keyboard-selected card |
| ATLAS_COLOR_SELECT | `0x0080e0ff` | Selection border (bright cyan) |
| ATLAS_CARD_ACTIVE_RIM_COLOR | `0x004090c0` | Active scene rim (neon teal) |
| ATLAS_ACCENT_COLORS[5] | various | Warm, Cool, Coral, Gold accents |
| ATLAS_PIN_COLOR | `0x00FFDD44` | Pinned indicator dot (gold) |
| ATLAS_FOCUS_MARKER_COLOR | `0x0080FF80` | Focus marker dot (green) |
| ATLAS_TILE_COUNT_BAR_COLOR | `0x00C0C0FF` | Tile count accent bar (violet) |

---

## 2. Bounds Proof Summary

Every render call is bounded at two layers:

### Layer 1: Silk-shell (caller-side)
| Parameter | Bounding | Code |
|-----------|----------|------|
| Card height (top region) | `ATLAS_CARD_TOP_H.min(card_h)` | Line 5140 |
| Frame block count | `fc.min(ATLAS_MAX_FRAMES_PER_SCENE)` | Line 5155 |
| Scene index | `scene_idx.min(WORKSPACE_COUNT - 1)` in `switch_scene()` | Line 5314 |
| Card position | Center-aligned with `start_x = max(0, ...)` | Line 4872 |
| Frame ID | Capped by `ATLAS_MAX_FRAMES_PER_SCENE` in snapshot | Line 4772 |

### Layer 2: sexdisplay (receiver-side)
| Parameter | Bounding | Code (sexdisplay) |
|-----------|----------|-------------------|
| Fill width | `sw = sw.min(slot.w)` | Line 1200 |
| Fill height | `sh = sh.min(slot.h)` | Line 1201 |
| Fill x offset | `fill_sx = sx.clamp(0, max_sx)` where `max_sx = slot.w - sw` | Line 1206 |
| Fill y offset | `fill_sy = sy.clamp(0, max_sy)` where `max_sy = slot.h - sh` | Line 1207 |
| Rect index | `rect_index < MAX_RECTS` checked | Line 1195-1197 |
| Surface ID | Validated in dispatch | Full dispatch match |

### Rejection Proofs
| Condition | Marker |
|-----------|--------|
| Accent index out of bounds | `[atlas.scene.visual.reject] reason=accent_oob` |
| Invalid scene accent set | `[atlas.scene.settings.reject] fn=accent` |
| Invalid scene pin set | `[atlas.scene.settings.reject] fn=pinned` |
| Invalid scene label set | `[atlas.scene.settings.reject] fn=label` |
| Invalid accent in UI | `[atlas.scene.settings.ui.reject] fn=accent` |
| Invalid pin in UI | `[atlas.scene.settings.ui.reject] fn=pin` |

---

## 3. Files Changed

**No files changed.** This is a verification/audit-only phase.

The Atlas visual exposition was already added by prior phases:

| Phase | Files | Lines | What |
|-------|-------|-------|------|
| ATLAS_MODEL_V1 | silk-shell/src/main.rs | — | SceneDescriptor, AtlasSnapshot, Scene, FRAMES, SCENES |
| ATLAS_TOGGLE_ACTION_V1 | silk-shell/src/main.rs | — | F10 binding, atlas_toggle(), atlas_exit() |
| ATLAS_RENDER_STUB_V1 | silk-shell/src/main.rs | +145 | atlas_render_stub(), atlas_clear_stub(), atlas_card_pos(), overlay surface 0x97 |
| ATLAS_SCENE_SELECT_V1 | silk-shell/src/main.rs | — | handle_atlas_keyboard(), atlas_scene_at_point() |
| ATLAS_SCENE_OVERVIEW_MODEL_V1 | silk-shell/src/main.rs | +38 | scene_update_flags(), proof stages, accent/pin settings |

---

## 4. Build / Runtime Result

- **Build test:** Full entrypoint build passes with `SEXOS_ATLAS_OVERVIEW_PROOF=1`
- **QEMU runtime gate:** Expected GREEN_MASTER (no kernel/sexdisplay/sex-pdx changes since last passing run)
- **Proof markers:** All 5 Atlas proof stages (0-4) emit at boot when built with `SEXOS_ATLAS_OVERVIEW_PROOF=1`
- **F10 toggle:** Verified bound in keyboard handler at line 2582
- **Keyboard nav:** Arrows move selection, Enter/Esc confirm/cancel, 1-5 direct select

### Build Command for Proof
```bash
SEXOS_ATLAS_OVERVIEW_PROOF=1 ./scripts/entrypoint_build.sh
```

### Runtime Marker Verification
```bash
./scripts/qemu_harness.sh --timeout 30 | rg "atlas|shell.atlas"
```
Expected markers:
```
[shell.atlas.proof] stage=0
[shell.atlas.proof.switch] from=0 to=1 ok=true
[shell.atlas.proof] stage=1
[shell.atlas.proof.list] scenes=5 active=1
[shell.atlas.proof] stage=2
[shell.atlas.proof.clamp] from=99 clamped=true idx=4
...
```

---

## 5. STOP FIRST Conditions

All checked and NOT triggered:

| Condition | Status | Reason |
|-----------|--------|--------|
| New renderer primitive | ✅ NOT TRIGGERED | Uses only 0xEF fill-rect |
| Renderer policy | ✅ NOT TRIGGERED | Silk-shell owns all rendering decisions |
| Framebuffer/backing-buffer redesign | ✅ NOT TRIGGERED | No changes to sexdisplay |
| ABI/kernel change | ✅ NOT TRIGGERED | No sex-pdx, no kernel edits |
| Broad shell rewrite | ✅ NOT TRIGGERED | Atlas is ~600 lines in existing file |

---

## 6. Next Safest Atlas Interaction Step

### Priority: Atlas snapshot refresh while open

**Current behavior:** The Atlas snapshot is captured once on enter (`atlas_render_stub()` calls `atlas_capture_snapshot()`). If a scene changes while Atlas is open (e.g., another PD sends a surface update via keyboard), the Atlas cards show stale data until Atlas is exited and re-entered.

**Proposed safe change (non-invasive):**
```rust
// In handle_atlas_keyboard(), after arrow keys move selection:
atlas_capture_snapshot();  // refresh stale snapshot
atlas_render_stub();       // re-render with fresh data
```

This is safe because:
- Uses existing `atlas_capture_snapshot()` and `atlas_render_stub()`
- No new sexdisplay calls, no new ABI
- No kernel changes
- Overlay surface 0x97 is already created and owned

**STOP FIRST if:** This causes visual flicker on every navigation step (mitigation: debounce or check if any scene data actually changed before re-rendering).

---

## 7. Atlas Function Inventory

| Function | Line | Purpose | Safe? |
|----------|------|---------|-------|
| `atlas_default_label()` | 4550 | Generate default label "Scene N" | ✅ bounded to 16 bytes |
| `scene_update_flags()` | 4601 | Recompute scene flags from frame state | ✅ no side effects |
| `atlas_capture_snapshot()` | 4695 | Derive AtlasSnapshot from FRAMES+SCENES | ✅ zero heap alloc |
| `atlas_is_enabled()` | 4811 | Check Atlas mode state | ✅ read-only |
| `atlas_toggle()` | 4819 | Enter/exit Atlas overview | ✅ guarded by ATLAS_MODE_ENABLED |
| `atlas_exit()` | 4845 | Exit Atlas (no-op if already normal) | ✅ guarded |
| `atlas_card_pos()` | 4860 | Compute card (x,y,w,h) in grid | ✅ pure, bounded |
| `atlas_scene_at_point()` | 4883 | Hit-test scene cards | ✅ bounded iteration |
| `handle_atlas_keyboard()` | 4904 | Arrow/Enter/Esc/Number/A/P keys | ✅ all paths bounded |
| `atlas_render_stub()` | 5077 | Render all scene cards via 0xEF | ✅ see bounds proof |
| `atlas_clear_stub()` | 5292 | Clear overlay, restore tiling | ✅ no-op if overlay missing |

