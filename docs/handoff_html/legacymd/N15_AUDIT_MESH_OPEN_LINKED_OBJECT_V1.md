# N15: Audit Mesh Open Linked Object in Quil

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Verify N14 Mesh PrintScreen → OpenLinkedObjectInQuil is safe. Confirm
N13 design assumptions held during implementation. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║        PASS_N14_MESH_OPEN_LINKED_OBJECT                      ║
╠══════════════════════════════════════════════════════════════╣
║ Dispatch/key behavior:    PASS                                ║
║ Collar gate:              PASS (gate inside callee)           ║
║ Fact/object safety:       PASS (read-only, validated callee)  ║
║ Side effects:             PASS (additive dispatch only)       ║
║ Boundaries:               INTAKT                               ║
║ Build:                    PASS (1619 sectors)                  ║
╚══════════════════════════════════════════════════════════════╝
```

## Source Documents

| Document | Reference |
|----------|-----------|
| `docs/handoff/N14_MESH_OPEN_LINKED_OBJECT_IN_QUIL_V1.md` | Implementation handoff |
| `docs/handoff/N13_MESH_OPEN_LINKED_OBJECT_DESIGN_V1.md` | Design doc (safety assumptions) |
| `docs/handoff/N12_AUDIT_MESH_FOCUS_LINEN_V1.md` | Prior audit (verification patterns) |
| `docs/handoff/J4_LINEN_OBJECT_TO_QUIL_BUFFER_V1.md` | J4 chain (collar gate, buffer ops) |
| `docs/handoff/J5_COLLAR_GATED_OPERATION_STUBS_V1.md` | Collar stub policy |
| `servers/silk-shell/src/main.rs` | Implementation (verified at lines 9345-9381, 9649-9663, 971-1098) |

## 1. Dispatch/Key Behavior

### Dispatch Chain Precedence (lines 9258-9381)

| Priority | Handler | Condition | 0x59 consumed here? |
|----------|---------|-----------|---------------------|
| 1 | Scene Settings Panel | `SCENE_SETTINGS_ACTIVE` | ❌ No — panel only intercepts 1/2/3/Esc |
| 2 | Command Palette | `COMMAND_PALETTE_OPEN` | ❌ No — palette intercepts J/K/Enter/Esc/backtick |
| 3 | Atlas | `ATLAS_MODE_ENABLED` && scancode != 0x44 | ❌ No — Atlas only intercepts nav keys |
| 4 | Bell focused-surface | `FOCUSED_SURFACE_ID == SURFACE_ID_BELL_PLACEHOLDER` | ❌ No — Bell only intercepts J/K/Enter |
| **5** | **Mesh focused-surface** | **`FOCUSED_SURFACE_ID == SURFACE_ID_MESH`** | **✅ Consumed by Mesh** |
| 6 | scancode_to_action | Default dispatch | ❌ Not reached when Mesh consumes |

**Dispatch precedence: PASS** — Panel → palette → atlas → Bell → **Mesh** → scancode_to_action.
All precedences preserved unchanged from prior milestones.

### Mesh-Only Intercept

The 0x59 capture gates on `FOCUSED_SURFACE_ID == SURFACE_ID_MESH` (line 9346):

```rust
} else if FOCUSED_SURFACE_ID == SURFACE_ID_MESH
    && (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C || scancode == 0x59)
```

| Focus | PrintScreen (0x59) reaches | Behavior |
|-------|---------------------------|----------|
| Mesh | Mesh dispatch (line 9372) | `[mesh.keyboard.open_in_quil]` → `open_linen_object_in_quil()` |
| Linen | scancode_to_action (line 9651) | `[shell.action.open_object_in_quil]` → same `open_linen_object_in_quil()` |
| Bell | scancode_to_action (line 9660) | `[linen.quil.open.reject] reason=not_focused` |
| Quil | scancode_to_action (line 9660) | `[linen.quil.open.reject] reason=not_focused` |
| Collar | scancode_to_action (line 9660) | `[linen.quil.open.reject] reason=not_focused` |
| Any other | scancode_to_action (line 9660) | `[linen.quil.open.reject] reason=not_focused` |

**Key finding:** Both Mesh-focused and Linen-focused PrintScreen reach the **identical**
`open_linen_object_in_quil()` function. The only difference is which code path calls it —
Mesh dispatch or scancode_to_action handler. Both go through the same Collar gate.

### N11 Enter Preserved

The Enter (0x1C) handler at line 9358 is **unchanged** from N11:
```rust
0x1C => {
    serial_println!("[mesh.keyboard.enter] sid={}", FOCUSED_SURFACE_ID);
    if mesh_emit_selected_fact_detail_proof() {
        if let Some(fact) = mesh_selected_fact_snapshot() {
            mesh_focus_linen_at_selected_fact(&fact);
        }
    }
}
```

**N11 Enter behavior: PRESERVED** — FocusLinen at selected fact, no link creation, unchanged.

### Global/Linen PrintScreen

When Mesh is NOT focused, 0x59 falls through to `scancode_to_action(0x59)` which returns
`Some(SurfaceAction::OpenObjectInQuil)` (line 2385). The handler at line 9651 gates on
`FOCUSED_SURFACE_ID == SURFACE_ID_LINEN`:

```rust
SurfaceAction::OpenObjectInQuil => {
    if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
        // ... open_linen_object_in_quil(obj_id)
    } else {
        serial_println!("[linen.quil.open.reject] reason=not_focused");
    }
}
```

**Global PrintScreen: PASS** — Reaches existing path when Mesh not focused.
Linen-focused PrintScreen works as before K9.

### Scenario Verification

| Scenario | Handled by | Result | Status |
|----------|-----------|--------|--------|
| Mesh focused, PrintScreen | Mesh dispatch | `open_linen_object_in_quil(fact.subject_id)` | ✅ Correct |
| Linen focused, PrintScreen | scancode_to_action | `open_linen_object_in_quil(SELECTED_LINEN_OBJECT_ID)` | ✅ Correct |
| Bell focused, PrintScreen | scancode_to_action | `[linen.quil.open.reject] reason=not_focused` | ✅ Correct |
| Quil focused, PrintScreen | scancode_to_action | `[linen.quil.open.reject] reason=not_focused` | ✅ Correct |
| Palette open, PrintScreen | Palette (pass-through) | Falls through to Mesh → same as above | ✅ Correct |
| Atlas active, PrintScreen | Atlas (pass-through) | Falls through to Mesh → same as above | ✅ Correct |

**Dispatch/Key behavior: PASS** — All scenarios verified. No regressions.

## 2. Collar Gate

### Collar Gate Location

The Collar gate is inside `open_linen_object_in_quil()` at line 998:

```rust
// 2.5 J5: Check Collar gate before linking.
let decision = collar_check_operation_stub(CollarOperation::LinkObjectToBuffer, object_id, 0);
if decision != CollarDecision::AllowStub {
    serial_println!("[linen.quil.open.reject.collar] decision={}", decision as u8);
    return false;
}
```

### Mesh Dispatch Call Path

The Mesh dispatch (line 9372) calls `open_linen_object_in_quil(fact.subject_id)`:

```
Mesh dispatch (line 9372)
  → open_linen_object_in_quil(fact.subject_id)  (line 971)
    → step 1: validate object exists             (line 975-990)
    → step 2: check grant_ref                    (line 993-995)
    → step 2.5: collar_check_operation_stub()    (line 998) ← COLLAR GATE
    → step 3: map buffer kind                    (line 1005-1010)
    → step 4: create/reuse buffer                (line 1012-1066)
    → step 5: update linked_surface_id           (line 1068-1073)
    → step 6: emit [linen.quil.buffer.linked]    (line 1075-1078)
    → step 7: open Quil surface                  (line 1081-1084)
    → step 8: mesh_emit_linen_quil_links()       (line 1089)
    → step 9: bell_emit_object_link_event()      (line 1092)
    → step 10: quil_render_buffer_list()         (line 1095)
```

### Collar Gate Verification Table

| Property | Implementation | Status |
|----------|---------------|--------|
| Mesh creates/reuses buffers directly | ❌ NO — only through `open_linen_object_in_quil()` | ✅ PASS |
| Mesh calls collar_check_operation_stub() directly | ❌ NO — gate is inside callee | ✅ PASS |
| open_linen_object_in_quil() still calls collar_check_operation_stub() | ✅ YES — line 998 unchanged | ✅ PASS |
| collar_check_operation_stub() rejects invalid objects | ✅ YES — returns DenyMissingObject for missing/zero object_id | ✅ PASS |
| reject/deny paths remain in callee | ✅ YES — `[linen.quil.open.reject.collar]` at line 1000-1001 | ✅ PASS |
| New CollarOperation kind added | ❌ NO — reuses existing LinkObjectToBuffer (op=6) | ✅ PASS |
| New authority/grant/revoke added | ❌ NO — existing AllowStub only | ✅ PASS |

### Collar Bypass: IMPOSSIBLE

Both callers go through the exact same gate:

```
Linen focused PrintScreen → scancode_to_action → open_linen_object_in_quil() → collar gate → buffer mutation
Mesh focused PrintScreen  → Mesh dispatch        → open_linen_object_in_quil() → collar gate → buffer mutation
```

The Collar gate is **inside** `open_linen_object_in_quil()`. There is no way for Mesh dispatch
to call any mutation function without going through the gate. N13 design assumption verified.

**Collar gate: PASS** — No bypass possible. All assumptions from N13 confirmed.

## 3. Fact/Object Safety

### Fact Snapshot: Read-Only

`mesh_selected_fact_snapshot()` (line 1489) returns a `Copy` of the `MeshFact` struct.
It reads from `MESH_FACTS` static array but never writes:

```rust
unsafe fn mesh_selected_fact_snapshot() -> Option<MeshFact> {
    // ... read-only iteration, returns Copy
}
```

| Property | Implementation | Status |
|----------|---------------|--------|
| Reads from MESH_FACTS | ✅ YES — iteration only | ✅ PASS |
| Writes to MESH_FACTS | ❌ NO — read-only | ✅ PASS |
| Writes to MESH_SELECTED_ROW | ❌ NO — not modified | ✅ PASS |
| Writes to MESH_FACT_WRITE_INDEX | ❌ NO — not modified | ✅ PASS |
| Allocates | ❌ NO — stack-only Copy | ✅ PASS |

### Fact Kind Handling

Mesh dispatch (line 9374) calls `open_linen_object_in_quil(fact.subject_id)` with the
selected fact's `subject_id`. The fact kind is not checked in the dispatch handler —
the `subject_id` is passed to the callee regardless of fact kind.

However, `open_linen_object_in_quil()` validates the object at step 1 (line 975-990).
If `subject_id` does not correspond to a valid Linen object, the function rejects with
`[linen.quil.open.reject.missing]` before any mutation.

| Scenario | subject_id type | Callee behavior | Status |
|----------|----------------|-----------------|--------|
| ObjectLinkedToBuffer fact | Linen object_id (valid at record time) | Validates → proceeds or rejects if stale | ✅ Safe |
| Future fact kind (subject_id may not be Linen object_id) | Could be any ID | Validates → rejects if not a valid Linen object | ✅ Safe — callee rejects invalid objects |

**Key safety:** The callee validates the object ID at step 1, before any Collar gate or
buffer mutation. Even if a future fact kind stores something other than a Linen object_id
in `subject_id`, the callee will safely reject it.

### Stale Fact Safety

When a fact references an object that has been deleted since the fact was recorded:

```
[mesh.keyboard.open_in_quil] sid=202
→ mesh_selected_fact_snapshot() → Some(fact)  [fact recorded when object existed]
→ open_linen_object_in_quil(fact.subject_id)
  → step 1: find LinenObject by ID
  → NOT FOUND → [linen.quil.open.reject.missing] id=N
  → returns false
  → no buffer creation, no linking, no side effects
```

**Stale fact: SAFE** — Callee rejects at step 1 before any mutation. Confirms N13 design
assumption.

### subject_id Mapping

`MeshFact::subject_id` for `ObjectLinkedToBuffer` facts is set from `o.object_id` in
`mesh_emit_linen_quil_links()` (line 1317-1326):

```rust
mesh_record_fact(MeshFactKind::ObjectLinkedToBuffer, o.object_id, buf.buffer_id, buf.linked_surface_id);
```

Where `o` is a valid `linen_object_by_id()` result. The fact's `subject_id` is always a
valid Linen object ID at the time the fact was recorded.

**subject_id: SOURCE-AUTHORITATIVE** — Written from live object table at record time.

### Object Existence Confirmation

When `open_linen_object_in_quil()` is called with a valid object ID that still exists:

```
step 1: find object → found → proceed
step 2.5: Collar gate → AllowStub (for valid objects per J5)
step 4: create/reuse buffer → either [linen.quil.open.dynamic_id] or [linen.quil.open.reuse_existing]
```

When the object was deleted since the fact was recorded:

```
step 1: find object → NOT found → [linen.quil.open.reject.missing] → return false
```

**Object validation: PASS** — Safe reject path for stale facts.

**Fact/object safety: PASS** — All read-only operations. Callee validates before mutation.

## 4. Side Effects

### Allowed Side Effects (Inside `open_linen_object_in_quil()`)

| Side Effect | Type | Guard | Status |
|-------------|------|-------|--------|
| Create/reuse Quil buffer | Buffer mutation (step 4) | Collar gate (step 2.5) | ✅ ALLOWED |
| Update linked_surface_id | Linen mutation (step 5) | Collar gate (step 2.5) | ✅ ALLOWED |
| Open Quil surface | Navigation (step 7) | None (pure navigation) | ✅ ALLOWED |
| Emit Mesh facts (J6) | Proof markers (step 8) | None (read-only ring) | ✅ ALLOWED |
| Emit Bell events (J7) | Proof markers (step 9) | None (read-only ring) | ✅ ALLOWED |
| Render Quil buffer list | Display (step 10) | None (existing function) | ✅ ALLOWED |
| Set `mutated = true` | Dispatch flag | Set after action | ✅ ALLOWED |
| Emit proof markers | Console output | None | ✅ ALLOWED |

### Forbidden Side Effects (Verified Absent)

| Side Effect | Searched | Evidence | Status |
|-------------|----------|----------|--------|
| Create Quil buffer from Mesh directly | `.rs` grep for `QUIL_BUFFERS` in Mesh dispatch | Not present — only through callee | ✅ CONFIRMED |
| Call collar_check_operation_stub() from Mesh | `.rs` grep for `collar_check` in Mesh dispatch (lines 9345-9380) | Not present — gate inside callee | ✅ CONFIRMED |
| Modify Mesh fact ring | `mesh_selected_fact_snapshot()` — read-only copy | No write to MESH_FACTS in Mesh dispatch | ✅ CONFIRMED |
| Modify Linen selection after action | Mesh dispatch (lines 9372-9376) | No SELECTED_LINEN_OBJECT_ID write | ✅ CONFIRMED |
| Any PDX/ABI/kernel change | All files | No changes outside silk-shell | ✅ CONFIRMED |
| Any sexdisplay change | All files | No changes to servers/sexdisplay/ | ✅ CONFIRMED |
| Bell behavior changes | All files | No changes to servers/bell/ | ✅ CONFIRMED |
| New surface/frame creation | Mesh dispatch | Only `open_quil_in_active_scene()` inside callee | ✅ CONFIRMED |

**Side effects: PASS** — All forbidden side effects confirmed absent. Only allowed side
effects through existing callee chain.

## 5. Boundary Check

| Area | Status | Evidence |
|------|--------|----------|
| `kernel/` | ✅ CLEAN | No kernel changes in N14 commit |
| `crates/sex-pdx/` | ✅ CLEAN | No ABI/opcode changes |
| `servers/sexdisplay/` | ✅ CLEAN | No sexdisplay changes |
| `servers/bell/` | ✅ CLEAN | No Bell code changes |
| `servers/mesh/` | ✅ CLEAN | No Mesh PD created |
| `servers/linen/` | ✅ CLEAN | No real linen server changes |
| `servers/quil/` | ✅ CLEAN | No real quil server changes |
| PDX ABI/opcodes | ✅ CLEAN | No opcode additions |
| Mesh PD creation | ✅ NOT TRIGGERED | Shell-local only |
| Collar authority | ✅ NOT TRIGGERED | Existing AllowStub only |
| Bell behavior | ✅ NOT TRIGGERED | No Bell ring changes |

### STOP FIRST Triggers

| Trigger | Status | Notes |
|---------|--------|-------|
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

### git diff Verification

The N14 commit (`c6ce058`) changes only one file:

```
git diff c6ce058^..c6ce058 --stat
 servers/silk-shell/src/main.rs                     |  12 +-
 docs/handoff/N14_MESH_OPEN_LINKED_OBJECT_IN_QUIL_V1.md | 217 ++++++++
```

Only `servers/silk-shell/src/main.rs` and the handoff doc were modified.
No kernel, sex-pdx, sexdisplay, bell, mesh, linen, or quil server changes.

**Boundaries: INTAKT** — All boundary areas confirmed clean.

## 6. N13 Design Assumptions Verifications

| N13 Assumption | Verification | Status |
|---------------|-------------|--------|
| Collar bypass impossible (gate inside callee) | `open_linen_object_in_quil()` line 998 unchanged | ✅ CONFIRMED |
| Stale facts safe (object validation before mutation) | Step 1 rejects missing objects (lines 975-990) | ✅ CONFIRMED |
| PrintScreen already gated to Linen focus | scancode_to_action handler line 9652 | ✅ CONFIRMED |
| N11 Enter behavior preserved | 0x1C handler line 9358-9366 unchanged | ✅ CONFIRMED |
| Capsule dispatch precedence preserved | Panel → palette → atlas → Bell → Mesh → action | ✅ CONFIRMED |
| Mesh does not create buffers directly | Only through `open_linen_object_in_quil()` | ✅ CONFIRMED |
| No new CollarOperation needed | Reuses existing LinkObjectToBuffer (op=6) | ✅ CONFIRMED |
| No kernel/ABI/sexdisplay changes | All files unchanged outside silk-shell | ✅ CONFIRMED |

**All N13 assumptions confirmed.** Implementation exactly matches design.

## 7. Remaining Risks

| Risk | Severity | Status | Mitigation |
|------|----------|--------|------------|
| Duplicate Bell events on repeated PrintScreen | LOW | Accepted | Ring overwrite-oldest handles overflow; duplicates acceptable for V1 |
| Duplicate Mesh facts on repeated PrintScreen | LOW | Accepted | Ring overwrite-oldest; duplicates harmless |
| PrintScreen intercepted by Mesh when Linen not focused | INFO | Documented | Consistent with "open selected thing" semantic |
| Fact recorded when object existed, but object deleted later | LOW | Safe | Callee rejects at step 1 (object validation) before any mutation |
| Future fact kind uses subject_id for non-Linen-object ID | LOW | Safe | Callee validates object existence; rejects non-Linen IDs |
| Single-thread dispatch ensures no race between fact snapshot and callee | INFO | Safe | No preemption in shell dispatch loop |

**No new risks introduced by N14.** All existing risk mitigations confirmed.

## 8. Next Safest Step

**Mesh milestone closure** — Document the complete Mesh subsystem as implemented
(M1-M6, N1-N14) in a closure doc summarizing all changes, boundaries, and remaining
risks. After which:

**Option A: Collar real authority model design** — Design the authority model for
Collar that will eventually replace `AllowStub` with real policy decisions. This is
a prerequisite for any production use of the existing Linen/Quil/Mesh chain.

**Option B: Other subsystem work** — Text input, storage integration, or display
multi-rect enhancements. Each requires STOP FIRST design before implementation.

## Build Result

```
ISO image produced: 1619 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — Verified at N14 commit `c6ce058`.
