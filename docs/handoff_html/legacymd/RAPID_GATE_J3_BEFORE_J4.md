# Rapid/8scan Gate: J3 → J4

**Status:** Complete
**Commit:** *(pending)*
**Purpose:** Audit J3 Quil buffer table conformance and gate J4 "open Linen object into Quil buffer."

## 1. Rapid Source Path

```
/home/xirtus_arch/Documents/microkernel/rapid/
```

## 2. Rapid / 8scan Files Used

| File | Role |
|------|------|
| `PHASE_04_LINEN_FILE_OBJECT_BROWSER.md` | Linen object model, Quil consumer role, open dispatch |
| `PHASE_05_QUIL_LANGUAGE_WORKSTATION.md` | Quil buffer model, editor deferral, Linen data source |
| `PHASE_00_BASELINE_PROOF_GATES.md` | Proof markers, STOP FIRST rules |
| `RAPID_DEPLOY_PLAN.md` | Phase ordering, ownership boundaries |
| `docs/handoff/H1_LINEN_OBJECT_MODEL_V1.md` | Linen object model spec |
| `docs/handoff/H2_QUIL_WORKSTATION_MODEL_V1.md` | Quil workstation spec |
| `docs/handoff/J1_LINEN_OBJECT_TABLE_V1.md` | Linen table implementation record |
| `docs/handoff/J3_QUIL_BUFFER_TABLE_V1.md` | Quil buffer table implementation record |

## 3. J3 Conformance Table

| Criterion | Rapid Source | Evidence | Verdict |
|-----------|-------------|----------|---------|
| 8 buffer kind variants ≤ max 16 | PHASE_05 §Object Types (12 types) | ✅ 8 variants, repr(u8) | PASS |
| 16-slot static table | PHASE_05: no heap, static arrays | ✅ QUIL_BUFFERS[16] | PASS |
| 6 seed buffers | PHASE_05 §Smallest First Step: hardcoded | ✅ QUIL_SEED_BUFFERS[6] | PASS |
| No editor implementation | PHASE_05: "Quil is not an IDE" | ✅ Buffer table only | PASS |
| No parser/compiler/build | PHASE_05: deferred | ✅ None | PASS |
| No storage/filesystem | PHASE_05: memory-only | ✅ Static array, no heap | PASS |
| No POSIX paths | PHASE_05: "no POSIX assumptions" | ✅ Buffer IDs only | PASS |
| Proof markers | PHASE_00 | ✅ init/seed/ready | PASS |
| No kernel/ABI/sexdisplay edits | PHASE_04 §Boundaries | ✅ None | PASS |
| No cross-PD new calls | PHASE_05 §Smallest First Step | ✅ silk-shell only | PASS |

## 4. J4 Readiness Table

### What J4 is (per rapid/8scan)

PHASE_04 says:
> **silk-shell (integration):** Linen surface lifecycle, **open-file dispatch**, chrome frame (line 126)
> **Object-type dispatcher:** Click opens object in capability-authorized viewer (source→Quil, image→sexdisplay, notification→Bell) (line 156)
> **Quil (consumer):** opens Document/SourceCode objects from Linen (line 124)

PHASE_05 says:
> **Linen (data source):** project tree, **file open/save**, object graph (line 29)
> **Linen-referenced storage** — buffers reference **Linen object IDs**, not raw files (H2 line 33)

H2 says:
> **H5:** Open Linen object into Quil buffer through proven path (line 189)
> **`open_buffer(obj_ref)`** — Open existing buffer or Linen object into editor view (line 108)

### What J4 MUST be (shell-local only)

J4 is **shell-local ID linking** — no sexfiles, no Collar, no storage, no PDX:

| Operation | Shell-local? | Allowed in J4? |
|-----------|-------------|----------------|
| Find LinenObject by ID | ✅ LINEN_OBJECTS table | ✅ YES |
| Find/Create QuilBuffer | ✅ QUIL_BUFFERS table | ✅ YES |
| Set buffer.linen_object_ref = object_id | ✅ u64 field assignment | ✅ YES |
| Set object.linked_surface_id = SURFACE_ID_QUIL | ✅ u64 field assignment | ✅ YES |
| Open/focus Quil surface | ✅ Existing open_quil_in_active_scene() | ✅ YES |
| Proof markers | ✅ serial_println | ✅ YES |
| Reading file bytes from sexfiles | ❌ Requires PDX call | 🛑 NOT YET |
| Collar grant check | ❌ Requires Collar PD | 🛑 NOT YET |
| Editor cursor/text rendering | ❌ Requires new sexdisplay primitives | 🛑 NOT YET |
| Parser/compiler | ❌ Requires new code | 🛑 NOT YET |
| Storage persistence | ❌ Requires sexstore | 🛑 NOT YET |

### J4 Allowed Operations

| Criterion | Verdict | Boundary |
|-----------|---------|----------|
| Link Linen object ID → Quil buffer ID | ✅ ALLOWED | Shell-local, both tables in silk-shell |
| Open Quil surface when object is "opened" | ✅ ALLOWED | Existing open_quil_in_active_scene() path |
| Emit [quil.linen.ref] proof marker | ✅ ALLOWED | PHASE_00 proof convention |
| Emit [linen.object.open.in.quil] proof marker | ✅ ALLOWED | PHASE_00 proof convention |
| Track open object in LinenObjectState | ✅ ALLOWED | State already exists (Loaded→Modified) |

### J4 Forbidden Operations

| Operation | STOP FIRST? | Reason |
|-----------|-------------|--------|
| Call sexfiles to read object data | 🛑 YES | No sexfiles integration yet |
| Call Collar for grant check | 🛑 YES | No Collar integration yet |
| Implement text cursor/rendering | 🛑 YES | No editor implementation |
| New PDX ops for Quil↔Linen comm | 🛑 YES | No cross-PD communication |
| Filesystem/storage access | 🛑 YES | E track owns storage |
| Kernel/ABI edits | 🛑 YES | Always STOP FIRST |

## 5. Forbidden-Area Check

| Area | J3 Edits | J4 Required Edits | Verdict |
|------|----------|-------------------|---------|
| `kernel/` | None | None needed | ✅ CLEAN |
| `crates/sex-pdx/` | None | None needed | ✅ CLEAN |
| `servers/sexdisplay/` | None | None needed | ✅ CLEAN |
| `servers/linen/` | None | None needed | ✅ CLEAN |
| `servers/quil/` | None | None needed | ✅ CLEAN |
| Storage/filesystem | None | None needed | ✅ CLEAN |
| PDX ABI/opcodes | None | None needed | ✅ CLEAN |
| LifecycleState enum | None | None needed | ✅ CLEAN |
| Tombstone ring | None | None needed | ✅ CLEAN |

## 6. Mismatches / Corrections

| Issue | Phase | Severity | Action |
|-------|-------|----------|--------|
| J3 done before J4 (Quil buffer table before open flow) | General | NOTE | Acceptable — J3 provides the target table that J4 populates. Dependency is J3→J4, not J4→J3. |
| QUIL_SEED_BUFFERS[2] linen_object_ref=2 points to J1 seed object "Compositor Lifecycle Spec" | J3 | NOTE | Forward-looking link. Not a bug — the ref field exists but no runtime follow happens until J4. |
| H2 calls this "H5" but we call it "J4" | Naming | NOTE | J4 = H5 in function. Naming follows session track (J1-J2-J3-J4). |

## 7. Final Verdict

```
PASS_CONTINUE_J4
```

**J3 conformance:** PASS — 8 buffer kinds, 16-slot static table, 6 seed buffers,
no forbidden areas, no editor/parser/compiler/storage implementation.

**J4 readiness:** PASS — J4 is shell-local ID linking between two in-memory
tables (LINEN_OBJECTS and QUIL_BUFFERS). All required data structures exist.
No sexfiles, no Collar, no storage, no PDX calls needed.

**Conditions for J4:**
1. J4 must be shell-local only (silk-shell main.rs)
2. J4 must NOT call sexfiles, Collar, or any PDX server
3. J4 must NOT implement editor features (cursor, text rendering, input handling)
4. J4 must NOT implement parser/compiler/build
5. J4 must NOT touch kernel/ABI/sexdisplay/sex-pdx
6. J4 proof markers: [quil.linen.ref], [linen.object.open.in.quil]

## 8. Exact Next Safest Step

**J4 — Implement a `linen_open_object_in_quil(object_id)` function:**

1. Look up LinenObject by ID (linen_object_by_id)
2. Find or create a QuilBuffer linked to that object
3. Set buffer.linen_object_ref = object_id
4. Set buffer.state = QuilBufferState::Open
5. Set object.linked_surface_id = SURFACE_ID_QUIL
6. Set object.state = LinenObjectState::Loaded (if currently Allocated)
7. Open/focus Quil surface (open_quil_in_active_scene)
8. Emit [quil.linen.ref] and [linen.object.open.in.quil] proof markers
9. Call from keybinding or existing dispatch in open_linen_in_active_scene
10. Build via ./scripts/entrypoint_build.sh
11. Handoff doc: docs/handoff/J4_OPEN_LINEN_INTO_QUIL_V1.md
