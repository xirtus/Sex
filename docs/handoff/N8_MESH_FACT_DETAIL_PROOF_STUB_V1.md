# N8: Mesh Selected-Fact Detail Proof Stub

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Add Enter-on-selected-Mesh-row proof stub. No action, no grants/revokes, no topology mutation, no Collar navigation, no Mesh PD.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║           PASS_N8_MESH_FACT_DETAIL_PROOF_STUB                ║
╠══════════════════════════════════════════════════════════════╣
║ Snapshot lookup:          NEWEST-FIRST (read-only)           ║
║ Guard 1 (focus):         SURFACE_ID_MESH only                ║
║ Guard 2 (exists):        None→[mesh.detail.reject] no_fact   ║
║ Guard 3 (kind):          ObjectLinkedToBuffer only           ║
║ Enter dispatch:          panel>palette>atlas>Bell>Mesh>act   ║
║ Boundaries:              INTAKT                              ║
║ Build:                   PASS (1618 sectors)                 ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (+65 insertions, 1 deletion)
**Commit:** *(to be added)*

### 1. `mesh_selected_fact_snapshot() -> Option<MeshFact>`

Maps `MESH_SELECTED_ROW` to a copy of the corresponding ring fact:

```rust
unsafe fn mesh_selected_fact_snapshot() -> Option<MeshFact> {
    let total = MESH_FACT_WRITE_INDEX;
    let count = mesh_fact_count();
    if count == 0 { return None; }
    let start = (total as usize).wrapping_sub(1) % MESH_FACT_RING_CAP;
    for i in 0..count {
        let idx = (start + MESH_FACT_RING_CAP - i) % MESH_FACT_RING_CAP;
        if let Some(fact) = MESH_FACTS[idx] {
            if (i as u8) == MESH_SELECTED_ROW {
                return Some(fact);
            }
        }
    }
    None
}
```

Key properties:
- **Newest-first** iteration (same order as `mesh_for_each_fact`)
- **Read-only**: returns a `Copy` of the fact, never mutates the ring
- **No allocation**: returns `Option<MeshFact>` by value (40 bytes, stack)
- **Empty-safe**: returns `None` if ring is empty or selected index has no fact

### 2. `mesh_emit_selected_fact_detail_proof()`

Three-guard proof stub:

| Guard | Condition | Reject Marker |
|-------|-----------|---------------|
| 1 | `FOCUSED_SURFACE_ID == SURFACE_ID_MESH` | `[mesh.detail.reject] reason=not_focused` |
| 2 | Fact exists at selected row | `[mesh.detail.reject] reason=no_fact` |
| 3 | `MeshFactKind::ObjectLinkedToBuffer` | (only kind supported — no reject path needed for V1) |

On success:
```
[mesh.detail.open] fact_id=N kind=ObjectLinkedToBuffer
[mesh.detail.fact] fact_id=N kind=ObjectLinkedToBuffer subject_id=N object_id=N ref_id=N
[mesh.detail.object_link] subject_id=N object_id=N ref_id=N
[mesh.detail.done] fact_id=N
```

### 3. Keyboard Dispatch

Extended Mesh focused-surface intercept to include Enter (0x1C):

```
} else if FOCUSED_SURFACE_ID == SURFACE_ID_MESH
    && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C)
{
    match scancode {
        0x24 => { [mesh.keyboard.next] mesh_select_next_row(); }
        0x25 => { [mesh.keyboard.prev] mesh_select_prev_row(); }
        0x1C => { [mesh.keyboard.enter] mesh_emit_selected_fact_detail_proof(); }
        _ => {}
    }
    mutated = true;
}
```

**Dispatch precedence preserved:**
```
panel intercept → command palette intercept → atlas intercept
→ Bell focused-surface intercept → Mesh focused-surface intercept (Enter + J/K)
→ scancode_to_action dispatch
```

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.detail.open]` | `mesh_emit_selected_fact_detail_proof()` | Passes focus guard, fact kind header |
| `[mesh.detail.fact]` | `mesh_emit_selected_fact_detail_proof()` | ObjectLinkedToBuffer detail with all IDs |
| `[mesh.detail.object_link]` | `mesh_emit_selected_fact_detail_proof()` | Subject/object/ref link IDs |
| `[mesh.detail.done]` | `mesh_emit_selected_fact_detail_proof()` | Detail proof complete |
| `[mesh.detail.reject]` | `mesh_emit_selected_fact_detail_proof()` | Reject with reason |
| `[mesh.keyboard.enter]` | Keyboard handler | Enter key consumed for Mesh |

### Reject Paths

| Marker | Condition |
|--------|-----------|
| `[mesh.detail.reject] reason=not_focused` | FOCUSED_SURFACE_ID != SURFACE_ID_MESH |
| `[mesh.detail.reject] reason=no_fact` | No fact at selected row (empty ring or missing slot) |

### Existing Markers Preserved

| Marker | Status |
|--------|--------|
| `[mesh.selection.*]` (6 markers) | ✅ Preserved |
| `[mesh.keyboard.next]` | ✅ Preserved |
| `[mesh.keyboard.prev]` | ✅ Preserved |
| `[mesh.fact_list.*]` (4 markers) | ✅ Preserved |
| `[mesh.row_visual.*]` (2 markers) | ✅ Preserved |
| `[mesh.fact.*]` (3 markers) | ✅ Preserved |
| `[mesh.object_link.*]` (4 markers) | ✅ Preserved |
| `[mesh.placeholder.*]` | ✅ Preserved |
| `[mesh.render.refresh]` | ✅ Preserved |

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
| Mesh fact ring | ✅ CLEAN (read-only snapshot) |
| Bell ring/code | ✅ CLEAN |
| Collar authority | ✅ CLEAN |
| Grants/revokes | ✅ CLEAN |
| Text rendering | ✅ CLEAN |

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

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Detail proof is marker-only — no real action | INFO | Intentional V1 design |
| Enter when Mesh focused does nothing visible | LOW | Console proof markers only; user sees no UI change |
| No detail pane rendering | LOW | Deferred — would require real Mesh PD or sexdisplay change (STOP FIRST) |
| Only ObjectLinkedToBuffer kind supported | LOW | V1 design; more kinds added when needed |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1618 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Changed Files

- `servers/silk-shell/src/main.rs` — +65 lines (snapshot lookup, detail proof stub, Enter dispatch)
- `docs/handoff/N8_MESH_FACT_DETAIL_PROOF_STUB_V1.md` — new

## Next Steps

**N9: Rapid audit of N6–N8** — close the Mesh selection/detail milestone.
After N9: evaluate Mesh row action dispatch (view linked object in Linen/Quil using existing handler chains), or move to other feature work.

N8 proves the detail proof stub is safe — focus-gated, read-only snapshot, reject-on-no-fact.
