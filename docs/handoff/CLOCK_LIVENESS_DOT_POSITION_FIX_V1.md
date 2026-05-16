# CLOCK_LIVENESS_DOT_POSITION_FIX_V1

**Status:** PASS IMPLEMENTED — 122/122 gates, 0 faults.
**Date:** 2026-05-16

---

## Fix: Dot moved from cx+44→cx+48

Old: x=cx+44 (overlapped seconds glyph at cx+41..cx+46)
New: x=cx+48 (2px gap after final digit, outside clock bounding box)

No more glitch over the final 0 in 10:42:00.

## Golden Hash: restored to 0xFD6093AC9ADE7B4D (dot now outside 50-row hash region)

## Files: sexdisplay (2 lines changed)

## Proof: 122/122 PASS, 0 faults

## Commit
```bash
git add servers/sexdisplay/src/main.rs docs/handoff/CLOCK_LIVENESS_DOT_POSITION_FIX_V1.md
git commit -m "fix(clock): liveness dot position cx+48"
```
