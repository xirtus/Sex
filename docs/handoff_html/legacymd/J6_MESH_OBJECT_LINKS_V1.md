# J6: Mesh Linen/Quil Object Links

**Status:** Handoff (implemented)
**Commit:** *(to be committed)*
**Build:** *(to be verified)*

## 1. Purpose

Expose Linen↔Quil object/buffer links to Mesh as shell-local diagnostic facts.
No live graph UI, no authority changes, no new PDX ops, no renderer changes.

### What J6 IS
- `mesh_emit_linen_quil_links()` — diagnostic helper that scans QUIL_BUFFERS
- Proof markers for each valid link row and stale reference
- Wired into J4 link path (after buffer link) and Mesh open path (when Mesh opens)

### What J6 IS NOT
- Not a live graph renderer
- Not authority enforcement
- Not a new PDX op or sex-pdx ABI change
- Not a sexdisplay change
- Not a filesystem/storage implementation
- Not an editor/parser/compiler/build implementation

## 2. Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +mesh_emit_linen_quil_links() helper, wired into open_linen_object_in_quil() and open_mesh_in_active_scene() |
| `docs/handoff/J6_MESH_OBJECT_LINKS_V1.md` | This document |

## 3. Diagnostic Helper

`mesh_emit_linen_quil_links()` scans the Quil buffer table for entries with
`linen_object_ref != 0` and emits one proof marker per valid link:

```
[mesh.object_link.start]
[mesh.object_link.row] object_id=3 object_kind=CodeFile buffer_id=3 buffer_kind=Code surface_id=201
[mesh.object_link.row] object_id=5 object_kind=BuildArtifact buffer_id=4 buffer_kind=BuildOutput surface_id=0
[mesh.object_link.done] links=2 stale=0
```

For stale references (buffer references a Linen object that no longer exists):

```
[mesh.object_link.reject.missing_object] buffer_id=4 linen_object_ref=99
```

### Link facts emitted (IDs and kind names only)
- `object_id` — Linen object ID
- `object_kind` — human-readable LinenObjectKind name
- `buffer_id` — Quil buffer ID
- `buffer_kind` — human-readable QuilBufferKind name
- `surface_id` — linked surface ID (201 = Quil surface)

No object contents, no file paths, no raw pointers, no authority mutation.

## 4. Wire Points

### open_linen_object_in_quil() — after successful link (step 8)
After emitting `[linen.quil.done]`, calls `mesh_emit_linen_quil_links()` to
report the current state of all Linen↔Quil links.

### open_mesh_in_active_scene() — after [mesh.placeholder.open]
Every time the Mesh placeholder opens, emits fresh diagnostic link facts.
This keeps Mesh-aware observers up to date even if links were established
while Mesh was closed.

## 5. Preserved Roles

| Component | Role | J6 Change |
|-----------|------|-----------|
| Mesh | Visualizes diagnostics only | ✅ Emits link facts as proof markers |
| Collar | Gates authority stubs | ❌ Not involved |
| Bell | Event contract placeholder | ❌ Not involved yet (J7) |
| Linen | Object table | ❌ Read-only access |
| Quil | Buffer table | ❌ Read-only access |
| sexdisplay | Sole framebuffer writer | ❌ Not touched |
| silk-shell | All tables | ✅ Diagnostic helper added |

## 6. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[mesh.object_link.start]` | mesh_emit_linen_quil_links() | Start of link scan |
| `[mesh.object_link.row]` | mesh_emit_linen_quil_links() | Valid Linen↔Quil link found |
| `[mesh.object_link.reject.missing_object]` | mesh_emit_linen_quil_links() | Stale buffer ref — object not found |
| `[mesh.object_link.done]` | mesh_emit_linen_quil_links() | Scan complete with counts |

## 7. Safety Invariants Preserved

1. **Read-only.** Never mutates LINEN_OBJECTS or QUIL_BUFFERS.
2. **No live graph.** Proof markers only — no renderer primitives, no UI.
3. **No authority changes.** No Collar/Bell interaction.
4. **No heap allocation.** Iterator and stack locals only.
5. **Safe degradation.** Empty tables produce `links=0 stale=0` — not a panic.
6. **Additive only.** Existing lifecycle, focus, tiling, atlas, close paths unchanged.
7. **No new dependencies.** Uses existing `linen_object_by_id()`, `linen_object_kind_name()`, `quil_buffer_kind_name()`.

## 8. Forbidden Areas Untouched

- kernel/: untouched
- crates/sex-pdx/: untouched
- servers/sexdisplay/: untouched
- servers/linen/: untouched
- servers/quil/: untouched
- WINDOWS Vec: untouched
- Lifecycle enum: untouched
- Tombstone ring: untouched
- Real Mesh graph renderer: untouched
- Real Collar grant enforcement: untouched
- Real Bell event implementation: untouched

## 9. STOP FIRST Status

**No STOP FIRST triggers hit.**

| Trigger | Status |
|---------|--------|
| Kernel edits | ✅ Not touched |
| sex-pdx ABI/opcode edits | ✅ Not touched |
| sexdisplay changes | ✅ Not touched |
| New PDX ops | ✅ Not added |
| Authority enforcement | ✅ Not touched |
| Secret/key handling | ✅ Not touched |
| Filesystem/storage | ✅ Not touched |
| Editor/parser/compiler/build | ✅ Not touched |
| Cross-PD raw pointers | ✅ Not used |
| Shared-memory/backing-buffer redesign | ✅ Not touched |

## 10. Build Result

*(to be filled after build)*

```sh
./scripts/entrypoint_build.sh
```
