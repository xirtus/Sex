# XHCI_TABLET_LINK_TRB_CYCLE_V2

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

Link TRB was written with new PCS (after toggle), but controller reaches it with old CCS (before toggle). Controller CCS=1, Link cycle=0 → no match → controller stops.

## Fix

Link TRB must use old PCS (before toggle):
- `old_pcs` = PCS before toggle → controller CCS matches
- `new_pcs` = PCS after toggle → NORMAL TRB at slot 0 uses this

```
Before: PCS toggled → Link written with new PCS → CCS mismatch → stop
After:  old_pcs saved → PCS toggled → NORMAL uses new_pcs, Link uses old_pcs → CCS matches ✓
```
