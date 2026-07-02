# XHCI_TABLET_RING_CYCLE_LINK_FIX_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

Race at ring wrap: Link TRB was updated before NORMAL TRB at slot 0 was written. Controller followed Link to slot 0, saw stale TRB with wrong cycle bit, stopped the transfer ring.

## Fix

Reorder at wrap: write NORMAL TRB at slot 0 FIRST, then update Link TRB cycle. Controller now sees the new TRB immediately after following the Link.

```
Before: Link TRB update → NORMAL TRB at 0 → doorbell  (race: controller beats NORMAL write)
After:  NORMAL TRB at 0 → Link TRB update → doorbell  (safe: TRB ready before Link)
```
