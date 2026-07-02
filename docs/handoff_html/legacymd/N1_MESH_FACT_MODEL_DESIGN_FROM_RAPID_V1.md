# N1: Mesh Fact Model Design from Rapid

**Status:** Design/Handoff (docs only — no code changes)
**Date:** 2026-05-05
**Purpose:** Design the smallest safe next Mesh implementation step using the
proven Bell ring pattern (M2) as a template. Avoid Bell overlap. Docs only.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               SAFE_SHELL_LOCAL_FACT_RING                     ║
╠══════════════════════════════════════════════════════════════╣
║ Verdict: Safe to implement shell-local Mesh fact ring.      ║
║ Follows proven Bell ring pattern (M2). No new PDX, ABI,     ║
║ kernel, or Collar changes. No Mesh PD creation.              ║
╚══════════════════════════════════════════════════════════════╝
```

## Mesh Docs Found in /rapid/

| Doc | Content |
|-----|---------|
| `PHASE_06_MESH_CAPABILITY_GRAPH.md` | Full Mesh+Collar vision: temporal graph, causal edges, pattern bounds, capability borrow-checker, Mesh query language. Extensive but visionary. |
| `F1_MESH_DIAGNOSTIC_MODEL_V1.md` | Canonical node/edge type definitions (15 node types, 14 edge types), invariants, proof gates, STOP FIRST triggers. |
| `I1_MESH_PLACEHOLDER_SURFACE_V1.md` | Mesh placeholder surface through Scene/Frame/Tab (Surface ID 202, Frame ID 4, F12 toggle). |
| `J6_MESH_OBJECT_LINKS_V1.md` | `mesh_emit_linen_quil_links()` — diagnostic proof-marker emission for Linen↔Quil links. |

## Current Mesh State

### What exists (implemented in silk-shell)

| Component | Status | Location |
|-----------|--------|----------|
| Mesh placeholder surface | ✅ I1 complete | Surface ID 202, Frame ID 4, F12 toggle |
| Frame/tab lifecycle | ✅ I1 | `ensure_mesh_frame()`, `open_mesh_in_active_scene()`, `toggle_mesh()`, `focus_or_open_mesh()` |
| Visual placeholder | ✅ I1 | Single `0xEF` fill rect with `MESH_PLACEHOLDER_COLOR = 0x00383010` (amber) |
| Object-link diagnostics | ✅ J6 | `mesh_emit_linen_quil_links()` — proof markers only, scans QUIL_BUFFERS |
| Wire points for J6 | ✅ J6 | Called from `open_linen_object_in_quil()` and `open_mesh_in_active_scene()` |
| Mesh PD (separate server) | ❌ NOT EXISTS | No `servers/mesh/` directory, no Mesh PD, no PDX opcodes |
| Fact ring/table storage | ❌ NOT EXISTS | Diagnostics are proof-marker-only, no stored facts |
| Live graph rendering | ❌ NOT EXISTS | No fact list render, no topology render, no row visuals |
| Collar/Mesh interaction | ❌ NOT EXISTS | No Collar authority interaction |
| Kernel integration | ❌ NOT EXISTS | No kernel PD spawn notification for Mesh |

### What the rapid doc envisions (long-term)

`PHASE_06_MESH_CAPABILITY_GRAPH.md` describes Mesh as a separate PD server with:
- Temporal graph store (4096 event ring buffer)
- Causal edge provenance (`reason: FixedStr<128>`)
- Live graph queries (`OP_MESH_QUERY` → `GraphSnapshot`)
- Pattern bounds engine (rolling-window counters per edge type)
- Kernel PD spawn → Mesh node registration

**None of this is implemented.** The gap between current state (placeholder + proof markers) and the rapid vision (full Mesh PD) is intentionally large. N1 bridges this gap with the smallest possible fact storage.

## Recommended Implementation Path

**Path A: SAFE_SHELL_LOCAL_FACT_RING** ✅ (selected)

A shell-local static ring buffer of Mesh facts, mirroring the proven Bell ring
pattern (M2). This is the smallest safe step because:

1. **Proven pattern** — Bell ring (M2) already demonstrates static ring buffers
   in silk-shell: `[Option<BellEvent>; 16]`, `bell_record_event()`, `bell_for_each_event()`
2. **No new dependencies** — Uses only existing `linen_object_by_id()`, `quil_buffer_by_id()`,
   and other shell-local helpers
3. **No ABI/kernel changes** — All state is `static mut` in silk-shell
4. **Gradual migration path** — Later, a real Mesh PD can absorb the fact ring
   via PDX without changing the fact schema
5. **Rendering ready** — Once facts are stored, `mesh_render_fact_list()` can
   display them using the same multi-rect pattern as Linen/Quil/Bell
6. **No Bell overlap** — Mesh facts are topology/relationship data, not event
   history. Bell owns attention/notification events. Mesh owns facts.

### Alternatives Considered

| Path | Verdict | Reason |
|------|---------|--------|
| **B: SAFE_EXISTING_MESH_PD** | ❌ REJECTED | No Mesh PD exists. Creating one is STOP FIRST (new PDX server, new PDX ops, slot allocation). |
| **C: BLOCKED_ABI_REQUIRED** | ❌ REJECTED | No ABI changes needed for a shell-local fact ring. |
| **D: BLOCKED_AUTHORITY_REQUIRED** | ❌ REJECTED | No authority changes needed. Mesh is read-only fact storage. |

## Minimal Fact Schema

```rust
/// Kinds of Mesh facts that can be stored in the shell-local fact ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MeshFactKind {
    /// A Linen object was linked to a Quil buffer.
    ObjectLinkedToBuffer = 0,
    /// Future: A surface was created and registered.
    // SurfacePresent = 1,
    /// Future: A PD is known to the system.
    // PdKnown = 2,
}

/// A single Mesh fact record stored in the shell-local ring buffer.
/// Fixed-size scalars only. No pointers, no strings, no heap.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct MeshFact {
    /// Monotonic fact ID (incremented per stored fact).
    fact_id: u64,
    /// Fact kind (V1: only ObjectLinkedToBuffer).
    kind: MeshFactKind,
    /// Primary subject ID (object_id for ObjectLinkedToBuffer).
    subject_id: u64,
    /// Primary object ID (buffer_id for ObjectLinkedToBuffer).
    object_id: u64,
    /// Secondary reference (linked_surface_id for ObjectLinkedToBuffer).
    ref_id: u64,
    /// Monotonic counter for ordering.
    sequence: u64,
}
```

**Design rationale:**
- 5 × u64 = 40 bytes per fact (same ballpark as BellEvent)
- `subject_id` = Linen object_id
- `object_id` = Quil buffer_id
- `ref_id` = linked_surface_id (e.g., SURFACE_ID_QUIL)
- `Clone + Copy` for safe snapshotting
- `repr(C)` for potential future PDX serialization

## Ring/Table Design

```rust
/// Capacity of the Mesh fact ring buffer (power of 2 for efficient modulo).
const MESH_FACT_RING_CAP: usize = 32;

/// Ring buffer of Mesh facts. Newest overwrites oldest when full.
static mut MESH_FACTS: [Option<MeshFact>; MESH_FACT_RING_CAP] = [None; MESH_FACT_RING_CAP];

/// Next write index (monotonic, wraps via modulo).
static mut MESH_FACT_WRITE_INDEX: u64 = 0;

/// Global fact sequence counter.
static mut MESH_FACT_SEQUENCE: u64 = 0;
```

**Ring behavior (mirrors Bell M2):**
- Static array, no heap
- Overwrite oldest when full
- Monotonic write index for ordering
- Sequence counter for global ordering
- `Clone + Copy` facts for safe snapshotting

### vs Bell Ring

| Aspect | Bell (M2) | Mesh (N1/N2) |
|--------|-----------|--------------|
| Ring capacity | 16 | 32 (facts accumulate slower) |
| Record trigger | User action (link) | System topology change |
| Consumer | Bell surface rows | Mesh surface rows + Collar queries (future) |
| Overflow | Overwrite oldest | Overwrite oldest |
| Fact kind | 1 (ObjectLinkedToBuffer) | 1 (ObjectLinkedToBuffer) |
| Ownership | Attention/notification memory | Topology/relationship memory |

## Overflow Policy

When `MESH_FACT_RING_CAP` is full, the oldest fact is silently overwritten.
Proof marker `[mesh.fact.overwrite]` emitted with overwritten fact_id and index.

**Rationale:** Mesh facts are topology observations. The most recent N facts
are the most relevant. Old topology (e.g., "object 3 linked to buffer 3 at boot")
is less useful than current topology. If a real Mesh PD later needs complete
history, it can maintain its own larger ring.

## Ownership Split

| Component | Owns | Does NOT own |
|-----------|------|-------------|
| **Mesh** (fact ring) | Topology facts, relationship records | Authority, events, rendering |
| **Collar** (future) | Authority grants, capability checks | Topology, events, rendering |
| **Bell** (ring) | Attention events, notifications | Topology, authority |
| **silk-shell** | Render dispatch, frame/tab lifecycle | Fact semantics, authority |
| **sexdisplay** | Framebuffer compositing | Fact meaning, topology policy |

### Who writes facts

| Emission Point | Fact Kind | When |
|---------------|-----------|------|
| `open_linen_object_in_quil()` | ObjectLinkedToBuffer | After successful J4 link |
| `mesh_emit_linen_quil_links()` | ObjectLinkedToBuffer | On Mesh surface open (re-play existing links) |
| Future: scene/frame creation | SurfacePresent | When frame/tab created |
| Future: PD registration | PdKnown | When shell learns of a PD |

### Who reads facts

| Consumer | Read Pattern | When |
|----------|-------------|------|
| `mesh_render_fact_list()` (future N2) | Newest-first iteration | On Mesh open, after fact write |
| Bell (future) | Fact reference for event context | When Bell wants topology context |
| Collar (future) | Fact query for authority decisions | When Collar needs topology evidence |

## Proof Markers (for N2 implementation)

| Marker | Type | Description |
|--------|------|-------------|
| `[mesh.fact.record]` | Write | Fact written to ring (fact_id, kind, subject_id, object_id) |
| `[mesh.fact.overwrite]` | Overflow | Oldest fact overwritten (index, prev_fact_id) |
| `[mesh.fact.reject]` | Reject | Fact rejected (reason, e.g., duplicate/empty subject) |
| `[mesh.fact.start]` | Read | Fact list render start |
| `[mesh.fact.row]` | Read | Single fact row emitted |
| `[mesh.fact.skip]` | Read | Fact skipped (max_rows) |
| `[mesh.fact.done]` | Read | Fact list render complete |
| `[mesh.fact_visual.rect]` | Visual | Fill rect sent per fact row |
| `[mesh.fact_visual.skip]` | Visual | Fill rect skipped (rect budget) |
| `[mesh.fact_visual.current]` | State | Current fact count + state |

## STOP FIRST Table

| Trigger | Status | Notes |
|---------|--------|-------|
| New PDX opcodes | ✅ NOT TRIGGERED | Shell-local ring only |
| New sex-pdx ABI constants | ✅ NOT TRIGGERED | No new ABI constants |
| Real capability grants/revokes | ✅ NOT TRIGGERED | Read-only fact storage |
| Cross-PD pointers | ✅ NOT TRIGGERED | Static array, no pointers |
| Live kernel introspection | ✅ NOT TRIGGERED | Shell-local tables only |
| Persistent graph storage | ✅ NOT TRIGGERED | In-memory only |
| Renderer-owned topology policy | ✅ NOT TRIGGERED | Shell renders via 0xEF |
| Bell/Collar behavior changes | ✅ NOT TRIGGERED | Bell ring untouched |
| Mesh PD creation | ✅ NOT TRIGGERED | Shell-local only |
| Kernel edits | ✅ NOT TRIGGERED | None needed |

## N2 Implementation Prompt

If approved, N2 implements:

**Changes to `servers/silk-shell/src/main.rs`:**
1. `MeshFactKind` enum (1 variant: `ObjectLinkedToBuffer`)
2. `MeshFact` struct (5 u64 fields, `Clone + Copy + repr(C)`)
3. `MESH_FACT_RING_CAP = 32` constant
4. `MESH_FACTS: [Option<MeshFact>; 32]` static ring
5. `MESH_FACT_WRITE_INDEX: u64` static
6. `MESH_FACT_SEQUENCE: u64` static
7. `mesh_record_fact(kind, subject_id, object_id, ref_id)` — writes to ring
8. `mesh_fact_count()` — returns current count
9. `mesh_for_each_fact()` — newest-first iterator (mirrors `bell_for_each_event()`)
10. Wire `mesh_record_fact()` into `mesh_emit_linen_quil_links()` (replace proof-marker-only with ring storage)
11. Wire `mesh_record_fact()` into `open_linen_object_in_quil()` (after successful link, alongside existing bell_record_event)

**No changes to:**
- kernel/, sex-pdx/, sexdisplay/, bell/, collar/
- Bell ring, event schema, or selection
- Linen/Quil table structure
- WINDOWS Vec or lifecycle enum
- No new keyboard commands
- No fact list rendering (N3)

**DO NOT:**
- Create Mesh PD
- Add PDX opcodes
- Add ABI constants
- Touch Bell/Collar code
- Render fact rows (deferred to N3)
- Add Enter action on Mesh (deferred)
- Implement live graph queries

**Build/commit:** Single commit with handoff doc

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| V1 | 2026-05-05 | Claude | Initial design from rapid docs + existing Mesh state |
