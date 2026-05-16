# CLOCK_VISIBILITY_STYLE_FIX_V1

**Status:** PASS IMPLEMENTED — 122/122 gates, 0 faults.
**Date:** 2026-05-16

---

## Visual Fix: Liveness dot next to clock

3×3 px dot at right edge of clock digits. Toggles every second:
- Even second (ss % 2 == 0) → dim grey (0x00404040)
- Odd second (ss % 2 == 1) → bright (CLOCK_FG = DEFAULT_THEME.text)

This makes clock liveness visually obvious even when 5×7 font seconds digits are too small to read.

## Golden Hash: 0x5413164AA874A0C5 (recaptured after dot addition)

Old: 0xFD6093AC9ADE7B4D

## Why red Frame Lights remain grey: close_allowed=0 — by design, not a bug

## Files: sexdisplay +11, master_gate (existing gate passes)

## Proof: 122/122 PASS, 0 faults

## Fault Count: **0**

## Rollback: to restore old hash, remove the liveness dot code and revert GOLDEN_TOP_STRIP_HASH to 0xFD6093AC9ADE7B4D

## Commit
```bash
git add servers/sexdisplay/src/main.rs docs/handoff/CLOCK_VISIBILITY_STYLE_FIX_V1.md
git commit -m "feat(clock): liveness dot visibility fix, hash 0x5413164AA874A0C5"
```
