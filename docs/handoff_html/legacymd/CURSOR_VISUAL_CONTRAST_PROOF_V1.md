# CURSOR_VISUAL_CONTRAST_PROOF_V1

## Mission
Make live cursor visually obvious without changing input routing or shell policy.

## Root Cause
Cursor was: 8×16 white pixels, no outline, no scaling. White on light-colored window background = invisible.

## Change

### servers/sexdisplay/src/main.rs

**Constants (was):**
```
const CURSOR_ARROW_COLOR: u32 = 0x00FFFFFF; // white
```

**Constants (now):**
```
const CURSOR_ARROW_COLOR: u32 = 0x00FF00FF; // bright magenta
const CURSOR_SCALE: usize = 2;              // 2× scale → 16×32 drawn px
```

**Draw loop replaced with two-pass renderer:**
- Pass 1: 1px black outline — for each set bitmap bit, draw (SCALE+2)×(SCALE+2) black block using `saturating_sub(1)` for underflow safety and `.min(h.saturating_sub(1))` for overflow safety. SilkBar zone guard (py < 51) preserved.
- Pass 2: 2×-scaled magenta fill — each bit becomes 2×2 magenta block. All original bounds checks preserved (`py >= h`, `px >= w`, `idx < total_pixels`).

**New marker emitted each draw (budgeted ×8):**
```
[sexdisplay.cursor.visual.contrast] x=N y=N w=16 h=32 color=0x00ff00ff outline=1 ok=1
```

Effective cursor: 18×34 px (16×32 magenta + 1px black halo on each edge). Arrow shape unchanged.

### scripts/daily_driver_master_gate.sh

Added `cursor_visual_contrast` gate:
- PASS if `sexdisplay.cursor.visual.contrast.*ok=1` marker present.
- SKIP if marker absent (proof not enabled this boot).
- Does not block other gates.

## Proof Log Verification

After live run, check:
```
rg -n "cursor.motion.bounds|sexdisplay.cursor.draw|sexdisplay.cursor.visual.contrast|cursor_surface.z_top" /tmp/cursor_visual_contrast_live.log | tail -120
```

Expected markers:
- `[sexdisplay.cursor_surface.z_top.ok]`
- `[sexdisplay.cursor.draw]`
- `[sexdisplay.cursor.visual.contrast] ... outline=1 ok=1`

## Scope Invariants
- Input routing: unchanged
- Silk-shell policy: unchanged
- Framebuffer bounds checks: all preserved
- SilkBar zone guard (y<51): preserved in both passes
- Bitmap shape: unchanged (same 8×16 NW arrow)
- No kernel/ABI/sex-pdx edits

## Backup
`servers/sexdisplay/src/main.rs.bak.contrast_v1`

## Files Changed
- `servers/sexdisplay/src/main.rs` — cursor color, scale, two-pass draw, visual.contrast marker
- `scripts/daily_driver_master_gate.sh` — cursor_visual_contrast gate
- `docs/handoff/CURSOR_VISUAL_CONTRAST_PROOF_V1.md` — this file
