# M5: Audit Bell Row Render

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Audit M4 Bell row rendering wiring. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                   PASS_M4_BELL_ROWS                          ║
╠══════════════════════════════════════════════════════════════╣
║ Ring/read conformance:          PASS                          ║
║ Visibility/refresh behavior:    PASS                          ║
║ Renderer/multi-rect boundary:   PASS                          ║
║ Authority boundary:             INTAKT                        ║
║ Forbidden areas:                CLEAN                         ║
╚══════════════════════════════════════════════════════════════╝
```

## Ring/Read Conformance

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Iterator is read-only | ✅ PASS | `bell_for_each_event()` takes `&BellEvent` ref, closure pattern, no mutation. Source line 1316: `fn bell_for_each_event<F>(mut f: F) where F: FnMut(&BellEvent)` |
| No mutation during render | ✅ PASS | `bell_render_event_list()` only reads via `bell_for_each_event()`, calls `pdx_call()` for display, emits serial markers. No writes to `BELL_EVENTS`, `BELL_RING_WRITE_INDEX`, or `BELL_EVENT_SEQUENCE`. |
| Newest-first order correct | ✅ PASS | Index formula at line 1321: `start = (total - 1) % BELL_RING_CAP`, iteration: `idx = (start + BELL_RING_CAP - i) % BELL_RING_CAP`. Correct for both wrapped and non-wrapped cases. |
| Invalid events skipped safely | ✅ PASS | `bell_for_each_event()` uses `if let Some(ref ev)` — `None` slots are silently skipped. No unwrap on ring entries. |
| Ring capacity respected | ✅ PASS | `bell_for_each_event()` iterates `bell_ring_count()` entries (max BELL_RING_CAP). `bell_render_event_list()` caps rows at `BELL_LIST_ROW_RECTS=7`. |
| Row rects within MAX_RECTS=8 | ✅ PASS | Header at rect_index=0, rows at rect_index=1..7. Maximum usage: 8 rects (fits sexdisplay MAX_RECTS=8). |

## Visibility/Refresh Behavior

| Scenario | Expected | Actual | Status |
|----------|----------|--------|--------|
| Open Bell (PageDown, first time) | Full event list render | ✅ `bell_render_event_list()` called at line 6088 |
| Open Bell while already visible (duplicate guard) | Focus only, no render | ✅ Duplicate guard at line 6032-6043 returns early before render |
| Valid link event while Bell visible | Write ring + re-render Bell | ✅ After `bell_record_event()` at line 1370, `bell_is_visible_in_active_scene()` check at line 1375 calls `bell_render_event_list()` at line 1377 |
| Valid link event while Bell hidden | Write ring only, no render | ✅ `bell_is_visible_in_active_scene()` returns false when Bell frame is absent, minimized, or in inactive scene |
| Rejected link (object missing) | Neither write nor render | ✅ Early return at line 1348 before any ring write or render call |
| Rejected link (buffer mismatch) | Neither write nor render | ✅ Early return at line 1359 before any ring write or render call |
| Tile/resize layout | Single-fill placeholder | ✅ Tile handler uses `0xEF` solid fill at lines 2284-2286 (consistent with all other placeholder surfaces) |
| Focus existing Bell via `focus_or_open_bell()` | Open path or focus | ✅ If already visible → focus only; else → open → `bell_render_event_list()` at line 6088 |

### Visibility Gate Correctness

`bell_is_visible_in_active_scene()` at lines 6172-6190 checks:
1. Frame ID == BELL_FRAME_ID ✓
2. scene_id == ACTIVE_SCENE_IDX ✓
3. Not minimized (FRAME_FLAG_MINIMIZED == 0) ✓
4. `surface_is_alive(sid)` returns true for Bell placeholder ✓

**No false positives:** If Bell is in inactive scene, minimized, or its surface is dead,
returns false. Link event writes to ring but does not trigger render.
**No false negatives:** If Bell is visible and active, returns true. Render fires.

## Renderer/Multi-Rect Boundary

| Concern | Result |
|---------|--------|
| Uses existing 0xEF rect-index path only | ✅ PASS — All calls use `pdx_call(SLOT_DISPLAY, 0xEF, ...)` with `(rect_index<<56)` packing |
| sexdisplay changes in M4 | ✅ NONE — No sexdisplay source modified |
| Max rects used: 1 header + 7 rows = 8 | ✅ PASS — Within sexdisplay MAX_RECTS=8 limit |
| Color isolation via `& 0x00FF_FFFF` | ✅ PASS — sexdisplay masks color; shell packs `(row_color as u64) << 32` at line 6231 |
| Bounds/clipping safe | ✅ PASS — `SURFACE_204_W/H` retrieved from shell state; 0×0 guard at line 6195 |
| Text rendering | ✅ NONE — Proof-marker rows only, no text primitives |
| Multi-rect is truly live (L7 finding) | ✅ CONFIRMED — sexdisplay Surface struct has MAX_RECTS=8 arrays, painter's algorithm compositing |

### Tile-Placeholder Coexistence

The tiling function at lines 2283-2286 draws a solid `BELL_PLACEHOLDER_COLOR` fill rect.
This is the same pattern used by Quil (line 2260), Mesh (line 2268), and Collar (line 2276).
The full event list render (`bell_render_event_list()`) overdraws with proper rows when
triggered by open or event-refresh paths. This is correct: tiling placeholders are quick
layout fills, row renders are semantic-refresh fills.

## Authority Boundary

| Concern | Result |
|---------|--------|
| Bell remains shell-local ring | ✅ PASS — No Bell PD, no cross-PD communication |
| No new IPC/opcodes/ABI | ✅ PASS — Only existing 0xEF and 0xEC opcodes used |
| No Bell PD creation | ✅ PASS — Not attempted |
| No Collar authority drift | ✅ PASS — No collar_check_operation_stub() calls in render path |
| No Mesh graph drift | ✅ PASS — No mesh_emit_linen_quil_links() calls in render path |
| No command execution from Bell rows | ✅ PASS — Rows are read-only visual, no action/click handler |
| No kernel edits | ✅ PASS — CLEAN |
| No sex-pdx edits | ✅ PASS — CLEAN |
| No sexdisplay edits | ✅ PASS — CLEAN |
| No WINDOWS Vec migration | ✅ PASS — CLEAN |

### Forbidden-Area Check

| Area | Status |
|------|--------|
| `kernel/` | ✅ CLEAN |
| `crates/sex-pdx/` | ✅ CLEAN |
| `servers/sexdisplay/` | ✅ CLEAN |
| `servers/bell/` | ✅ CLEAN |
| `servers/linen/` | ✅ CLEAN |
| `servers/quil/` | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |

## Full Proof Marker Inventory

All markers that should exist after M4, with audit confirmation:

| Marker | Exists? | Source Location |
|--------|---------|-----------------|
| `[bell.event_list.render]` | ✅ Confirmed | Line 6197 |
| `[bell.event_list.row]` | ✅ Confirmed | Line 6219 |
| `[bell.event_list.skip]` | ✅ Confirmed | Line 6212 |
| `[bell.row_visual.rect]` | ✅ Confirmed | Line 6234 |
| `[bell.row_visual.skip]` | ✅ Confirmed | Line 6238 |
| `[bell.event_list.done]` | ✅ Confirmed | Line 6242 |
| `[bell.render.refresh]` | ✅ Confirmed | Line 1376 |
| `[bell.ring.write]` | ✅ Preserved | Line 1303 |
| `[bell.ring.overwrite]` | ✅ Preserved | Line 1300 |
| `[bell.ring.done]` | ✅ Preserved | Line 1371-1372 |
| `[bell.event.stub]` | ✅ Preserved | Line 1335 |
| `[bell.event.object_link]` | ✅ Preserved | Line 1361 |
| `[bell.event.done]` | ✅ Preserved | Line 1379 |
| `[bell.event.reject.missing]` | ✅ Preserved | Lines 1346, 1356 |
| `[bell.placeholder.*]` | ✅ Preserved | I3 lines (unchanged) |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Tile placeholder uses solid fill, not event rows | LOW | Consistent with all other placeholder surfaces (Quil, Mesh, Collar). Event rows appear on open/refresh. |
| Limited to 7 visible rows | LOW | BELL_RING_CAP=16 events but only 7 visible at once due to MAX_RECTS=8 (1 header + 7 rows). Oldest events scroll off visually. |
| No row selection/click/ack | MEDIUM | M4 is read-only visual. Selection and action deferred to M6. |
| No text labels on rows | MEDIUM | Requires sexdisplay text primitive (STOP FIRST). Current: proof-marker-only identification of events. |
| No keyboard nav for Bell rows | MEDIUM | Bell surface not focusable for arrow-key navigation. Deferred to M6. |
| Object/buffer validation on render | LOW | `bell_row_color()` calls `linen_object_by_id()` which may return None for stale events whose object was removed. Gracefully handled via fallback color (0x00404060). |

## Exact Next Safest Step

**M6: Bell selected-row visual + keyboard nav** — Add a selected-row index to the
Bell surface, arrow-key navigation (up/down) when Bell is focused, and a visual
highlight for the selected row. Keep it read-only on the ring: no ack, no delete,
no destroy, no Collar grant. No text rendering. No Bell PD.

After M6: evaluate Bell row actions (view linked object in Linen/Quil) using
existing handler chains.

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| V1 | 2026-05-05 | Claude | Initial audit of M4 Bell row rendering |
