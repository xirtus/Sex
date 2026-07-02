# N10: Mesh Selected-Fact Action Path Design

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Design the safest Mesh selected-row action path. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║       SAFE_FOCUS_EXISTING_SURFACE_ONLY                       ║
╠══════════════════════════════════════════════════════════════╣
║ Collar required:         NO (for focus-only navigation)      ║
║ Chosen N11 behavior:     FocusLinenAtSelectedFact            ║
║ Allowed side effects:    Open Linen + select object          ║
║ Forbidden side effects:  New links, grants, buffer mutation  ║
║ STOP FIRST:              NOT TRIGGERED                       ║
╚══════════════════════════════════════════════════════════════╝
```

## Action Classification Table

| # | Action | Description | Collar? | Side Effects | Verdict |
|---|--------|-------------|---------|--------------|---------|
| 1 | **DetailProofOnly** | Current N8: emit markers, no UI change | None | None | ✅ **Current state** |
| 2 | **FocusLinenAtSelectedFact** | Open/focus Linen + set SELECTED_LINEN_OBJECT_ID from fact.subject_id | **No** | Focus, selection state | ✅ **SAFE — recommended N11** |
| 3 | **FocusQuilAtSelectedFact** | Open/focus Quil (no buffer selection yet — no Quil buffer select exists) | **No** | Focus only | ✅ SAFE but limited |
| 4 | **FocusLinenAndQuil** | Open both Linen and Quil, select object on Linen | **No** | Focus, selection state | ✅ SAFE but redundant |
| 5 | **OpenLinkedObjectInQuil** | Full J4 chain: Collar gate → create/reuse buffer → link → emit | AllowStub | Buffer creation, focus, links | ✅ SAFE but not pure nav |
| 6 | **OpenCollarForGrantReview** | Focus Collar placeholder for authority review | **No** | Focus only | ✅ SAFE but no Collar grant logic exists |
| 7 | **SaveBuffer / BuildTarget / RunTarget** | Mutation/authority ops | BlockedStopFirst | Storage, build | ❌ **STOP FIRST** |
| 8 | **RenameObject / ArchiveObject** | Needs real Collar grant | NeedsGrantLater | Metadata mutation | ❌ **STOP FIRST** |

## Classification Details

### Action 2: FocusLinenAtSelectedFact (Recommended)

**Path:**
```
Enter on Mesh selected fact
  → mesh_emit_selected_fact_detail_proof()  [proof markers]
  → validate FOCUSED_SURFACE_ID == SURFACE_ID_MESH
  → mesh_selected_fact_snapshot() → Some(fact)
  → fact.kind == ObjectLinkedToBuffer
  → subject_id = fact.subject_id  (Linen object_id)
  → [mesh.action.focus_linen] subject_id=N
  → open_linen_in_active_scene()
    → [linen.placeholder.*]  (existing)
    → try_set_focus()         (existing)
  → SELECTED_LINEN_OBJECT_ID = subject_id
  → linen_render_object_list()  (refresh with selection)
```

**Side effects:**
- Opens/focuses Linen surface (if not already visible)
- Sets `SELECTED_LINEN_OBJECT_ID` to the fact's `subject_id`
- Renders Linen object list with new selection
- Does NOT create/change any Quil buffers
- Does NOT call Collar gate
- Does NOT mutate Mesh facts

**Authority analysis:**
- `open_linen_in_active_scene()` is pure shell navigation — no Collar gate
- `SELECTED_LINEN_OBJECT_ID` is a shell-local static — no Collar gate
- No `open_linen_object_in_quil()` call — that's where Collar gating lives
- **Collar is NOT required** for this action

### Action 5: OpenLinkedObjectInQuil (Deferred)

**Path:**
```
Enter on Mesh selected fact
  → validate Mesh focused, fact exists, kind supported
  → subject_id = fact.subject_id  (Linen object_id)
  → [mesh.action.open_in_quil] subject_id=N
  → open_linen_object_in_quil(subject_id)
    → J4 chain (full): Collar gate → buffer create/reuse → link → emit
    → [collar.gate.*] [linen.quil.*] [mesh.object_link.*] [quil.buffer_list.*]
```

**Authority analysis:**
- `open_linen_object_in_quil()` calls `collar_check_operation_stub(LinkObjectToBuffer)`
- Current J5 policy: `LinkObjectToBuffer` → `AllowStub` (safe)
- BUT: this creates/reuses buffers, which is more than "pure navigation"
- If the link already exists (reuse path), it's effectively a focus+highlight
- If the link doesn't exist (create path), it mutates the buffer table

**Verdict:** SAFE but deferred. The Collar gate already allows it, but creating new links
from Mesh is a semantic step beyond simply navigating to the existing surface.

### Actions 6-8: Require Collar or STOP FIRST

| Action | Collar Decision | Reason |
|--------|----------------|--------|
| OpenCollarForGrantReview | `NeedsGrantLater` | No grant logic exists in V1 |
| SaveBuffer | `BlockedStopFirst` | STOP FIRST for storage |
| BuildTarget | `BlockedStopFirst` | STOP FIRST for build |
| RunTarget | `BlockedStopFirst` | STOP FIRST for execution |
| RenameObject | `NeedsGrantLater` | Requires real Collar grant |
| ArchiveObject | `NeedsGrantLater` | Requires real Collar grant |

## Collar Requirement Analysis

**Question:** Does "view linked object" require Collar-gated authority?

**Answer: NO** — IF the action is limited to:
1. Focusing existing Linen surface (pure shell navigation)
2. Setting `SELECTED_LINEN_OBJECT_ID` (shell-local static)
3. Rendering Linen object list (no PDX/authority)

**Collar IS required** if the action:
1. Creates new Linen↔Quil buffer links
2. Grants/revokes capabilities
3. Modifies buffer or object state beyond selection

**Key distinction:**
- `open_linen_in_active_scene()` — pure navigation. **No Collar.**
- `open_linen_object_in_quil()` — creates links. **Collar gate present** (AllowStub per J5).
- `SELECTED_LINEN_OBJECT_ID = N` — shell-local selection. **No Collar.**

## Chosen N11 Behavior: FocusLinenAtSelectedFact

### Allowed Side Effects

| Side Effect | Status | Rationale |
|-------------|--------|-----------|
| Focus Linen surface via `open_linen_in_active_scene()` | ✅ ALLOWED | Pure shell navigation, no Collar |
| Set `SELECTED_LINEN_OBJECT_ID` from `fact.subject_id` | ✅ ALLOWED | Shell-local static, no Collar |
| Call `linen_render_object_list()` for visual refresh | ✅ ALLOWED | Existing render function, no side effects |
| Emit proof markers for action tracing | ✅ ALLOWED | Console-only, no side effects |
| Call `snap_capture_layout()` if needed | ✅ ALLOWED | Existing layout capture |

### Forbidden Side Effects

| Side Effect | Status | Rationale |
|-------------|--------|-----------|
| Create new Quil buffer | ❌ FORBIDDEN | Mutates buffer table |
| Link object to buffer | ❌ FORBIDDEN | Requires Collar gate (even if AllowStub) |
| Call `collar_check_operation_stub()` | ❌ FORBIDDEN | Not needed for focus-only |
| Mutate Mesh fact ring | ❌ FORBIDDEN | Read-only selection invariant |
| Change focus away from Linen after action | ❌ FORBIDDEN | Only focus Linen |
| Open Quil surface | ❌ FORBIDDEN | Not part of focus-only action |
| Grant/revoke capabilities | ❌ FORBIDDEN | No Collar authority in V1 |
| Any PDX/ABI/kernel change | ❌ FORBIDDEN | STOP FIRST |

## STOP FIRST Table

| Trigger | Status for N11 | Notes |
|---------|---------------|-------|
| New PDX opcodes | ✅ NOT TRIGGERED | Uses existing open_linen_in_active_scene() |
| sex-pdx ABI constants | ✅ NOT TRIGGERED | No ABI changes |
| Capability grants/revokes | ✅ NOT TRIGGERED | No Collar calls |
| Cross-PD pointers | ✅ NOT TRIGGERED | Shell-local only |
| Kernel introspection | ✅ NOT TRIGGERED | No kernel changes |
| Persistent storage | ✅ NOT TRIGGERED | No storage changes |
| Renderer policy | ✅ NOT TRIGGERED | No sexdisplay changes |
| Mesh PD creation | ✅ NOT TRIGGERED | Shell-local only |
| Bell/Collar behavior | ✅ NOT TRIGGERED | No Bell or Collar changes |
| New object-buffer links | ✅ NOT TRIGGERED | Focus-only, no open_linen_object_in_quil() |
| New surface/frame creation | ✅ IF LINEN NOT ALREADY VISIBLE | open_linen_in_active_scene() may create frame — same as F8/F9/command palette |
| Mutate Linen object table | ✅ NOT TRIGGERED | Read-only via linen_object_by_id() |

**STOP FIRST: NOT TRIGGERED** — All forbidden areas clean.

## Existing Proof Marker Integration

When a Mesh fact is selected and Enter is pressed with FocusLinenAtSelectedFact:

```
[mesh.keyboard.enter] sid=202
→ mesh_emit_selected_fact_detail_proof()
  [mesh.detail.open] fact_id=N kind=ObjectLinkedToBuffer
  [mesh.detail.fact] fact_id=N ... subject_id=N ...
  [mesh.detail.object_link] subject_id=N object_id=N ref_id=N
  [mesh.detail.done] fact_id=N

→ NEW: FocusLinenAtSelectedFact
  [mesh.action.focus_linen] subject_id=N
  → open_linen_in_active_scene()
    [linen.placeholder.reject.duplicate]  (or [linen.placeholder.*])
    [linen.placeholder.focus]
  → SELECTED_LINEN_OBJECT_ID = subject_id
    [linen.object_select.current] id=N
  → linen_render_object_list()
    [linen.object_list.*]
```

All existing proof markers preserved. Mesh detail markers remain as-is.

## N11 Implementation Prompt Summary

**N11: Focus Linen on selected Mesh fact.**

1. Modify `mesh_emit_selected_fact_detail_proof()` to add focus action:
   - After `[mesh.detail.done]`, emit `[mesh.action.focus_linen] subject_id=N`
   - Guard: `FOCUSED_SURFACE_ID == SURFACE_ID_MESH` (already checked)
   - Guard: `fact.kind == ObjectLinkedToBuffer` (already checked in match)
   - Call `open_linen_in_active_scene()` — opens/focuses Linen
   - Set `SELECTED_LINEN_OBJECT_ID = fact.subject_id`
   - Call `linen_render_object_list()` — refresh selection visual
   - No buffer creation, no linking, no Collar gate, no Quil

2. Add new marker: `[mesh.action.focus_linen]`

3. Do NOT:
   - Call `open_linen_object_in_quil()` (deferred)
   - Call `collar_check_operation_stub()` (not needed)
   - Open Quil surface
   - Mutate buffers or create links
   - Change Mesh/other state after focus

4. Boundaries:
   - `open_linen_in_active_scene()` may create Linen frame if not visible
   - `SELECTED_LINEN_OBJECT_ID` may repair if current object no longer exists
   - These are existing behaviors, not new side effects

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Focusing Linen may create a new frame if Linen not visible | LOW | Same as F8/command palette — existing behavior |
| SELECTED_LINEN_OBJECT_ID may repair if object deleted | LOW | Existing behavior — safe |
| User expects "Open in Quil" but gets "Focus Linen" | INFO | Iterative: Focus first, OpenInQuil later |
| No visible effect if Linen already focused at same object | INFO | Harmless — proof markers still fire |
