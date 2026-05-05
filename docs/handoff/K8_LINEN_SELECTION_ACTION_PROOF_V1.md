# K8: Linen Selection Action Proof — Deterministic Trace

**Status:** Handoff (docs only — no code changes)
**Date:** 2026-05-05
**Purpose:** Document the complete deterministic trace from Linen selection through
open action, verifying every proof marker is wired in order. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                  PASS_K8_ACTION_PROOF                        ║
╠══════════════════════════════════════════════════════════════╣
║ All markers exist and are wired in correct order.           ║
║ No broken links, no missing steps, no STOP FIRST.            ║
║ Docs only — zero code changes.                              ║
╚══════════════════════════════════════════════════════════════╝
```

**Verdict: PASS_K8_ACTION_PROOF**

## 1. Trigger Path

### Selection Change (J/K keys)

```
User presses J (0x24) or K (0x25)
  → scancode_to_action() returns SelectNextLinenObject / SelectPrevLinenObject
  → Handler checks FOCUSED_SURFACE_ID == SURFACE_ID_LINEN
    → if false: [linen.object_select.reject] reason=not_focused (no-op)
    → if true:  linen_select_next_object() or linen_select_prev_object()
                 → [linen.object_select.next] prev=N next=M  (or .prev / .wrap)
                 → [linen.object_select.reject] reason=single_object (if <2 objects)
                linen_render_object_list()
                 → [linen.selection_visual.header] object_id=N color=0xXXXXXX
                 → [linen.object_select.current] id=N
                 → [linen.object_list.render/row/skip/done] per object
```

### Open Action (PrintScreen 0x59)

```
User presses PrintScreen (0x59)
  → scancode_to_action() returns OpenObjectInQuil
    → Note: PrintScreen is a global debug trigger, NOT scoped to Linen focus (K9 target)
  → linen_selected_object_id()
    → if 0: linen_select_first_valid_object()
            → if still 0: [linen.object_select.reject] reason=no_objects → abort
            → [linen.object_select.repair] id=N
    → [linen.object_select.current] id=N
  → open_linen_object_in_quil(obj_id)
    → [linen.quil.open.request] id=N
    → find object by ID
      → if not found: [linen.quil.open.reject.missing] → abort
    → [linen.quil.open.no_grant] (stub — all grant_ref=0)
    → collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)
      → [collar.gate.check] op=6 object_id=N buffer_id=0
      → [collar.gate.allow_stub] op=6 (or .reject / .needs_grant)
      → if not AllowStub: [linen.quil.open.reject.collar] → abort
    → map LinenObjectKind → QuilBufferKind
    → compute dynamic_buffer_id = QUIL_DYNAMIC_BUFFER_ID_BASE + object_id (= 1000+N)
    → scan for existing buffer
      → if found: [linen.quil.open.reuse_existing] object_id=N buffer_id=1000+N
      → if not found:
        → pre-flight collision check
          → if collision: [linen.quil.open.reject.buffer_id_collision] → abort
        → allocate free slot
          → if full: [linen.quil.open.reject.full] → abort
          → [linen.quil.open.dynamic_id] object_id=N dynamic_buffer_id=1000+N
    → update LinenObject.linked_surface_id = SURFACE_ID_QUIL
    → [linen.quil.buffer.linked] object_id=N buffer_id=1000+N kind=K
    → open_quil_in_active_scene() if not already open
    → mesh_emit_linen_quil_links()
      → [mesh.object_link.start]
      → for each buffer with non-zero linen_object_ref:
        → find LinenObject by ref
          → if found: [mesh.object_link.row] object_id=N kind=K buffer_id=M kind=K surface_id=201
          → if not found: [mesh.object_link.reject.missing_object] buffer_id=M ref=N
      → [mesh.object_link.done] links=N stale=N
    → bell_emit_object_link_event(object_id, dynamic_buffer_id)
      → [bell.event.stub] kind=ObjectLinkedToBuffer object_id=N buffer_id=1000+N
      → validate object and buffer exist + ref match
        → [bell.event.reject.missing] if either missing
        → [bell.event.reject.missing] reason=buffer_ref_mismatch if ref mismatch
        → [bell.event.object_link] object_id=N kind=K buffer_id=1000+N kind=K
      → [bell.event.done] reason=emitted (or reason=rejected)
    → quil_render_buffer_list()
      → [quil.buffer_list.render] w=N h=N
      → for each buffer:
        → [quil.buffer_list.row] buffer_id=1000+N kind=K state=Open linen_ref=N surface_id=201 name=X
        → [quil.buffer_list.skip] if over max rows
      → [quil.buffer_list.done] count=N rows=N
    → [linen.quil.done] object_id=N buffer_created=true/false
```

## 2. Expected Marker Order (Success Path)

```
[linen.object_select.repair] id=N              (only on first use)
[linen.object_select.current] id=N
[linen.quil.open.request] id=N
[linen.quil.open.no_grant] id=N kind=K
[collar.gate.check] op=6 object_id=N buffer_id=0
[collar.gate.allow_stub] op=6
[linen.quil.open.dynamic_id] object_id=N dynamic_buffer_id=1000+N
[linen.quil.buffer.linked] object_id=N buffer_id=1000+N kind=K
[mesh.object_link.start]
[mesh.object_link.row] object_id=N kind=K buffer_id=1000+N kind=K surface_id=201
[mesh.object_link.done] links=N stale=0
[bell.event.stub] kind=ObjectLinkedToBuffer object_id=N buffer_id=1000+N
[bell.event.object_link] object_id=N kind=K buffer_id=1000+N kind=K
[bell.event.done] reason=emitted
[quil.buffer_list.render] w=N h=N
[quil.buffer_list.row] buffer_id=1000+N kind=K state=Open linen_ref=N surface_id=201 name=X
[quil.buffer_list.done] count=N rows=N
[linen.quil.done] object_id=N buffer_created=true
```

Total: 22 proof markers on success (including the boot-time init markers).

## 3. ID/Namespace Proof

| ID | Value | Scope | Violation? |
|----|-------|-------|------------|
| Selected object_id | 1-6 (seed), up to 16 (dynamic) | Shell-local (PKEY 3) | ✅ None |
| Dynamic buffer_id | 1000+object_id (range 1001-1016) | Shell-local | ✅ None — guaranteed > max seed ID (6) |
| linked_surface_id | `SURFACE_ID_QUIL` = 201 | Shell-local surface tier | ✅ None — opaque to sexdisplay |
| grant_ref | `GRANT_REF_STUB` = 0 | Shell-local | ✅ None — documented placeholder |
| PDX slot | `SLOT_DISPLAY` (slot 5) | IPCPKU_MAP canon | ✅ None — existing established slot |
| PDX opcode | 0xEF (fill rect) | sex-pdx canon | ✅ None — existing established opcode |
| PKEY | 3 (silk-shell) | IPCPKU_MAP canon | ✅ None — all work in silk-shell |

**All shell-local. No namespace pollution.**

## 4. Boundary Proof

| Boundary | Status |
|----------|--------|
| `kernel/` | ✅ No edits — all work is shell-local static state |
| `crates/sex-pdx/` | ✅ No edits — uses existing 0xEF, 0x59, no new opcodes |
| `servers/sexdisplay/` | ✅ No edits — sexdisplay treats fill rects as opaque |
| `servers/linen/` (real server) | ✅ No edits — Linen model is silk-shell-local |
| `servers/quil/` (real server) | ✅ No edits — Quil model is silk-shell-local |
| Storage/filesystem | ✅ No edits — no persistence |
| Real Bell queue/UI | ✅ No edits — Bell is silk-shell-local stubs |
| Real Mesh graph | ✅ No edits — Mesh is silk-shell-local stubs |
| Real Collar authority | ✅ No edits — Collar is stub gate |

**Boundaries intact.**

## 5. Failure/Reject Paths

| Path | Marker | Condition |
|------|--------|-----------|
| No selection | `[linen.object_select.reject] reason=no_objects` | LINEN_OBJECTS all None |
| J/K not focused | `[linen.object_select.reject] reason=not_focused` | FOCUSED_SURFACE_ID != SURFACE_ID_LINEN |
| Single object, can't cycle | `[linen.object_select.reject] reason=single_object` | Only 1 object in table |
| Missing object | `[linen.quil.open.reject.missing]` | object_id not found in Linen table |
| Collar stub denies | `[collar.gate.reject]` + `[linen.quil.open.reject.collar]` | decision != AllowStub |
| Buffer ID collision | `[linen.quil.open.reject.buffer_id_collision]` | dynamic_buffer_id taken by different ref |
| Table full | `[linen.quil.open.reject.full]` | No free QUIL_BUFFERS slot |
| Bell missing/mismatch | `[bell.event.reject.missing]` / `[bell.event.done] reason=rejected` | Object or buffer missing after link |

All reject paths produce proof markers. No silent failures.

## 6. Remaining Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| PrintScreen global debug trigger | LOW | Documented; K9 target to scope to Linen focus |
| Bell placeholder naming (BELL_PLACEHOLDER vs BELL) | LOW | Documented in K2B §3.4 |
| Seed pre-links coherent but not J5/J7-runtime-created | LOW | K2C boot sync; documented in K2B §4 |
| No real Collar authorization | MEDIUM | All grant_ref=GRANT_REF_STUB; STOP FIRST for real |

## 7. Next Safest Step

**K9: Scope PrintScreen trigger to Linen focus.**

Currently PrintScreen fires globally (any surface focused). Change:
```
if FOCUSED_SURFACE_ID == SURFACE_ID_LINEN {
    ... open selected object ...
} else {
    [linen.quil.open.reject] reason=not_focused
}
```

This makes the open action symmetric with J/K gating. No other changes needed.

After K9: decide between:
- **Command palette stub** (new placeholder surface, I1-I3 pattern)
- **Multi-rect display** (STOP FIRST — sexdisplay change for per-row highlights)
- **Proof chain hardening** (real Bell event contract implementation)
