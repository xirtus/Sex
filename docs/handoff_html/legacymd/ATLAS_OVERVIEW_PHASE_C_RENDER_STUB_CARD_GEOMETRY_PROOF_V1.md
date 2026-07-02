# ATLAS_OVERVIEW_PHASE_C_RENDER_STUB_CARD_GEOMETRY_PROOF_V1

## Result: PASS BUILT — gate awaits runtime proof

## Status

| Field | Value |
|-------|-------|
| Build (default) | PASS (`[SEXOS ENTRYPOINT] success`) |
| Build (proof enabled) | PASS (`[SEXOS ENTRYPOINT] success`) |
| Gate script syntax | PASS (`bash -n` clean) |
| Runtime proof | Requires `SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1` build flag |

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexdisplay/src/main.rs` | Add gate constant + `draw_atlas_cards_pass()` function; wire into `render()` and `redraw_surface_area()` | +130 |
| `servers/silk-shell/src/main.rs` | Add gate constants + `maybe_run_atlas_phase_c_render_stub_proof()` function; wire into main loop dispatch | +85 |
| `scripts/daily_driver_master_gate.sh` | Add `gate_atlas_phase_c_render_stub` variable, gate logic, and summary array entry | +18 |

## Exact Root Cause / Gap Closed

**Gap:** No Atlas card geometry rendering existed. Phase A established the state model (ShellViewMode enum, mode toggle awareness). Phase B added metadata snapshot capture (frame counts, geometry, visibility). But no visual card outlines were drawn on the framebuffer during Atlas mode rendering.

**Closed:**
1. Added `draw_atlas_cards_pass()` in sexdisplay — draws bounded 2px border outlines around each active surface in the below-bar area (y>=51). Cards use a flat teal-cyan border color (`0x0089DCE6`). Each card is individually clamped via `clamp_surface()` with per-pixel bounds checks.
2. Added `maybe_run_atlas_phase_c_render_stub_proof()` in silk-shell — emits begin/done markers and delegates to the existing snapshot collector for honest scene/frame counts. Shell-side proof does NOT toggle Atlas mode — operates on whatever mode is current.
3. Gate: `SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1` (unset = zero behavior change)

## Card Geometry Rules

- 2px border outline (`ATLAS_CARD_BORDER_PX = 2`)
- Flat teal-cyan color (`ATLAS_CARD_BORDER_COLOR = 0x0089DCE6`)
- Clamped to framebuffer bounds via existing `clamp_surface()`
- Only active surfaces rendered (cursor/launcher surfaces skipped)
- Cards must be entirely below SilkBar strip (sy >= 51)
- Max 16 cards per pass (`ATLAS_CARD_MAX_CARDS`)
- One-shot: cards drawn exactly once per boot
- No fills, no text labels, no alpha, no blur, no shadows
- No thumbnails, no live capture, no drag, no animation

## Exact Markers Added

### silk-shell (shell-side proof)
```
[silk.atlas.phase_c.begin] scenes=N active=S
[silk.atlas.phase_c.snapshot] ok=1 reason=collected_for_card_geometry
[silk.atlas.phase_c.done] ok=1 reason=shell_side_participates
```

### sexdisplay (card geometry pass)
```
[sexdisplay.atlas.card.layout] scene=N x=X y=Y w=W h=H active=A
[sexdisplay.atlas.card.draw] scene=N ok=1
[sexdisplay.atlas.card.skip] scene=N reason=...
[sexdisplay.atlas.phase_c.done] cards=N ok=1
```

## Proof Commands

Build with Phase C proof enabled:
```fish
SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1 ./scripts/entrypoint_build.sh
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

Expected gate PASS: `atlas_phase_c_render_stub` shows PASS when `[sexdisplay.atlas.phase_c.done]` found with `ok=1` and `cards=N`.

## Proof Result

Build (default): PASS (compiled without warnings, ISO produced).
Build (SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1): PASS (compiled without warnings, ISO produced).
Runtime: Awaits boot log with `SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1`.

## STOP FIRST Boundaries Preserved

| Boundary | Status |
|----------|--------|
| No kernel edits | Preserved |
| No sex-pdx ABI edits | Preserved (only local const/function additions) |
| No new compositor protocol | Preserved |
| No sexdisplay policy change | Preserved (render-only, no shell policy) |
| No framebuffer/backing-buffer redesign | Preserved |
| No shared-memory redesign | Preserved |
| No broad refactor | Preserved |
| No renderer policy ownership changes | Preserved |
| No input policy outside silk-shell | Preserved |
| No mixed feature + refactor patch | Preserved |
| No unwrap on optional display state | All uses safe nullable access patterns |
| No OOB indexing | Bounded by MAX_SURFACES (16), per-pixel w/h/total_pixels checks |
| No unbounded loops | Bounded by MAX_SURFACES (16), ATLAS_CARD_MAX_CARDS (16) |
| No new unsafe beyond existing pattern | Follows existing renderer unsafe fn pattern |
| No dead-surface panic | active flag check + clamp_surface guard |
| No framebuffer writes above strip | sy >= 51 guard, ty >= 51 guard |
| No behavior change when proof env is unset | Early return at fn entry |
| No thumbnail pixels | Borders only, no surface content capture |
| No new renderer protocol | Proof reads existing SURFACES state only |
| No drag/drop | Preserved |
| No animation | Preserved |
| No alpha/blur/shadow | Flat ARGB writes only |
| Shell starts in Desktop mode | ATLAS_MODE_ENABLED=false at boot |
| No policy in sexdisplay | Card pass is pure rendering |
| No focus/drag/scene mutation in renderer | Read-only SURFACES access |

## Remaining Phases D-F

| Phase | What | Status |
|-------|------|--------|
| Phase A | State model proof | Built, gate added |
| Phase B | Atlas snapshot/capture integration | Built, gate added |
| **Phase C** | **Render stub + card geometry** (this doc) | Built, gate added |
| Phase D | Thumbnails and frame previews | Deferred |
| Phase E | Drag between Scenes | Deferred |
| Phase F | Animations, blur, alpha, shadows | Deferred |

## Explicit Note

Phase C is static card geometry only — no thumbnails, no surface capture, no drag between Scenes, no animation, no alpha blending, no blur, no shadows. Cards are 2px border outlines drawn around active surfaces using flat ARGB color.

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-21 | Add draw_atlas_cards_pass(), maybe_run_atlas_phase_c_render_stub_proof(), gate | ATLAS_OVERVIEW_PHASE_C_RENDER_STUB_CARD_GEOMETRY_PROOF_V1 |
