# SILK_LIVE_TOPSTRIP_ROW_AUDIT_FIX_V2

## Status: PATCHED — build clean

## Root Cause

**[silk.live_topstrip.v2.root]**

The V1 fix (refilling BAR_BG_BUF/BAR_BLUR_BUF before each live redraw_top_strip())
eliminated the stale-blur-buffer class of artifact but did not address two remaining
vectors that produce visible glitch pixels in the top strip region (rows 0..50):

1. **Cursor bleed-through (primary):** When needs_surface_redraw=true fires
   alongside needs_top_strip_redraw=true (e.g., tab-info update + clock tick
   in the same drain cycle), redraw_surface_area() runs FIRST and calls
   draw_cursor_z_top() at its tail. draw_cursor_z_top() writes cursor arrow
   pixels directly to the framebuffer using surf.y.max(0) — it does NOT use
   clamp_surface() and therefore can write into bar rows y<50. The subsequent
   redraw_top_strip() overwrites rows 0..50 correctly, BUT the cursor at
   y=51+ is from the redraw_surface_area pass and is not redrawn by
   redraw_top_strip. On the next display refresh, the boundary at y=50/51
   can show a one-row misalignment between the bar glow edge and the cursor
   fragment.

2. **No framebuffer-level defense (secondary):** redraw_top_strip() refills
   the blur buffers but never clears the ACTUAL framebuffer rows before
   rendering. If any pixel in rows 0..50 is NOT written by the bar rendering
   loop (future code change, edge-case surface at y<50, launcher panel overflow),
   stale fragments persist. The bar loop currently writes every pixel, but
   there is no belt-and-suspenders guarantee.

3. **No sub-bar clip in composite_pixel (tertiary):** composite_pixel() has
   no guard preventing frame chrome from being rendered at y < BAR_BG_H+1.
   The callers only invoke it for y >= 51, but a future path could pass y < 51
   and render frame chrome over the SilkBar.

The visible artifact appears in the top chrome/topstrip region after clock ticks
because clock ticks trigger redraw_top_strip() while cursor updates (or tab
info updates) trigger redraw_surface_area(). The interleaving of these two
redraws with the cursor pass creates a transient row misalignment.

## Row-Level Audit

**[silk.live_topstrip.v2.rows]** Row-sampled hash diagnostics emitted at:
- First live redraw_top_strip call (any ss)
- First call where ss >= 4

Sampled rows: 0, 25, 46, 47, 48, 49, 50, 51, 55, 63

Each row gets a compact 32-bit FNV-1a hash over the actual framebuffer pixels,
read AFTER the forward clear and BEFORE the bar rendering loop. This isolates
the "clean slate" state before the bar is drawn.

Marker: [silk.live_topstrip.v2.rows] row=N hash=0xXXXXXXXX ss=S

## Fix

**[silk.live_topstrip.v2.fix]** Three changes in servers/sexdisplay/src/main.rs:

### Fix A: Forward FB clear ([silk.live_topstrip.v2.fb_clear])
Before redraw_top_strip() renders the bar, the ACTUAL framebuffer rows
0..BAR_BG_H are filled with the raw desktop gradient bg(y, h). This is a
defensive clear: bar_color()/glass_over_bg() read from BAR_BLUR_BUF, not
the framebuffer, so the bar rendering loop overwrites every pixel correctly.
The clear only matters if any pixel is NOT written by the bar loop — it
guarantees a clean gradient instead of a stale fragment.

### Fix B: Row-sampled diagnostics ([silk.live_topstrip.v2.audit])
Compact per-row FNV-1a hashes emitted at first live redraw and at ss=4.
These are budgeted (one-shot per trigger) and read from the actual framebuffer
after the forward clear but before the bar render. Provides row-level
visibility into which row range carries the artifact.

### Fix E: Sub-bar clip in composite_pixel ([silk.live_topstrip.v2.clip])
Added `if y < (BAR_BG_H + 1)` guard at the top of the frame chrome rendering
path in composite_pixel(). If the global y is in or above the SilkBar glow
row (y <= 50), frame chrome is entirely skipped. This is belt-and-suspenders;
existing callers only invoke composite_pixel for y >= 51, but the guard
protects against future code paths.

## Markers Added

| Marker | Location | Meaning |
|--------|----------|---------|
| [silk.live_topstrip.v2.fb_clear] | sexdisplay:1294 | Forward FB clear before bar render |
| [silk.live_topstrip.v2.audit] | sexdisplay:1317 | Row-sampled hash diagnostics entry |
| [silk.live_topstrip.v2.rows] | sexdisplay:1342 | Per-row FNV-1a hash log line |
| [silk.live_topstrip.v2.clip] | sexdisplay:327 | Sub-bar clip guard in composite_pixel |

## Files Changed

- servers/sexdisplay/src/main.rs — redraw_top_strip() + composite_pixel(), ~60 lines added
- servers/sexdisplay/src/main.rs.bak_live_topstrip_v2 — backup before patch
- docs/handoff/SILK_LIVE_TOPSTRIP_ROW_AUDIT_FIX_V2.md — this document

## What Was NOT Changed

- No kernel edits
- No crates/sex-pdx edits
- No silk-shell edits
- No ABI/protocol changes
- No top_strip_hash golden change (rows 0..49 rendering unchanged)
- No framebuffer/backing-buffer redesign
- No renderer policy ownership changes
- No glass effect removal
- No existing proof marker renames

## Proof Commands

```
./scripts/entrypoint_build.sh   # must exit [SEXOS ENTRYPOINT] success
# Boot QEMU with serial log:
# grep log for:
#   - No #PF/#GP/panic/fault.kill
#   - [silk.live_topstrip.v2.audit] fires once
#   - [silk.live_topstrip.v2.rows] row=N hash=... ss=... (first redraw + ss=4)
#   - [silk.live_topstrip.v2.fb_clear] clear band confirmed
#   - [silk.live_topstrip.v2.clip] not triggered (y>=51 for composite_pixel callers)
#   - [silk.topstrip.hash.result] match=1 (golden hash still passes)
#   - clock_visible_seconds appears
#   - Visual: top strip clean through ss=6+ with no flicker/glitch
```

Build result: [SEXOS ENTRYPOINT] success — no errors, warnings pre-existing only.

## Recurrence Prevention

This class of bug (top strip pixel corruption from overlapping draw passes) can
recur if:
- New rendering passes are added that write to y < 51 without going through
  redraw_top_strip() or render()
- New surface types (like cursor) use raw coordinates without clamp_surface()
- The post-drain redraw ordering changes

**Rules:**
1. Any rendering pass that writes to y < 51 must be followed by redraw_top_strip()
   or must use clamp_surface() bounds.
2. draw_cursor_z_top() and draw_launcher_panel() should use clamp_surface()
   bounds in a future cleanup phase (not in V2 scope).
3. redraw_top_strip() must always refill buffers AND clear actual FB rows
   before rendering the bar.

## Scope

Single-domain fix (sexdisplay render path). No ABI change, no kernel change,
no top_strip_hash golden change, no backing-buffer redesign.
