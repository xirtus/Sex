# POINTER_ABS_CLICK_COHERENCE_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

In sexinput's `normalize_pointer_report_v1`, EV_BTN events were emitted BEFORE EV_ABS events. Silk-shell's main loop processes PDX messages in FIFO order, so the click handler ran with the PREVIOUS cursor position (stale POINTER_X/Y), not the position from the current tablet report. Visual cursor was updated later when EV_ABS was processed.

## Fix: Emit ABS before BTN

Swapped emit order in `normalize_pointer_report_v1`:
1. **First:** emit EV_ABS (cursor position update)
2. **Then:** emit EV_BTN (button click) — uses updated POINTER_X/Y

This ensures click hit-test uses the same coordinate as the visual cursor.

## Changed file

| File | Change |
|------|--------|
| `servers/sexinput/src/main.rs` | Moved ABS emission before BTN in normalize_pointer_report_v1 |
