# N5: Audit Mesh Fact Row Rendering

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Verify N4 Mesh fact row rendering is safe and conformant. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N4_MESH_FACT_ROWS                         ║
╠══════════════════════════════════════════════════════════════╣
║ Render/read behavior:       PASS                             ║
║ Refresh behavior:           PASS                             ║
║ Renderer boundary:          INTAKT                            ║
║ Authority boundary:         INTAKT                            ║
║ Existing markers preserved: PASS                             ║
║ Build:                      PASS (1612 sectors)              ║
╚══════════════════════════════════════════════════════════════╝
```

## 1. Render/Read Conformance Table

| Criterion | Expected (N4 Spec) | Actual (Implementation) | Status |
|-----------|--------------------|-------------------------|--------|
| Iterates fact ring read-only | No mutation during render | `mesh_for_each_fact()` passes `&MeshFact` reference, never writes to ring (line 1395) | ✅ PASS |
| Newest-first order | `(total-1) % cap` start, backwards | `start = (total-1) % CAP`, `idx = (start + CAP - i) % CAP` (lines 1279-1281) | ✅ PASS |
| None/empty slots skipped | Inner `if let Some` guard | `if let Some(ref fact) = MESH_FACTS[idx]` (line 1282) | ✅ PASS |
| Row count <= MAX_RECTS | 7 rects (header + 6 rows) | `MESH_LIST_ROW_RECTS = 7` (line 5717), header rect_index=0, rows 1-6, fits within sexdisplay MAX_RECTS=8 | ✅ PASS |
| Empty ring returns early | `mesh_fact_count() == 0` check | `if count == 0 { return; }` in `mesh_for_each_fact()` (line 1277) | ✅ PASS |
| Header uses MESH_PLACEHOLDER_COLOR | Amber diagnostic 0x00383010 | `MESH_PLACEHOLDER_COLOR` (line 1388) | ✅ PASS |
| Row color from linen_kind_color via subject_id | Deterministic per object kind | `mesh_fact_row_color()` → `linen_object_by_id(subject_id)` → `linen_kind_color(obj.kind)` (lines 1343-1351) | ✅ PASS |
| Row color fallback | Mesh amber 0x00383010 | `0x00383010` for missing object (line 1349) | ✅ PASS |
| arg2 rect-index packing | `(rect_index<<56) \| (color<<32) \| (sh<<16) \| sw` | Same format (lines 1411-1414) | ✅ PASS |
| Matching Bell M4 pattern | Same header/row/gap constants | `MESH_LIST_HEADER_H=28`, `MESH_LIST_ROW_H=26`, `MESH_LIST_ROW_GAP=2` — identical to Bell | ✅ PASS |

**Render/Read: PASS** — No mutation during render, newest-first, safe empty-slot handling, fits within MAX_RECTS.

## 2. Refresh Behavior Table

| Scenario | Expected Behavior | Actual | Status |
|----------|------------------|--------|--------|
| **Open Mesh** (no existing links) | Render empty fact list | `mesh_render_fact_list()` called at line 5836 → count=0, header only | ✅ PASS |
| **Open Mesh** (with existing links) | Render, then emit_links records facts → conditional refresh | `mesh_render_fact_list()` at 5836, then `mesh_emit_linen_quil_links()` → `mesh_record_fact()` → `mesh_is_visible_in_active_scene()` true → `mesh_render_fact_list()` (lines 5836-1261) | ✅ PASS (double render harmless) |
| **Link object to buffer while Mesh open** | Record fact, then refresh Mesh | `mesh_record_fact()` → `mesh_is_visible()` true → `mesh_render_fact_list()` (lines 1259-1262) | ✅ PASS |
| **Link object to buffer while Mesh hidden** | Record fact only, no render | `mesh_is_visible_in_active_scene()` false → no render (line 1259) | ✅ PASS |
| **Focus existing Mesh (duplicate guard)** | No render, no fact write | Duplicate guard at line 5779 returns early, no render, no link emit | ✅ PASS |
| **Stale ref path** | Reject marker only, no fact, no render | `[mesh.object_link.reject.missing_object]` at line 1321, no `mesh_record_fact()` call | ✅ PASS |
| **Minimize/restore Mesh** | Existing lifecycle paths, no extra render | Minimize at line 5876, restore via open at 5868 → `open_mesh_in_active_scene()` → render | ✅ PASS |
| **Tile placeholder unchanged** | Solid fill during layout | Line 2453: `pdx_call(0xEF, SURFACE_ID_MESH, ...)` with `MESH_PLACEHOLDER_COLOR` | ✅ PASS |

**Refresh Behavior: PASS** — All expected behaviors match implementation. Double render on open with existing links is harmless (same pattern as Bell).

## 3. Renderer Boundary Check

| Concern | Result |
|---------|--------|
| Uses existing 0xEF rect-index path only | ✅ Yes — all renders via `pdx_call(SLOT_DISPLAY, 0xEF, ...)` (lines 1386, 1409) |
| sexdisplay source modified? | ✅ No — forbidden area clean |
| Number of 0xEF calls increased per surface? | ✅ Yes — from 1 (solid fill) to up to 7 (header + 6 rows), all within sexdisplay MAX_RECTS=8 |
| arg2 rect_index packing correct? | ✅ Yes — `(rect_index & 0xF) << 56` per L7 audit |
| Preserves framebuffer bounds checks? | ✅ Yes — sexdisplay handles bounds |
| New display primitives? | ✅ No — 0xEF fill rects only |
| Text rendering? | ✅ No — no text, no strings on surface |
| Renderer-owned topology policy? | ✅ No — Mesh fact ring is shell-local, renderer treats colors as opaque |

**Verdict: RENDERER_BOUNDARY_INTAKT**

## 4. Authority/Subsystem Boundary Check

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/mesh/ | ✅ CLEAN (no Mesh PD, no PD creation) |
| servers/linen/ | ✅ CLEAN (read-only via linen_object_by_id) |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Bell ring/code | ✅ CLEAN |
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
| Selection state | ✅ NOT ADDED |
| Keyboard navigation | ✅ NOT ADDED |

**Authority/Subsystem Boundary: INTAKT**

## 5. Existing Marker Preservation

| Marker | Location | Status |
|--------|----------|--------|
| `[mesh.fact.write]` | mesh_record_fact() | ✅ Preserved (line 1254) |
| `[mesh.fact.overwrite]` | mesh_record_fact() | ✅ Preserved (line 1251) |
| `[mesh.fact.done]` | mesh_record_fact() | ✅ Preserved (line 1256) |
| `[mesh.object_link.start]` | mesh_emit_linen_quil_links() | ✅ Preserved (line 1299) |
| `[mesh.object_link.row]` | mesh_emit_linen_quil_links() | ✅ Preserved (line 1303) |
| `[mesh.object_link.reject.missing_object]` | mesh_emit_linen_quil_links() | ✅ Preserved (line 1321) |
| `[mesh.object_link.done]` | mesh_emit_linen_quil_links() | ✅ Preserved (line 1332) |
| `[mesh.placeholder.open]` | open_mesh_in_active_scene() | ✅ Preserved (line 5838) |
| `[shell.mesh.tile.placeholder]` | Tile handler | ✅ Preserved (line 2456) |

### N4 New Markers Verified

| Marker | Location | Trigger | Status |
|--------|----------|---------|--------|
| `[mesh.fact_list.render]` | mesh_render_fact_list() | Render start | ✅ Present (line 1382) |
| `[mesh.fact_list.row]` | mesh_render_fact_list() | Per-fact metadata | ✅ Present (line 1400) |
| `[mesh.fact_list.skip]` | mesh_render_fact_list() | Row budget exceeded | ✅ Present (line 1397) |
| `[mesh.fact_list.done]` | mesh_render_fact_list() | Render complete | ✅ Present (line 1423) |
| `[mesh.row_visual.rect]` | mesh_render_fact_list() | Row fill rect sent | ✅ Present (line 1415) |
| `[mesh.row_visual.skip]` | mesh_render_fact_list() | Rect budget exceeded | ✅ Present (line 1419) |
| `[mesh.render.refresh]` | mesh_record_fact() | Mesh visible after fact record | ✅ Present (line 1260) |

**Existing and New Markers: ALL PRESENT**

## 6. Key Code Path Trace

### Open with no links
```
open_mesh_in_active_scene() [line 5836]
  → mesh_render_fact_list()          [render empty list, header only]
  → mesh_emit_linen_quil_links()     [scan buffers, no links → done]
  → [mesh.placeholder.open]
```

### Open with existing links (e.g., 2 seed links)
```
open_mesh_in_active_scene() [line 5836]
  → mesh_render_fact_list()          [render current fact ring state]
  → mesh_emit_linen_quil_links()     [scan buffers]
    → [mesh.object_link.row] x2      [for each valid link]
    → mesh_record_fact() x2          [record 2 facts]
      → [mesh.fact.write] x2
      → mesh_is_visible() → true x2  [Mesh is visible → refresh]
      → mesh_render_fact_list() x2   [render after each fact]
  → [mesh.placeholder.open]
```
Result: 3 renders total (1 initial + 2 refreshes). Harmless.

### Link operation while Mesh visible (via J4 path)
```
open_linen_object_in_quil() [line 1089]
  → mesh_emit_linen_quil_links()
    → mesh_record_fact()
      → [mesh.fact.write]
      → mesh_is_visible() → true
      → mesh_render_fact_list()      [live refresh]
  → [linen.quil.done]
```

### Link operation while Mesh hidden
```
open_linen_object_in_quil() [line 1089]
  → mesh_emit_linen_quil_links()
    → mesh_record_fact()
      → [mesh.fact.write]
      → mesh_is_visible() → false
      → no render                   [correct — no visible surface]
```

## 7. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Double/triple render on open with existing links | LOW | Harmless — same pattern as Bell; renders are cheap (PDX calls only) |
| No keyboard nav on Mesh | LOW | Deferred — N6 would add J/K read-only selection |
| No selection highlight on Mesh | LOW | Deferred — N6 adds visual selection |
| Tile placeholder still uses solid fill | INFO | Same pattern as all other placeholder surfaces |
| Only ObjectLinkedToBuffer kind supported | LOW | V1 design; more kinds added when needed (no code changes to existing) |

## 8. Next Safest Step

**N6: Mesh selected-row visual + J/K keyboard navigation** — Mirror Bell M6 pattern:
1. Add `MESH_SELECTED_ROW` static (u8, start 0)
2. Add `mesh_selected_row_highlight(color)` — +0x40 per channel with clamping
3. Add `mesh_select_next_row()` / `mesh_select_prev_row()` — J/K wrap navigation
4. Add keyboard intercept for J/K when Mesh surface focused (same dispatch chain)
5. Add selection repair on render (clamp if ring shrinks)
6. Read-only on ring — no ack, no delete, no action, no Collar grant

After N6: Mesh will have full row visibility + selection parity with Bell.
