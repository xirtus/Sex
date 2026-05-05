# N7: Audit Mesh Selection

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Verify N6 Mesh selected-row navigation and visual are safe and conformant. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N6_MESH_SELECTION                         ║
╠══════════════════════════════════════════════════════════════╣
║ Selection state:              PASS                            ║
║ Keyboard precedence:          PASS                            ║
║ Visual behavior:              PASS                            ║
║ Boundary check:               INTAKT                           ║
║ Existing markers preserved:   PASS                            ║
║ Build:                        PASS (1618 sectors)             ║
╚══════════════════════════════════════════════════════════════╝
```

## 1. Selection State Table

| Criterion | Expected (N6 Spec) | Actual (Implementation) | Status |
|-----------|--------------------|-------------------------|--------|
| Selection is visible-row index, not fact_id | Index into visible rows (0 = newest) | `MESH_SELECTED_ROW: u8` — static index, independent of fact_id (line 5784) | ✅ PASS |
| No fact/ring mutation | Read-only on fact ring | Selection stored separately from `MESH_FACTS` ring. `mesh_for_each_fact()` passes `&MeshFact` — never writes to ring (line 1282) | ✅ PASS |
| No ring mutation in render | `mesh_for_each_fact` is read-only | `FnMut(&MeshFact)` — read-only reference (line 1274) | ✅ PASS |
| Repair: empty ring (0 facts) | MESH_SELECTED_ROW = 0 | `if visible == 0 { MESH_SELECTED_ROW = 0; }` (line 1384-1385) | ✅ PASS |
| Repair: ring shrinks below selected | Clamp to visible - 1 | `else if MESH_SELECTED_ROW >= visible { MESH_SELECTED_ROW = visible.wrapping_sub(1); }` (lines 1386-1388) | ✅ PASS |
| Repair proof marker | `[mesh.selection.repair] old/new/count` | `serial_println!("[mesh.selection.repair] old={} new={} count={}", ...)` (line 1389) | ✅ PASS |
| Current marker | `[mesh.selection.current] row/visible` | `serial_println!("[mesh.selection.current] row={} visible={}", ...)` (line 1391) | ✅ PASS |
| Range guard | `MESH_SELECTED_ROW` always < visible | Clamp before any render use (lines 1384-1389) | ✅ PASS |
| Single fact (count=1) repair | Selection stays 0, navigation rejects | Reject in `mesh_select_next_row/prev_row` when `count <= 1` (lines 1464, 1478) | ✅ PASS |
| Persistence across open/close | Static, not reset | `static mut` — persists across Mesh toggle cycles | ✅ PASS |

### Count Helper

| Function | Purpose | Status |
|----------|---------|--------|
| `mesh_visible_fact_count()` | Returns `min(mesh_fact_count(), MESH_LIST_ROW_RECTS)` | ✅ Line 1446-1449 |
| Empty ring returns 0 | Early guard | ✅ `if count == 0 { return 0; }` (line 1448) |
| Caps at ROW_RECTS | Visible row budget | ✅ `core::cmp::min(count as u8, MESH_LIST_ROW_RECTS)` (line 1449) |

### Navigation Helpers

| Function | Behavior | Status |
|----------|----------|--------|
| `mesh_select_next_row()` | Next visible row, wrap, reject count≤1 | ✅ Lines 1462-1473 |
| `mesh_select_prev_row()` | Prev visible row, wrap, reject count≤1 | ✅ Lines 1476-1487 |
| Wrap formula (next) | `if current + 1 >= count { 0 } else { current + 1 }` | ✅ Line 1469 |
| Wrap formula (prev) | `if current == 0 { count - 1 } else { current - 1 }` | ✅ Line 1483 |
| Reject proof marker | `[mesh.selection.reject] reason=single_or_empty` | ✅ Lines 1465, 1479 |
| Next proof marker | `[mesh.selection.next] prev={} next={}` | ✅ Line 1471 |
| Prev proof marker | `[mesh.selection.prev] prev={} next={}` | ✅ Line 1485 |
| Re-render after nav | `mesh_render_fact_list()` called after mutation | ✅ Lines 1472, 1486 |

**Selection State: PASS** — No ring mutation, safe clamp/repair, correct wrap navigation.

## 2. Keyboard Precedence Table

| Priority | Handler | J/K Behavior | Status |
|----------|---------|-------------|--------|
| 1 | Panel/command palette | Consumes J/K if palette is open (backtick toggle) | ✅ Lines 9240-9259 |
| 2 | Atlas intercept | Consumes all keys except F10 when Atlas active | ✅ Lines 9260-9265 |
| 3 | Bell focused-surface | Consumes J/K + Enter when `FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER` | ✅ Lines 9266-9285 |
| **4** | **Mesh focused-surface (NEW)** | **Consumes J/K when `FOCUSED_SURFACE_ID == SURFACE_ID_MESH`** | **✅ Lines 9286-9301** |
| 5 | scancode_to_action dispatch | J/K → SelectNext/PrevLinenObject (Linen focus only) | ✅ Lines 9302+ |

**Precedence: PASS** — Mesh is correctly positioned after Bell but before scancode_to_action.

### Gate Conditions

| Condition | Bell Intercept | Mesh Intercept | scancode_to_action | Status |
|-----------|---------------|----------------|-------------------|--------|
| Bell focused, J/K | ✅ Consumes | ❌ Not reached | ❌ Not reached | ✅ Correct |
| Mesh focused, J/K | ❌ Not Bell focused | ✅ Consumes | ❌ Not reached | ✅ Correct |
| Linen focused, J/K | ❌ Not Bell focused | ❌ Not Mesh focused | ✅ SelectNext/PrevLinenObject | ✅ Correct |
| Bell focused, Enter | ✅ Consumed by Bell | ❌ Not reached | ❌ Not reached | ✅ Correct |
| Mesh focused, Enter | ❌ Not Bell focused | ❌ Mesh only matches J/K | ✅ scancode_to_action | ✅ Correct (no Enter action for Mesh) |
| Palette open, any key | ✅ Consumed by panel | ❌ Not reached | ❌ Not reached | ✅ Correct |
| Atlas active, any key | ✅ Consumed by Atlas | ❌ Not reached | ❌ Not reached | ✅ Correct |

**Gate Conditions: PASS** — All focus/distpatch scenarios produce correct behavior.

## 3. Visual Behavior Table

| Aspect | Expected (N6 Spec) | Actual (Implementation) | Status |
|--------|--------------------|-------------------------|--------|
| Selected row highlight | `mesh_selected_row_highlight(base_color)` | `if rows_emitted == MESH_SELECTED_ROW { let highlighted = mesh_selected_row_highlight(base_color); ... }` (lines 1420-1425) | ✅ PASS |
| Highlight function | +0x40 per channel, clamp 0xFF | `core::cmp::min(((color >> 16) & 0xFF).wrapping_add(0x40), 0xFF)` etc. (lines 1454-1458) | ✅ PASS |
| Non-selected rows unchanged | `mesh_fact_row_color(fact)` | `else { base_color }` (lines 1426-1427) | ✅ PASS |
| Header unchanged | `MESH_PLACEHOLDER_COLOR` | Same as N4 (line 1398) | ✅ PASS |
| Visual proof marker | `[mesh.selection_visual.row]` | `serial_println!("[mesh.selection_visual.row] fact_id={} index={} base={:#010x} highlight={:#010x}", ...)` (lines 1422-1423) | ✅ PASS |
| No color/rect corruption | Highlight only modifies RGB bits | `mesh_selected_row_highlight` only touches bits 0-23 (RGB), leaves bits 24-31 zero | ✅ PASS |
| No text rendering | Fill rects only | 0xEF calls only — no text primitives | ✅ PASS |

### Color Safety

The `mesh_selected_row_highlight` function operates on a 0x00RRGGBB color and returns the same format:

```rust
fn mesh_selected_row_highlight(color: u32) -> u32 {
    let r = core::cmp::min(((color >> 16) & 0xFF).wrapping_add(0x40), 0xFF);
    let g = core::cmp::min(((color >> 8) & 0xFF).wrapping_add(0x40), 0xFF);
    let b = core::cmp::min((color & 0xFF).wrapping_add(0x40), 0xFF);
    (r << 16) | (g << 8) | b
}
```

- Only bits 0-23 (RGB) are modified
- Bits 24-31 remain zero (0x00RRGGBB format)
- The result is OR'd into arg2 at bits 32-55: `((row_color as u64) << 32)`
- The rect_index is at bits 56-59, completely isolated from color bits
- No risk of corrupting rect_index or surface geometry

**Visual Behavior: PASS** — Correct highlight application, no color/rect packing corruption.

## 4. Boundary Check

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
| Mesh fact ring | ✅ CLEAN (no mutation) |
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
| Enter/action behavior | ✅ NOT ADDED |
| Text rendering | ✅ NOT ADDED |

**Boundaries: INTAKT** — All forbidden areas clean.

## 5. Existing Marker Preservation

| Marker | Location | Status |
|--------|----------|--------|
| `[mesh.fact_list.render]` | mesh_render_fact_list() | ✅ Preserved (line 1393) |
| `[mesh.fact_list.row]` | mesh_render_fact_list() | ✅ Preserved (lines 1404-1405) |
| `[mesh.fact_list.skip]` | mesh_render_fact_list() | ✅ Preserved (line 1400-1401) |
| `[mesh.fact_list.done]` | mesh_render_fact_list() | ✅ Preserved (line 1439) |
| `[mesh.row_visual.rect]` | mesh_render_fact_list() | ✅ Preserved (lines 1431-1432) |
| `[mesh.row_visual.skip]` | mesh_render_fact_list() | ✅ Preserved (line 1435) |
| `[mesh.render.refresh]` | mesh_record_fact() | ✅ Preserved (line 1322) |
| `[mesh.fact.write]` | mesh_record_fact() | ✅ Preserved (line 1316) |
| `[mesh.object_link.*]` | J6 helpers | ✅ Preserved |
| `[mesh.placeholder.*]` | I1 lifecycle | ✅ Preserved |

### N6 New Markers Verified

| Marker | Location | Trigger | Status |
|--------|----------|---------|--------|
| `[mesh.selection.current]` | mesh_render_fact_list() | Selected row + visible count | ✅ Present (line 1391) |
| `[mesh.selection.next]` | mesh_select_next_row() | Next navigation | ✅ Present (line 1471) |
| `[mesh.selection.prev]` | mesh_select_prev_row() | Previous navigation | ✅ Present (line 1485) |
| `[mesh.selection.repair]` | mesh_render_fact_list() | Clamp after shrink | ✅ Present (line 1389) |
| `[mesh.selection.reject]` | nav helpers | Reject count≤1 | ✅ Present (lines 1465, 1479) |
| `[mesh.selection_visual.row]` | per-row render | Selected row highlight | ✅ Present (line 1422) |
| `[mesh.keyboard.next]` | keyboard handler | J key consumed | ✅ Present (line 9292) |
| `[mesh.keyboard.prev]` | keyboard handler | K key consumed | ✅ Present (line 9296) |

**Markers: ALL PRESENT**

## 6. Key Code Path Trace

### Mesh open, navigate down one row
```
open_mesh_in_active_scene()
  → mesh_render_fact_list()
    → [mesh.selection.current] row=0 visible=N
    → [mesh.fact_list.render] ...
    → [mesh.fact_list.row] fact_id=...  (row 0, selected → highlighted)
    → [mesh.selection_visual.row] fact_id=... index=0 ...
    → [mesh.row_visual.rect] ...
    → [mesh.fact_list.row] fact_id=...  (row 1, not selected → base color)
    → [mesh.row_visual.rect] ...
    → [mesh.fact_list.done] ...

User presses J:
  → [mesh.keyboard.next] sid=202
  → mesh_select_next_row()
    → [mesh.selection.next] prev=0 next=1
    → mesh_render_fact_list()
      → [mesh.selection.current] row=1 visible=N
      → ...
      → [mesh.selection_visual.row] fact_id=... index=1 ...  (row 1 now highlighted)
      → ...
```

### Mesh open, ring empty (0 facts)
```
open_mesh_in_active_scene()
  → mesh_render_fact_list()
    → mesh_visible_fact_count() = 0
    → MESH_SELECTED_ROW = 0
    → [mesh.selection.current] row=0 visible=0
    → [mesh.fact_list.render] count=0
    → header only (no rows)
    → [mesh.fact_list.done] count=0 rows=0 rects=0
```

### J/K while Linen focused (Bell and Mesh not focused)
```
scancode 0x24 (J):
  → panel_consumed? false
  → Atlas active? false
  → Bell focused? false (FOCUSED_SURFACE_ID == SURFACE_ID_LINEN)
  → Mesh focused? false
  → scancode_to_action(0x24) → Some(SelectNextLinenObject)
  → SurfaceAction::SelectNextLinenObject
  → [linen.object_select.next] ...
```

## 7. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| No Enter/action on selected row | LOW | Deferred — N8 adds detail proof stub (markers only) |
| Selection cannot trigger Collar grants | LOW | No grants/authority in V1 |
| Only one fact kind supported | LOW | V1 design; more kinds added when needed |
| No zoom/scroll for many facts | LOW | 7-row budget limits visible facts; overflow silently skipped |

## 8. Next Safest Step

**N8: Mesh selected-fact detail proof stub** — Mirror Bell M8 pattern:
1. Add `mesh_selected_fact_snapshot()` — maps `MESH_SELECTED_ROW` to ring fact copy
2. Add `mesh_emit_selected_fact_detail_proof()` — three-guard proof stub (no fact, no Collar, no action)
3. Extend keyboard intercept to match Enter (0x1C) when Mesh focused
4. Emit `[mesh.detail.proof.stub]`, `[mesh.detail.proof.no_fact]`, `[mesh.detail.proof.collar_stub]`, `[mesh.detail.proof.done]`
5. No real action, no grants, no Collar navigation

After N8: Mesh will have full parity with Bell's proof-stub read-only detail model.
