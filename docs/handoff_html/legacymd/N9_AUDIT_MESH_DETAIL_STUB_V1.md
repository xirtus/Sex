# N9: Audit Mesh Detail Proof Stub

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Verify N8 Mesh selected-fact detail proof stub and close N6-N8 Mesh selection/detail milestone. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║           PASS_N8_MESH_DETAIL_STUB                           ║
╠══════════════════════════════════════════════════════════════╣
║ Selected-fact lookup:      PASS                               ║
║ Detail stub behavior:      PASS                               ║
║ Keyboard precedence:       PASS                               ║
║ Boundary check:            INTAKT                              ║
║ Build:                     PASS (1618 sectors)                ║
╚══════════════════════════════════════════════════════════════╝
```

## 1. Selected-Fact Lookup Table

| Criterion | Expected (N8 Spec) | Actual (Implementation) | Status |
|-----------|--------------------|-------------------------|--------|
| Newest-first mapping | Same order as `mesh_for_each_fact` | `start = (total-1) % CAP`, `idx = (start + CAP - i) % CAP` (lines 1497-1499) — identical to `mesh_for_each_fact` | ✅ PASS |
| Read-only ring access | Returns copy, no mutation | `if let Some(fact) = MESH_FACTS[idx]` — reads `Copy` value, never writes (line 1500) | ✅ PASS |
| Returns copy, not reference | `Option<MeshFact>` by value | `mesh_selected_fact_snapshot() -> Option<MeshFact>` (line 1493) — returns owned `Copy` | ✅ PASS |
| Empty ring safety | Returns None | `if count == 0 { return None; }` (line 1496) | ✅ PASS |
| Selected row too large (clamped by render) | Returns None | Falls through loop when `(i as u8) != MESH_SELECTED_ROW` for all i → `None` (line 1506) | ✅ PASS |
| Selected row on valid fact | Returns Some(fact) | `if (i as u8) == MESH_SELECTED_ROW { return Some(fact); }` (lines 1501-1502) | ✅ PASS |
| None slots skipped | Skips gaps in ring | `if let Some(fact) = MESH_FACTS[idx]` — only processes occupied slots (line 1500) | ✅ PASS |
| Nth visible row matches render order | Same iteration formula | Both use `(total-1) % CAP` → `(start + CAP - i) % CAP` | ✅ PASS |

**Memory safety:**
- Returns `MeshFact` by value (6 × u64 = 48 bytes on stack)
- No heap allocation
- No references into the ring buffer
- Ring is never mutated during iteration

**Selected-Fact Lookup: PASS** — Matches Bell M8 pattern, read-only, safe for all edge cases.

## 2. Detail Stub Behavior Table

| Guard | Condition | Reject Marker | Actual | Status |
|-------|-----------|---------------|--------|--------|
| Guard 1 (focus) | `FOCUSED_SURFACE_ID == SURFACE_ID_MESH` | `[mesh.detail.reject] reason=not_focused` | Line 1512-1513 | ✅ PASS |
| Guard 2 (exists) | Fact at selected row is `Some` | `[mesh.detail.reject] reason=no_fact` | Lines 1518-1519 | ✅ PASS |
| Guard 3 (kind) | `ObjectLinkedToBuffer` only | (exhaustive match — compiler-enforced) | Lines 1524-1530 | ✅ PASS |

### Success Path
```
[mesh.detail.open] fact_id=N kind=ObjectLinkedToBuffer
[mesh.detail.fact] fact_id=N kind=ObjectLinkedToBuffer subject_id=N object_id=N ref_id=N
[mesh.detail.object_link] subject_id=N object_id=N ref_id=N
[mesh.detail.done] fact_id=N
```

All 4 markers are emitted in order (lines 1523, 1526, 1528, 1532).

### Reject Paths

| Scenario | Markers | Line |
|----------|---------|------|
| Mesh not focused (enter key on another surface) | `[mesh.detail.reject] reason=not_focused` only | 1513 |
| Empty ring or selected index has no fact | `[mesh.detail.reject] reason=no_fact` only | 1519 |

### What the stub does NOT do

| Action | Status | Evidence |
|--------|--------|----------|
| Mutate fact ring | ✅ NOT DONE | Read-only `mesh_selected_fact_snapshot()` only |
| Mutate selection | ✅ NOT DONE | `MESH_SELECTED_ROW` unchanged |
| Change focus | ✅ NOT DONE | No `try_set_focus()` or focus mutation |
| Call Linen/Quil/Collar/Bell | ✅ NOT DONE | No cross-subsystem calls |
| Grant/revoke capability | ✅ NOT DONE | No Collar interaction |
| Render UI | ✅ NOT DONE | No `pdx_call` or `mesh_render_fact_list()` |
| Allocate heap | ✅ NOT DONE | Stack-local only |

### Kind Safety

`MeshFactKind` has exactly one variant:
```rust
enum MeshFactKind {
    ObjectLinkedToBuffer = 0,
}
```

The match in `mesh_emit_selected_fact_detail_proof` is exhaustive:
```rust
match fact.kind {
    MeshFactKind::ObjectLinkedToBuffer => { ... }
}
```

Unlike Bell M8 which uses `_ => { [bell.detail.reject] reason=unsupported_kind }`, Mesh uses an exhaustive match. If a new `MeshFactKind` variant is added, the compiler will produce an error at this match arm — forcing the developer to explicitly handle the new kind. This is **safer** than a silent catch-all.

**Detail Stub Behavior: PASS** — Guards correct, no side effects, exhaustive kind match.

## 3. Keyboard Precedence Check

### Dispatch Chain (Enter = 0x1C)

| Priority | Handler | Enter Behavior | Status |
|----------|---------|---------------|--------|
| 1 | Command palette | Close + execute selected | ✅ Line 9289-9293 |
| 2 | Atlas | Atlas navigation confirm | ✅ Line 9309-9311 (handled by `handle_atlas_keyboard`) |
| 3 | Bell focused-surface | `bell_emit_selected_event_detail_proof()` | ✅ Lines 9312-9331 |
| **4** | **Mesh focused-surface** | **`mesh_emit_selected_fact_detail_proof()`** | **✅ Lines 9332-9351** |
| 5 | scancode_to_action | `SurfaceAction::AccessActivate` | ✅ Line 2344 |

### Enter Dispatch Scenarios

| Scenario | Correct Handler | Precedence | Status |
|----------|----------------|------------|--------|
| Palette open, any focus | Command palette | Panel | ✅ Correct |
| Atlas active, any focus | Atlas | Atlas | ✅ Correct |
| Bell focused, palette closed | Bell detail proof | Bell | ✅ Correct |
| Mesh focused, palette closed, Atlas off | Mesh detail proof | **Mesh** | ✅ Correct |
| Linen focused, palette closed, Atlas off | `AccessActivate` | scancode_to_action | ✅ Correct |
| Quil focused, palette closed, Atlas off | `AccessActivate` | scancode_to_action | ✅ Correct |

### J/K Dispatch Scenarios (0x24/0x25)

| Scenario | Correct Handler | Precedence | Status |
|----------|----------------|------------|--------|
| Palette open, any focus | Palette J/K | Panel | ✅ Correct |
| Atlas active, any focus | Atlas nav keys | Atlas | ✅ Correct |
| Bell focused | Bell J/K nav | Bell | ✅ Correct |
| Mesh focused | Mesh J/K nav | **Mesh** | ✅ Correct |
| Linen focused | `SelectNext/PrevLinenObject` | scancode_to_action | ✅ Correct |

**Keyboard Precedence: PASS** — All dispatch scenarios produce correct behavior.

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
| Mesh fact ring | ✅ CLEAN (read-only snapshot) |
| Mesh selection state | ✅ CLEAN (not mutated) |
| Bell ring/code | ✅ CLEAN |
| Collar authority | ✅ CLEAN |
| Grants/revokes | ✅ CLEAN |
| Text rendering | ✅ CLEAN |
| Focus lifecycle | ✅ CLEAN (no focus changes) |
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
| Row action dispatch | ✅ NOT TRIGGERED (proof stub only) |

**Boundaries: INTAKT** — All forbidden areas clean.

## 5. Milestone Status: N6-N8 Mesh Selection/Detail

| Phase | Status | Commit |
|-------|--------|--------|
| N4 Mesh fact row rendering | ✅ PASS | `1011319` (N4 handoff: `ea3bde1`) |
| N5 Audit Mesh row rendering | ✅ PASS | `61a5354` |
| N6 Mesh selected-row visual + J/K nav | ✅ PASS | `46b4ead` |
| N7 Audit Mesh selection | ✅ PASS | `97de790` |
| N8 Mesh detail proof stub | ✅ PASS | `207ed10` |
| N9 Audit Mesh detail stub | ✅ **COMPLETE** | *(this commit)* |

**Milestone: CLOSED** — Mesh has full read-only selection parity with Bell.

## 6. Existing Marker Preservation

| Marker Source | Location | Status |
|--------------|----------|--------|
| All N4 markers (8) | mesh_render_fact_list(), mesh_record_fact() | ✅ Preserved |
| All N6 markers (8) | mesh_select_next/prev_row, render, keyboard | ✅ Preserved |
| All N2 markers (3) | mesh_record_fact() | ✅ Preserved |
| All J6 markers (4) | mesh_emit_linen_quil_links() | ✅ Preserved |
| All I1 markers | lifecycle | ✅ Preserved |

### N8 New Markers Verified

| Marker | Location | Line | Status |
|--------|----------|------|--------|
| `[mesh.detail.open]` | mesh_emit_selected_fact_detail_proof() | 1523 | ✅ Present |
| `[mesh.detail.fact]` | mesh_emit_selected_fact_detail_proof() | 1526 | ✅ Present |
| `[mesh.detail.object_link]` | mesh_emit_selected_fact_detail_proof() | 1528 | ✅ Present |
| `[mesh.detail.done]` | mesh_emit_selected_fact_detail_proof() | 1532 | ✅ Present |
| `[mesh.detail.reject]` | mesh_emit_selected_fact_detail_proof() | 1513, 1519 | ✅ Present |
| `[mesh.keyboard.enter]` | Keyboard handler | 9346 | ✅ Present |

## 7. Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Detail proof is marker-only — no visible UI | INFO | Intentional V1. User sees no UI change on Enter. |
| No row action dispatch | LOW | Deferred — N10 designs action dispatch without authority mutation |
| Only ObjectLinkedToBuffer kind supported | LOW | V1 design; compiler enforces handle of new variants |
| Enter when palette open but not focused still processed | INFO | Palette intercept fires first; safe |

## 8. Next Safest Step

**N10: Mesh row action design only (docs)** — Design how "view linked object in Linen/Quil" would work:
1. NO code changes — docs only
2. Decide whether "view linked object" is pure focus/navigation (FOCUSED_SURFACE_ID = SURFACE_ID_LINEN, object selected) or requires Collar-gated authority
3. If pure focus: route through existing `open_linen_in_active_scene()` + `open_quil_in_active_scene()` with `SELECTED_LINEN_OBJECT_ID` set
4. If Collar-gated: requires STOP FIRST for Collar grant → deferred
5. Document the exact dispatch chain from Mesh Enter → selected fact → handler call

After N10: implement action dispatch if design shows it's safe (focus/navigation only, no authority).

Alternatively: move to other subsystem work (Bell actions, real shell features).
