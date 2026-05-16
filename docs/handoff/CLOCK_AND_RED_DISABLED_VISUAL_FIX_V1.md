# CLOCK_AND_RED_DISABLED_VISUAL_FIX_V1

**Status:** PASS IMPLEMENTED — 122/122 gates, 0 faults.
**Date:** 2026-05-16

---

## Fix 1 — Clock seconds pulse block

10×5 px colored rectangle at right of clock digits (x=cx+48..cx+58, y=cy+1..cy+6). Pulses every second:
- Even second → dim teal (0x00386050)
- Odd second → bright green (0x0044FF44)

Much more visible than the 3×3 dot. User can clearly see the clock ticking.

## Fix 2 — Red disabled Frame Light

Close light base alpha increased: 48 → 72. The disabled red close light now renders as a visible dim red instead of nearly-invisible grey. Yellow/green remain at 224. close_allowed remains 0, close_impl remains 0 — red is still disabled, just visibly red now.

## Golden Hash: unchanged — 0xFD6093AC9ADE7B4D (both fixes outside top 50 rows)

## Files: sexdisplay +6 (pulse block), +1 (alpha)

## Proof: 122/122 PASS, 0 faults

## Commit
```bash
git add servers/sexdisplay/src/main.rs docs/handoff/CLOCK_AND_RED_DISABLED_VISUAL_FIX_V1.md
git commit -m "feat(ux): clock pulse block + red dim Frame Light"
```
