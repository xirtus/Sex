# N11: Focus Linen at Selected Mesh Fact

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** When Enter is pressed on a selected Mesh fact, focus the Linen surface and select the linked object. No buffer creation, no linking, no Collar gate.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║       PASS_N11_FOCUS_LINEN_AT_SELECTED_FACT                  ║
╠══════════════════════════════════════════════════════════════╣
║ Action:                 FocusLinen + select object          ║
║ Guard:                  After successful detail proof       ║
║ Collar:                 NOT REQUIRED (pure navigation)      ║
║ New links/buffers:      NOT CREATED                         ║
║ Boundaries:             INTAKT                              ║
║ Build:                  PASS (1618 sectors)                 ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (+28 lines, -1 deletion)
**Commit:** *(to be added)*

### 1. `mesh_emit_selected_fact_detail_proof() -> bool`

Changed return type from `()` to `bool`:
- Returns `true` on successful detail proof (fact existed, kind supported)
- Returns `false` on rejection (not_focused, no_fact)

This allows the caller to conditionally proceed with focus action.

### 2. `mesh_focus_linen_at_selected_fact(fact: &MeshFact)`

New function that focuses Linen and selects the object referenced by a Mesh fact:

```rust
unsafe fn mesh_focus_linen_at_selected_fact(fact: &MeshFact) {
    serial_println!("[mesh.action.focus_linen] subject_id={}", fact.subject_id);
    open_linen_in_active_scene();
    SELECTED_LINEN_OBJECT_ID = fact.subject_id;
    linen_render_object_list();
}
```

Key properties:
- **Only uses subject_id** (Linen object_id) from ObjectLinkedToBuffer facts
- **No buffer creation**: does NOT call `open_linen_object_in_quil()`
- **No linking**: does NOT call `collar_check_operation_stub()`
- **No Collar gate**: pure shell navigation
- **Reuses existing functions**: `open_linen_in_active_scene()`, `linen_render_object_list()`

### 3. Enter handler

The keyboard dispatch now chains proof → focus:

```
0x1C => {
    [mesh.keyboard.enter] sid=N
    if mesh_emit_selected_fact_detail_proof() {    // N8: proof markers (returns bool)
        if let Some(fact) = mesh_selected_fact_snapshot() {
            mesh_focus_linen_at_selected_fact(&fact);  // N11: focus Linen
        }
    }
}
```

The focus action is only called when:
1. Mesh surface is focused (Guard 1 in proof)
2. A valid fact exists at the selected row (Guard 2 in proof)
3. The fact kind is supported (Guard 3 in proof — exhaustive match)

## Proof Markers

| Marker | Location | Description |
|--------|----------|-------------|
| `[mesh.action.focus_linen]` | `mesh_focus_linen_at_selected_fact()` | Focus Linen with subject_id |

### Existing Markers Preserved

All N8 detail proof markers preserved:
- `[mesh.detail.reject] reason=not_focused` — guard 1
- `[mesh.detail.reject] reason=no_fact` — guard 2
- `[mesh.detail.open]` — detail header
- `[mesh.detail.fact]` — fact details
- `[mesh.detail.object_link]` — link IDs
- `[mesh.detail.done]` — detail done

Plus all N6 selection, N4 rendering, N2 ring, J6 link, I1 lifecycle markers.

## Execution Trace

### Success: Mesh focused, valid ObjectLinkedToBuffer fact selected

```
Enter (0x1C) → Mesh dispatch
  [mesh.keyboard.enter] sid=202
  → mesh_emit_selected_fact_detail_proof()  → returns true
    [mesh.detail.open] fact_id=N kind=ObjectLinkedToBuffer
    [mesh.detail.fact] fact_id=N kind=ObjectLinkedToBuffer subject_id=3 ...
    [mesh.detail.object_link] subject_id=3 object_id=N ref_id=N
    [mesh.detail.done] fact_id=N
  → mesh_selected_fact_snapshot() → Some(fact)
  → mesh_focus_linen_at_selected_fact(&fact)
    [mesh.action.focus_linen] subject_id=3
    → open_linen_in_active_scene()
      [linen.placeholder.reject.duplicate] (or [linen.placeholder.attach.*])
      [linen.placeholder.focus]
    → SELECTED_LINEN_OBJECT_ID = 3
      [linen.object_select.current] id=3
    → linen_render_object_list()
      [linen.object_list.*]
```

### Reject: Mesh focused but no fact at selected row

```
Enter (0x1C) → Mesh dispatch
  [mesh.keyboard.enter] sid=202
  → mesh_emit_selected_fact_detail_proof()  → returns false
    [mesh.detail.reject] reason=no_fact
  → focus action NOT called
```

### Reject: Mesh not focused (Enter handled by other surface)

```
Enter (0x1C) → not Mesh dispatch (panel/atlas/Bell/scancode_to_action wins)
  → mesh_emit_selected_fact_detail_proof() NOT called
  → focus action NOT called
```

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/mesh/ | ✅ CLEAN (no Mesh PD) |
| servers/linen/ | ✅ READ-ONLY surface navigation |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Mesh fact ring | ✅ CLEAN (read-only snapshot) |
| Mesh selection state | ✅ CLEAN (not mutated) |
| Collar authority | ✅ CLEAN (not called) |
| Grants/revokes | ✅ CLEAN |
| Buffer creation | ✅ CLEAN (not called) |
| Object linking | ✅ CLEAN (not called) |

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
| New buffer/links | ✅ NOT TRIGGERED |
| Collar gate calls | ✅ NOT TRIGGERED |

**Boundaries: INTAKT** — All forbidden areas clean.

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| May create Linen frame if not already visible | LOW | Same behavior as F8/command palette FocusLinen |
| SELECTED_LINEN_OBJECT_ID may repair if object deleted | LOW | Existing safe behavior |
| No Quil surface opened | INFO | User sees Linen selection only; OpenInQuil deferred |
| Double snapshot call (proof + focus) | LOW | Stack-only, no allocation, trivial cost |

## Next Steps

**N12: Rapid audit of N11** — verify FocusLinenAtSelectedFact is safe.
After N12: evaluate adding Quil focus or OpenLinkedObjectInQuil as a follow-up action.
