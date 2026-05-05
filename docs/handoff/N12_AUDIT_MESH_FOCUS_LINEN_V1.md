# N12: Audit Mesh Focus Linen Action

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Verify N11 FocusLinenAtSelectedFact is safe — no accidental link creation, no Collar bypass, pure navigation only.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║           PASS_N11_MESH_FOCUS_LINEN                          ║
╠══════════════════════════════════════════════════════════════╣
║ Bool guard:                PASS                               ║
║ Focus action:              PASS (pure navigation)            ║
║ Forbidden side effects:    NOT PRESENT                        ║
║ Dispatch precedence:       PASS                               ║
║ Boundaries:                INTAKT                              ║
║ Build:                     PASS (1618 sectors)                ║
╚══════════════════════════════════════════════════════════════╝
```

## 1. Bool Guard Table

| Gate | Condition | Returns | Focus Linen called? | Line |
|------|-----------|---------|--------------------|------|
| 1 | `FOCUSED_SURFACE_ID != SURFACE_ID_MESH` | `false` + `[mesh.detail.reject] reason=not_focused` | ❌ No | 1514-1516 |
| 2 | `mesh_selected_fact_snapshot()` returns `None` | `false` + `[mesh.detail.reject] reason=no_fact` | ❌ No | 1520-1522 |
| 3 | `fact.kind` match succeeds | `true` + all detail markers | ✅ Yes | 1523-1533 |

**Bool Guard: PASS** — All reject paths return `false` before any focus action is reached. Focus Linen is only called when `mesh_emit_selected_fact_detail_proof()` returns `true`.

### Execution Path Table

| Scenario | Returns | Markers emitted | Focus Linen? |
|----------|---------|----------------|--------------|
| Mesh not focused, Enter pressed | `false` | `[mesh.detail.reject] reason=not_focused` | ❌ No |
| Mesh focused, empty ring | `false` | `[mesh.detail.reject] reason=no_fact` | ❌ No |
| Mesh focused, valid fact selected | `true` | `[mesh.detail.open/fact/object_link/done]` | ✅ Yes |
| Mesh focused, selected row has no fact (gap) | `false` | `[mesh.detail.reject] reason=no_fact` | ❌ No |

### Double-Snapshot Safety

The keyboard handler calls `mesh_selected_fact_snapshot()` twice:
1. Inside `mesh_emit_selected_fact_detail_proof()` — reads fact for proof markers
2. In the Enter handler — reads fact again for focus action

Both calls are read-only (`Copy` by value, no ring mutation). The ring could theoretically change between the two calls on a concurrent system, but SexOS is single-threaded for shell dispatch — no preemption between the two reads. If the ring changes (e.g., interrupt handler), the second read returns a different (but still valid) fact, which is safe.

**Double-snapshot: SAFE** — No mutation, no allocation, no race in single-threaded dispatch.

## 2. Focus Action Side-Effect Table

| Side Effect | Implementation | Type | Status |
|------------|----------------|------|--------|
| Emit `[mesh.action.focus_linen]` | `serial_println!(...)` | Console proof marker | ✅ ALLOWED |
| Call `open_linen_in_active_scene()` | Opens/focuses Linen surface | Pure shell navigation | ✅ ALLOWED |
| Set `SELECTED_LINEN_OBJECT_ID` | `= fact.subject_id` | Shell-local static | ✅ ALLOWED |
| Call `linen_render_object_list()` | Renders Linen rows | Existing render function | ✅ ALLOWED |

### `open_linen_in_active_scene()` Chain

```
open_linen_in_active_scene()
  → duplicate guard (if already visible → focus + render + return)
  → ensure_linen_frame() (may create frame if not exists — same as F8)
  → un-minimize if needed
  → tile_active_scene_frames()
  → try_set_focus(sid)
  → linen_render_object_list()
```

This is the **identical function** called by:
- F8 keyboard shortcut
- Command palette "Focus Linen" command
- SurfaceAction::FocusLinen

**No new behavior.** No Collar, no linking, no buffer mutation.

### `SELECTED_LINEN_OBJECT_ID` Safety

Setting `SELECTED_LINEN_OBJECT_ID = fact.subject_id` is a **raw assignment** — it does not validate that the object still exists. However:

1. `linen_render_object_list()` is called immediately after
2. `linen_render_object_list()` calls `linen_selected_object_id()` which **repairs** the selection if the object no longer exists (line 385-391)
3. The repair sets `SELECTED_LINEN_OBJECT_ID` to the first valid object or 0

This is the **exact same pattern** used by `linen_select_object_by_id()` when it sets the selection directly (line 401-402). The render validates on refresh.

**Selection safety: VALIDATED** — Repair in render ensures consistency.

### `subject_id` is a Linen object_id

`MeshFact::subject_id` for `ObjectLinkedToBuffer` facts is set from `o.object_id` in `mesh_emit_linen_quil_links()` (line 1317-1326):
```rust
mesh_record_fact(MeshFactKind::ObjectLinkedToBuffer, o.object_id, buf.buffer_id, buf.linked_surface_id);
```

Where `o` is a valid `linen_object_by_id()` result. The fact's `subject_id` is always a valid Linen object ID at the time the fact was recorded.

**subject_id validity: SOURCE-AUTHORITATIVE** — Written from live object table at record time.

## 3. Forbidden Side-Effect Check

| Forbidden Action | Called? | Evidence |
|-----------------|---------|----------|
| `open_linen_object_in_quil()` | ❌ NOT CALLED | Not in `mesh_focus_linen_at_selected_fact()` or call chain (line 1541-1545) |
| `open_quil_in_active_scene()` | ❌ NOT CALLED | Not in `mesh_focus_linen_at_selected_fact()` (line 1541-1545) |
| Buffer creation/reuse | ❌ NOT CALLED | No `QUIL_BUFFERS` access in focus path |
| `collar_check_operation_stub()` | ❌ NOT CALLED | No CollarOperation in focus path |
| Mesh ring mutation | ❌ NOT DONE | Read-only `mesh_selected_fact_snapshot()` only |
| Mesh selection mutation | ❌ NOT DONE | `MESH_SELECTED_ROW` unchanged |
| Mesh render during focus | ❌ NOT DONE | No `mesh_render_fact_list()` call |
| `pdx_call(SLOT_DISPLAY, ...)` | ❌ NOT CALLED | No display changes from focus action |
| Bell code changes | ❌ NOT DONE | No Bell interaction |
| PDX/ABI/kernel changes | ❌ NOT DONE | Shell-local only |

**Forbidden Side Effects: NOT PRESENT** — All forbidden actions confirmed absent.

## 4. Dispatch Precedence Check

### Enter (0x1C) Dispatch Chain

| Priority | Handler | Condition | Mesh Focus Linen reached? |
|----------|---------|-----------|--------------------------|
| 1 | Command palette | `COMMAND_PALETTE_OPEN == true` | ❌ Consumed by palette |
| 2 | Atlas | `ATLAS_MODE_ENABLED == true` && `scancode != 0x44` | ❌ Consumed by Atlas |
| 3 | Bell focused-surface | `FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER` | ❌ Consumed by Bell |
| **4** | **Mesh focused-surface** | **`FOCUSED_SURFACE_ID == SURFACE_ID_MESH`** | **✅ Mesh handles Enter** |
| 5 | scancode_to_action | Default dispatch | ❌ Not reached for Mesh Enter |

### Scenario Verification

| Scenario | Correct Handler | Focus Linen? | Status |
|----------|----------------|-------------|--------|
| Palette open, Enter | Command palette execute | ❌ No | ✅ Correct |
| Atlas active, Enter | Atlas confirm | ❌ No | ✅ Correct |
| Bell focused, Enter | Bell detail proof | ❌ No | ✅ Correct |
| Mesh focused, Enter | Mesh detail proof + Focus Linen | ✅ Yes | ✅ Correct |
| Linen focused, Enter | `AccessActivate` (scancode_to_action) | ❌ No | ✅ Correct |
| Quil focused, Enter | `AccessActivate` (scancode_to_action) | ❌ No | ✅ Correct |

### J/K (0x24/0x25) Dispatch — Unchanged

| Scenario | J/K handled by | Linen J/K affected? |
|----------|---------------|--------------------|
| Palette open | Palette J/K (next/prev command) | ❌ No |
| Atlas active | Atlas nav | ❌ No |
| Bell focused | Bell J/K (next/prev event) | ❌ No |
| Mesh focused | Mesh J/K (next/prev fact) | ❌ No |
| Linen focused | scancode_to_action (SelectNext/PrevLinenObject) | ✅ Yes — unchanged |

**Dispatch: PASS** — All precedences preserved. Linen J/K and Enter unaffected.

## 5. Boundary Check

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
| Mesh selection | ✅ CLEAN (not mutated) |
| Collar authority | ✅ CLEAN (not called) |
| Buffer/links | ✅ CLEAN (not created) |
| Heap allocation | ✅ CLEAN (stack-only) |

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

**Boundaries: INTAKT** — All forbidden areas clean. No Collar bypass: `collar_check_operation_stub()` is simply not needed for pure navigation.

## 6. Collar Bypass Analysis

**Key question:** Does setting `SELECTED_LINEN_OBJECT_ID` and focusing Linen bypass Collar authority?

**Answer: NO** — because:
1. `SELECTED_LINEN_OBJECT_ID` is a **shell-local index state**, not an authority grant
2. Setting it does not:
   - Open/modify the object's content
   - Create any buffer or link
   - Grant any capability
   - Execute any operation that requires Collar approval
3. The Collar gate remains in `open_linen_object_in_quil()` for when the user subsequently presses PrintScreen to open the selected object in Quil

**Analogy:** Setting `SELECTED_LINEN_OBJECT_ID` to an object is equivalent to clicking on a file in a file manager — it changes which item is highlighted, but does not open/edit/execute it. The Collar gate only fires when the user triggers an actual operation (Open in Quil → PrintScreen).

## 7. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| SELECTED_LINEN_OBJECT_ID may repair on render if object deleted | LOW | Existing safe behavior — no crash, just re-selection |
| Double snapshot read may return different fact between calls | LOW | Single-threaded dispatch — no race. If fact changes (e.g., overwrite), second read returns different but valid fact |
| Linen frame may be created if not visible | LOW | Same behavior as F8/command palette FocusLinen |
| No visible feedback on Mesh surface | INFO | Focus changes to Linen — user sees Linen surface activate |

## 8. Next Safest Step

**N13: Design only — OpenLinkedObjectInQuil from Mesh** — Docs-only design for the full J4 chain path:
1. Mesh Enter → detail proof → focus Linen ✅ (current)
2. Mesh PrintScreen → OpenLinkedObjectInQuil (Collar-stub gated, reuses existing `open_linen_object_in_quil()`)
3. Or: extend Mesh Enter to also open in Quil after focus

The key question for N13 design: should OpenLinkedObjectInQuil from Mesh be:
- A new keyboard shortcut (e.g., PrintScreen while Mesh focused)
- An Enter follow-up (after FocusLinen, also open in Quil)
- A separate Mesh action (new Enter mode)

**N13 must verify that the existing J4 Collar gate (`AllowStub` for `LinkObjectToBuffer`) is sufficient**, or whether opening from Mesh requires a new Collar policy.
