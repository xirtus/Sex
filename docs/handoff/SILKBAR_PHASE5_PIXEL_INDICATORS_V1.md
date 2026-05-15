# SILKBAR_PHASE5_PIXEL_INDICATORS_V1

Date: 2026-05-15
Status: PASS
Scope: servers/sexdisplay/src/main.rs only — tiny bounded pixel indicators, zero layout redesign

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | Three tiny pixel indicators drawn inside SilkBar strip. 17/17 gates PASS. Zero faults. |
| **STOP FIRST** | No kernel/sex-pdx/silk-shell/silkbar edits. No layout redesign. No FB bounds changes. y<50 preserved. |

## 2. Attempts Used

| Attempt | Result | Notes |
|---------|--------|-------|
| 1 | PASS | All three indicators draw correctly within existing bar dispatch |

## 3. Draw Path Audit

```
redraw_top_strip(fb, w, h, bar)
  ├── clock_fg_at(x, y, bar)          → clock digits
  ├── bell_badge_at(x, y, bar)        → bell badge
  ├── phase5_active_app_at(x, y, bar) → [NEW] 4x4 app dot
  ├── phase5_tint_swatch_at(x, y, bar)→ [NEW] 4x4 tint swatch
  ├── phase5_palette_dot_at(x, y, bar)→ [NEW] 3x3 palette dot
  └── bar_color(x, y, bar)            → bar background + existing modules
```

- **No renderer redesign**: indicators plug into existing pixel dispatch as `Option<u32>` layers
- **y<50 preserved**: all indicators render within the SilkBar top strip region
- **FB bounds preserved**: same `total_pixels` check applies
- **No text drawing**: numeric surface IDs → colored dots (safe, no font dependencies)
- **Accent palette**: 8-entry const array matching Atlas accent colors

## 4. Indicator Table

| Indicator | Type | Position | Size | Color | Active When |
|-----------|------|----------|------|-------|------------|
| Active app | Colored dot | x=155, y=22 | 4×4 px | Surface ID → color (Linen=green, Quil=mauve, etc.) | `active_app_sid != 0` |
| Tint swatch | Colored square | x=1045, y=22 | 4×4 px | Accent index → palette color | Always (shows current tint) |
| Palette dot | Green/dark dot | x=1060, y=23 | 3×3 px | `0x44FF44` (bright green when open) / `0x224422` (dim when closed) | Always (shows open/closed) |

### Surface ID → Color Mapping

| SID | App | Color |
|-----|-----|-------|
| 200 | Linen | Green `0xA6E3A1` |
| 201 | Quil | Mauve `0xCBA6F7` |
| 202 | Mesh | Teal `0x94E2D5` |
| 203 | Collar | Peach `0xFAB387` |
| 204 | Bell | Red `0xF38BA8` |
| 153 | Spindle | Blue `0x89B4FA` |
| 0 | (none) | Invisible |
| other | Unknown | Dim gray `0x444444` |

### Accent Palette

| idx | Color | Name |
|-----|-------|------|
| 0 | `0x89B4FA` | Blue |
| 1 | `0xA6E3A1` | Green |
| 2 | `0xF9E2AF` | Yellow |
| 3 | `0xFAB387` | Peach |
| 4 | `0xF38BA8` | Red |
| 5 | `0xCBA6F7` | Mauve |
| 6 | `0xF5C2E7` | Pink |
| 7 | `0x94E2D5` | Teal |

## 5. Files Changed

- `servers/sexdisplay/src/main.rs` — additive pixel indicators:
  - Added `SILKBAR_PHASE5_PIXEL_PROOF_ENABLED` compile-time gate
  - Added `app_indicator_color()` — const fn mapping surface ID → RGB
  - Added `accent_swatch_color()` — const fn mapping accent index → RGB
  - Added `phase5_active_app_at()` — 4×4 active app dot (x=155, y=22)
  - Added `phase5_tint_swatch_at()` — 4×4 tint swatch (x=1045, y=22)
  - Added `phase5_palette_dot_at()` — 3×3 palette dot (x=1060, y=23)
  - Hooked into `redraw_top_strip()` pixel dispatch (3 new layers)
  - Added `[sexdisplay.silkbar.phase5.draw]` budgeted marker
- `docs/handoff/SILKBAR_PHASE5_PIXEL_INDICATORS_V1.md` — this handoff

## 6. Build/Proof Result

```
Phase2+3+5 build → PASS
Baseline build → PASS (zero behavior change, zero pixel change when gate off)
Daily driver proof → 17/17 PASS, 0 faults
```

## 7. Runtime Proof Counts

From 30s headless QEMU boot:

```
[sexdisplay.silkbar.phase5.draw] active=201 tint=0 palette_open=0
[sexdisplay.silkbar.phase5.draw] active=202 tint=0 palette_open=0
[sexdisplay.silkbar.phase5.draw] active=100 tint=0 palette_open=0
```

| Metric | Count |
|--------|-------|
| Phase 2 sends | 7 |
| Phase 3 receives | 7 |
| Phase 5 draw markers | 8 (budgeted) |
| Faults | **0** |
| Daily driver gates | **17/17 PASS** |

## 8. Preserved Constraints

- No kernel edits
- No sex-pdx edits
- No silk-shell/silkbar edits
- No layout redesign (indicators are 3-4px dots, no geometry changes)
- No FB bounds changes (y<50 region preserved)
- No text drawing (safe colored dots only)
- No renderer redesign (plug-in Option<u32> layers)
- Baseline build has zero pixel change
- Zero faults

## Handoff Path

```
docs/handoff/SILKBAR_PHASE5_PIXEL_INDICATORS_V1.md     ← THIS DOCUMENT
docs/handoff/SILKBAR_ABI_PHASE3_RECEIVE_RENDER_V1.md     ← Phase 3 receive
docs/handoff/SILKBAR_ABI_PHASE2_SHELL_SEND_V1.md         ← Phase 2 producer
docs/handoff/SILKBAR_ABI_PHASE1_MODEL_V1.md              ← Phase 1 model
docs/handoff/SILKBAR_ABI_EXTENSION_PLAN_V1.md             ← design authority
```

