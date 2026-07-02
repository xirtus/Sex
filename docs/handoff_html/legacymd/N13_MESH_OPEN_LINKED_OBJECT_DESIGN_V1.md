# N13: Mesh Open Linked Object in Quil — Design

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Design the safest path for Mesh selected-fact → OpenLinkedObjectInQuil. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║       SAFE_COLLAR_STUB_GATED_ALT_KEY                         ║
╠══════════════════════════════════════════════════════════════╣
║ Collar required:         YES — existing AllowStub sufficient ║
║ Trigger key:             PrintScreen (0x59) when Mesh focused ║
║ Reuses existing handler: open_linen_object_in_quil()         ║
║ Collar bypass risk:      NONE (gate inside callee)           ║
║ N11 Enter behavior:      PRESERVED (FocusLinen only)        ║
╚══════════════════════════════════════════════════════════════╝
```

## Existing Operation Analysis

### `open_linen_object_in_quil(object_id)` — Full Chain

The existing J4 function performs these steps in order:

| Step | Operation | Mutates? | Authority Required |
|------|-----------|----------|-------------------|
| 1 | Validate object exists in LINEN_OBJECTS | Read-only | None |
| 2 | Check grant_ref, emit `[linen.quil.open.no_grant]` | Read-only | None (advisory) |
| **2.5** | **`collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)`** | **Read-only** | **AllowStub per J5** |
| 3 | Map LinenObjectKind → QuilBufferKind | Local | None |
| 4 | Find existing buffer or create new in QUIL_BUFFERS | ✅ **Writes QUIL_BUFFERS** | Collar gate passed |
| 5 | Update LINEN_OBJECTS.linked_surface_id | ✅ **Writes LINEN_OBJECTS** | Collar gate passed |
| 6 | Emit `[linen.quil.buffer.linked]` | Proof marker | None |
| 7 | `open_quil_in_active_scene()` | Navigation | None |
| 8 | J6: `mesh_emit_linen_quil_links()` | ✅ **Records Mesh facts** | Collar gate passed |
| 9 | J7: `bell_emit_object_link_event()` | ✅ **Records Bell events** | Collar gate passed |
| 10 | K3: `quil_render_buffer_list()` | Display only | None |

**Key insight:** The Collar gate (step 2.5) fires BEFORE any mutation. All buffer/object mutations are protected behind the Collar decision.

### Collar Gate Decision (J5)

```
CollarOperation::LinkObjectToBuffer (op=6):
  → object_id == 0?  → DenyMissingObject (reject)
  → object_id not in LINEN_OBJECTS? → DenyMissingObject (reject)
  → never blocked by STOP FIRST → AllowStub
```

**Current policy:** `LinkObjectToBuffer` always returns `AllowStub` for valid objects — no real Collar authority is exercised in V1.

## Mesh-Triggered Risk Table

| Risk | Existing Protection | Severity | Mitigation |
|------|--------------------|----------|------------|
| Duplicate link (already exists) | Step 4 reuses existing buffer — no duplicate | LOW | Reuse path is safe, no double-creation |
| Duplicate Mesh fact recording | `mesh_emit_linen_quil_links()` records ALL links — idempotent | LOW | Same facts recorded each time; overwrite-oldest ring handles duplicates |
| Duplicate Bell event | `bell_emit_object_link_event()` records a new event each time | LOW | Ring may get duplicate events; acceptable for V1 diagnostic |
| Opening wrong object (stale fact) | Step 1 validates object → `[linen.quil.open.reject.missing]` if gone | LOW | Reject path fires, no mutation |
| Collar gate bypass | Not possible — `collar_check_operation_stub()` is inside `open_linen_object_in_quil()` | **NONE** | Gate fires before any mutation regardless of caller |
| Selection semantics bypass | N11 Enter = FocusLinen, PrintScreen = OpenInQuil — separate keys, separate behavior | **NONE** | No semantic collision |
| Buffer table full (16/16) | Step 4 → `[linen.quil.open.reject.full]` | LOW | Existing safe reject path |
| Buffer ID collision | Step 4 pre-flight check | LOW | Existing safe reject path |
| Shell dispatch collision | Mesh dispatch comes before scancode_to_action | **NONE** | PrintScreen intercepted in Mesh dispatch, Linen PrintScreen unaffected |

### Risk: Stale Fact → Wrong Object

A Mesh fact's `subject_id` is the Linen object_id at the time the fact was recorded. If the object is later deleted, `open_linen_object_in_quil()` rejects at step 1:

```
[mesh.keyboard.open_in_quil] subject_id=N
→ open_linen_object_in_quil(N)
  → [linen.quil.open.reject.missing] id=N
  → returns false
  → no buffer creation, no linking
```

**Safe.** The function validates object existence before any mutation.

### Risk: Duplicate Event/Fact Recording

`open_linen_object_in_quil()` calls `mesh_emit_linen_quil_links()` (step 8) which records Mesh facts for ALL existing links, not just the newly created one. This means calling OpenInQuil from Mesh for an already-linked object will:

1. Reuse existing buffer (step 4 — no mutation if already exists)
2. Record Mesh facts again (step 8 — overwrite-oldest ring handles this)
3. Record Bell event again (step 9 — ring may get duplicate)

**Acceptable for V1.** The ring overwrite-oldest policy prevents infinite growth, and duplicates are diagnostic-only.

## Chosen Key/Dispatch Behavior

### Decision: Intercept PrintScreen (0x59) in Mesh Dispatch

**Preserve N11:** Enter (0x1C) → FocusLinen at selected fact (unchanged).

**Add:** PrintScreen (0x59) while Mesh focused → OpenLinkedObjectInQuil via existing `open_linen_object_in_quil()`.

### Dispatch Chain

```
Enter (0x1C):  panel → palette → atlas → Bell → Mesh → scancode_to_action
                 [N11: Mesh focused → FocusLinen at selected fact]
PrintScreen (0x59):
                 panel → palette → atlas → Bell → Mesh → scancode_to_action
                   [NEW: Mesh focused → OpenLinkedObjectInQuil]
                   [Linen focused → (falls through to scancode_to_action → existing handler)]
```

### Implementation Sketch

```rust
// In the Mesh dispatch block, add 0x59 alongside existing 0x24/0x25/0x1C:
} else if FOCUSED_SURFACE_ID == SURFACE_ID_MESH
    && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C || scancode == 0x59)
{
    match scancode {
        0x24 => { /* J → next row */ }
        0x25 => { /* K → prev row */ }
        0x1C => { /* Enter → FocusLinen (N11) */ }
        0x59 => {
            // N14: Open linked object in Quil
            serial_println!("[mesh.keyboard.open_in_quil] sid={}", FOCUSED_SURFACE_ID);
            if let Some(fact) = mesh_selected_fact_snapshot() {
                open_linen_object_in_quil(fact.subject_id);
            }
        }
        _ => {}
    }
    mutated = true;
}
```

### Key: Why PrintScreen (0x59)?

1. **Already bound** to `SurfaceAction::OpenObjectInQuil` in scancode_to_action
2. **Already gated** to Linen focus in the handler (rejects if not Linen focused)
3. **Same semantic** — "open the selected thing in Quil" whether selected via Linen J/K or Mesh J/K
4. **No new key** needs to be documented — PrintScreen already means "Open in Quil"
5. **N11 preserved** — Enter remains FocusLinen, separate concern

### Key: Why NOT 'O'?

- 'O' is an unassigned scancode, would need new documentation
- PrintScreen already carries the "Open in Quil" semantic
- Using PrintScreen is consistent with Linen behavior

## Authority Requirement

### Collar: Already Present

`open_linen_object_in_quil()` calls `collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0)` at step 2.5. This gate fires regardless of who calls the function.

**No new Collar authority needed.** The existing J5 stub policy is sufficient for V1.

### Collar Bypass: Impossible

Because `collar_check_operation_stub()` is **inside** `open_linen_object_in_quil()`, not at the call site:

```
Caller A (Linen PrintScreen) → open_linen_object_in_quil() → collar gate → buffer mutation
Caller B (Mesh PrintScreen)  → open_linen_object_in_quil() → collar gate → buffer mutation
```

Both paths go through the same gate. Mesh cannot bypass Collar.

### Future Real Collar

When real Collar authority is implemented:
- J5's `AllowStub` is replaced with real `Allow`/`Deny`
- Both Linen-triggered and Mesh-triggered OpenInQuil respect the real Collar
- No change needed in Mesh dispatch — the gate is inside the shared function

## Allowed Side Effects (N14)

| Side Effect | Status | Rationale |
|-------------|--------|-----------|
| Call `open_linen_object_in_quil(fact.subject_id)` | ✅ ALLOWED | Reuses existing J4 chain with Collar gate |
| Create/reuse Quil buffer | ✅ ALLOWED (via existing function) | Inside Collar-guarded function |
| Open/focus Quil surface | ✅ ALLOWED (via existing function) | Inside `open_linen_object_in_quil()` |
| Record Mesh facts (J6) | ✅ ALLOWED (via existing function) | Inside `open_linen_object_in_quil()` |
| Record Bell events (J7) | ✅ ALLOWED (via existing function) | Inside `open_linen_object_in_quil()` |
| Render Quil buffer list | ✅ ALLOWED (via existing function) | Inside `open_linen_object_in_quil()` |
| Emit proof markers | ✅ ALLOWED | Console-only |
| Set `mutated = true` | ✅ ALLOWED | Required for dispatch |

## Forbidden Side Effects (N14)

| Side Effect | Status | Rationale |
|-------------|--------|-----------|
| Create Quil buffer from Mesh directly | ❌ FORBIDDEN | Only through `open_linen_object_in_quil()` |
| Call `collar_check_operation_stub()` from Mesh | ❌ FORBIDDEN | Not needed — gate is inside callee |
| Modify Mesh fact ring | ❌ FORBIDDEN | Read-only selection invariant |
| Modify Linen selection after action | ❌ FORBIDDEN | N11 Enter already handles this separately |
| Any PDX/ABI/kernel change | ❌ FORBIDDEN | STOP FIRST |
| Any sexdisplay change | ❌ FORBIDDEN | STOP FIRST |
| Bell behavior changes | ❌ FORBIDDEN | No Bell interaction from Mesh dispatch |
| New surface/frame creation (beyond open_quil_in_active_scene) | ❌ FORBIDDEN | Only existing navigation paths |

## STOP FIRST Table

| Trigger | Status for N14 | Notes |
|---------|---------------|-------|
| New PDX opcodes | ✅ NOT TRIGGERED | Uses existing opcodes only |
| sex-pdx ABI constants | ✅ NOT TRIGGERED | No ABI changes |
| Capability grants/revokes | ✅ NOT TRIGGERED | Existing AllowStub only |
| Cross-PD pointers | ✅ NOT TRIGGERED | Shell-local only |
| Kernel introspection | ✅ NOT TRIGGERED | No kernel changes |
| Persistent storage | ✅ NOT TRIGGERED | No storage changes |
| Renderer policy | ✅ NOT TRIGGERED | No sexdisplay changes |
| Mesh PD creation | ✅ NOT TRIGGERED | Shell-local only |
| Bell/Collar behavior | ✅ NOT TRIGGERED | No Bell/Collar changes |
| New Collar operation kind | ✅ NOT TRIGGERED | Reuses existing LinkObjectToBuffer |
| Direct buffer mutation from Mesh | ✅ NOT TRIGGERED | Only through existing function |

**STOP FIRST: NOT TRIGGERED** — All forbidden areas clean.

## N14 Implementation Prompt Summary

**N14: Open linked object in Quil from Mesh via PrintScreen.**

### Changes to `servers/silk-shell/src/main.rs`:

1. **Widen 0x59 capture in dispatch** — Prevent PrintScreen from falling through to
   scancode_to_action when Mesh is focused. The existing scancode_to_action handler
   already gates on `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN`, so when Mesh is focused,
   it emits `[linen.quil.open.reject] reason=not_focused`. The Mesh intercept consumes
   the key before this reject fires.

2. **Add 0x59 to Mesh dispatch** — In the existing `else if FOCUSED_SURFACE_ID == SURFACE_ID_MESH
   && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C)` block, add `|| scancode == 0x59`.

3. **Add match arm** — For `0x59`, emit `[mesh.keyboard.open_in_quil]`, snapshot the
   selected fact, call `open_linen_object_in_quil(fact.subject_id)`. The function validates
   object existence and calls the Collar gate internally.

4. **Add new proof markers** — `[mesh.keyboard.open_in_quil]` in the keyboard handler.
   All other markers come from `open_linen_object_in_quil()` chain (J4/J5/J6/J7/K3).

### Do NOT:
- Change `mesh_emit_selected_fact_detail_proof()` — FocusLinen stays on Enter
- Change `mesh_focus_linen_at_selected_fact()` — still called on Enter
- Add new CollarOperation or gate
- Create buffers from Mesh directly
- Modify any existing functions (additive dispatch only)

### Execution Trace (Success):

```
PrintScreen (0x59) while Mesh focused
  → [mesh.keyboard.open_in_quil] sid=202
  → mesh_selected_fact_snapshot() → Some(fact)
  → open_linen_object_in_quil(fact.subject_id)
    → [linen.quil.open.request] id=N
    → [linen.quil.open.no_grant] (if grant_ref == 0)
    → [collar.gate.check] op=6 object_id=N buffer_id=0
    → [collar.gate.allow_stub] op=6
    → [linen.quil.open.reuse_existing] OR [linen.quil.open.dynamic_id]
    → [linen.quil.buffer.linked] object_id=N buffer_id=1000+N
    → [linen.quil.quil_opened] object_id=N (if Quil not visible)
    → [linen.quil.done] object_id=N buffer_created=true/false
    → [mesh.object_link.*] (J6 chain)
    → [bell.event.*] (J7 chain)
    → [quil.buffer_list.*] (K3 chain)
```

### Execution Trace (Reject — stale fact):

```
PrintScreen (0x59) while Mesh focused
  → [mesh.keyboard.open_in_quil] sid=202
  → mesh_selected_fact_snapshot() → Some(fact)  [fact recorded when object existed]
  → open_linen_object_in_quil(fact.subject_id)
    → [linen.quil.open.reject.missing] id=N     [object no longer exists]
    → returns false
  → no buffer creation, no linking, no side effects
```

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Duplicate Bell events on repeated PrintScreen | LOW | Ring overwrite-oldest handles overflow; duplicates acceptable for V1 |
| Duplicate Mesh facts on repeated PrintScreen | LOW | Ring overwrite-oldest; duplicates harmless |
| PrintScreen intercepted by Mesh when Linen not focused | INFO | Consistent with "open selected thing" — Mesh has its own selection |
| User confusion: Enter = FocusLinen, PrintScreen = OpenInQuil | INFO | Both behaviors documented; PrintScreen consistent with Linen |
