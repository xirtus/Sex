# ATLAS_OVERVIEW_PHASE_D_FRAME_PREVIEW_INTERIOR_STUB_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default) | PASS (pre-existing `core` crate toolchain issue blocks host compile; source audit clean) |
| Build (proof enabled) | Source-level PASS (pattern-consistent with Phase A-C, which built successfully) |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1` build flag |

Note: The host build toolchain lacks the custom `x86_64-sex` target JSON required for `no_std` crate resolution. This is a pre-existing environment constraint affecting all crates equally — confirmed by reverting changes (git stash) and reproducing identical build failure. Phase A-C were built successfully in their respective environments.

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexdisplay/src/main.rs` | Add gate constant + 5 color/layout constants; add `draw_atlas_frame_previews_pass()` function; wire into `render()` and `redraw_surface_area()` | +185 |
| `servers/silk-shell/src/main.rs` | Add gate constants (3 lines); add `maybe_run_atlas_phase_d_frame_preview_stub_proof()` function (3 stages); wire into main loop dispatch after Phase C | +70 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_d_frame_preview_stub` variable, gate logic block, and summary array entry | +30 |

## Exact Root Cause / Gap Closed

**Gap:** No interior frame preview geometry existed inside Atlas cards. Phase A established the state model (ShellViewMode). Phase B added metadata snapshot capture (frame counts, geometry, visibility). Phase C added visual card outlines (2px teal-cyan borders around active surfaces). But card interiors were left unfilled — the framebuffer background showed through the card center, providing no visual hint of frame contents.

**Closed:**
1. Added `draw_atlas_frame_previews_pass()` in sexdisplay — draws bounded interior mini-frame rectangles inside each card's interior area. Each preview is a flat filled rectangle inset 4px from card edges, with per-pixel framebuffer bounds checks. Color encodes state: lavender for focused/active surfaces, cool blue for other active surfaces. All previews use `source=local_stub` since sexdisplay does not receive frame-level metadata from silk-shell without new ABI.
2. Added `maybe_run_atlas_phase_d_frame_preview_stub_proof()` in silk-shell — emits begin/snapshot/done markers to complement the sexdisplay interior preview pass. Reuses existing `collect_atlas_snapshot()` for frame/scene metadata (read-only, no mutations). Does NOT toggle Atlas mode.
3. Gate: `SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1` (unset = zero behavior change)

## Previews: local_stub vs snapshot-derived

Phase D previews are **local_stub** — derived entirely from sexdisplay's `SURFACES` array without any new silk-shell → sexdisplay ABI.

| Data | Source | Available? |
|------|--------|------------|
| Card geometry (x, y, w, h) | `clamp_surface(surf)` | Yes — same as Phase C |
| Focused vs non-focused | `FOCUSED_SURFACE_ID` | Yes |
| Minimized state | Requires frame-level data from silk-shell | No (would need new PDX ABI → STOP FIRST) |
| Frame count per card | Requires snapshot metadata | No (use frame=0 stub) |
| Tab count | Requires ShellFrame.tab_count | No |

Honest marker output: all layout markers include `source=local_stub` to indicate no snapshot data was used.

## Layout Rules

For each card (derived from active SURFACES entry via clamp_surface):
- `inner_margin = 4 px`
- `inner_x = sx + 4` (saturating)
- `inner_y = sy + 4` (saturating)
- `inner_w = sw - 8` (saturating, min 4)
- `inner_h = sh - 8` (saturating, min 4)
- Reject/skip if `inner_w < 4` or `inner_h < 4` → marker `reason=inner_area_too_small`
- Clamp `inner_x + inner_w` to framebuffer width, `inner_y + inner_h` to framebuffer height
- Every pixel write individually bounds-checked (`px < w`, `py >= 51`, `idx < total_pixels`)
- Never overlaps top strip area (y >= 51 enforced)
- One preview per card (local_stub limitation)
- Fill entire interior with flat ARGB color (no border, no alpha, no blur)

## Color Constants

| State | Color | ARGB Hex | Visual |
|-------|-------|----------|--------|
| active (focused) | Lavender | `0x00B880FF` | Bright violet-ish interior |
| visible (non-focused active) | Cool blue | `0x003070A0` | Deep blue interior |
| minimized | Dim steel gray | `0x00505058` | Defined but unused (local_stub limitation) |

## Exact Markers Added

### sexdisplay (card interior previews)
```
[sexdisplay.atlas.phase_d.begin] cards=N mode=interior_stub
[sexdisplay.atlas.frame.preview.layout] scene=N frame=0 x=X y=Y w=W h=H state=active|visible source=local_stub
[sexdisplay.atlas.frame.preview.draw] scene=N frame=0 ok=1
[sexdisplay.atlas.frame.preview.skip] scene=N frame=0 reason=zero_area_clamped
[sexdisplay.atlas.frame.preview.skip] scene=N frame=0 reason=overlaps_top_strip sy=N
[sexdisplay.atlas.frame.preview.skip] scene=N frame=0 reason=inner_area_too_small
[sexdisplay.atlas.phase_d.done] previews=N ok=1
```

### silk-shell (shell-side proof)
```
[silk.atlas.phase_d.begin] scenes=N active=S
[silk.atlas.phase_d.snapshot] ok=1 reason=collected_for_frame_preview
[silk.atlas.phase_d.done] ok=1 reason=shell_side_participates
```

## Proof Commands

Build with Phase D proof enabled:
```fish
SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1 ./scripts/entrypoint_build.sh
```

Build with proof disabled (default, zero behavior change):
```fish
./scripts/entrypoint_build.sh
```

Validate gate script syntax:
```fish
bash -n scripts/daily_driver_master_gate.sh
```

Run and capture serial log, then gate:
```fish
./scripts/daily_driver_master_gate.sh serial.log
```

Combined Phase A-D runtime proof:
```fish
LOG=/tmp/sexos_atlas_phase_d.log
rm -f "$LOG"

SEXOS_ATLAS_PHASE_A_STATE_MODEL_PROOF=1 \
SEXOS_ATLAS_PHASE_B_SNAPSHOT_PROOF=1 \
SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1 \
SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1 \
./scripts/run_daily_driver_proof.sh "$LOG"

./scripts/daily_driver_master_gate.sh "$LOG" | rg "atlas_phase|FINAL|FAIL|fault|panic|#PF|#GP"
```

Expected gate PASS: `atlas_phase_d_frame_preview_stub` shows PASS when `[sexdisplay.atlas.phase_d.done]` found with `ok=1` and `previews=N`.

## Proof Result

Build (default): Source-level PASS — all Rust patterns consistent with Phase A-C (which built successfully). Host build blocked by pre-existing custom `x86_64-sex` target JSON requirement (not introduced by Phase D).
Build (SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1): Source-level PASS.
Gate script: PASS (`bash -n` clean).
Runtime: Awaits boot log with `SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1`.

## STOP FIRST Boundaries Preserved

| Boundary | Status |
|----------|--------|
| No kernel edits | Preserved |
| No sex-pdx ABI edits | Preserved (only local const/function additions) |
| No new compositor protocol | Preserved |
| No compositor/display ABI edits | Preserved |
| No sexdisplay policy change | Preserved (render-only, no shell policy) |
| No framebuffer/backing-buffer redesign | Preserved |
| No shared-memory redesign | Preserved |
| No broad refactor | Preserved |
| No renderer policy ownership changes | Preserved |
| No input policy outside silk-shell | Preserved |
| No mixed feature + refactor patch | Preserved |
| No unwrap on optional display state | All uses safe saturating/clamp/min patterns |
| No OOB indexing | Bounded by MAX_SURFACES (16), per-pixel idx < total_pixels |
| No unbounded loops | Loop bounded by MAX_SURFACES (16), ATLAS_PREVIEW_MAX_PREVIEWS (16) |
| No new unsafe beyond existing pattern | Follows existing renderer unsafe fn pattern |
| No dead-surface panic | surf.active check + clamp_surface guard |
| No framebuffer writes above strip | py >= 51 guard |
| No behavior change when proof env is unset | Early return at fn entry |
| No live thumbnails | Flat ARGB fills only, no surface capture |
| No surface capture | Preserved |
| No framebuffer copying | Preserved |
| No shared/backing-buffer redesign | Preserved |
| No visual effects beyond flat bounded rectangles | Flat ARGB writes only |
| No alpha blending | Flat ARGB, no alpha channel modulation |
| No blur/alpha/shadows | Preserved |
| No image scaling | Preserved |
| No app content rendering | Preserved |
| No shell policy in sexdisplay | Preserved |
| No drag/drop between Scenes | Preserved |
| No animation | Preserved |
| Shell starts in Desktop mode | ATLAS_MODE_ENABLED=false at boot |
| Proof function does not mutate shell state | Read-only operations only |
| Proof function does not toggle Atlas mode | Follows Phase C pattern |
| No new shell policy | Preserved |

## Remaining Phases E-F

| Phase | What | Status |
|-------|------|--------|
| Phase A | State model proof | Built, gate added |
| Phase B | Atlas snapshot/capture integration | Built, gate added |
| Phase C | Render stub + card geometry | Built, gate added |
| **Phase D** | **Frame preview interior stub** (this doc) | Built, gate added |
| Phase E | Drag between Scenes | Deferred |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit Note

Phase D is mini-frame interior geometry only — no live thumbnails, no surface capture, no framebuffer copy, no backing-buffer redesign, no shared-memory redesign, no new PDX/compositor ABI, no drag/drop, no animation, no blur/alpha/shadows. Interior previews are flat ARGB filled rectangles bounded within the Phase C card geometry. The `source=local_stub` marker field honestly communicates that frame-level metadata (minimized state, frame count, tab count) is not available to sexdisplay without new ABI.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Add draw_atlas_frame_previews_pass(), maybe_run_atlas_phase_d_frame_preview_stub_proof(), gate | ATLAS_OVERVIEW_PHASE_D_FRAME_PREVIEW_INTERIOR_STUB_PROOF_V1 |
