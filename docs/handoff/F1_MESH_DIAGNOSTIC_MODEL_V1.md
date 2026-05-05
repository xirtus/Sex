# F1: Mesh Diagnostic Model — Spec

**Status:** Approved (Docs/Model only)
**Commit:** *(pending)*
**Build:** N/A (no code changes)

## 1. Purpose

Mesh is SexOS's **authority and runtime topology diagnostic view**. It visualizes
what the system is and how it connects — PDs, PDX routes, capability slots,
MPK/PKEY domains, surfaces, Scene/Frame/Tab ownership, devices, and lifecycle
state. Mesh is **diagnostic and explanatory first**, not authority-granting.
Collar (F2) later controls authority; Mesh visualizes authority.

### Key constraints

- **Mesh reads/visualizes only.** Never grants or revokes authority in F1.
- **Mesh never writes the framebuffer.** Authority topology is a data model,
  rendered by silk-shell or a dedicated PD through existing 0xEC/0xEF/0xEE primitives.
- **Mesh never bypasses PDX/MPK.** No raw cross-PD pointers, no shared memory
  for graph data.
- **Mesh degrades safely** if a data source is missing or a PD is unreachable.
- **No kernel/ABI changes** in F1.

## 2. Node Types

Each node in the Mesh graph represents a single discoverable entity. Nodes are
identified by shell-local stable IDs (surface_id, frame_id, scene index, slot
number) — never raw pointers or kernel object handles.

### Core infrastructure

| Node Type | Identifier | Source | Notes |
|-----------|-----------|--------|-------|
| `kernel` | `KERNEL` | Hardcoded | Always present. Root of authority. |
| `pd` | PD slot number | `sex_pdx::SLOT_*` constants | Each registered PD: sexdisplay, silk-shell, silkbar, linen, quil, sexstore, sexusb, etc. |
| `pdx_endpoint` | Slot number | PDX registry | A PDX slot that a PD can invoke. |
| `cap_slot` | Slot number | Capability grant/check | Capability slot on a PD. |
| `pkey_domain` | PKEY index (0..15) | MPK/PKU allocation | MPK-protected memory region. |

### Shell-owned objects

| Node Type | Identifier | Source | Notes |
|-----------|-----------|--------|-------|
| `surface` | `surface_id` (u64) | silk-shell surface table | All registered surfaces (app, panel, cursor, overlay). |
| `scene` | scene index (0..4) | `SCENES[]` / `ATLAS_SNAPSHOT` | B1 scene model. Contains frames. |
| `frame` | `frame_id` (u32) | `FRAMES[]` | B1 ShellFrame. Contains tabs. Belongs to one scene. |
| `tab` | `(frame_id, tab_index)` | `ShellFrame.tabs[]` | B1 ShellTab. Contains one surface_id. |

### Application placeholders

| Node Type | Identifier | Source | Notes |
|-----------|-----------|--------|-------|
| `linen_object` | `SURFACE_ID_LINEN` (200) | D1 | Placeholder; real Linen object model deferred. |
| `quil_placeholder` | `SURFACE_ID_QUIL` (201) | E1 | Placeholder; real Quil editor deferred. |

### Lifecycle and events

| Node Type | Identifier | Source | Notes |
|-----------|-----------|--------|-------|
| `lifecycle_state` | `(surface_id, generation)` | A3 LIFECYCLE_TABLE | Current lifecycle state + generation. |
| `tombstone_event` | Ring index | A6 TOMBSTONE_RING | Recorded lifecycle termination event. |
| `fault_event` | PD slot + type | Future | Crashed/faulted PD record. Not yet implemented. |

## 3. Edge Types

Edges represent relationships between nodes. All edges are directional unless
noted as bidirectional. Edge presence is derived from shell-local state, never
from cross-PD probing.

### Authority edges

| Edge | Direction | Semantics | Source |
|------|-----------|-----------|--------|
| `PDX_CALL_ALLOWED` | PD → Slot | PD may invoke this slot | Capability grant (kernel-side) |
| `PDX_CALL_DENIED` | PD → Slot | PD attempted invocation, denied | Capability check failure log |
| `CAP_GRANT` | Kernel → PD | Kernel granted capability to PD | Boot/init capability setup |
| `CAP_REVOKED` | Kernel → PD | Capability was revoked | Future (not yet implemented) |
| `MPK_DOMAIN` | Bidirectional | PD belongs to PKEY domain | PKU/PKEY allocation docs |

### Ownership edges (shell-local)

| Edge | Direction | Semantics | Source |
|------|-----------|-----------|--------|
| `SURFACE_OWNED_BY` | Surface → PD | PD owns/renders this surface | Surface ID range convention |
| `TAB_CONTAINS_SURFACE` | Tab → Surface | Tab wraps this surface_id | ShellFrame.tabs[].surface_id |
| `FRAME_CONTAINS_TAB` | Frame → Tab | Frame contains this tab | FRAMES[].tabs[] |
| `SCENE_CONTAINS_FRAME` | Scene → Frame | Scene contains this frame | FRAMES[].scene_id matching |
| `INPUT_FOCUS_ROUTE` | FocusRef → Surface | Current focus targets this surface | FOCUSED_SURFACE / FocusRef |
| `RENDER_ROUTE` | Shell → sexdisplay | Shell sends display ops for surface | PDX slot SLOT_DISPLAY |

### Event edges

| Edge | Direction | Semantics | Source |
|------|-----------|-----------|--------|
| `LIFECYCLE_TRANSITION` | Surface → LifecycleState | Surface's current lifecycle state | A3 LIFECYCLE_TABLE |
| `TOMBSTONE_EVENT` | Surface → TombstoneEvent | Surface had tombstone recorded | A6 TOMBSTONE_RING |
| `FAULT_EVENT` | PD → FaultEvent | PD faulted/crashed | Future |

## 4. Data Sources (Available Now)

All data for F1 Mesh comes from existing shell-local structures. No new
kernel/ABI probes required.

| Source | Data Provided | Location |
|--------|--------------|----------|
| A3 lifecycle table | Per-surface lifecycle state + generation | `LIFECYCLE_TABLE: [Option<(u64, SurfaceLifecycle)>; 32]` |
| A6 tombstone ring | Recent tombstone events with full context | `TOMBSTONE_RING: [Option<TombstoneEvent>; 16]` |
| B1 scene model | Scene flags, labels, frame/tab membership | `SCENES[5]`, `FRAMES[4]`, each frame has `tabs[8]` |
| B2 focus guard | Active scene + focus validation state | `ACTIVE_SCENE_IDX`, `FOCUSED_SURFACE` (FocusRef) |
| B3 tiling | Frame geometry, visible frame set | `tile_active_scene_frames()` output |
| C1 Atlas snapshot | Derived scene snapshot with filtering | `atlas_capture_snapshot()` → `ATLAS_SNAPSHOT` |
| D1/D2 Linen | Linen frame/tab lifecycle proof markers | `LINEN_FRAME_ID=2`, lifecycle state |
| E1/E2 Quil | Quil frame/tab lifecycle proof markers | `QUIL_FRAME_ID=3`, lifecycle state |
| PDX slot constants | Known PD slots and their purposes | `sex_pdx::SLOT_*` constants |
| Surface ID range | Surface owner convention | `SURFACE_ID_*` constants (0x90-0x97 panels, 100-103 apps, 200 linen, 201 quil) |

## 5. Invariants

1. **Read-only.** Mesh reads/visualizes existing shell state. Never grants or
   revokes authority.
2. **No framebuffer ownership.** Mesh never writes to the display. Rendering is
   done by silk-shell or a dedicated PD through existing 0xEC/0xEF/0xEE primitives.
3. **No PDX/MPK bypass.** Mesh must not store raw cross-PD pointers, kernel
   object handles, or shared-memory references.
4. **Safe degradation.** If a data source is missing (e.g., tombstone ring empty,
   frame slot None), Mesh skips that node/edge rather than faulting.
5. **Shell-local only.** All graph data is derived from silk-shell's own
   structures. No cross-PD probing, no kernel object enumeration in F1.
6. **No kernel/ABI changes.** F1 uses only existing `sex_pdx` constants and
   shell-internal data.

## 6. Future Implementation Plan

### F1 (this document)
- ✅ Docs/model definition
- No implementation

### F2 — Collar Authority Map (docs/model)
- Define Collar as authority control layer (separate from Mesh diagnostic)
- Collar later reads Mesh state to make authority decisions
- Docs only in F2

### F3 — Mesh Snapshot Provider (implementation)
- Add Mesh snapshot collection to silk-shell or a dedicated Mesh PD
- Copy shell-local graph state into a snapshot structure
- `mesh_capture_snapshot()` — similar to `atlas_capture_snapshot()`
- Emit `[mesh.snapshot.start]` / `[mesh.snapshot.node]` / `[mesh.snapshot.edge]`
- **Requires STOP FIRST review** before any implementation

### F4 — Mesh Placeholder Surface (implementation)
- Mesh surface through proven Scene/Frame/Tab path (mirror D1/E1)
- Toggle via key binding
- No live graph rendering yet — placeholder fill rect only
- **Requires STOP FIRST review**

### F5 — Live Graph Rendering (implementation)
- Render authority topology using existing 0xEC/0xEF/0xEE primitives
- No new display protocol, no renderer changes
- Node positions determined by shell layout logic (not force-directed in V1)
- **Requires STOP FIRST review**

## 7. Proof Gates (Future Implementation)

For any F3+ implementation commit:

1. **Build:** ISO produced cleanly
2. **Boot:** All PDs reachable, no #PF/#GP/null jump/panic
3. **Graph safety:** Mesh graph contains only shell-local copied IDs/state — no
   raw pointers, no kernel object references
4. **No new ABI:** `git diff -- crates/sex-pdx/` must be empty unless STOP FIRST
5. **No renderer policy:** Mesh never owns display policy — silk-shell or
   dedicated PD renders under shell authority
6. **Safe degradation:** Mesh snapshot handles missing data gracefully
7. **Proof markers:** `[mesh.*]` markers at all capture, node, and edge points

## 8. STOP FIRST Triggers

Stop all Mesh work and escalate if any of the following are required:

- **Kernel edits** — Mesh must never require kernel changes
- **`crates/sex-pdx/` ABI/opcode edits** — Mesh uses existing constants only
- **Raw pointer sharing** — Mesh data is IDs and enums, never raw pointers
- **Mesh grants or revokes authority** — Collar (F2+) owns authority decisions
- **Renderer-owned graph policy** — sexdisplay never decides graph layout
- **Shared-memory/backing-buffer redesign** — Mesh uses existing PDX/display path
- **Filesystem/network/device live access** — Mesh reads shell state only in F1

## 9. Edge Encoding Reference

For future implementation, edges can be encoded as a simple enum:

```rust
#[repr(u8)]
enum MeshEdgeKind {
    PdxCallAllowed = 0,
    PdxCallDenied = 1,
    CapGrant = 2,
    CapRevoked = 3,
    MpkDomain = 4,
    SurfaceOwnedBy = 5,
    TabContainsSurface = 6,
    FrameContainsTab = 7,
    SceneContainsFrame = 8,
    InputFocusRoute = 9,
    RenderRoute = 10,
    LifecycleTransition = 11,
    TombstoneEvent = 12,
    FaultEvent = 13,
}
```

## 10. Node Encoding Reference

For future implementation, nodes can be encoded as a tagged union:

```rust
#[repr(u8)]
enum MeshNodeKind {
    Kernel = 0,
    Pd = 1,
    PdxEndpoint = 2,
    CapSlot = 3,
    PkeyDomain = 4,
    Surface = 5,
    Scene = 6,
    Frame = 7,
    Tab = 8,
    LinenObject = 9,
    QuilPlaceholder = 10,
    Device = 11,
    LifecycleState = 12,
    TombstoneEvent = 13,
    FaultEvent = 14,
}
```
