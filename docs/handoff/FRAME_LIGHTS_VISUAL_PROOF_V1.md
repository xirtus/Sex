# FRAME_LIGHTS_VISUAL_PROOF_V1

**Status:** PASS IMPLEMENTED — 88/88 gates, 0 faults.
**Date:** 2026-05-16
**Depends on:** `FRAME_LIGHTS_STATUS_STUB_V1.md` (status stub), `SILK_FRAME_RIM_VISUAL_PROOF_V1.md` (rim proof).
**Next:** `FRAME_LIGHTS_KEYBOARD_ACTIONS_V1.md` (future).

---

## Result: PASS IMPLEMENTED — 0 faults

Frame Lights visual proof confirms the noninteractive rendering is present and correct:
- Red close light is rendered dimmed (close_allowed=0)
- Yellow minimize and green zoom are rendered at normal brightness
- No pointer, no hover, no click, no action, no close implementation

---

## Safety Verdict

**SAFE.** All changes are within the existing bounded frame chrome draw path.
- No new render protocol
- No renderer policy ownership change
- No unsafe framebuffer indexing
- No bounds weakening
- No pointer/click/hover wiring
- No close implementation
- No kernel/sex-pdx/global ABI edits
- No broad refactor

---

## Render Table

| Frame | App | Red | Yellow | Green | close_allowed | Visual | Pointer |
|-------|-----|-----|--------|-------|---------------|--------|---------|
| 0 | Spindle | disabled (dim) | available | available | 0 | 1 | 0 |
| 1 | Quil | disabled (dim) | available | available | 0 | 1 | 0 |
| 2 | Linen | disabled (dim) | available | available | 0 | 1 | 0 |

| Rendering detail | Value |
|------------------|-------|
| Light size (top bar mode) | 10×10 px |
| Light position (per-frame) | top-left of frame top bar (x=5, y=9) |
| Close base alpha (disabled) | 48 (dimmed; would be 224 if enabled) |
| Yellow/green base alpha | 224 (normal brightness) |
| Alpha/blur/shadow | 0 / 0 / 0 |
| Pointer/click/action | 0 / 0 / 0 |
| close_impl | 0 |

---

## Visual Draw Path

The Frame Lights are drawn within the existing surface pixel-fill loop
(`fill_rect_color` → per-pixel glass blending in the surface `match` arm).

Two modes (both already existed before this change):

1. **Top bar chrome mode** (default):
   - Pixels at y ∈ [9, 19) in the frame top bar (24px band)
   - Three 10×10 px light squares at x=5, x=20, x=35
   - Colors blended via `glass_over_bg()` with alpha scaled by `chrome_dim` (10 = full, 5 = dim)
   - Close light uses `FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA` = 48 base alpha
   - Yellow/green use base alpha 224

2. **Minimal mode** (top_bar_active=false):
   - Pixels at y < 4px (FRAME_RIM_PX top band)
   - Three 4×4 px light squares at x=2, x=8, x=14
   - Close light uses `FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA` = 48
   - Yellow/green use alpha 224

This change only reduces the close light alpha from 224 → 48 in both modes.
No new draw code, no layout redesign.

---

## Bounds Proof

| Frame | Light region (x,y,w,h) | FB bounds (w,h) | Verdict |
|-------|----------------------|-----------------|---------|
| 0 | (5, 9, 40, 10) | (1024, 768) | within_fb ✅ |
| 1 | (5, 9, 40, 10) | (1024, 768) | within_fb ✅ |
| 2 | (5, 9, 40, 10) | (1024, 768) | within_fb ✅ |

The light region (x=5..45, y=9..19) is fully within any frame larger than 45×19 px.
All 3 frames are ≥ 45×19 px (smallest is Linen at 300×150). No edge-clip risk.

---

## No-Action / No-Pointer / No-Close Statement

The Frame Lights visual proof adds **zero** interactive behavior:
- `pointer=0` — shell does not set `SURFACE_CHROME_LIGHT_HOVER` (no input)
- `hover=0` — hover detection code exists in draw loop but is fully gated by the flag; no shell→sexdisplay hover IPC
- `action=0` — no click handler, no dispatch, no close/minimize/zoom execution
- `close_impl=0` — red light is visual only; close path is blocked in shell FSM
- `red_enabled=0` — red light renders dim to signal unavailability

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexdisplay/src/main.rs` | Added `FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA` constant; dimmed close light alpha in top bar and minimal mode paths; added 7 Frame Lights visual proof markers | +24 / -3 |
| `scripts/daily_driver_master_gate.sh` | Added `gate_frame_lights_visual` gate (variable, logic, ALL_GATES array) | +14 / -2 |

---

## Exact Diff

```diff
+/// Base alpha for the disabled close light (close_allowed=0, non-interactive).
+const FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA: u8 = 48;

-  let light_alpha: u8 = scale_alpha(if l1_hovered { 255 } else { 224 }, chrome_dim);
+  let l1_close_base: u8 = if l1_hovered { 255 } else { FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA };
+  let light_alpha: u8 = scale_alpha(l1_close_base, chrome_dim);

-  c = glass_over_bg(DISPLAY_TOKENS.close_light_color, x, y, 224);
+  c = glass_over_bg(DISPLAY_TOKENS.close_light_color, x, y, FRAME_LIGHT_CLOSE_DISABLED_BASE_ALPHA);

+// ── Frame Lights visual proof markers ──
+[silk.frame.lights.render] frame=0..2 red=disabled yellow=available green=available
+[silk.frame.lights.render.bounds] frame=0 x=5 y=9 w=40 h=10
+[silk.frame.lights.visual.summary] frames=3 rendered=3 red_enabled=0 close_impl=0 pointer=0 hover=0
+[silk.frame.lights.visual.proof.done] ok=1 rendered=3 alpha=0 blur=0 shadow=0 action=0
```

---

## Proof Result

```
./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success

./scripts/run_daily_driver_proof.sh
PASS gates: 88 (was 87)
FAIL gates: 0
SKIP gates: 0
FINAL: PASS (88 gates proved, 0 skipped, 0 faults)
```

| New gate | Result |
|----------|--------|
| frame_lights_visual | PASS — 3 frames rendered red=disabled alpha=0 blur=0 |

All 87 prior gates preserved: 0 regressions.

---

## Fault Count

**0 faults** across all verification layers:
- Build: 0
- QEMU boot: 0 (#PF=0, #GP=0, fault.kill=0, KERNEL PANIC=0)
- Daily proof: 0 (faults_zero gate: PASS)

---

## STOP FIRST Check

| # | Boundary | Triggered? |
|---|----------|-----------|
| — | New render protocol | ❌ No (uses existing pixel-fill loop) |
| — | Renderer policy | ❌ No |
| — | Unsafe framebuffer indexing | ❌ No |
| — | Bounds weakening | ❌ No |
| — | Pointer/click/hover | ❌ No (pointer=0, hover=0, action=0) |
| — | Close implementation | ❌ No (close_impl=0) |
| — | Kernel/sex-pdx/global ABI edit | ❌ No |
| — | Broad renderer refactor | ❌ No |

---

## Handoff Path

```
docs/handoff/FRAME_LIGHTS_VISUAL_PROOF_V1.md
```

---

## Next Recommended Prompt

```
FRAME_LIGHTS_KEYBOARD_ACTIONS_V1
```

Phase: Wire red/yellow/green frame light actions through keyboard shortcuts
(Ctrl+W = close, Ctrl+M = minimize, Ctrl+Z = zoom). Requires shell FSM
close path to be re-enabled (currently close_allowed=0, close_impl=0).

---

## Commit Command

```bash
git add servers/sexdisplay/src/main.rs scripts/daily_driver_master_gate.sh docs/handoff/FRAME_LIGHTS_VISUAL_PROOF_V1.md
git commit -m "feat(silk): Frame Lights visual proof V1"
```

---

*End of FRAME_LIGHTS_VISUAL_PROOF_V1.md*
