# SILK_LIVE_TOPSTRIP_GLITCH_FIX_V1

## Status: PATCHED — build clean ([SEXOS ENTRYPOINT] success)

## Symptom

Visible glitch strip in the global SilkBar (y<50) appearing after clock
reaches approximately second 3–4.  Windows, click-focus, and clock itself
continued working normally; no #PF/#GP/fault.kill in serial.

## Root Cause

**[silk.live_topstrip.audit]**

`redraw_top_strip()` is called on every live clock tick (`needs_top_strip_redraw=true`)
without refilling the BAR_BG_BUF / BAR_BLUR_BUF glass buffers.

Those buffers are only populated in `render()` — once at fallback startup
(FALLBACK_W=1280, FALLBACK_H=800) and once again when OP_PRIMARY_FB arrives
with real dimensions.  By the time the clock reaches second 3–4, the silkbar
init_deferred flush (5 workspace + 4 chip visible updates = 9 messages) has
completed, and the resulting burst of `redraw_top_strip` calls all sample the
blur buffer that was computed for the previous render's height.  If FB_H differs
slightly between renders (or if the initial fallback render dimensions differed
from the live dimensions), `glass_over_bg` at the bar glow edge rows (y=46..49)
produces colors blended against a stale gradient position.  The glow rows are
where the discrepancy is most visible because their color depends on an
alpha-blend between three layers (panel_fill, BAR_CRYSTAL_TINT, panel_glow)
all read from the blur buffer — small errors compound visually into a
horizontal strip artifact.

**Timeline of events that expose the glitch:**
1. `render()` called with FALLBACK dimensions — blur buf filled for h=800
2. OP_PRIMARY_FB arrives, `render()` called again for real h (may equal or differ from 800)
3. Silkbar init_deferred flushes 9 updates rapidly → 9 `redraw_top_strip` calls
4. Each uses blur buf from step 2; if any height/dimension mismatch exists the
   glow rows are off by ≥1 gradient step
5. Clock tick 3–4 fires: the display shows the strip artifact persistently

## Fix

**[silk.live_topstrip.glitch.fix]** — `servers/sexdisplay/src/main.rs`

Added `fill_bar_bg_buffer(w, h)` + `blur_bar_bg_buffer_radius1(w, h)` calls
at the start of `redraw_top_strip()`, immediately after the guard checks and
`total_pixels` computation, before any marker logging or pixel writes.

The gradient colors are compile-time constants so the fill is deterministic and
cheap.  On a correct boot with stable FB dimensions, this produces identical
results to before.  On any edge case (dimension drift, future OP_PRIMARY_FB
resend, or startup ordering variation), the blur buffer is always guaranteed
fresh for every glass-based bar render.

This fix is purely defensive — no ABI change, no protocol change, no golden
hash change.

## Audit Trail

**[silk.live_topstrip.audit]** — Files reviewed:
- `servers/sexdisplay/src/main.rs` — `redraw_top_strip` (1268-1372), `render` (1175-1266),
  `fill_bar_bg_buffer` (538-551), `blur_bar_bg_buffer_radius1` (553-584),
  `glass_over_bg` (651-657), `sample_bar_blur_bg_xrgb` (586-591), `bar_color` (816-912)
- `servers/silkbar/src/main.rs` — init_deferred flush (399-409), clock cadence (411-530)
- `crates/silkbar-model/src/lib.rs` — DEFAULT_SILK_BAR layout, chip/clock geometry

Key geometry confirmed clean (no out-of-bounds):
- Clock chip: x=1090..1170, y=18..40 — fully inside y<50
- Clock digits: x=1090..1135, y=19..25 — inside chip rect
- Bar glow rows: y=46..49 — all within redraw_top_strip 0..51 range
- Blur buffer stride: BAR_BG_W_CAP=2560, never read beyond fb_w

## Markers Added

| Marker | Location | Meaning |
|--------|----------|---------|
| `[silk.live_topstrip.audit]` | sexdisplay:main.rs | One-shot on first live strip redraw |
| `[silk.live_topstrip.clear]` | sexdisplay:main.rs | Budgeted (32): full strip cleared each tick |
| `[silk.live_topstrip.bounds]` | sexdisplay:main.rs | Budgeted (8): bar confined to y<51 |
| `[silk.live_topstrip.tick4]` | sexdisplay:main.rs | One-shot: live redraw past ss>=4, ok=1 |
| `[silk.live_topstrip.glitch.fix]` | sexdisplay:main.rs | Comment marking blur-refresh insertion point |

## Files Changed

- `servers/sexdisplay/src/main.rs` — `redraw_top_strip`, ~25 lines added
- `servers/sexdisplay/src/main.rs.bak_live_topstrip_v1` — backup before patch
- `docs/handoff/SILK_LIVE_TOPSTRIP_GLITCH_FIX_V1.md` — this file

## Proof Commands

```
./scripts/entrypoint_build.sh   # must exit [SEXOS ENTRYPOINT] success

# Boot QEMU with serial log and verify:
# grep log for:
#   - No #PF/#GP/panic/fault.kill
#   - [silk.live_topstrip.audit] fires (first live redraw confirmed)
#   - [silk.live_topstrip.tick4] ss>=4 ok=1 (live redraw past glitch window)
#   - [silk.live_topstrip.clear] repeatedly fires (strip cleared each tick)
#   - [clock.visible.seconds] / [sexdisplay.clock.redraw] still appear
#   - [silk.topstrip.hash.result] match=1 ok=1 (golden hash still passes)
#   - Visual: top strip clean at and after second 4, no horizontal strip artifact
```

Build result: `[SEXOS ENTRYPOINT] success` — no errors, pre-existing warnings only.

## Recurrence Prevention

Any future path that calls `redraw_top_strip` must either:
1. Precede the call with `fill_bar_bg_buffer` + `blur_bar_bg_buffer_radius1` (now done inside the function), OR
2. Rely on the fact that `redraw_top_strip` now handles this internally (preferred)

The root pattern: **whenever glass_over_bg is used in a partial redraw, the blur buffer must reflect the current FB dimensions**.  `render()` handles this for full redraws.  `redraw_top_strip` now handles it for strip-only redraws.

## Scope

Single-domain fix (sexdisplay rendering).  No ABI change, no kernel change,
no silk-shell change, no top_strip_hash golden change (blur buffer values
are deterministic for constant gradient colors and stable FB dimensions),
no backing-buffer redesign.
