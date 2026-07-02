# M6: Bell Selected-Row Visual + Keyboard Nav

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Add Bell selected-row visual highlight and J/K keyboard navigation
over the shell-local Bell event ring. Read-only on the ring: no ack, no delete,
no action, no Collar grant.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                    PASS_M6_BELL_SELECTION                    ║
╠══════════════════════════════════════════════════════════════╣
║ Selection state:             STATIC INDEX (read-only ring)   ║
║ Keyboard nav:                J/K when Bell focused            ║
║ Visual highlight:            +0x40 per RGB channel            ║
║ Repair on shrink:            CLAMP to visible-1               ║
║ Boundaries:                  INTAKT                           ║
║ Build:                       PASS                             ║
╚══════════════════════════════════════════════════════════════╝
```

## Selection State Model

```rust
/// Currently selected visible row index in the Bell event list.
/// 0 = newest event row. Repaired during render if ring shrinks.
static mut BELL_SELECTED_ROW: u8 = 0;
```

Key properties:
- **Index into visible rows**, not event_id. Row 0 = newest event (first displayed).
- **Range** 0..visible_event_count-1 (max 6 for 7 rows, but typically fewer).
- **No ring mutation** — the index is independent of the BellEvent ring.
- **Persistent** across Bell open/close cycles (static variable).
- **Repaired** on render if ring count shrinks below the stored index.

## Helpers Added

### `bell_visible_event_count() -> u8`
Returns `min(bell_ring_count(), BELL_LIST_ROW_RECTS)`. Used for clamping and
navigation bounds. Returns 0 when ring is empty.

### `bell_selected_row_highlight(color: u32) -> u32`
Brightens a 0x00RRGGBB color by adding 0x40 (~25%) to each RGB component with
per-channel clamping at 0xFF. Used to make the selected row visually distinct
from non-selected rows.

### `bell_select_next_row()`
Wraps forward through visible rows. Rejects if count ≤ 1 (nothing to navigate).
Calls `bell_render_event_list()` after update.

### `bell_select_prev_row()`
Wraps backward through visible rows. Rejects if count ≤ 1.
Calls `bell_render_event_list()` after update.

## Keyboard Navigation

**Gate:** `FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER`

| Key | Scancode | Action |
|-----|----------|--------|
| J | 0x24 | `bell_select_next_row()` — next visible event (wrap) |
| K | 0x25 | `bell_select_prev_row()` — previous visible event (wrap) |

**Placement in dispatch chain:**
```
panel intercept → command palette intercept → atlas intercept
→ Bell focused-surface intercept (NEW) → scancode_to_action dispatch
```

The Bell intercept fires ONLY when:
1. No panel/command palette/atlas consumed the key (those all come first)
2. `FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER`
3. Scancode is 0x24 (J) or 0x25 (K)

When Bell is NOT focused, J/K fall through to normal `scancode_to_action`
dispatch (SelectNextLinenObject / SelectPrevLinenObject, gated to Linen focus).

## Visual Behavior

**In `bell_render_event_list()`**, at the top:
1. Clamp repair: if `BELL_SELECTED_ROW >= visible`, set to `visible - 1`
2. Emit `[bell.selection.current]` with row and visible count

**Per-row rendering:**
- Selected row: `bell_selected_row_highlight(bell_row_color(ev))` — brightened by +0x40 per channel
- Non-selected rows: `bell_row_color(ev)` — unchanged from M4

Example color transformations:
| Event Kind | Base Color | Selected Highlight |
|------------|-----------|-------------------|
| CodeFile | 0x00C0A040 | 0x00FFE080 |
| Document | 0x0040C080 | 0x0080FFC0 |
| QuilWorkspaceRef | 0x0040C0C0 | 0x0080FFFF |
| Reference | 0x006060C0 | 0x00A0A0FF |
| MeshDiagnosticRef | 0x00A060C0 | 0x00E0A0FF |
| Fallback | 0x00404060 | 0x008080A0 |

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Ring empty (0 events) | `BELL_SELECTED_ROW = 0`, no rows rendered |
| Ring shrinks below selected | Clamped to `visible - 1`, `[bell.selection.repair]` emitted |
| Single event (count=1) | J/K rejects: `[bell.selection.reject] reason=single_or_empty` |
| Bell minimized while selected | Selection preserved; on restore, render repairs if needed |
| Events added while Bell hidden | Selection repaired on next `bell_render_event_list()` call |
| J/K while command palette open | Palette intercept fires first (consumes J/K), Bell never reached |
| J/K while Atlas active | Atlas intercept fires first, Bell never reached |

## Proof Markers

| Marker | Location | Description |
|--------|----------|-------------|
| `[bell.selection.current]` | `bell_render_event_list()` | Current selected row + visible count |
| `[bell.selection.next]` | `bell_select_next_row()` | Next navigation with prev/next values |
| `[bell.selection.prev]` | `bell_select_prev_row()` | Previous navigation with prev/next values |
| `[bell.selection.repair]` | `bell_render_event_list()` | Clamp after ring shrink (old/new/count) |
| `[bell.selection.reject]` | nav helpers | Reject when count ≤ 1 |
| `[bell.selection_visual.row]` | per-row render | Selected row highlight applied (event_id, index, base, highlight) |
| `[bell.keyboard.next]` | keyboard handler | J key consumed for Bell |
| `[bell.keyboard.prev]` | keyboard handler | K key consumed for Bell |
| Existing M4 markers | Unchanged | All `[bell.event_list.*]`, `[bell.row_visual.*]`, `[bell.render.*]`, `[bell.ring.*]` |

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Ring mutation | ✅ NONE (read-only index) |
| Bell PD creation | ✅ NONE |
| New event kinds | ✅ NONE |
| Ack/delete/action | ✅ NONE |
| Text rendering | ✅ NONE |
| Collar authority | ✅ NONE |
| Mesh graph | ✅ NONE |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1609 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Changed Files

- `servers/silk-shell/src/main.rs` — 80 insertions, 1 deletion
- `docs/handoff/M6_BELL_SELECTED_ROW_NAV_V1.md` — new

## Next Steps

**M7: Rapid audit of M6** — close the Bell selection milestone.
After M7: evaluate Bell row actions (view linked object in Linen/Quil) using
existing handler chains, or move to other real feature work.

M6 proves selection state + navigation + visual are safe and read-only.
Ack/delete/action on Bell rows would require M8 (Bell row action dispatch)
and remains blocked until M7 closes.
