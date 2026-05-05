# N6: Mesh Selected-Row Visual + Keyboard Nav

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Add Mesh selected-row visual highlight and J/K keyboard navigation over the shell-local Mesh fact ring. Read-only on the ring: no ack, no delete, no action, no Collar grant.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                    PASS_N6_MESH_SELECTION                    ║
╠══════════════════════════════════════════════════════════════╣
║ Selection state:             STATIC INDEX (read-only ring)   ║
║ Keyboard nav:                J/K when Mesh focused            ║
║ Visual highlight:            +0x40 per RGB channel            ║
║ Repair on shrink:            CLAMP to visible-1               ║
║ Boundaries:                  INTAKT                           ║
║ Build:                       PASS (1618 sectors)             ║
╚══════════════════════════════════════════════════════════════╝
```

## Selection State Model

```rust
/// Currently selected visible row index in the Mesh fact list.
/// 0 = newest fact row. Repaired during render if ring shrinks.
static mut MESH_SELECTED_ROW: u8 = 0;
```

Key properties:
- **Index into visible rows**, not fact_id. Row 0 = newest fact (first displayed).
- **Range** 0..visible_fact_count-1 (max 6 for 7 rows, but typically fewer).
- **No ring mutation** — the index is independent of the MeshFact ring.
- **Persistent** across Mesh open/close cycles (static variable).
- **Repaired** on render if ring count shrinks below the stored index.

## Helpers Added

### `mesh_visible_fact_count() -> u8`
Returns `min(mesh_fact_count(), MESH_LIST_ROW_RECTS)`. Used for clamping and navigation bounds. Returns 0 when ring is empty.

### `mesh_selected_row_highlight(color: u32) -> u32`
Brightens a 0x00RRGGBB color by adding 0x40 (~25%) to each RGB component with per-channel clamping at 0xFF. Used to make the selected row visually distinct from non-selected rows.

### `mesh_select_next_row()`
Wraps forward through visible rows. Rejects if count ≤ 1 (nothing to navigate). Calls `mesh_render_fact_list()` after update.

### `mesh_select_prev_row()`
Wraps backward through visible rows. Rejects if count ≤ 1. Calls `mesh_render_fact_list()` after update.

## Keyboard Navigation

**Gate:** `FOCUSED_SURFACE_ID == SURFACE_ID_MESH`

| Key | Scancode | Action |
|-----|----------|--------|
| J | 0x24 | `mesh_select_next_row()` — next visible fact (wrap) |
| K | 0x25 | `mesh_select_prev_row()` — previous visible fact (wrap) |

**Placement in dispatch chain:**
```
panel intercept → command palette intercept → atlas intercept
→ Bell focused-surface intercept → Mesh focused-surface intercept (NEW)
→ scancode_to_action dispatch
```

The Mesh intercept fires ONLY when:
1. No panel/command palette/atlas/Bell consumed the key (those all come first)
2. `FOCUSED_SURFACE_ID == SURFACE_ID_MESH`
3. Scancode is 0x24 (J) or 0x25 (K)

When Mesh is NOT focused, J/K fall through to normal `scancode_to_action` dispatch (SelectNextLinenObject / SelectPrevLinenObject, gated to Linen focus).

## Visual Behavior

**In `mesh_render_fact_list()`**, at the top:
1. Clamp repair: if `MESH_SELECTED_ROW >= visible`, set to `visible - 1`
2. Emit `[mesh.selection.current]` with row and visible count

**Per-row rendering:**
- Selected row: `mesh_selected_row_highlight(mesh_fact_row_color(fact))` — brightened by +0x40 per channel
- Non-selected rows: `mesh_fact_row_color(fact)` — unchanged from N4

Example color transformations:

| Subject Kind | Base Color | Selected Highlight |
|-------------|-----------|-------------------|
| CodeFile | 0x00C0A040 | 0x00FFE080 |
| Document | 0x0040C080 | 0x0080FFC0 |
| QuilWorkspaceRef | 0x0040C0C0 | 0x0080FFFF |
| Reference | 0x006060C0 | 0x00A0A0FF |
| MeshDiagnosticRef | 0x00A060C0 | 0x00E0A0FF |
| Fallback (no object) | 0x00383010 | 0x00787050 |

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Ring empty (0 facts) | `MESH_SELECTED_ROW = 0`, no rows rendered |
| Ring shrinks below selected | Clamped to `visible - 1`, `[mesh.selection.repair]` emitted |
| Single fact (count=1) | J/K rejects: `[mesh.selection.reject] reason=single_or_empty` |
| Mesh minimized while selected | Selection preserved; on restore, render repairs if needed |
| Facts added while Mesh hidden | Selection repaired on next `mesh_render_fact_list()` call |
| J/K while command palette open | Palette intercept fires first (consumes J/K), Mesh never reached |
| J/K while Atlas active | Atlas intercept fires first, Mesh never reached |
| J/K while Bell focused | Bell intercept fires first, Mesh never reached |

## Proof Markers

| Marker | Location | Description |
|--------|----------|-------------|
| `[mesh.selection.current]` | `mesh_render_fact_list()` | Current selected row + visible count |
| `[mesh.selection.next]` | `mesh_select_next_row()` | Next navigation with prev/next values |
| `[mesh.selection.prev]` | `mesh_select_prev_row()` | Previous navigation with prev/next values |
| `[mesh.selection.repair]` | `mesh_render_fact_list()` | Clamp after ring shrink (old/new/count) |
| `[mesh.selection.reject]` | nav helpers | Reject when count ≤ 1 |
| `[mesh.selection_visual.row]` | per-row render | Selected row highlight applied (fact_id, index, base, highlight) |
| `[mesh.keyboard.next]` | keyboard handler | J key consumed for Mesh |
| `[mesh.keyboard.prev]` | keyboard handler | K key consumed for Mesh |

### Existing N4/N2 Markers Preserved

| Marker | Location | Status |
|--------|----------|--------|
| `[mesh.fact_list.render]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.fact_list.row]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.fact_list.skip]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.fact_list.done]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.row_visual.rect]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.row_visual.skip]` | `mesh_render_fact_list()` | ✅ Preserved |
| `[mesh.render.refresh]` | `mesh_record_fact()` | ✅ Preserved |
| `[mesh.fact.write]` | `mesh_record_fact()` | ✅ Preserved |
| `[mesh.object_link.*]` | J6 helpers | ✅ Preserved |
| `[mesh.placeholder.*]` | I1 lifecycle | ✅ Preserved |

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/mesh/ | ✅ CLEAN (no Mesh PD) |
| servers/linen/ | ✅ CLEAN |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Bell ring | ✅ CLEAN |
| WINDOWS Vec | ✅ CLEAN |
| Lifecycle enum | ✅ CLEAN |
| Ring mutation | ✅ NONE (read-only index) |
| Mesh PD creation | ✅ NONE |
| New fact kinds | ✅ NONE |
| Ack/delete/action | ✅ NONE |
| Text rendering | ✅ NONE |
| Collar authority | ✅ NONE |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| No Enter/action on selected row | LOW | Deferred — read-only navigation only |
| Selection cannot trigger Collar grants | LOW | No grants/authority in V1 |
| Only one fact kind supported | LOW | V1 design; more kinds added when needed |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1618 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Changed Files

- `servers/silk-shell/src/main.rs` — 87 insertions, 13 deletions
- `docs/handoff/N6_MESH_SELECTED_ROW_NAV_V1.md` — new

## Next Steps

**N7: Rapid audit of N6** — close the Mesh selection milestone.
After N7: evaluate Mesh row actions (view linked object in Linen/Quil) using existing handler chains, or move to other feature work.

N6 proves selection state + navigation + visual are safe and read-only.
Ack/delete/action on Mesh rows would require N8 (Mesh row action dispatch) and remains blocked until N7 closes.
