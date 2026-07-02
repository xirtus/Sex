# N4: Mesh Fact Row Rendering

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Render shell-local Mesh fact ring as row fill rects on the Mesh placeholder surface using the existing L7 multi-rect pattern. No selection, no keyboard nav, no actions.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N4_MESH_FACT_ROWS                         ║
╠══════════════════════════════════════════════════════════════╣
║ Surface:                  SURFACE_ID_MESH (202)              ║
║ Surface height:           480 (unchanged)                    ║
║ Row rects:                7 (header + 6 rows)               ║
║ Iteration order:          Newest-first                       ║
║ Color mapping:            linen_kind_color via subject_id    ║
║ Refresh wire:             open + conditional after record    ║
║ Tile placeholder:         Preserved (solid fill on tile)    ║
║ Boundaries:               INTAKT                             ║
║ Build:                    PASS (1612 sectors)                ║
╚══════════════════════════════════════════════════════════════╝
```

## Constants Added

| Constant | Value | Description |
|----------|-------|-------------|
| `MESH_LIST_HEADER_H` | 28 | Header bar height, pixels |
| `MESH_LIST_ROW_H` | 26 | Each fact row height, pixels |
| `MESH_LIST_ROW_GAP` | 2 | Vertical gap between row rects, pixels |
| `MESH_LIST_ROW_RECTS` | 7 | Max visual fill rects (header + 6 rows) |

All values match the Bell M4 constant pattern exactly.

## Surface Height Decision

| Concern | Decision |
|---------|----------|
| Mesh surface height | **480 (unchanged)** — SURFACE_202_H = 480 |
| Header + 6 rows height | 28 + 6*(26+2) = 196px, fits within 480 |
| No height increase needed | ✅ Sufficient for 7 rects (header + 6 rows) |

No global default height change required. The existing 640×480 surface is adequate.

## Functions Added

### `mesh_fact_row_color(fact: &MeshFact) -> u32`
Derives a deterministic row color from a Mesh fact:
- `ObjectLinkedToBuffer`: Looks up `fact.subject_id` (linen object_id) via `linen_object_by_id()`, returns `linen_kind_color(obj.kind)`
- Fallback: `0x00383010` (Mesh amber diagnostic color)

Mirrors `bell_row_color()` pattern exactly.

### `mesh_is_visible_in_active_scene() -> bool`
Returns true when:
1. Mesh frame exists (`frame_id == MESH_FRAME_ID`)
2. Frame is in active scene (`scene_id == ACTIVE_SCENE_IDX`)
3. Frame is not minimized (`FRAME_FLAG_MINIMIZED == 0`)
4. Surface is alive via `surface_is_alive()`

Mirrors `bell_is_visible_in_active_scene()` pattern exactly.

### `mesh_render_fact_list()`
Renders the Mesh fact ring as row fill rects:
1. **Header bar** (rect_index=0): `MESH_PLACEHOLDER_COLOR` (amber/diagnostic, 0x00383010)
2. **Fact rows** (rect_index=1..N): One rect per valid fact, newest-first iteration via `mesh_for_each_fact()`
3. **Termination**: Stops at `MESH_LIST_ROW_RECTS` (7), emits skip markers for overflow
4. **None slots**: Skipped by `mesh_for_each_fact()` (inner `if let Some(ref fact)` guard)

Mirrors `bell_render_event_list()` pattern exactly. No selection, no navigation, no actions.

## Wire Points

### 1. `open_mesh_in_active_scene()` (line ~5836)
**Replaced** the old single-fill placeholder with `mesh_render_fact_list()`:
```
Before: pdx_call(0xEF, SURFACE_ID_MESH, 0,
            (MESH_PLACEHOLDER_COLOR << 32) | ...
After:  mesh_render_fact_list();
```

This ensures the fact list is rendered on every Mesh open (including when Mesh becomes visible after being hidden).

### 2. After `mesh_record_fact()` (line ~1259)
Added conditional refresh:
```rust
if mesh_is_visible_in_active_scene() {
    serial_println!("[mesh.render.refresh] reason=visible_after_fact ...");
    mesh_render_fact_list();
}
```

This mirrors the Bell pattern where `bell_record_event()` conditionally refreshes if Bell is visible.

### 3. Tile placeholder (line ~2453, preserved unchanged)
The existing solid fill in the tile handler is **preserved** — same pattern as Bell (`BELL_PLACEHOLDER_COLOR` solid fill in tile handler, row rendering only on open). Tile placeholders are quick layout fills; detailed rendering happens on explicit open/render calls.

## Color Mapping

| Fact Kind | Derivation | Example Colors |
|-----------|------------|----------------|
| ObjectLinkedToBuffer | `linen_kind_color()` via `subject_id` | 0x00C0A040 (CodeFile), 0x0040C080 (Document), 0x0040C0C0 (QuilWorkspaceRef), 0x006060C0 (Reference), 0x00A060C0 (MeshDiagnosticRef) |
| Fallback | Mesh amber | 0x00383010 |

Colors are deterministic per linked object type. Same colors as Linen object list rows and Bell event rows for the same object kind.

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.fact_list.render]` | `mesh_render_fact_list()` start | Render starts with surface dimensions and fact count |
| `[mesh.fact_list.row]` | `mesh_render_fact_list()` loop | Per-fact row metadata (kind, IDs) |
| `[mesh.fact_list.skip]` | `mesh_render_fact_list()` loop | Row budget exceeded (max_rows) |
| `[mesh.fact_list.done]` | `mesh_render_fact_list()` end | Render complete with count, rows, rects |
| `[mesh.row_visual.rect]` | `mesh_render_fact_list()` loop | Per-row 0xEF fill rect sent (index, fact_id, kind, color) |
| `[mesh.row_visual.skip]` | `mesh_render_fact_list()` loop | Row fill rect budget exceeded (rect_budget) |
| `[mesh.render.refresh]` | `mesh_record_fact()` conditional | Mesh visible after fact record, refresh triggered |

## Existing Markers Preserved

| Marker | Location | Status |
|--------|----------|--------|
| `[mesh.fact.write]` | `mesh_record_fact()` | ✅ Preserved |
| `[mesh.fact.overwrite]` | `mesh_record_fact()` | ✅ Preserved |
| `[mesh.fact.done]` | `mesh_record_fact()` | ✅ Preserved |
| `[mesh.object_link.start]` | `mesh_emit_linen_quil_links()` | ✅ Preserved |
| `[mesh.object_link.row]` | `mesh_emit_linen_quil_links()` | ✅ Preserved |
| `[mesh.object_link.reject.missing_object]` | `mesh_emit_linen_quil_links()` | ✅ Preserved |
| `[mesh.object_link.done]` | `mesh_emit_linen_quil_links()` | ✅ Preserved |
| `[mesh.placeholder.open]` | `open_mesh_in_active_scene()` | ✅ Preserved |
| `[shell.mesh.tile.placeholder]` | Tile handler | ✅ Preserved |

## Boundary Check

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
| Heap allocation | ✅ CLEAN (static-only) |

### STOP FIRST Check

| Trigger | Status |
|---------|--------|
| New PDX opcodes | ✅ NOT TRIGGERED |
| sex-pdx ABI constants | ✅ NOT TRIGGERED |
| Capability grants/revokes | ✅ NOT TRIGGERED |
| Cross-PD pointers | ✅ NOT TRIGGERED |
| Kernel introspection | ✅ NOT TRIGGERED |
| Persistent storage | ✅ NOT TRIGGERED |
| Renderer policy | ✅ NOT TRIGGERED |
| Mesh PD creation | ✅ NOT TRIGGERED |
| Bell/Collar behavior | ✅ NOT TRIGGERED |
| sexdisplay changes | ✅ NOT TRIGGERED |
| Selection state | ✅ NOT TRIGGERED (no selection added) |
| Keyboard navigation | ✅ NOT TRIGGERED (no nav added) |

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| No keyboard nav or selection on Mesh | LOW | Deferred — render-only for V1 |
| Double render on open (render + fact record triggers) | LOW | Same pattern as Bell; harmless extra renders |
| Tile placeholder still uses solid fill | INFO | Same pattern as Bell/Quil/Collar — tile fills don't need row rendering |
| Only ObjectLinkedToBuffer kind supported | LOW | V1 design; more kinds added when needed |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1612 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Changed Files

- `servers/silk-shell/src/main.rs` — 104 insertions (constants, 3 helpers, render, 2 wire points)
- `docs/handoff/N4_MESH_FACT_ROW_RENDER_V1.md` — new

## Next Steps

**N5: Rapid audit of N4** — verify Mesh fact row rendering is safe and conformant before adding any Mesh selection/navigation/actions.

After N5: **N6: Mesh selection (optional)** — if needed, add row selection cycling via J/K. Or move to other subsystem work.
