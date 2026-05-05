# N14: Mesh Open Linked Object in Quil

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Implement OpenLinkedObjectInQuil from Mesh via PrintScreen interception,
following the N13 design exactly. Additive dispatch only — all Collar/Buffer/Link
safety inside existing `open_linen_object_in_quil()`.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N14_MESH_OPEN_IN_QUIL                     ║
╠══════════════════════════════════════════════════════════════╣
║ Design source:            N13 design doc                     ║
║ Trigger:                  PrintScreen (0x59) when Mesh focused ║
║ Collar gate:              LinkObjectToBuffer → AllowStub      ║
║ Collar bypass:            IMPOSSIBLE (gate inside callee)     ║
║ N11 Enter behavior:       PRESERVED (FocusLinen only)        ║
║ Boundaries:              INTAKT                               ║
║ Build:                    PASS (1619 sectors)                 ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (3 insertions, 1 modification)
**Handoff:** `docs/handoff/N14_MESH_OPEN_LINKED_OBJECT_IN_QUIL_V1.md`
**Commit:** *(to be added)*

### 1. Widen Mesh dispatch condition (line 9347)

```rust
// Before:
&& (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C)

// After:
&& (scancode == 0x24 || scancode == 0x25 || scancode == 0x1C || scancode == 0x59)
```

### 2. Add 0x59 match arm (lines 9369-9378)

```rust
// N14: Open selected fact's linked object in Quil via PrintScreen.
// Reuses existing open_linen_object_in_quil() which contains the
// Collar gate (LinkObjectToBuffer → AllowStub). Mesh cannot bypass
// Collar because the gate is inside the callee, not at the call site.
0x59 => {
    serial_println!("[mesh.keyboard.open_in_quil] sid={}", FOCUSED_SURFACE_ID);
    if let Some(fact) = mesh_selected_fact_snapshot() {
        open_linen_object_in_quil(fact.subject_id);
    }
}
```

## Design Decisions

### Key Choice: PrintScreen (0x59)

1. **Already bound** to `SurfaceAction::OpenObjectInQuil` in scancode_to_action
2. **Already gated** to Linen focus in the handler — but Mesh intercepts it first
3. **Same semantic** — "open the selected thing in Quil" whether selected via Linen J/K or Mesh J/K
4. **No new key** — PrintScreen already means "Open in Quil" in the system
5. **N11 preserved** — Enter remains FocusLinen, separate concern

### Collar Bypass: Impossible

`collar_check_operation_stub()` is **inside** `open_linen_object_in_quil()`, not at the call site:

```
Linen PrintScreen → open_linen_object_in_quil() → collar gate → buffer mutation
Mesh PrintScreen  → open_linen_object_in_quil() → collar gate → buffer mutation
```

Both paths go through the exact same gate. Mesh cannot bypass Collar.

### Dispatch Chain

```
PrintScreen (0x59):
  panel → palette → atlas → Bell → Mesh [NEW: OpenInQuil] → scancode_to_action
    → Mesh focused: consumed, [mesh.keyboard.open_in_quil] emitted
    → Linen focused: falls through to scancode_to_action → existing handler
    → Other focused: falls through to scancode_to_action → [linen.quil.open.reject] reason=not_focused
```

## Proof Markers

| Marker | Location | Description |
|--------|----------|-------------|
| `[mesh.keyboard.open_in_quil]` | dispatch handler | PrintScreen consumed while Mesh focused |
| `[linen.quil.open.request]` | open_linen_object_in_quil | J4 chain step 1 |
| `[linen.quil.open.no_grant]` | open_linen_object_in_quil | J4 chain step 2 |
| `[collar.gate.check]` | collar_check_operation_stub | J5 chain — always fires |
| `[collar.gate.allow_stub]` | collar_check_operation_stub | J5 chain — AllowStub for valid objects |
| `[linen.quil.open.reuse_existing]` | open_linen_object_in_quil | J4 chain step 4 — found existing |
| `[linen.quil.open.dynamic_id]` | open_linen_object_in_quil | J4 chain step 4 — created new |
| `[linen.quil.buffer.linked]` | open_linen_object_in_quil | J4 chain step 5 |
| `[linen.quil.done]` | open_linen_object_in_quil | J4 chain step end |
| `[mesh.object_link.*]` | mesh_emit_linen_quil_links | J6 chain (inside callee) |
| `[bell.event.*]` | bell_emit_object_link_event | J7 chain (inside callee) |
| `[quil.buffer_list.*]` | quil_render_buffer_list | K3 chain (inside callee) |

All markers from `open_linen_object_in_quil()` chain fire automatically — no new markers needed in Mesh dispatch.

## Execution Traces

### Success — Valid fact, object exists, buffer created/reused

```
[mesh.keyboard.open_in_quil] sid=202
→ mesh_selected_fact_snapshot() → Some(fact)
→ open_linen_object_in_quil(fact.subject_id)
  → [linen.quil.open.request] id=N
  → [linen.quil.open.no_grant] id=N kind=K
  → [collar.gate.check] op=6 object_id=N buffer_id=0
  → [collar.gate.allow_stub] op=6
  → [linen.quil.open.dynamic_id] object_id=N dynamic_buffer_id=1000+N
    OR [linen.quil.open.reuse_existing] object_id=N buffer_id=1000+N
  → [linen.quil.buffer.linked] object_id=N buffer_id=1000+N kind=K
  → [linen.quil.quil_opened] object_id=N (if Quil not visible)
  → [linen.quil.done] object_id=N buffer_created=true/false
  → [mesh.object_link.start/row/done] (J6 chain)
  → [bell.event.stub/object_link/done] (J7 chain)
  → [quil.buffer_list.render/row/done] (K3 chain)
```

### Reject — Stale fact (object deleted)

```
[mesh.keyboard.open_in_quil] sid=202
→ mesh_selected_fact_snapshot() → Some(fact)  [fact recorded when object existed]
→ open_linen_object_in_quil(fact.subject_id)
  → [linen.quil.open.reject.missing] id=N     [object no longer exists]
  → returns false
→ no buffer creation, no linking, no side effects
```

### Reject — No fact selected (empty ring)

```
[mesh.keyboard.open_in_quil] sid=202
→ mesh_selected_fact_snapshot() → None
→ no action taken
```

## Boundaries

| Area | Status |
|------|--------|
| `kernel/` | ✅ CLEAN |
| `crates/sex-pdx/` | ✅ CLEAN |
| `servers/sexdisplay/` | ✅ CLEAN |
| `servers/bell/` | ✅ CLEAN |
| `servers/mesh/` | ✅ CLEAN |
| `servers/linen/` | ✅ CLEAN |
| `servers/quil/` | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Mesh PD creation | ✅ NOT TRIGGERED |
| Collar authority | ✅ NOT TRIGGERED (existing AllowStub only) |
| Bell behavior | ✅ NOT TRIGGERED |

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
| New Collar operation kind | ✅ NOT TRIGGERED |

**STOP FIRST: NOT TRIGGERED** — All forbidden areas clean.

## Forbidden Side Effects (Verified Clean)

| Side Effect | Status |
|-------------|--------|
| Create Quil buffer from Mesh directly | ❌ NOT DONE — only through `open_linen_object_in_quil()` |
| Call `collar_check_operation_stub()` from Mesh | ❌ NOT DONE — gate is inside callee |
| Modify Mesh fact ring | ❌ NOT DONE — read-only selection invariant |
| Modify Linen selection after action | ❌ NOT DONE — N11 Enter handles this separately |
| Any PDX/ABI/kernel change | ❌ NOT DONE |
| Any sexdisplay change | ❌ NOT DONE |
| Bell behavior changes | ❌ NOT DONE |
| New surface/frame creation | ❌ NOT DONE — only existing navigation paths |

## Build Result

```
[TRACE] stage=package_iso
ISO image produced: 1619 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Duplicate Bell events on repeated PrintScreen | LOW | Ring overwrite-oldest handles overflow |
| Duplicate Mesh facts on repeated PrintScreen | LOW | Ring overwrite-oldest; duplicates harmless |
| PrintScreen intercepted by Mesh when Linen not focused | INFO | Consistent with "open selected thing" |

## Next Steps

**N15: Rapid audit of N14** — verify no accidental Collar bypass, no duplicate fact/event
issues, no boundary violations. Docs only.

After N15: evaluate next subsystem work (Mesh fact rendering improvements, or move
to Shell/Text/Storage feature work).
