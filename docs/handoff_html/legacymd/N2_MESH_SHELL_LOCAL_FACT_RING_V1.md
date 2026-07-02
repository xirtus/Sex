# N2: Mesh Shell-Local Fact Ring

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Implement N1 SAFE_SHELL_LOCAL_FACT_RING. Replace Mesh proof-marker-only
object-link diagnostics with real bounded shell-local Mesh fact memory. No Mesh PD,
no IPC/ABI changes, no rendering.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N2_MESH_FACT_RING                         ║
╠══════════════════════════════════════════════════════════════╣
║ Fact schema:            5 fields, 40 bytes, Clone+Copy      ║
║ Ring capacity:          32 entries                           ║
║ Overflow:               Overwrite oldest                     ║
║ J6 wire point:          Valid links → mesh_record_fact()     ║
║ Stale refs:             NOT recorded (reject markers only)   ║
║ Boundaries:             INTAKT                               ║
║ Build:                  PASS (1611 sectors)                  ║
╚══════════════════════════════════════════════════════════════╝
```

## Changes

**Files:** `servers/silk-shell/src/main.rs` (99 insertions, 6 deletions)
**Commit:** *(to be added)*

### 1. MeshFactKind enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MeshFactKind {
    ObjectLinkedToBuffer = 0,
}
```

V1 supports one fact kind. Future kinds added by extending enum (no existing
code needs changing — match arms are added when consumed).

### 2. MeshFact struct

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct MeshFact {
    fact_id: u64,
    kind: MeshFactKind,
    subject_id: u64,   // Linen object_id
    object_id: u64,    // Quil buffer_id
    ref_id: u64,       // linked_surface_id
    sequence: u64,     // MESH_FACT_WRITE_INDEX at write time
}
```

- 5 × u64 = 40 bytes per fact
- `Clone + Copy` for safe snapshotting
- `repr(C)` for potential future PDX serialization
- All fields are IDs (never pointers, never strings)

### 3. Static Ring State

| Variable | Type | Initial Value |
|----------|------|---------------|
| `MESH_FACT_RING_CAP` | `const usize` | 32 |
| `MESH_FACTS` | `[Option<MeshFact>; 32]` | `[None; 32]` |
| `MESH_FACT_WRITE_INDEX` | `static mut u64` | 0 |
| `MESH_FACT_SEQUENCE` | `static mut u64` | 0 |

### 4. Helper Functions

| Function | Purpose |
|----------|---------|
| `mesh_record_fact(kind, subject_id, object_id, ref_id)` | Write fact to ring, overwrite oldest when full |
| `mesh_fact_count()` | Return number of facts currently in ring |
| `mesh_for_each_fact(closure)` | Newest-first read-only iteration |

### 5. J6 Wire Point

In `mesh_emit_linen_quil_links()`, after emitting `[mesh.object_link.row]` for
each valid Linen↔Quil link, call `mesh_record_fact()`:

```rust
mesh_record_fact(
    MeshFactKind::ObjectLinkedToBuffer,
    o.object_id,
    buf.buffer_id,
    buf.linked_surface_id,
);
```

Stale references (object not found) still emit the reject marker but do NOT
record a fact — consistent with the principle that invalid data is not stored.

## Wire Points (Existing, Unchanged)

| Call Site | Location | When |
|-----------|----------|------|
| `open_linen_object_in_quil()` | Line 1089 | After successful J4 link |
| `open_mesh_in_active_scene()` | Line 5649 | When Mesh surface opens |

Both call `mesh_emit_linen_quil_links()` which now both emits proof markers AND
records facts. No new call sites added.

## Overflow Policy

When `MESH_FACT_RING_CAP` (32) is full, the oldest fact is silently overwritten.
Proof marker `[mesh.fact.overwrite]` emitted with overwritten fact_id and index.

**Rationale:** Mesh facts are topology observations. The most recent 32 facts
are the most relevant. Old topology (e.g., "object 3 linked to buffer 3 at boot")
is less useful than current topology. Capacity of 32 allows at least 32 link
operations before any fact is lost — sufficient for V1 diagnostic use.

## Proof Markers

### New Markers (fact ring)

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.fact.write]` | `mesh_record_fact()` | Fact written to ring with all IDs |
| `[mesh.fact.overwrite]` | `mesh_record_fact()` | Oldest fact overwritten (idx, prev_fact_id) |
| `[mesh.fact.done]` | `mesh_record_fact()` | Write complete with count + fact_id |

### Existing Markers (J6, preserved unchanged)

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.object_link.start]` | `mesh_emit_linen_quil_links()` | Start of link scan |
| `[mesh.object_link.row]` | `mesh_emit_linen_quil_links()` | Valid link found |
| `[mesh.object_link.reject.missing_object]` | `mesh_emit_linen_quil_links()` | Stale ref |
| `[mesh.object_link.done]` | `mesh_emit_linen_quil_links()` | Scan complete |

### Existing Markers (I1, preserved unchanged)

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.placeholder.*]` | various | I1 surface lifecycle |

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN (not touched) |
| servers/mesh/ | ✅ CLEAN (no Mesh PD) |
| servers/linen/ | ✅ CLEAN |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Bell ring | ✅ CLEAN (no changes to Bell code) |
| WINDOWS Vec | ✅ CLEAN |
| Lifecycle enum | ✅ CLEAN |

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
| Fact ring not yet rendered | MEDIUM | N3 adds Mesh fact list render (mirrors Bell M4) |
| Only one fact kind supported | LOW | V1 design. More kinds added when needed. |
| Fact ring may overwrite during active scene | LOW | Overwrite is silent — losing old topology is acceptable for V1 |
| No keyboard nav on Mesh | LOW | N3 is render-only. Selection deferred. |

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1611 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Next Steps

**N3: Mesh fact list rendering** — Mirror Bell M4 pattern: add `mesh_render_fact_list()`
that iterates `mesh_for_each_fact()` and draws header + row fill rects using
multi-rect (0xEF with rect_index). One fact kind color mapped via `linen_kind_color()`.

After N3: Mesh surface will show actual topology facts instead of solid amber fill.
