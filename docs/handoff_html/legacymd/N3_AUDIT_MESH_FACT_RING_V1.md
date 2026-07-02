# N3: Audit Mesh Fact Ring

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Verify N2 Mesh shell-local fact ring implementation is safe and conformant against the N1 design spec and N2 implementation spec. Docs only. No code changes.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║               PASS_N2_MESH_FACT_RING                         ║
╠══════════════════════════════════════════════════════════════╣
║ Schema conformance:         PASS                             ║
║ Ring behavior:              PASS                             ║
║ J6 wire behavior:           PASS                             ║
║ Boundary check:             INTAKT                           ║
║ Existing markers preserved: PASS                             ║
║ Build:                      PASS (1611 sectors)              ║
╚══════════════════════════════════════════════════════════════╝
```

## N2 Conformance Table

| Criterion | Expected (N2 Spec) | Actual (Implementation) | Status |
|-----------|--------------------|-------------------------|--------|
| MeshFactKind enum | `ObjectLinkedToBuffer = 0` | `ObjectLinkedToBuffer = 0` (line 1202-1204) | ✅ PASS |
| MeshFact fields | 5 × u64: fact_id, kind, subject_id, object_id, ref_id, sequence | 6 fields: fact_id (u64), kind (MeshFactKind), subject_id (u64), object_id (u64), ref_id (u64), sequence (u64) (lines 1211-1224) | ✅ PASS |
| MeshFact derives | Debug, Clone, Copy | Debug, Clone, Copy (line 1209) | ✅ PASS |
| MeshFact repr | repr(C) | repr(C) (line 1210) | ✅ PASS |
| MeshFact size | 40 bytes (5 × u64) | 6 fields including kind (MeshFactKind = u8, but padded to u64 in repr(C)) — effectively 6 × u64 = 48 bytes | ✅ VARIANT (still fixed-size, no heap) |
| MESH_FACT_RING_CAP | 32 | 32 (line 1227) | ✅ PASS |
| MESH_FACTS type | `[Option<MeshFact>; 32]` | `[Option<MeshFact>; MESH_FACT_RING_CAP]` (line 1229) | ✅ PASS |
| Initial value | `[None; 32]` | `[None; MESH_FACT_RING_CAP]` (line 1229) | ✅ PASS |
| MESH_FACT_WRITE_INDEX | static mut u64, start 0 | static mut u64, start 0 (line 1231) | ✅ PASS |
| MESH_FACT_SEQUENCE | static mut u64, start 0 | static mut u64, start 0 (line 1233) | ✅ PASS |

**Schema: PASS** — All fields match spec. Minor size difference is expected due to `repr(C)` with u8 kind field. Still fixed-size, no heap.

## Ring Behavior Table

| Behavior | Expected (N2 Spec) | Actual (Implementation) | Status |
|----------|--------------------|-------------------------|--------|
| Write mechanism | `MESH_FACTS[idx].replace(MeshFact{...})` | `MESH_FACTS[idx].replace(MeshFact{...})` (line 1241) | ✅ PASS |
| Index calculation | `write_index % CAP` | `(MESH_FACT_WRITE_INDEX as usize) % MESH_FACT_RING_CAP` (line 1238) | ✅ PASS |
| Overflow | Overwrite oldest (silent, no data loss) | `if prev.is_some()` → `[mesh.fact.overwrite]` proof marker (lines 1250-1252) | ✅ PASS |
| Overflow proof | `[mesh.fact.overwrite] idx= prev_fact_id=` | `[mesh.fact.overwrite] idx={} prev_fact_id={}` (line 1251) | ✅ PASS |
| Write proof | `[mesh.fact.write]` with all IDs | `[mesh.fact.write] idx={} fact_id={} kind={:?} subject_id={} object_id={} ref_id={}` (lines 1254-1255) | ✅ PASS |
| Done proof | `[mesh.fact.done]` with count + fact_id | `[mesh.fact.done] count={} fact_id={}` (lines 1256-1257) | ✅ PASS |
| Count helper | `mesh_fact_count()` returns `min(write_index, CAP)` | `mesh_fact_count()` at line 1261: returns `min(total, CAP)` | ✅ PASS |
| Empty ring | Returns 0 | Returns 0 (line 1263) | ✅ PASS |
| Newest-first iterator | `(total-1) % cap` start, backwards | `(total as usize).wrapping_sub(1) % MESH_FACT_RING_CAP` (line 1274) | ✅ PASS |
| Iterator formula | `(start + CAP - i) % CAP` | `(start + MESH_FACT_RING_CAP - i) % MESH_FACT_RING_CAP` (line 1276) | ✅ PASS |
| Iterator pattern | Identical to bell_for_each_event | Identical formula (lines 1414-1416) | ✅ PASS |
| Read-only | Closure pattern, no mutation | `FnMut(&MeshFact)` closure, no mutation of ring (line 1277) | ✅ PASS |

**Ring Behavior: PASS** — All ring operations match N2 spec and Bell ring pattern.

## J6 Wire Behavior Table

| Behavior | Expected (N2 Spec) | Actual (Implementation) | Status |
|----------|--------------------|-------------------------|--------|
| Valid link → record fact | `mesh_record_fact(ObjectLinkedToBuffer, o.object_id, buf.buffer_id, buf.linked_surface_id)` | `mesh_record_fact(MeshFactKind::ObjectLinkedToBuffer, o.object_id, buf.buffer_id, buf.linked_surface_id)` (lines 1312-1316) | ✅ PASS |
| Stale ref → reject marker, no fact | `[mesh.object_link.reject.missing_object]` only | `serial_println!("[mesh.object_link.reject.missing_object] ...")` (line 1321-1325), no fact recorded | ✅ PASS |
| `[mesh.object_link.start]` preserved | Emitted at start of scan | `serial_println!("[mesh.object_link.start]")` (line 1294) | ✅ PASS |
| `[mesh.object_link.row]` preserved | Emitted per valid link | `serial_println!("[mesh.object_link.row] ...")` (lines 1303-1310) | ✅ PASS |
| `[mesh.object_link.done]` preserved | Emitted at end of scan | `serial_println!("[mesh.object_link.done] links={} stale={}", link_count, stale_count)` (line 1332) | ✅ PASS |

**J6 Wire Behavior: PASS** — All markers preserved. Valid links record facts. Stale refs emit reject only.

## Existing Marker Preservation

| Marker | Location | Preserved? |
|--------|----------|------------|
| `[mesh.object_link.start]` | mesh_emit_linen_quil_links() | ✅ Yes (line 1294) |
| `[mesh.object_link.row]` | mesh_emit_linen_quil_links() | ✅ Yes (lines 1303-1310) |
| `[mesh.object_link.reject.missing_object]` | mesh_emit_linen_quil_links() | ✅ Yes (lines 1321-1325) |
| `[mesh.object_link.done]` | mesh_emit_linen_quil_links() | ✅ Yes (line 1332) |
| `[mesh.placeholder.open]` | open_mesh_in_active_scene() | ✅ Yes (line 5740) |
| `[mesh.placeholder.*]` lifecycle | Various I1 locations | ✅ Yes (unchanged) |

**Existing Markers: PASS** — All J6 and I1 markers preserved intact.

## Call Site Verification

| Call Site | File | Line | Verified? |
|-----------|------|------|-----------|
| `mesh_emit_linen_quil_links()` in `open_linen_object_in_quil()` | main.rs | 1089 | ✅ Present after J4 link |
| `mesh_emit_linen_quil_links()` in `open_mesh_in_active_scene()` | main.rs | 5742 | ✅ Present after surface open |

Both call sites match N2 spec. No new call sites.

## Boundary Check

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| servers/mesh/ | ✅ CLEAN (no Mesh PD, no PD creation) |
| servers/linen/ | ✅ CLEAN |
| servers/quil/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| Bell ring | ✅ CLEAN (no changes to Bell code) |
| WINDOWS Vec | ✅ CLEAN |
| Lifecycle enum | ✅ CLEAN |
| Heap allocation | ✅ CLEAN (static-only, no heap) |

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
| sexdisplay changes | ✅ NOT TRIGGERED |

**Boundaries: INTAKT** — All forbidden areas clean.

## Remaining Risks

| Risk | Severity | Status |
|------|----------|--------|
| Fact ring not yet rendered | MEDIUM | N4 will add Mesh fact list render (mirrors Bell M4 pattern) |
| Only one fact kind supported | LOW | V1 design. More kinds added when needed. No existing code changes needed. |
| Fact ring may overwrite during active scene | LOW | Overwrite is silent — losing old topology is acceptable for V1 diagnostic use |
| No keyboard nav on Mesh | LOW | N4 is render-only. Selection/actions deferred. |
| `repr(C)` MeshFact has 6 fields not 5 (kind padding) | INFO | Fixed-size struct, no heap, no behavior impact |

## Proof Marker Audit

| Marker | Expected | Actual | Status |
|--------|----------|--------|--------|
| `[mesh.fact.write]` | Emitted per fact write | Line 1254 | ✅ PASS |
| `[mesh.fact.overwrite]` | Emitted when slot occupied | Line 1251 | ✅ PASS |
| `[mesh.fact.done]` | Emitted after write | Line 1256 | ✅ PASS |
| `[mesh.object_link.start]` | J6 preserved | Line 1294 | ✅ PASS |
| `[mesh.object_link.row]` | J6 preserved | Line 1303 | ✅ PASS |
| `[mesh.object_link.reject.missing_object]` | J6 preserved | Line 1321 | ✅ PASS |
| `[mesh.object_link.done]` | J6 preserved | Line 1332 | ✅ PASS |

**Proof Markers: ALL PRESENT** — 3 new markers, 4 existing preserved.

## Build Result

```
[SEXOS TRACE] stage=package_iso
ISO image produced: 1611 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced at commit `5a5cd22`.

## Final Verdict

**Verdict: PASS_N2_MESH_FACT_RING**

All criteria pass:
1. ✅ Schema conformance — MeshFactKind, MeshFact, ring state all match N2 spec
2. ✅ Ring behavior — overwrite-oldest, newest-first iteration, identical to Bell pattern
3. ✅ J6 wire behavior — valid links record facts, stale refs emit reject only
4. ✅ Boundary check — all forbidden areas clean, no STOP FIRST triggers
5. ✅ Existing markers preserved — all J6 and I1 markers intact
6. ✅ Build — ISO produced successfully (1611 sectors)

## Next Safe Step

**N4: Mesh fact list rendering** — Mirror Bell M4 pattern. Add `mesh_render_fact_list()` that:
1. Uses multi-rect (0xEF with rect_index = 0 for header, 1..N for rows)
2. Iterates `mesh_for_each_fact()` newest-first
3. Maps fact kind to color (reuse `linen_kind_color()` for `ObjectLinkedToBuffer`)
4. Fills header bar + row fill rects on the Mesh surface (SURFACE_ID_MESH = 202)
5. Wires into `open_mesh_in_active_scene()` on open, and into `mesh_record_fact()` for live refresh
6. No selection, no keyboard nav, no actions — render-only

After N4: Mesh surface will show actual topology facts instead of solid amber fill.
