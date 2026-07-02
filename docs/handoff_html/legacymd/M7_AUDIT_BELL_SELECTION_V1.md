# M7: Audit Bell Event Selection

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Audit M6 Bell selected-row visual + keyboard nav. Docs only.
No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                PASS_M6_BELL_SELECTION                        ║
╠══════════════════════════════════════════════════════════════╣
║ Selection state:               PASS (read-only row index)    ║
║ Keyboard routing:              PASS (correct precedence)     ║
║ Repair/clamp:                  PASS (safe at all counts)     ║
║ Visual behavior:               PASS (safe color brighten)    ║
║ Boundaries:                    INTAKT                         ║
║ Forbidden areas:               CLEAN                          ║
╚══════════════════════════════════════════════════════════════╝
```

## Selection State Conformance

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Visible-row index only | ✅ PASS | `BELL_SELECTED_ROW: u8` — 0..6, independent of event_id. Line 5928. |
| No event_id dependency | ✅ PASS | Selection is an index into visible display order, not a ring key. Survives ring overwrite (oldest events evicted). |
| No ring mutation | ✅ PASS | `bell_render_event_list()` reads ring via `bell_for_each_event()` only. `bell_select_next_row/prev_row` call `bell_render_event_list()` which does not write to `BELL_EVENTS`, `BELL_RING_WRITE_INDEX`, or `BELL_EVENT_SEQUENCE`. |
| No allocation | ✅ PASS | Static u8. No heap, no Vec, no dynamic allocation. |

## Repair/Clamp Behavior

| Ring State | Selected Before | Action | Selected After | Marker |
|------------|----------------|--------|----------------|--------|
| Empty (0 events) | any | Render repair | 0 | `[bell.selection.repair] old=N new=0 count=0` |
| 1 event | 0 | No repair needed | 0 | `[bell.selection.current] row=0 visible=1` |
| 1 event | 1 (invalid) | Clamp to 0 | 0 | `[bell.selection.repair] old=1 new=0 count=1` |
| 3 events | 2 | No repair needed | 2 | (valid, within range) |
| 3 events shrink to 1 | 2 | Clamp to 0 | 0 | `[bell.selection.repair] old=2 new=0 count=1` |
| 7 events shrink to 3 | 5 | Clamp to 2 | 2 | `[bell.selection.repair] old=5 new=2 count=3` |
| 0 events (render) | 0 (repaired) | Render returns early | 0 | No rows emitted, `[bell.event_list.done] count=0 rows=0 rects=0` |

**Repair formula:** `if visible == 0 { BELL_SELECTED_ROW = 0 } else if BELL_SELECTED_ROW >= visible { BELL_SELECTED_ROW = visible - 1 }`

All transitions produce proof markers. No silent clamp.

## Keyboard Precedence Table

Dispatch order for J (0x24) / K (0x25):

| Context | Handler | Line | Precedence | Result |
|---------|---------|------|------------|--------|
| Command palette open | Palette intercept | 8935-8936 | 1 (highest) | `palette_select_next/prev()`, sets `panel_consumed` |
| Atlas active | Atlas intercept | 8958 | 2 | `handle_atlas_keyboard()` |
| Bell focused | Bell intercept | 8961-8969 | 3 | `bell_select_next/prev_row()` |
| Linen focused (default) | `scancode_to_action` → `SelectNextLinenObject` | 2089-2090 / 9246-9268 | 4 | `linen_select_next/prev_object()` |
| Any other focus | `scancode_to_action` → `SelectNextLinenObject` | 2089-2090 / 9246-9268 | 4 | Action rejected (not Linen focused) |

### Specific Scenarios

| Scenario | Expected | Actual | Status |
|----------|----------|--------|--------|
| Bell focused, J pressed, palette closed | Bell next row | ✅ Bell nav at line 8966 |
| Bell focused, K pressed, palette closed | Bell prev row | ✅ Bell nav at line 8969 |
| Bell focused, J pressed, palette open | Palette next | ✅ Palette intercept at line 8935 fires first, sets panel_consumed |
| Bell focused, Enter pressed | Normal dispatch | ✅ Bell intercept skips (not J/K), falls through to scancode_to_action at line 8972 |
| Linen focused, J pressed | Linen next object | ✅ Bell intercept condition fails (FOCUSED_SURFACE_ID != Bell), falls through to Linen action |
| Palette open, J pressed (Bell not focused) | Palette next | ✅ Palette intercept fires regardless of focus |
| Atlas active, J pressed (Bell focused) | Atlas nav | ✅ Atlas intercept at line 8958 fires before Bell |
| Bell focused, non-J/K key | Normal dispatch | ✅ Falls through to scancode_to_action (line 8972) |

**Verdict: PASS** — all precedence rules correct. No interference between Bell,
palette, atlas, and Linen selection.

## Visual Behavior

### Row Color Logic (in `bell_render_event_list()`, line 6287-6293)

```rust
let base_color = bell_row_color(ev);
let row_color = if rows_emitted == BELL_SELECTED_ROW {
    let highlighted = bell_selected_row_highlight(base_color);
    highlighted
} else {
    base_color
};
```

| Aspect | Status | Evidence |
|--------|--------|----------|
| Header unchanged | ✅ PASS | Header always uses `BELL_PLACEHOLDER_COLOR` (0x00402020). No selection influence. Line 6251-6253. |
| Non-selected rows unchanged | ✅ PASS | Non-selected rows use `bell_row_color(ev)` directly. No change from M4. |
| Selected row brighter | ✅ PASS | `bell_selected_row_highlight()` adds 0x40 per channel, clamped at 0xFF. |
| Color format safety | ✅ PASS | `bell_selected_row_highlight()` returns `u32` in 0x00RRGGBB format. Used in `(row_color as u64) << 32` — bits 32-63. Never corrupts rect_index (bits 56-59). |
| No overflow in rect_index bits | ✅ PASS | Max color value is 0x00FFFFFF. Shifted by 32: 0x00FFFFFF00000000. rect_index occupies bits 56-59, which requires bits 56+. The color in bits 32-55 is distinct from rect_index bits 56-59. |
| proof marker per selected row | ✅ PASS | `[bell.selection_visual.row]` emitted with event_id, index, base color, highlight color. |
| No text rendering | ✅ PASS | Fill rects only. No text primitives. |

### Example Color Transformations (Verified)

| Base Color | Base R/G/B | Highlight R/G/B | Highlight Color |
|-----------|-----------|-----------------|----------------|
| 0x00C0A040 | 192/160/64 | 255/224/128 | 0x00FFE080 |
| 0x0040C080 | 64/192/128 | 128/255/192 | 0x0080FFC0 |
| 0x0040C0C0 | 64/192/192 | 128/255/255 | 0x0080FFFF |
| 0x006060C0 | 96/96/192 | 160/160/255 | 0x00A0A0FF |
| 0x00A060C0 | 160/96/192 | 224/160/255 | 0x00E0A0FF |
| 0x00404060 | 64/64/96 | 128/128/160 | 0x008080A0 |

All clamped values ≤ 0xFF. No overflow.

## Boundary Check

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/linen/ | ✅ CLEAN |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| WINDOWS Vec | ✅ CLEAN |

### Authority Check

| Concern | Status | Evidence |
|---------|--------|----------|
| No ack/delete/action semantics | ✅ PASS | Selection is purely visual. No Enter handler, no execute, no state mutation on ring. |
| No Bell PD | ✅ PASS | All state is `static mut` in silk-shell. No cross-PD communication. |
| No IPC changes | ✅ PASS | Only existing 0xEF opcode. No new PDX calls. |
| No sexdisplay changes | ✅ PASS | No sexdisplay source modified. |
| No Collar authority drift | ✅ PASS | No `collar_check_operation_stub()` calls in Bell selection path. |
| No Mesh graph drift | ✅ PASS | No `mesh_emit_linen_quil_links()` calls in Bell selection path. |
| No command execution from selection | ✅ PASS | Selection does not trigger any `SurfaceAction`, `palette_execute_selected()`, or `open_*` call. |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| No Enter/action on selected row | MEDIUM | M6 is selection-only. User cannot act on selected event. Deferred to M8 (event detail proof stub). |
| Selection not visible when Bell minimized | LOW | BELL_SELECTED_ROW persists across minimize/restore. On restore, repair logic handles ring changes. |
| No way to see which event is selected when Bell is closed | LOW | Selection is only visible when Bell surface is open. Expected UX — Bell is event feed. |
| Color brighten is simple additive, not perceptual brightness | LOW | +0x40 per channel works for all Bell event colors. May produce unexpected results for future event kinds with very bright base colors (clipping). Acceptable for V1. |
| J/K used across multiple surfaces (Linen, Bell, command palette) | LOW | Precedence rules prevent conflict. Each surface only receives J/K when it is the active context. |

## STOP FIRST Triggers

No STOP FIRST conditions triggered. M6 stays within shell-local state:
- No kernel edits
- No sex-pdx ABI changes
- No sexdisplay changes
- No Bell PD creation
- No heap allocation
- No new IPC opcodes
- No shared memory

## Exact Next Safest Step

**M8: Bell event detail proof stub** — Pressing Enter on the selected Bell row
emits proof markers only. No action, no ack, no delete, no Collar grant, no
real Bell PD dispatch. Proof-marker-only trace of what a future row-action
dispatch would look like.

After M8: evaluate whether to wire the selected event detail to existing
handler chains (view linked object in Linen/Quil), or move to subsystem work.

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| V1 | 2026-05-05 | Claude | Initial audit of M6 Bell selection |
