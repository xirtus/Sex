# SILK_TAB_HIT_REORDER_V1 — Tab Hit Testing and Reorder Handoff

**Status:** COMPLETE  
**Branch:** master  
**Date:** 2026-05-20  
**Prerequisites:** SILK_POINTER_RESIZE_STATE_V1, SILK_POINTER_RESIZE_GEOMETRY_V1, SILK_DRAG_TO_SNAP_V1

---

## What Was Done

Implemented minimal multi-tab pointer hit testing and horizontal tab reorder in `servers/silk-shell/src/main.rs`. No sexdisplay, kernel, or sex-pdx changes.

### New Markers

| Marker | Condition |
|--------|-----------|
| `[silk.tab.hit]` | Pointer down on a valid tab slot in the tab strip |
| `[silk.tab.select]` | Tab activated via pointer click (switch_to_tab succeeded) |
| `[silk.tab.drag.begin]` | Pointer moved ≥6px while holding a tab strip hit |
| `[silk.tab.reorder.swap]` | Two tab slots swapped during drag |
| `[silk.tab.reorder.reject]` | Swap rejected (invalid index, None slot, or bounds) |
| `[silk.tab.drag.end]` | Button released from TabDragging state |

---

## Changes Made

### `servers/silk-shell/src/main.rs`

**1. `InteractionState::TabDragging` variant** (added after `Resizing`)
```
TabDragging { frame_id: u32, start_tab: u8, start_x: i32, start_y: i32, current_tab: u8 }
```

**2. `try_transition` — new allowed transitions**
- `ClickPending → TabDragging` (drag start from tab strip hit)
- `TabDragging → Idle` (button release)

**3. New static vars** (after `DRAG_PENDING_START_Y`)
- `TAB_DRAG_PENDING_FRAME_ID: u32`
- `TAB_DRAG_PENDING_START_TAB: u8`

**4. `DRAG_PENDING_KIND = 5`** for `FRAME_CHROME_TAB_STRIP` hits in `if left_held {}` block (was 4 for all non-rim, non-resize chrome)

**5. Tab strip click handler** (`click_hit_test_and_focus`, `FRAME_CHROME_TAB_STRIP` arm):
- Emits `[silk.tab.hit]` before switch
- Sets `TAB_DRAG_PENDING_FRAME_ID` / `TAB_DRAG_PENDING_START_TAB`
- Emits `[silk.tab.select]` on successful switch

**6. `update_frame_hover_at`** — skips hover update during `TabDragging` (same guard as `Dragging`)

**7. `swap_frame_tabs(frame_id, idx_a, idx_b) -> bool`** — new function after `frame_tab_at`:
- Validates both indices < `tab_count` and `!= None`
- Calls `frame.tabs.swap(a, b)`
- Updates `active_tab` if either swapped slot was active
- Emits `[silk.tab.reorder.swap]`
- Returns false (with no panic) on any invalid input

**8. Tab drag threshold + reorder** — added to both EV_ABS and EV_REL pointer-move paths in main message loop:
- `ClickPending + DRAG_PENDING_KIND == 5 + dist >= 6` → `try_transition(TabDragging {...})`
- `TabDragging` on move → `frame_tab_at` → `swap_frame_tabs` → `send_frame_tab_info`

**9. `TabDragging` button-up arm** — added to both `handle_hid_event` and main loop EV_BTN handlers:
- Emits `[silk.tab.drag.end]`
- Clears `DRAG_PENDING_ACTIVE`
- Transitions to `Idle`

---

## Safety Invariants

- Fixed array bounds never violated: both indices checked `< tab_count` before `swap`
- Tombstoned/None tab slots rejected: `frame.tabs[idx].is_none()` check before swap
- `active_tab` kept consistent after every swap
- `frame_tab_at` (called during move) guards `frame_accepts_input` → no tab interaction with minimized/dead/wrong-scene frames
- No heap allocation: all new state in `InteractionState::TabDragging` fields and two new statics

---

## Proof Results

**Build gate:** `scripts/entrypoint_build.sh` — PASS (no warnings added)  
**Proof gate:** `scripts/run_daily_driver_proof.sh`

```
PASS gates: 272
FAIL gates: 1   ← sexnet_dns_source3_proof_v1 (pre-existing DNS network gate, unrelated)
SKIP gates: 58
faults_zero: PASS   0 fault markers
frame_chrome_model: PASS   scenes=1 frames=3 tabs=3
```

No `#PF`, `#GP`, `panic`, or `fault.kill` in proof log.

**Pre-existing failure:** `sexnet_dns_source3_proof_v1` (source2 DNS markers present in source3 proof lane) — existed before this patch. Not caused by silk-shell changes.

---

## What Is NOT Done (Deferred)

- Tab detach into new frame (separate task)
- Visual tab strip rendering changes (sexdisplay not touched)
- Synthetic proof gate for new markers (no EV_BTN/EV_REL injection of tab strip coordinates in existing proof sequences)

---

## Key File Locations

| Symbol | Line (approx) |
|--------|---------------|
| `InteractionState::TabDragging` | ~6813 |
| `TAB_DRAG_PENDING_FRAME_ID` | ~8213 |
| `swap_frame_tabs` | ~16719 |
| Tab strip click handler (markers) | ~18030 |
| EV_ABS tab drag handling | ~20873 |
| EV_REL tab drag threshold + reorder | ~20920 |
| `handle_hid_event` TabDragging button-up | ~8832 |
| Main loop TabDragging button-up | ~21030 |
