# K15: Command Palette Action Trace Proof

**Status:** Handoff (docs only — no code changes)
**Date:** 2026-05-05
**Purpose:** Document the complete deterministic trace from command palette selection
through existing handler chains, proving palette commands produce identical marker
chains to keyboard-triggered actions. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                PASS_K15_ACTION_TRACE                         ║
╠══════════════════════════════════════════════════════════════╣
║ All 5 palette commands route through existing handlers.     ║
║ No new execution paths. No authority drift.                  ║
║ OpenSelectedInQuil chain identical to K8.                    ║
║ Focus/Scene/Atlas chains use existing focus/toggle paths.    ║
║ Docs only — zero code changes.                              ║
╚══════════════════════════════════════════════════════════════╝
```

## Palette Command Table

| Command | Handler Called | Original Shortcut | Route Document |
|---------|---------------|-------------------|----------------|
| OpenSelectedInQuil | `open_linen_object_in_quil()` | PrintScreen (0x59) | K8 chain |
| FocusLinen | `open_linen_in_active_scene()` | F8 (0x42) | D1 frame/tab |
| FocusQuil | `open_quil_in_active_scene()` | F9 (0x43) | E1 frame/tab |
| SceneNext | `switch_scene()` | (deferred binding) | B2 scene switch |
| OpenAtlas | `atlas_toggle()` | F10 (0x44) | C1/C2 atlas |

All five route through the **same** handler functions that keyboard shortcuts call.
Zero divergence.

## Expected Proof Chains

### OpenSelectedInQuil (Success)

```
[command_palette.execute] cmd=0 name="Open in Quil"
[command_palette.selection_visual.header]  (from render before close)

→ palette closes via toggle_command_palette()
→ [command_palette.close]

Inside palette_execute_selected:
  → linen_selected_object_id()
    → [linen.object_select.current] id=N

  → open_linen_object_in_quil(obj_id)
    → [linen.quil.open.request] id=N
    → [linen.quil.open.no_grant] id=N kind=K
    → collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)
      → [collar.gate.check] op=6 object_id=N buffer_id=0
      → [collar.gate.allow_stub] op=6
    → create/reuse buffer
      → [linen.quil.open.dynamic_id] object_id=N dynamic_buffer_id=1000+N
      OR [linen.quil.open.reuse_existing] object_id=N buffer_id=1000+N
    → [linen.quil.buffer.linked] object_id=N buffer_id=1000+N kind=K
    → open_quil_in_active_scene() if not already open
    → mesh_emit_linen_quil_links()
      → [mesh.object_link.start]
      → [mesh.object_link.row] object_id=N kind=K buffer_id=1000+N kind=K surface_id=201
      → [mesh.object_link.done] links=N stale=0
    → bell_emit_object_link_event(object_id, dynamic_buffer_id)
      → [bell.event.stub] kind=ObjectLinkedToBuffer object_id=N buffer_id=1000+N
      → [bell.event.object_link] object_id=N kind=K buffer_id=1000+N kind=K
      → [bell.event.done] reason=emitted
    → quil_render_buffer_list()
      → [quil.buffer_list.render] w=N h=N
      → [quil.buffer_list.row] buffer_id=1000+N kind=K state=Open linen_ref=N surface_id=201 name=X
      → [quil.buffer_list.done] count=N rows=N
    → [linen.quil.done] object_id=N buffer_created=true/false
```

**Chain identical to K8.** All 22 proof markers fire the same way as PrintScreen-triggered.

### OpenSelectedInQuil (Reject — Not Focused)

```
[command_palette.execute] cmd=0 name="Open in Quil"
  → FOCUSED_SURFACE_ID != SURFACE_ID_LINEN
  → [command_palette.reject] cmd=0 reason=not_focused
```

### FocusLinen (Success)

```
[command_palette.execute] cmd=1 name="Focus Linen"
  → open_linen_in_active_scene()
    → [linen.placeholder.reject.duplicate] (if already visible)
    → or frame/tab creation:
      → [linen.placeholder.attach.frame]
      → [linen.placeholder.attach.tab]
    → try_set_focus() via existing focus path
      → [focus.lifecycle.reject] or [focus.ref.commit]
    → tile_active_scene_frames() call chain
      → [tiling.*] markers
```

Identical chain to F8 key toggle.

### FocusQuil (Success)

```
[command_palette.execute] cmd=2 name="Focus Quil"
  → open_quil_in_active_scene()
    → [quil.placeholder.reject.duplicate] (if already visible)
    → or frame/tab creation:
      → [quil.placeholder.attach.frame]
      → [quil.placeholder.attach.tab]
    → try_set_focus() via existing focus path
    → tile_active_scene_frames()
```

Identical chain to F9 key toggle.

### SceneNext (Success)

```
[command_palette.execute] cmd=3 name="Next Scene"
  → switch_scene(next)
    → [scene.switch] idx=N → N+1 (or wrap to 0)
    → scene_update_flags() for current and next scene
    → tile_active_scene_frames()
    → try_set_focus() validation on new active scene
      → [scene.focus.reject.inactive] if focus in wrong scene
```

### OpenAtlas (Success)

```
[command_palette.execute] cmd=4 name="Open Atlas"
  → atlas_toggle()
    → if entering: [atlas.view.enter]
    → [atlas.nav.enter.select]
    → atlas_capture_snapshot()
      → [atlas.snapshot.*] markers
    → if exiting: [atlas.view.exit]
```

Identical chain to F10 key toggle.

## Reject Paths

| Path | Marker | Condition |
|------|--------|-----------|
| Invalid index | (no marker — early return) | `COMMAND_PALETTE_SELECTED` out of range |
| Open not focused | `[command_palette.reject] reason=not_focused` | `FOCUSED_SURFACE_ID != SURFACE_ID_LINEN` |
| No selection | `[linen.quil.open.reject.no_selection]` | `linen_selected_object_id()` returns 0 |
| Collar denies | `[collar.gate.reject]` + `[linen.quil.open.reject.collar]` | decision != AllowStub |
| Buffer collision | `[linen.quil.open.reject.buffer_id_collision]` | dynamic_buffer_id taken by different ref |
| Table full | `[linen.quil.open.reject.full]` | No free QUIL_BUFFERS slot |
| Missing object | `[linen.quil.open.reject.missing]` | object_id not found |

All reject paths produce proof markers. No silent failures.

## Boundary Proof

| Boundary | Status |
|----------|--------|
| No new execution paths | ✅ All 5 commands route through existing handlers |
| No new authority | ✅ Collar gates (J5) still apply to OpenSelectedInQuil |
| No new PDX/ABI | ✅ No new opcodes, no sexdisplay changes, no kernel edits |
| No sexdisplay changes | ✅ Single 0xEF fill rect only |
| No storage/editor/build | ✅ Static command list, no filesystem |
| No app command manifests | ✅ 5 static commands hardcoded |
| No text input/history | ✅ No input, no history, no persistence |

**Boundaries intact.**

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Command palette is proof-marker/list stub | LOW | No visual rows, single header only |
| One-fill-rect visual limitation | MEDIUM | Requires sexdisplay multi-rect (STOP FIRST) |
| Real command manifests require STOP FIRST | MEDIUM | Cross-PD protocol → new PDX opcodes |
| Real Collar grants require STOP FIRST | MEDIUM | grant_ref=0 for all current usage |

K15 introduces no new risks.

## Next Safest Step

**K16: Rapid audit K10-K15** — close the command palette milestone with a comprehensive
audit of palette design, stub, visual, and action trace. Docs only.

After K16: real feature work options:
- **Multi-rect display** (STOP FIRST — sexdisplay change for per-row highlights)
- **Bell event real implementation** (stub → real if Collar/Mesh ready)
- **Text input caveat for command palette** (requires sexdisplay text primitive — STOP FIRST)
