# N16: Mesh Milestone Closure

**Status:** Complete
**Date:** 2026-05-06
**Purpose:** Close the Mesh N1-N15 milestone with a compact summary of what Mesh
now does, its safety invariants, input map, proof markers, known risks, and the
exact next pivot. Gives future agents a clean stop point and prevents accidental
Mesh scope creep.

## Final Verdict

```
╔══════════════════════════════════════════════════════════════╗
║           PASS_MESH_N1_N15_MILESTONE                         ║
╠══════════════════════════════════════════════════════════════╣
║ Phases:               15 (N1 design → N15 audit)             ║
║ Commits:              16 (1 design + 7 feat + 8 audit)      ║
║ Files changed:         1 (servers/silk-shell/src/main.rs)     ║
║ Lines added:         ~400                                    ║
║ Boundaries:           INTAKT (no kernel/ABI/sexdisplay)      ║
║ Build:                PASS (1619 sectors)                     ║
╚══════════════════════════════════════════════════════════════╝
```

## What Mesh Now Does

### Shell-Local Mesh Fact Ring

A bounded, shell-local ring of `MeshFact` records that capture topology observations
from Linen↔Quil buffer links:

| Property | Value |
|----------|-------|
| Capacity | 32 entries |
| Fact size | 40 bytes (5 × u64) |
| Overflow | Overwrite oldest |
| Storage | `static mut [Option<MeshFact>; 32]` — no heap |
| Fact kind | `ObjectLinkedToBuffer` (V1 — one kind) |

### Fact Recording

When a Linen→Quil link is established via `open_linen_object_in_quil()`, the J6
chain calls `mesh_emit_linen_quil_links()` which scans all valid Linen↔Quil links
and records a `MeshFact::ObjectLinkedToBuffer` for each:

```
mesh_emit_linen_quil_links()
  → mesh_record_fact(ObjectLinkedToBuffer, object_id, buffer_id, linked_surface_id)
  → [mesh.object_link.start/row/done] (for valid links)
  → [mesh.object_link.reject.missing_object] (for stale refs — no fact recorded)
```

### Fact Rendering

`mesh_render_fact_list()` draws the Mesh surface (SURFACE_ID_MESH = 202) with:

| Element | Color | Source |
|---------|-------|--------|
| Header bar | Amber diagnostic `0x00383010` | `MESH_PLACEHOLDER_COLOR` |
| Row fill rects | Per-kind color via `linen_kind_color()` | 7 row rects (`MESH_LIST_ROW_RECTS`) |
| Selected row | +0x40 per channel highlight | `mesh_selected_row_highlight()` |

Renders on:
1. `open_mesh_in_active_scene()` — when Mesh surface opens (via F12)
2. `mesh_record_fact()` — when a new fact is recorded and Mesh is visible

### Fact Navigation

| Key | Scancode | Action |
|-----|----------|--------|
| J | 0x24 | Select next (newer) fact row — wraps |
| K | 0x25 | Select previous (older) fact row — wraps |
| Enter | 0x1C | Focus Linen surface at selected fact's object |
| PrintScreen | 0x59 | Open selected fact's linked object in Quil |

### Selection State

| Variable | Type | Initial | Range |
|----------|------|---------|-------|
| `MESH_SELECTED_ROW` | `static mut u8` | 0 | 0..visible_fact_count-1 |

Selection is a **visible-row index** (not a fact_id). It is repaired on render:
- Empty ring → clamped to 0
- Ring shrinks below selected → clamped to `visible - 1`

### Actions from Selection

**Enter (N11):** Focus Linen at selected fact — pure navigation. Sets
`SELECTED_LINEN_OBJECT_ID = fact.subject_id` and calls `open_linen_in_active_scene()`.
No Collar gate, no buffer creation, no linking. Only fires after
`mesh_emit_selected_fact_detail_proof()` returns `true` (bool guard).

**PrintScreen (N14):** Open linked object in Quil — calls
`open_linen_object_in_quil(fact.subject_id)`. The Collar gate
(`collar_check_operation_stub(LinkObjectToBuffer)`) is inside the callee, so
Mesh cannot bypass Collar. All J4/J5/J6/J7/K3 side effects fire inside the callee.

## N-Phase Summary

### 1. Design & Data Model (N1-N3)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N1 | Design | `N1_MESH_FACT_MODEL_DESIGN_FROM_RAPID_V1.md` | MeshFactKind enum, MeshFact struct, ring design |
| N2 | Implementation | `N2_MESH_SHELL_LOCAL_FACT_RING_V1.md` | MESH_FACTS[32] ring, mesh_record_fact(), mesh_fact_count(), mesh_for_each_fact() |
| N3 | Audit | `N3_AUDIT_MESH_FACT_RING_V1.md` | Ring schema, overflow, J6 wire, boundaries |

### 2. Rendering & Visual (N4-N5)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N4 | Implementation | `N4_MESH_FACT_ROW_RENDER_V1.md` | mesh_render_fact_list(), MESH_LIST_ROW_RECTS=7, multi-rect fills |
| N5 | Audit | `N5_AUDIT_MESH_FACT_ROW_RENDER_V1.md` | Render safety, read-only iteration, boundaries |

### 3. Navigation & Selection (N6-N7)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N6 | Implementation | `N6_MESH_SELECTED_ROW_NAV_V1.md` | MESH_SELECTED_ROW, J/K wrap nav, selected row highlight |
| N7 | Audit | `N7_AUDIT_MESH_SELECTION_V1.md` | Selection invariants, clamp repair, dispatch precedence |

### 4. Detail Proof & Action Design (N8-N10)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N8 | Implementation | `N8_MESH_FACT_DETAIL_PROOF_STUB_V1.md` | mesh_selected_fact_snapshot(), mesh_emit_selected_fact_detail_proof() |
| N9 | Audit | `N9_AUDIT_MESH_DETAIL_STUB_V1.md` | Detail proof safety, exhaustive match, read-only |
| N10 | Design | `N10_MESH_ROW_ACTION_DESIGN_V1.md` | Designed FocusLinen + OpenInQuil action paths |

### 5. Action: Focus Linen (N11-N12)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N11 | Implementation | `N11_MESH_FOCUS_LINEN_AT_SELECTED_FACT_V1.md` | mesh_focus_linen_at_selected_fact(), bool guard on Enter |
| N12 | Audit | `N12_AUDIT_MESH_FOCUS_LINEN_V1.md` | Bool guard verified, pure navigation, no Collar bypass |

### 6. Action: Open in Quil (N13-N15)

| Phase | Type | Document | What Was Built |
|-------|------|----------|---------------|
| N13 | Design | `N13_MESH_OPEN_LINKED_OBJECT_DESIGN_V1.md` | Collar-gated design, PrintScreen key, stale-fact safety |
| N14 | Implementation | `N14_MESH_OPEN_LINKED_OBJECT_IN_QUIL_V1.md` | 0x59 Mesh dispatch, open_linen_object_in_quil(fact.subject_id) |
| N15 | Audit | `N15_AUDIT_MESH_OPEN_LINKED_OBJECT_V1.md` | Collar bypass impossible, stale fact safe, boundaries intact |

## Safety Invariants Preserved

### Core Invariants

| # | Invariant | Verification | Status |
|---|-----------|-------------|--------|
| 1 | No Mesh PD/server created | All Mesh code is shell-local in silk-shell/src/main.rs | ✅ |
| 2 | No PDX/ABI/opcode/kernel changes | No changes outside silk-shell/src/main.rs in N1-N15 commits | ✅ |
| 3 | No sexdisplay changes after existing 0xEF path | Mesh uses only 0xEF with color+rect_index, no new sexdisplay concepts | ✅ |
| 4 | No direct buffer creation from Mesh | QUIL_BUFFERS only modified inside open_linen_object_in_quil() | ✅ |
| 5 | No direct Collar bypass | collar_check_operation_stub() is inside open_linen_object_in_quil(), not callable from Mesh dispatch | ✅ |
| 6 | No Mesh fact mutation from render/nav/action | mesh_for_each_fact() passes &MeshFact read-only; mesh_selected_fact_snapshot() returns Copy; MESH_SELECTED_ROW is separate from ring | ✅ |
| 7 | No Bell behavior changes | No changes to servers/bell/ or bell_* functions in silk-shell | ✅ |
| 8 | No heap allocation | All storage is static arrays (MESH_FACTS[32], MESH_SELECTED_ROW: u8) | ✅ |
| 9 | Selection is read-only on fact ring | MESH_SELECTED_ROW is an independent index, does not modify MESH_FACTS | ✅ |
| 10 | Stale facts safe | open_linen_object_in_quil() validates object at step 1 before any mutation | ✅ |

### STOP FIRST Invariants

| Trigger | Status | Notes |
|---------|--------|-------|
| New PDX opcodes | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| sex-pdx ABI constants | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Capability grants/revokes | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Cross-PD pointers | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Kernel introspection | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Persistent storage | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Renderer policy | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Mesh PD creation | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| Bell/Collar behavior | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |
| New Collar operation kind | ⛔ STOP FIRST if triggered | Not triggered in N1-N15 |

**STOP FIRST: NOT TRIGGERED** — All N1-N15 phases pass without triggering any STOP FIRST condition.

## Input Map

| Key | Scancode | Context | Action | Last Updated |
|-----|----------|---------|--------|-------------|
| F12 | 0x58 | Global | Toggle Mesh surface (SURFACE_ID_MESH = 202) | I1 |
| J | 0x24 | Mesh focused | Select next (newer) fact row — wraps | N6 |
| K | 0x25 | Mesh focused | Select previous (older) fact row — wraps | N6 |
| Enter | 0x1C | Mesh focused | Focus Linen at selected fact's object | N11 |
| PrintScreen | 0x59 | Mesh focused | Open linked object in Quil (Collar-gated) | N14 |

### Dispatch Precedence for Mesh Keys

```
J (0x24):    panel → palette → atlas → Bell → Mesh [✅ select next] → scancode_to_action [SelectNextLinenObject if Linen]
K (0x25):    panel → palette → atlas → Bell → Mesh [✅ select prev] → scancode_to_action [SelectPrevLinenObject if Linen]
Enter (0x1C): panel → palette → atlas → Bell → Mesh [✅ FocusLinen]  → scancode_to_action [AccessActivate if Linen/Quil]
PrintScreen (0x59): panel → palette → atlas → Bell → Mesh [✅ OpenInQuil] → scancode_to_action [OpenObjectInQuil if Linen]
```

When Mesh is **not** focused, all four keys fall through to scancode_to_action where
they reach their original handlers (Linen J/K, Linen/Quil Enter, Linen PrintScreen).

## Proof Marker Map

### Fact Ring Markers

| Marker | Location | When |
|--------|----------|------|
| `[mesh.object_link.start]` | `mesh_emit_linen_quil_links()` | Start of link scan |
| `[mesh.object_link.row]` | `mesh_emit_linen_quil_links()` | Valid link found |
| `[mesh.object_link.reject.missing_object]` | `mesh_emit_linen_quil_links()` | Stale ref |
| `[mesh.object_link.done]` | `mesh_emit_linen_quil_links()` | Scan complete |
| `[mesh.fact.write]` | `mesh_record_fact()` | Fact written to ring |
| `[mesh.fact.overwrite]` | `mesh_record_fact()` | Ring full, oldest overwritten |
| `[mesh.fact.done]` | `mesh_record_fact()` | Write complete |

### Row Render Markers

| Marker | Location | When |
|--------|----------|------|
| `[mesh.fact_list.render]` | `mesh_render_fact_list()` | Start of render |
| `[mesh.fact_list.row]` | `mesh_render_fact_list()` | Fact row emitted |
| `[mesh.fact_list.skip]` | `mesh_render_fact_list()` | Row skipped (max rows) |
| `[mesh.fact_list.done]` | `mesh_render_fact_list()` | Render complete |
| `[mesh.row_visual.rect]` | `mesh_render_fact_list()` | Fill rect sent |
| `[mesh.row_visual.skip]` | `mesh_render_fact_list()` | Rect budget exhausted |
| `[mesh.selection.current]` | `mesh_render_fact_list()` | Current selected row |
| `[mesh.selection.repair]` | `mesh_render_fact_list()` | Clamp after shrink |

### Selection Markers

| Marker | Location | When |
|--------|----------|------|
| `[mesh.selection.next]` | `mesh_select_next_row()` | J pressed |
| `[mesh.selection.prev]` | `mesh_select_prev_row()` | K pressed |
| `[mesh.selection.reject]` | nav helpers | Count ≤ 1 |
| `[mesh.selection_visual.row]` | `mesh_render_fact_list()` | Selected row highlight |

### Detail Proof Markers

| Marker | Location | When |
|--------|----------|------|
| `[mesh.detail.reject] reason=not_focused` | `mesh_emit_selected_fact_detail_proof()` | Mesh not focused |
| `[mesh.detail.reject] reason=no_fact` | `mesh_emit_selected_fact_detail_proof()` | No fact at selection |
| `[mesh.detail.open]` | `mesh_emit_selected_fact_detail_proof()` | Start of detail proof |
| `[mesh.detail.fact]` | `mesh_emit_selected_fact_detail_proof()` | Fact details |
| `[mesh.detail.object_link]` | `mesh_emit_selected_fact_detail_proof()` | Object link details |
| `[mesh.detail.done]` | `mesh_emit_selected_fact_detail_proof()` | Proof complete |

### Action Markers

| Marker | Location | When |
|--------|----------|------|
| `[mesh.action.focus_linen]` | `mesh_focus_linen_at_selected_fact()` | Focus Linen at object |
| `[mesh.keyboard.open_in_quil]` | Mesh dispatch handler | PrintScreen while Mesh focused |
| `[mesh.keyboard.next]` | Mesh dispatch handler | J consumed for Mesh |
| `[mesh.keyboard.prev]` | Mesh dispatch handler | K consumed for Mesh |
| `[mesh.keyboard.enter]` | Mesh dispatch handler | Enter consumed for Mesh |

## Known Risks / Deferred

| Risk / Deferred Item | Severity | Impact | Requires STOP FIRST? |
|----------------------|----------|--------|---------------------|
| Real Mesh PD | HIGH | Mesh currently has no real PD — all state is shell-local. Real Mesh would require PDX, IPC, capability isolation | ⛔ YES — STOP FIRST |
| Persistent topology graph | MEDIUM | Fact ring is memory-only. Reboot loses all topology history | ⛔ YES — STOP FIRST (storage) |
| Richer fact kinds | LOW | Only `ObjectLinkedToBuffer`. Future kinds (scene open, focus change) need enum extension | ✅ No — additive |
| Real Collar authority | MEDIUM | All LinkObjectToBuffer operations currently get AllowStub. Real Collar would deny operations based on policy | ⛔ YES — STOP FIRST (Collar) |
| Row text/details | MEDIUM | Rows are fill-rect only. No text labels, no detail pane | ⛔ YES — STOP FIRST (sexdisplay text) |
| Graph visualization | HIGH | Mesh topology is a list, not a graph. No node/edge visualization | ⛔ YES — STOP FIRST (sexdisplay primitives) |
| Deletion/ack/mutation | MEDIUM | No way to delete facts, ack actions, mutate rings | ⛔ YES — STOP FIRST (ring mutation from UI) |
| Selected object validation on stale facts | LOW | `mesh_focus_linen_at_selected_fact()` does not validate object before setting SELECTED_LINEN_OBJECT_ID — relies on render repair | ✅ Acceptable |
| Duplicate Bell events on repeated PrintScreen | LOW | Each OpenInQuil call records a new Bell event | ✅ Acceptable for V1 |
| Duplicate Mesh facts on repeated PrintScreen | LOW | `mesh_emit_linen_quil_links()` records all links each time | ✅ Acceptable — overwrite-oldest ring |

### What Requires STOP FIRST Before Any Mesh Expansion

Any future Mesh work MUST pass STOP FIRST if it touches:

1. **Mesh PD creation** — Creating a real Mesh server process
2. **New PDX opcodes** — Any IPC protocol changes
3. **Kernel/ABI changes** — Capability grants, MPK, syscalls
4. **sexdisplay changes** — New display primitives beyond 0xEF
5. **Real Collar authority** — Grant-based policy decisions
6. **Persistent storage** — Filesystem or disk access
7. **Ring mutation from UI** — User-triggered fact deletion/editing

## Code Architecture

### Files Changed (N1-N15)

Only one file was modified across all 15 phases:

```
servers/silk-shell/src/main.rs
```

No changes to:
- `kernel/`
- `crates/sex-pdx/`
- `servers/sexdisplay/`
- `servers/bell/`
- `servers/mesh/` (no Mesh PD exists)
- `servers/linen/` (no real linen server changes)
- `servers/quil/` (no real quil server changes)

### Key Functions (alphabetical)

| Function | Lines | Purpose |
|----------|-------|---------|
| `mesh_emit_linen_quil_links()` | 1298-1337 | J6: emit all Linen↔Quil links as facts |
| `mesh_emit_selected_fact_detail_proof()` | 1509-1536 | Emit detail proof markers for selected fact |
| `mesh_fact_count()` | 1266-1271 | Count facts currently in ring (capped) |
| `mesh_for_each_fact()` | 1274-1290 | Read-only newest-first iteration |
| `mesh_focus_linen_at_selected_fact()` | 1541-1546 | Focus Linen at selected fact's subject_id |
| `mesh_record_fact()` | 1237-1261 | Write fact to ring (overwrite oldest) |
| `mesh_render_fact_list()` | 1377-1442 | Render header + row fill rects |
| `mesh_selected_fact_snapshot()` | 1493-1507 | Copy of fact at MESH_SELECTED_ROW |
| `mesh_selected_row_highlight()` | 1454-1459 | Brighten color for selected row |
| `mesh_select_next_row()` | 1462-1473 | J: next fact row |
| `mesh_select_prev_row()` | 1476-1487 | K: previous fact row |
| `mesh_visible_fact_count()` | 1446-1450 | Visible count (capped at MESH_LIST_ROW_RECTS) |

### Key Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `SURFACE_ID_MESH` | 202 | Mesh surface ID |
| `MESH_FACT_RING_CAP` | 32 | Max facts in ring |
| `MESH_LIST_ROW_RECTS` | 7 | Max row rects to draw |
| `MESH_PLACEHOLDER_COLOR` | 0x00383010 | Amber diagnostic header |

## Build History

| Phase | Sectors | Build Result |
|-------|---------|-------------|
| N2 (fact ring) | 1611 | PASS |
| N4 (row render) | — | PASS |
| N6 (selection) | — | PASS |
| N8 (detail proof) | — | PASS |
| N11 (FocusLinen) | 1618 | PASS |
| N14 (OpenInQuil) | 1619 | PASS |
| **Current** | **1619** | **PASS** |

## Next Pivot

### C1: Collar Real Authority Model Design

**Type:** Docs only. No code changes.

**Purpose:** Design the real Collar authority model that will eventually replace
`AllowStub` with policy decisions. This is a prerequisite for any production use
of the existing Linen/Quil/Mesh action chain.

**Key questions for C1:**
1. What authority objects exist? (operations, resources, subjects)
2. What policy dimensions apply? (identity, role, capability, context)
3. How does Collar grant/revoke state work?
4. How does Collar integrate with Mesh topology?
5. How does Bell notify authority failures?
6. What is the minimal safe Collar for V2?
7. What STOP FIRST conditions apply?

**After C1:** Real Collar implementation, or other subsystem work.

---

*End of Mesh N1-N15 milestone. No further Mesh action expansion without STOP FIRST review.*
