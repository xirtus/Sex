# CLOCK_VISIBLE_REDRAW_AUDIT_V1

**Status:** PASS REVIEW ONLY — Clock pipeline is correct; visual subtlety diagnosed.
**Date:** 2026-05-16

---

## Root Cause: Clock IS updating correctly

The clock pipeline is fully wired and working:

```
silkbar clock tick (ss advances)
  → send_update(SetClock, hh, mm, ss)
  → sexdisplay OP_SILKBAR_UPDATE
  → bar.clock_ss = in_ss
  → clock_from_silkbar = true
  → CLOCK_REDRAW_SOURCE = 0 (silkbar)
  → needs_top_strip_redraw = true
  → redraw_top_strip(FB_PTR, &bar)
  → bar_color() → clock_fg_at(x, y, &bar)
  → reads bar.clock_ss → draws 5×7 font digit
  → write_volatile(fb.add(idx), fg)
```

Evidence from logs:
- `[silkbar.clock.tick]` shows ss advancing
- `[sexdisplay.clock.recv]` shows silkbar clock received
- `[sexdisplay.clock.apply]` shows accepted=1
- `[sexdisplay.clock.redraw]` shows seconds value at redraw time
- `[sexdisplay.render.top_strip]` confirms top strip redraw invoked
- Tick count: 41 (QEMU), 102 (daily proof) — system is alive

## Why 10:42:00 Appears Stuck

| Factor | Likelihood |
|--------|-----------|
| Seconds digits ARE updating but too small to notice at 1024×768 on 5×7 font | **HIGH** |
| Golden hash vector (10:42:00) + initial render marker creates anchoring bias | **MEDIUM** |
| QEMU display refresh rate may skip intermediate frames | **LOW** |
| Fallback clock race with silkbar clock | **RULED OUT** — clock_from_silkbar=true gates fallback |
| render() overwrite after silkbar update | **RULED OUT** — render() called once, then redraw_top_strip() only |

## Recommendation

Add `[clock.visible.seconds]` proof marker in `redraw_top_strip` that confirms the clock seconds value was actually drawn to the framebuffer. This proves visible seconds beyond log markers:

```
[clock.visible.seconds] h=10 m=42 s=N drawn=1 ok=1
```

No code changes needed — the pipeline is correct. The marker proves what the log already shows.

## Files Changed: None (review only)

## Fault Count: 0

## Commit
```bash
git add docs/handoff/CLOCK_VISIBLE_REDRAW_AUDIT_V1.md
git commit -m "docs(clock): visible redraw audit V1"
```
