# K2A: Fix Quil Buffer ID Collision

**Status:** Complete
**Phase:** K2A — fix for Risk #1 from REAL_CLAUDE_K1_ARCHITECTURE_RISK_REVIEW.md
**Files changed:** `servers/silk-shell/src/main.rs`
**Forbidden areas:** Untouched (kernel/, sex-pdx/, sexdisplay/, linen/, quil/, ABI, lifecycle, tombstone)

---

## Root Cause (from Risk #1 audit)

J4's original rule `buffer_id = object_id` collided with seed buffer IDs 1-6.
Seed `QUIL_BUFFERS[2]` had `buffer_id=3, linen_object_ref=0`.
J4 duplicate check matched on `linen_object_ref == object_id` (0 ≠ 3) → MISS.
J4 created a second `buffer_id=3` in a later slot.
`quil_buffer_by_id(3)` first-match-wins returned seed DesignNote (ref=0).
J7 Bell cross-check: `buf.linen_object_ref (0) != object_id (3)` → **silent rejection on every PrintScreen**.

---

## Fix Applied

### 1. Namespace constant

```rust
const QUIL_DYNAMIC_BUFFER_ID_BASE: u64 = 1000;
```

Seed buffers: IDs 1-6 (low namespace, static, manually curated).
Dynamic J4-created buffers: `QUIL_DYNAMIC_BUFFER_ID_BASE + object_id` (high namespace, no overlap).

For current test trigger (object_id=3): dynamic buffer_id = 1003. Seed buffer_id=3 unaffected.

### 2. open_linen_object_in_quil() — slot 4 rewrite

- Computes `dynamic_buffer_id = QUIL_DYNAMIC_BUFFER_ID_BASE + object_id` at entry to step 4.
- Duplicate scan now finds existing buffer by `linen_object_ref == object_id` (unchanged logic).
  - On match: updates state, emits `[linen.quil.open.reuse_existing]`.
- On no match (new link):
  - Pre-flight: checks if dynamic_buffer_id already taken by different ref → emits `[linen.quil.open.reject.buffer_id_collision]`, returns false.
  - Allocates first None slot with `buffer_id = dynamic_buffer_id`.
  - Emits `[linen.quil.open.dynamic_id]`.
  - If no free slot: emits `[linen.quil.open.reject.full]`, returns false.

### 3. Downstream callers updated

- Step 6 proof marker: uses `dynamic_buffer_id` (not object_id) for `buffer_id=` field.
- Step 9 Bell call: `bell_emit_object_link_event(object_id, dynamic_buffer_id)`.
  - Bell now looks up `quil_buffer_by_id(1003)` for object_id=3 → finds correct Code buffer (ref=3).
  - Cross-check: `buf.linen_object_ref (3) == object_id (3)` → **PASS**.
  - Emits `[bell.event.object_link]` correctly.

---

## New Proof Markers

| Marker | When emitted |
|--------|-------------|
| `[linen.quil.open.dynamic_id]` | New dynamic buffer allocated with high-namespace ID |
| `[linen.quil.open.reuse_existing]` | Existing buffer reused (second open of same object) |
| `[linen.quil.open.reject.full]` | No free slot in QUIL_BUFFERS |
| `[linen.quil.open.reject.buffer_id_collision]` | dynamic_buffer_id already taken by different linen_object_ref |

---

## Corrected PrintScreen Flow (object_id=3)

```
PrintScreen → open_linen_object_in_quil(3)
  dynamic_buffer_id = 1003
  step 1: LINEN_OBJECTS scan → object_id=3 found (CodeFile)
  step 2: grant_ref=0 → [linen.quil.open.no_grant] (informational)
  step 2.5: collar check → AllowStub
  step 3: CodeFile → QuilBufferKind::Code
  step 4:
    scan for linen_object_ref==3 → NOT FOUND (no buffer has ref=3 yet)
    pre-flight: quil_buffer_by_id(1003) → None (no collision)
    allocate slot 6 with buffer_id=1003, ref=3
    [linen.quil.open.dynamic_id] object_id=3 dynamic_buffer_id=1003
  step 5: LINEN_OBJECTS[2].linked_surface_id = SURFACE_ID_QUIL
  step 6: [linen.quil.buffer.linked] object_id=3 buffer_id=1003 kind=1
  step 7: open_quil_in_active_scene()
  step 8: mesh_emit_linen_quil_links()
    → sees buffer_id=2 (ref=2) → valid row [mesh.object_link.row]  (ghost link, K2-C)
    → sees buffer_id=4 (ref=5) → valid row [mesh.object_link.row]  (ghost link, K2-C)
    → sees buffer_id=1003 (ref=3) → valid row [mesh.object_link.row]  ✓
  step 9: bell_emit_object_link_event(3, 1003)
    → quil_buffer_by_id(1003) → Code buffer (ref=3)  ✓
    → buf.linen_object_ref (3) == object_id (3)  ✓
    → [bell.event.object_link] EMITTED  ✓
```

---

## Table-full Behavior

QUIL_BUFFERS: 16 slots total.
- Slots 0-5: 6 seed buffers (IDs 1-6).
- Slots 6-15: 10 free for dynamic J4 buffers.
- On 11th new dynamic open (all 10 dynamic slots occupied): `[linen.quil.open.reject.full]`, returns false.
- Pre-flight collision check fires first if dynamic_buffer_id already taken.

---

## Remaining K2-C Issue (not fixed here)

Seed buffers 2 and 4 have pre-set `linen_object_ref` values (2 and 5) without J4 proof trail.
Mesh emits rows for these at boot — ghost links with no Collar/Bell trace.
LinenObject 5 has `linked_surface_id=0` but quil_buffer 4 has `linen_object_ref=5` — tables disagree.

**Do not fix K2-C yet.** Requires coherent seed data redesign. Needs real Claude.
The ghost links are diagnostic-only noise; they do not cause any failure path.

---

## Build Result

`[SEXOS ENTRYPOINT] success` — ISO produced cleanly.
