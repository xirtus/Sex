# Real Claude K1 Architecture Risk Review — J1-J7

**Date:** 2026-05-05
**Branch:** master
**Reviewer:** Real Claude (claude-sonnet-4-6)
**Scope:** J1-J7 implementation in `servers/silk-shell/src/main.rs` + handoff docs
**Mode:** Read-only analysis. No code touched.

---

## Executive Verdict

```
FIX_FIRST
```

J1-J7 code is structurally sound and K1 audit correctly confirmed spec conformance.
However: **one silent correctness bug exists that causes every J7 Bell event to reject**
for the primary test trigger (PrintScreen → object_id=3). J4 succeeds, but J7 always
fires `[bell.event.reject.missing] reason=buffer_ref_mismatch`. The bug is not visible
in spec conformance checks — it requires cross-checking seed data against J4's dynamic
buffer creation rule.

---

## Top 10 Risks (ranked)

### RISK 1 — CRITICAL: Duplicate buffer_id collision (seed vs J4 dynamic)

**Evidence:**
- `QUIL_SEED_BUFFERS[2]`: `buffer_id=3, kind=DesignNote, linen_object_ref=0` (main.rs:502-512)
- J4 deterministic rule: `buffer_id = object_id` (main.rs:676)
- J4 duplicate check: finds existing buffer only if `buf.linen_object_ref == object_id` (main.rs:663)
- For object_id=3: seed buffer_id=3 has linen_object_ref=0 (0 != 3) → duplicate check MISSES
- J4 then creates new `QuilBuffer { buffer_id: 3, linen_object_ref: 3 }` in first `None` slot (slot 6)
- Now two entries have buffer_id=3: seed DesignNote (slot 2) and J4-created Code (slot 6)
- `quil_buffer_by_id(3)` does first-match-wins → returns seed DesignNote (linen_object_ref=0)
- bell_emit_object_link_event(3, 3): `buf.linen_object_ref (0) != object_id (3)` → **REJECTION**

**Silent failure path:**
```
PrintScreen → open_linen_object_in_quil(3)
  → collar check: PASS
  → duplicate guard: MISS (seed buf_id=3 has ref=0, not 3)
  → creates slot-6 buffer_id=3 (Code, ref=3)
  → step 5: updates LINEN_OBJECTS[2].linked_surface_id = SURFACE_ID_QUIL ✓
  → [linen.quil.buffer.linked] emitted ✓ (misleading — J4 claims success)
  → mesh_emit_linen_quil_links() — finds slot-2 DesignNote (ref=0) first, skips it; finds slot-6 Code (ref=3) — emits row ✓
  → bell_emit_object_link_event(3, 3)
      → quil_buffer_by_id(3) returns slot-2 DesignNote (ref=0) ← WRONG BUFFER
      → buf.linen_object_ref (0) != object_id (3)
      → [bell.event.reject.missing] reason=buffer_ref_mismatch ← SILENT BUG
```

**Affected object_ids:** 1, 3, 4, 5, 6 — all have seed buffer_ids 1,3,4,5,6 with linen_object_ref=0.
**Only safe case:** object_id=2 (seed buffer_id=2 has linen_object_ref=2 → duplicate check matches → correct update-existing path).

---

### RISK 2 — HIGH: Ghost links in seed data bypass J4 proof trail

**Evidence:**
- `QUIL_SEED_BUFFERS[1]`: `buffer_id=2, linen_object_ref=2` (main.rs:491-500)
- `QUIL_SEED_BUFFERS[3]`: `buffer_id=4, linen_object_ref=5` (main.rs:513-523)
- These links exist at boot without any J4 call, Collar check, or Bell event

**Effect:** `mesh_emit_linen_quil_links()` fires at boot (via `open_mesh_in_active_scene()`) and emits
`[mesh.object_link.row]` for buffer_id=2 (ref=2) and buffer_id=4 (ref=5). These links have no
J4/J5/J7 proof trail. Mesh diagnostic appears to show active links that were never gated.

---

### RISK 3 — MEDIUM: Linen/Quil table state disagreement for pre-seeded links

**Evidence:**
- `LINEN_SEED_OBJECTS[4]` (object_id=5, BuildArtifact): `linked_surface_id=0` (main.rs:271-280)
- `QUIL_SEED_BUFFERS[3]` (buffer_id=4, linen_object_ref=5): `linked_surface_id=0` (main.rs:513-523)
- J4 never called for object_id=5 (only trigger is PrintScreen → object_id=3)

**Effect:** Buffer table says object_id=5 has a buffer (buffer_id=4). Linen table says object_id=5
has no linked surface (linked_surface_id=0). Tables disagree on whether a Quil surface is open.
If future code checks `linen_object.linked_surface_id != 0` to determine "open in Quil," it will
incorrectly report object_id=5 as unopened.

---

### RISK 4 — MEDIUM: PrintScreen trigger hardcoded to single object_id

**Evidence:** `main.rs:8157`: `open_linen_object_in_quil(3)` — literal constant

**Effect:** All J4/J5/J6/J7 testing flows through object_id=3, which is the exact case broken by
Risk #1. The "passing" K1 audit never caught J7 rejection because the audit verified code structure,
not runtime behavior. There is no test path for object_ids 1, 2, 4, 5, 6.

---

### RISK 5 — MEDIUM: Table-full silent failure at capacity

**Evidence:** `QUIL_MAX_BUFFERS = 16`. 6 slots used by seeds. Each J4 call for a new object
(non-duplicate case) consumes one more slot. At 10 J4 calls, table is full.

**Effect:** `open_linen_object_in_quil` emits `[linen.quil.open.reject.missing] reason=no_buffer_slot`
and returns false. No pre-flight capacity check. Caller in main event loop just skips (mutated stays
false). Silent denial with no user-visible indication. Same applies to LINEN_OBJECTS (16 slots, 6
seeded, 10 remaining).

---

### RISK 6 — LOW: J4 two-pass table scan (efficiency + correctness risk)

**Evidence:**
- Step 1 (main.rs:621-636): scans LINEN_OBJECTS for object_id, copies value into `found_obj`
- Step 5 (main.rs:698-705): scans LINEN_OBJECTS AGAIN to update `linked_surface_id`

**Effect:** In single-threaded no_std PD, no correctness issue. But the copy-then-rescan pattern
means if table content changes between steps 1 and 5 (only possible via re-entrant PDX), step 5
could update wrong slot or miss the object. Implicit assumption: no re-entrancy between steps.
Assumption is currently safe but undocumented.

---

### RISK 7 — LOW: J4 caller cannot distinguish link-reuse from new-link

**Evidence:** `open_linen_object_in_quil` returns `bool` (success/fail). `buffer_created` is
local variable emitted in proof marker (main.rs:717) but not returned.

**Effect:** Event-loop caller at main.rs:8157 only knows "did it succeed?" Cannot distinguish
"buffer already existed, state updated" vs "new buffer created." Future UI (show "already open"
indicator vs "opened for first time") has no signal from J4.

---

### RISK 8 — LOW: Collar redundantly validates object that J4 already validated

**Evidence:**
- J4 step 1 validates object_id exists (returns false on missing)
- J4 step 2.5 calls `collar_check_operation_stub(LinkObjectToBuffer, object_id, 0)`
- Collar re-scans LINEN_OBJECTS for object_id (main.rs:779-793)
- buffer_id=0 passed → Collar's buffer validation always skipped

**Effect:** Collar can never see a missing-object case from J4's call path (J4 already handles it).
Collar's DenyMissingObject branch is dead code in the J4 → Collar call chain. Not a bug, but the
collar check adds ~16 iterations of wasted scan. Acceptable in no_std serial model.

---

### RISK 9 — LOW: J6 Mesh scan fires BEFORE J7 Bell event

**Evidence:**
- Step 8 (main.rs:720): `mesh_emit_linen_quil_links()`
- Step 9 (main.rs:723): `bell_emit_object_link_event(object_id, object_id)`

**Effect:** Mesh scan sees the newly linked buffer (slot 6) and emits its row correctly. Then Bell
fires and (due to Risk #1) rejects. The Mesh scan captures correct state; Bell captures wrong state.
Correct ordering when both become real: Bell should fire at transition, Mesh should emit the confirmed
post-transition state. Current ordering is reversed from the semantic intent (event → diagnostic).
Low risk now (both are proof markers), but establishes wrong ordering for future real implementations.

---

### RISK 10 — LOW: Static mut table aliasing assumption undocumented

**Evidence:** `static mut LINEN_OBJECTS` and `static mut QUIL_BUFFERS` accessed in multiple
`unsafe fn` across J1-J7 (main.rs:222, 476).

**Effect:** Single-threaded no_std PD model prevents data races. But the safety invariant
("this PD is single-threaded, no concurrent access possible") is implicit — not stated in code
comments or CLAUDE.md. Future work adding timer callbacks, interrupt handlers, or async PDX
responses could introduce aliasing. Should be documented.

---

## Correction Phases (smallest first)

### Phase K2-A: Fix seed buffer ID collision (BLOCKS J7 correctness) — 1 file, seed data only

**What:** Renumber QUIL_SEED_BUFFERS so no seed buffer_id overlaps with any seed object_id
that will be opened via J4. Two options:

- Option A (minimal): Renumber seed buffers 1-6 → 10-15. Dynamic J4 buffers use buffer_id=object_id
  (1-9), seeds use 10-15. No namespace collision.
- Option B (correct fix): Add linen_object_ref to the seed buffers that already have logical links
  (buffer_id=1 → ref=0 is fine since it's standalone; buffer_id=3, DesignNote → this should either
  not have buffer_id=3 or get a ref pointing to an object it represents). Cleanest: remove the
  implicit overlap entirely by separating ID spaces.

**Requires real Claude** for namespace decision. Do NOT fix with deepseekclaude — ID renaming touches
seed data semantics.

### Phase K2-B: Document static mut safety invariant — comment only

**What:** Add one-line comment above LINEN_OBJECTS and QUIL_BUFFERS declarations: "Safety: accessed
only from single-threaded PD event loop. No concurrent access possible."

**Safe for deepseekclaude** (comment-only, non-semantic change).

### Phase K2-C: Fix Linen/Quil state coherence for pre-seeded links

**What:** Either (a) remove linen_object_ref from seed buffers that were never J4-opened (set to 0),
or (b) set the corresponding LinenObject.linked_surface_id correctly in seed data.

**Requires real Claude** (touching both tables coherently).

### Phase K2-D: Add J4 capacity pre-flight check

**What:** Before scanning for None slot, assert remaining slot count > 0. Emit warning marker.

**Safe for deepseekclaude** (additive check, no logic change).

---

## Do Not Fix Yet

| Item | Reason |
|------|--------|
| PrintScreen trigger hardcoding | Intentional test path. Replace with selection-driven action only after Linen PD surface exists. STOP FIRST (requires surface state model change). |
| J6 Mesh → J7 Bell ordering | Both are proof markers. Reordering now creates false impression of maturity. Fix only when either becomes real. |
| Collar redundant object scan | Optimization only. Not blocking. Do not touch until J5 becomes real Collar. STOP FIRST (real Collar requires PDX ABI). |
| buffer_created return value | Not blocking until UI needs open-vs-reuse distinction. Future feature, not current bug. |
| Table size increase (>16 slots) | No current pressure. Touch only after dynamic object model is spec'd. |

---

## Next 3 Safe Prompts for deepseekclaude --bare

**Prompt 1 — Collision audit (research only):**
```
Read QUIL_SEED_BUFFERS in servers/silk-shell/src/main.rs.
For each seed buffer: record buffer_id and linen_object_ref.
Read LINEN_SEED_OBJECTS: record all object_ids.
List every case where buffer_id in QUIL_SEED_BUFFERS equals an object_id in LINEN_SEED_OBJECTS
AND linen_object_ref != that object_id (i.e. the ID matches but the ref is wrong/zero).
Do not fix anything. Output: docs/handoff/K2_SEED_ID_COLLISION_AUDIT.md
```

**Prompt 2 — Bell rejection trace (research only):**
```
Trace open_linen_object_in_quil(3) in servers/silk-shell/src/main.rs step by step.
For each step, state what table state is read/written.
Then trace bell_emit_object_link_event(3, 3):
  what quil_buffer_by_id(3) returns, what linen_object_ref value is on that buffer,
  and why the ref_mismatch check fires.
Do not fix anything. Output: docs/handoff/K2_BELL_REJECTION_TRACE.md
```

**Prompt 3 — Static mut safety comment (safe edit):**
```
In servers/silk-shell/src/main.rs, find the lines:
  static mut LINEN_OBJECTS: [Option<LinenObject>; LINEN_MAX_OBJECTS] = [None; LINEN_MAX_OBJECTS];
  static mut QUIL_BUFFERS: [Option<QuilBuffer>; QUIL_MAX_BUFFERS] = [None; QUIL_MAX_BUFFERS];
Add a one-line safety comment above each:
  // Safety: single-threaded PD event loop; no concurrent access possible.
No other changes. Commit: docs(safety): annotate static mut table access invariants
```

---

## What Requires Real Claude

- **K2-A seed buffer ID renaming** — namespace decision affects all future object opens
- **K2-C Linen/Quil seed coherence** — touching both tables requires understanding cross-table semantics
- **Any fix that touches the J4 duplicate-guard logic** — the guard encodes invariants about ID spaces
- **Any decision about buffer_id vs object_id namespace separation** — architectural, not mechanical

---

## What Requires STOP FIRST

| Next step | Why STOP FIRST |
|-----------|---------------|
| Add multi-rect sexdisplay opcode to enable full J2 list | sexdisplay ABI/opcode edit |
| Replace PrintScreen with selection-driven open trigger | Requires surface state model → possible PDX ABI |
| Implement real Collar grant enforcement | Real Collar PD + PDX ABI + grant semantics |
| Implement real Bell event queue/notification surface | Bell PD + PDX send + notification surface creation |
| Implement real Mesh graph renderer | sexdisplay renderer policy + new opcodes |
| Move object/buffer tables from silk-shell to Linen/Quil PDs | Cross-PD migration, new PDX ops required |
| Add persistence for object/buffer state | Filesystem/storage — explicitly forbidden |
| Add Quil editor/parser/compiler/build | Explicitly forbidden until storage exists |

---

## Proof Verification

```
SAFE_TO_CONTINUE: NO
FIX_FIRST: YES — Risk #1 (duplicate buffer_id) causes silent J7 rejection on every PrintScreen
BLOCKED_STOP_FIRST: NO — no STOP FIRST trigger in J1-J7 code itself; future next steps may require it

top 10 risks: listed above, ranked by severity
static mut: LINEN_OBJECTS and QUIL_BUFFERS, [None; 16], single-threaded safe but undocumented
boot ordering: lifecycle_init_all → scene_init_all → linen_object_table_init → quil_buffer_table_init — CORRECT
duplicate link: J4 duplicate guard checks linen_object_ref==object_id; misses when seed buffer_id matches but ref is wrong/zero
stale ref: J6 mesh_emit detects stale refs correctly; J7 bell cross-check fails due to wrong buffer returned by quil_buffer_by_id
cross-phase coupling: J4→J5→J6→J7 chain is correct; J5 redundantly validates already-checked object; J6/J7 ordering reversed from semantic intent
PrintScreen: scancode 0x59, hardcoded object_id=3, deterministic test path acceptable for now
renderer-safe: J2 fill_rect only, no text rows, correct — next UI step requires STOP FIRST (new sexdisplay opcode)
STOP FIRST: all future real Collar/Bell/Mesh/Linen-PD/Quil-PD work requires STOP FIRST
deepseekclaude: 3 prompts defined above (collision audit, bell rejection trace, safety comment)
```
