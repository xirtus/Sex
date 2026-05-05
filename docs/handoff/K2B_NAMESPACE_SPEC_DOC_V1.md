# K2B: Shell-Local Namespace Specification

**Status:** Handoff (docs only — no code changes)
**Commit:** *(to be committed)*
**Source:** `docs/handoff/K2_NAMESPACE_AUDIT_DESIGN_V1.md` §Corrected Namespace Table
**Purpose:** Formal namespace spec for all shell-local ID tiers in silk-shell.
These namespaces are local to PKEY 3 (silk-shell). They do NOT appear in PDX
opcodes, ABI contracts, or cross-PD messages.

## 1. Scope

This document enumerates all shell-local ID tiers that exist in
`servers/silk-shell/src/main.rs`. These are identifiers used within the shell
process only. They are communicated to sexdisplay at call time via PDX message
arguments (surface IDs in 0xEC/0xEF/0xEE calls), but the ranges themselves are
shell-defined conventions, not ABI contracts.

### What these namespaces ARE
- Linen object IDs (1-16)
- Quil seed buffer IDs (1-6)
- Quil dynamic buffer IDs (1001-1016)
- Surface ID tiers (OS panels, app surfaces, workstation surfaces)
- grant_ref placeholder values
- Shell-local enum discriminants (CollarOperation, CollarDecision, BellEventKind)

### What these namespaces ARE NOT
- PDX capability slots — those are in IPCPKU_MAP.md
- PDX opcodes — those are in `crates/sex-pdx/src/lib.rs`
- PKEY assignments — those are in IPCPKU_MAP.md
- Cross-PD stable identifiers — these are silk-shell-local only

## 2. Current Status (post K2A + K2C)

### K2A — Dynamic Buffer ID Collision Fix (committed `a0c4198`)
- **Problem:** J4 dynamic buffers used `buffer_id = object_id`, colliding with
  seed buffer IDs (1-6) when object_id was 1-6.
- **Fix:** `QUIL_DYNAMIC_BUFFER_ID_BASE = 1000`. Dynamic buffer IDs are now
  `1000 + object_id`, yielding range 1001-1016 for object_ids 1-16.
- **Collision check:** J4 pre-flights that `dynamic_buffer_id` is not already
  taken by a buffer with a different `linen_object_ref`.

### K2C — Seed Coherence Init (committed `2731d5e`)
- **Problem:** Seed buffers declared `linen_object_ref` values (buffers 2→obj 2,
  buffer 4→obj 5) without a J4 proof trail, creating ghost Mesh link rows at boot.
- **Fix:** `linen_quil_seed_coherence_init()` runs at boot after both tables
  init. For each seed buffer with non-zero `linen_object_ref` and non-zero
  `linked_surface_id`, it synchronizes the matching
  `LinenObject.linked_surface_id` to match. Emits
  `[linen.quil.seed_link]` and `[linen.quil.seed_coherence.done]` proof markers.

## 3. Namespace Rules

### 3.1 Linen Object IDs

| Property | Value |
|----------|-------|
| Type | `u64` (stored in `LinenObject.object_id`) |
| Range | 1..=16 |
| Seeds | 1-6 (compile-time `LINEN_SEED_OBJECTS`) |
| Max | `LINEN_MAX_OBJECTS = 16` |
| ID 0 | Reserved — means "no object" |
| Dynamic allocation | Linear scan for `None` slot (J4) |

**Rules:**
- Object IDs are sequential, starting at 1.
- ID 0 is reserved (no object / unset).
- Dynamic objects created via J4 get `object_id` from the caller (currently
  hardcoded to 3 via PrintScreen trigger).
- Object IDs must never exceed `LINEN_MAX_OBJECTS`.
- These IDs are shell-local. They are NOT transmitted across PD boundaries.

### 3.2 Quil Seed Buffer IDs

| Property | Value |
|----------|-------|
| Type | `u64` (stored in `QuilBuffer.buffer_id`) |
| Range | 1..=6 |
| Seeds | `QUIL_SEED_BUFFERS[6]` |
| Max | `QUIL_MAX_BUFFERS = 16` |

**Rules:**
- Seed buffer IDs occupy the low namespace (1-6), sharing the same table as
  dynamic buffers.
- Dynamic buffer IDs use `QUIL_DYNAMIC_BUFFER_ID_BASE + object_id` to avoid
  collision with seed IDs.
- Seed IDs are compile-time constants. They never change at runtime.
- Seed IDs must not overlap with `QUIL_DYNAMIC_BUFFER_ID_BASE + any_object_id`.

### 3.3 Quil Dynamic Buffer IDs

| Property | Value |
|----------|-------|
| Base constant | `QUIL_DYNAMIC_BUFFER_ID_BASE = 1000` |
| Formula | `buffer_id = 1000 + object_id` |
| Effective range | 1001..=1016 (for object_ids 1-16) |
| Max | `QUIL_DYNAMIC_BUFFER_ID_BASE + LINEN_MAX_OBJECTS` = 1016 |

**Rules:**
- Dynamic buffer IDs must never collide with seed buffer IDs (1-6).
- Formula guarantees no overlap: min dynamic ID = 1001 > max seed ID = 6.
- J4 includes a pre-flight collision check: if `dynamic_buffer_id` is already
  occupied by a buffer with a different `linen_object_ref`, the link is
  rejected with `[linen.quil.open.reject.buffer_id_collision]`.
- Dynamic buffer IDs are deterministic: same object_id always produces same
  buffer_id.

### 3.4 Surface ID Tiers

| Tier | Range | Currently Assigned | Purpose |
|------|-------|-------------------|---------|
| OS UI panels | `0x90`–`0x97` (144–151) | CURSOR, LAUNCHER, STATUS, CLOCK, BELL, SCENE_SETTINGS, ATLAS (7 of 8) | Always-present OS-owned surfaces |
| App surfaces | 100–103 | APP, STATIC, TEST3, TEST4 (4 of 4) | User application surfaces |
| Workstation | 200–204 | LINEN, QUIL, MESH, COLLAR, BELL_PLACEHOLDER (5 of 5) | OS workstation surfaces |

**Rules:**
- Surface IDs are shared with sexdisplay at call time (0xEC create, 0xEF fill,
  0xEE deactivate), but the ranges are shell-defined conventions.
- Sexdisplay treats surface IDs as opaque identifiers. It does not interpret
  the numeric value.
- Tiers are for human organization only. No code logic depends on numeric tier.
- New surfaces should be allocated from the appropriate tier.
- All currently assigned IDs in `APP_SURFACES` registry are validated at boot
  for duplicates (surface_id + frame_id).

**Warning:** `SURFACE_ID_BELL_PLACEHOLDER = 204` (workstation tier) is
semantically ambiguous with `SURFACE_ID_BELL = 0x95` (OS panel tier). Both
are "Bell" but serve different purposes. Rename deferred (requires multi-site
edit; real Claude if pursued).

### 3.5 grant_ref

| Property | Value |
|----------|-------|
| Type | `u64` |
| Stub value | 0 |
| Meaning | No real Collar grant exists yet |

**Rules:**
- All current code uses `grant_ref = 0` for both seeds and dynamic buffers.
- A non-zero grant_ref would indicate a real Collar capability grant.
- Real Collar grants are deferred (STOP FIRST).
- No constant alias (`GRANT_REF_STUB = 0`) currently exists — K2D patch
  should add one for clarity.

### 3.6 Shell-Local Enum Ranges

| Enum | Variants | Range | Defined |
|------|----------|-------|---------|
| `CollarOperation` | 7 | 0-6 | `src/main.rs` |
| `CollarDecision` | 5 | 0-4 | `src/main.rs` |
| `BellEventKind` | 4 | 0-3 | `src/main.rs` |

These enums are never serialized, never transmitted via PDX, and never stored
in persistent state. They are match-branched only.

## 4. Seed Link Semantics

### Pre-Link Declaration vs Runtime Proof

Seed buffers may declare a `linen_object_ref` at compile time (buffer 2
references object 2, buffer 4 references object 5). These represent a
**semantic pre-link** — the seed data suggests these objects are related,
even though no J4 runtime link (with Collar gate + Mesh diagnostic + Bell
event) has been executed.

The K2C coherence init (`linen_quil_seed_coherence_init()`) reconciles these
pre-links by synchronizing `LinenObject.linked_surface_id` from the buffer to
the object at boot time. This ensures:
- Both tables agree on "this object is displayed on surface X"
- Mesh diagnostic rows show seed links with valid surface references
- No ghost rows or stale cross-references

### J4 Runtime Links vs Seed Pre-Links

| Property | Seed Pre-Link | J4 Runtime Link |
|----------|---------------|-----------------|
| When established | Compile time | Runtime (PrintScreen key) |
| Collar gate (J5) | ❌ | ✅ |
| Mesh diagnostic (J6) | ✅ (via K2C sync) | ✅ |
| Bell event (J7) | ❌ | ✅ |
| Proof markers | `[linen.quil.seed_link]` | `[linen.quil.buffer.linked]` + J5/J6/J7 chain |

Future work should either:
- Remove seed `linen_object_ref` values entirely (clean stubs), or
- Add a boot-time synthetic J5/J7 stub for seed links

## 5. Future Reserved Ranges

| Namespace | Reserved | Rationale |
|-----------|----------|-----------|
| Linen object IDs | 17-999 | Growth room before dynamic buffer base |
| Quil seed buffer IDs | 7-999 | Growth room for additional seeds |
| Quil dynamic buffer IDs | 1000-1999 | Current rule: 1000+object_id |
| Surface IDs — workstation | 205-0x8F (unassigned) | Expansion room |
| Surface IDs — app | 104-143 | Expansion room |
| Surface IDs — OS panels | 0x98-0x9F | Remaining panel IDs |

## 6. Coherence Invariants

1. **Dynamic buffer ID ≠ seed buffer ID.** Guaranteed by
   `QUIL_DYNAMIC_BUFFER_ID_BASE = 1000 > max seed ID = 6`.
2. **Dynamic buffer ID ≠ surface ID.** Dynamic IDs start at 1001; surface IDs
   max out at 204 (workstation tier).
3. **Object ID ≠ buffer ID (after K2A).** Object IDs are 1-16; dynamic buffer
   IDs are 1001-1016. Seed buffer IDs 1-6 coincidentally match object IDs but
   are in separate tables.
4. **linked_surface_id is a reference, not ownership.** The surface identified
   by `linked_surface_id` may not be the surface that owns the object. It
   indicates which surface displays the object.
5. **grant_ref = 0 means no real Collar grant.** All current code operates in
   stub mode.
6. **Shell-local enums never leave the shell.** `CollarOperation`,
   `CollarDecision`, `BellEventKind` are match-branched only.

## 7. Verification

These invariants are enforced by:
- **Compile-time constants:** `QUIL_DYNAMIC_BUFFER_ID_BASE`, `LINEN_MAX_OBJECTS`,
  `QUIL_MAX_BUFFERS`.
- **Runtime checks:** J4 pre-flights buffer_id collision before creating dynamic
  buffer; K2C coherence init synchronizes seed pre-links at boot.
- **Boot validation:** `app_surface_registry_validate()` checks for duplicate
  surface_id and frame_id in APP_SURFACES.
- **Proof markers:** Each namespace operation emits proof markers that can be
  verified in the boot log.
