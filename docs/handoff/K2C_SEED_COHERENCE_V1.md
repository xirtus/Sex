# K2C: Seed Coherence Init

**Status:** Complete
**Phase:** K2C — fix Risk #3 from REAL_CLAUDE_K1_ARCHITECTURE_RISK_REVIEW.md
**Files changed:** `servers/silk-shell/src/main.rs`
**Forbidden areas:** Untouched

---

## Root Cause (from K2 audit)

Seed `QUIL_SEED_BUFFERS` pre-declared two `linen_object_ref` values:
- buffer_id=2: `linen_object_ref=2, linked_surface_id=SURFACE_ID_QUIL`
- buffer_id=4: `linen_object_ref=5, linked_surface_id=0`

These were intentional semantic pre-links (buffer_id=2 = "Compositor Lifecycle Spec" doc,
buffer_id=4 = "Current ISO Build" artifact) but were established at compile-time without
going through the J4 proof trail.

`LINEN_SEED_OBJECTS[1]` (object_id=2) had `linked_surface_id=0`, meaning the Quil buffer
table and the Linen object table **disagreed** on whether object_id=2 was displayed on Quil.

---

## Decision: Option B (coherence pass, not removal)

Option A (remove ghost refs) would delete intentional semantic pre-links.
Option B (additive boot coherence) preserves them and makes both tables agree.

Seed buffers with pre-set `linen_object_ref` are valid — they represent "always-available"
Quil buffers that conceptually correspond to known objects. The only missing piece was the
reciprocal update on the Linen side.

---

## Fix Applied

### New function: `linen_quil_seed_coherence_init()`

Called once at boot, immediately after `quil_buffer_table_init()`.

**Logic:**
For every buffer in `QUIL_BUFFERS` where `linen_object_ref != 0` AND `linked_surface_id != 0`:
- Find the matching `LinenObject` by `object_id == linen_object_ref`
- If its `linked_surface_id` disagrees with the buffer's, update it
- Emit `[linen.quil.seed_link]` proof marker

**Effect at boot:**
- `LINEN_OBJECTS[1]` (object_id=2): `linked_surface_id` → `SURFACE_ID_QUIL` ✓
- `LINEN_OBJECTS[4]` (object_id=5): unchanged (buffer_id=4 has `linked_surface_id=0`, no update)

Both tables now agree: object_id=2 is displayed in Quil; object_id=5 has a buffer but is not
yet displayed.

### Proof markers

| Marker | When |
|--------|------|
| `[linen.quil.seed_link] object_id={} buffer_id={} surface_id={}` | Seed coherence update applied |
| `[linen.quil.seed_coherence.done] linked={}` | Scan complete |

### Boot order (post K2C)

```
lifecycle_init_all()
scene_init_all()
linen_object_table_init()        ← J1
quil_buffer_table_init()         ← J3
linen_quil_seed_coherence_init() ← K2C (new)
snap_capture_layout()
app_surface_registry_validate()
```

---

## Mesh Diagnostic Coherence (post K2C)

Before K2C: `mesh_emit_linen_quil_links()` showed buffer_id=2 (ref=2) with valid object,
but LinenObject had `linked_surface_id=0`. Mesh row was technically correct (link exists)
but the linked_surface_id in the Mesh row reflected the buffer's value, not the object's.

After K2C: LinenObject 2 has `linked_surface_id=SURFACE_ID_QUIL`. Mesh row now fully coherent.

---

## J4 Interaction (no regression)

If `open_linen_object_in_quil(2)` is called after K2C:
- Duplicate guard scans for `linen_object_ref == 2` → finds buffer_id=2 (seed) → reuse path
- Emits `[linen.quil.open.reuse_existing]`
- Updates buffer state to Open, linked_surface_id = SURFACE_ID_QUIL
- Step 5: LinenObject 2 already has linked_surface_id=SURFACE_ID_QUIL (set by K2C) — idempotent write

No double-allocation. No collision. Reuse path works correctly.

---

## Build Result

`[SEXOS ENTRYPOINT] success` — ISO produced cleanly.

---

## Remaining K2 Items

| Item | Status |
|------|--------|
| K2A buffer_id collision | Complete (commit a0c4198) |
| K2B namespace spec doc | Pending — safe for deepseekclaude |
| K2C seed coherence | **Complete (this doc)** |
| K2D small constants + comment fix | Pending — items 1+2 safe for deepseekclaude |
| K2E IPCPKU_MAP addendum | Pending — safe for deepseekclaude |
