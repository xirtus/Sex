# SexObject Canonical Model V4

**Date:** 2026-05-06
**Status:** PROPOSED / DOC-ONLY / OQ ANSWERS ARE DESIGN RECOMMENDATIONS
**Replaces:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V3.md
**Scope:** Naming canon, OQ answers, M1 readiness gate. No implementation.

---

## Naming Canon (Locked)

| Name | Role | Status |
|---|---|---|
| `SexObject` | The concept. The semantic OS unit. Never a struct; always the idea. | Canon |
| `SexObjectHeader` | Proposed fixed-size no_std header struct. Lives in model crate. | PROPOSED |
| `SexObjectKind` | Proposed bounded kind enum. `repr(u32)` in model; fits u16 in V1 storage (see §Kind Constraint). | PROPOSED |
| `SexObjectRef` | Opaque cross-PD reference. Two u64 scalars. Fits existing `PdxMessage` fields. | PROPOSED |
| `SexObjectRecord` | Preferred future name for the storage record when SexFiles extends it. | PROPOSED |
| `SexfilesObjectEntry` | Current verified SexFiles storage implementation. Internal to SexFiles server. | VERIFIED, internal only |

**Rule:** Do NOT expose `SexfilesObjectEntry` as the OS-wide concept name.
**Rule:** SexFiles stores SexObjects. SexObject is not named after SexFiles.
**Rule:** `SexObjectRef` is what crosses PD boundaries — never the full header.
**Rule:** Do not rename code until model crate (M1) exists and compiles.

---

## Open Questions — Design Answers

### OQ1 — Who allocates `object_id`? [ANSWERED]

**Answer:** SexFiles, via `create_object_entry`.

**Evidence:** `NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1)` at
`servers/sexfiles/src/backends/diskfs.rs:124`. Each `create_object_entry` calls
`NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst)`. Global monotonic counter.
Starts at 1; 0 is invalid sentinel.

**Recommendation:** Keep this. SexFiles is the single authoritative allocator.
No per-PD allocation. No Collar/Mesh/Bell allocation of `object_id`.

**Constraint:** `DISKFS_MAX_OBJECTS = 16`. Object count is hard-capped at 16 in V1.
This must be noted in migration planning — `SexObjectRecord` table expansion is a
future M2+ concern. Don't block M1 model crate on it.

---

### OQ2 — Where is `SexObjectHeader` stored? [ANSWERED — design recommendation]

**Existing layout:** `SexfilesObjectEntry` holds `object_id`, `kind`, `owner_pd`,
`rights_generation`, `object_size_bytes`, `first_block`, `metadata_generation`,
`checksum`, `in_use`. No block ref for a separate header.

**Recommendation:** Do NOT add a separate header block in V1.
`SexObjectRecord` (the M2 extension of `SexfilesObjectEntry`) adds fields inline:
`content_generation`, `policy_flags`, `parent_ref`. `metadata_ref` and `policy_ref`
are out-of-band for V1 — set to 0 until M2+ storage work.

In V1, `SexObjectHeader` exists only in the model crate as a logical view type.
It is constructed on-the-fly from a `SexfilesObjectEntry` + defaults for the
unimplemented fields. No disk format change in V1.

---

### OQ3 — What is sexshop's role relative to SexObject? [ANSWERED — conservative]

**Evidence:** `StoreProtocol` (in `crates/sex-pdx/src/lib.rs`, used by
`servers/sexshop/src/pdx.rs`) implements: `ObjectPut/Get/Exists/Move` (content-addressed
blobs by hash), `FetchPackage/CacheBinary` (package/binary cache), `KVGet/KVSet/KVDelete`
(KV store), `TransactionBegin/Commit/Abort`, `SyncFilesystem`, `Stats`.

**Recommendation:** V1 role is conservative:
- sexshop is the content-addressed blob cache for large SexObject payloads.
- `SexObjectHeader.content_ref` for large blobs → sexshop hash (u64 content address).
- `SexObjectHeader.content_ref` for small objects → SexFiles `first_block` directly.
- sexshop does NOT own `object_id` allocation. SexFiles does.
- sexshop does NOT replace SexFiles. Complementary.
- `SexObjectKind::Package` maps to sexshop `ObjectPut` blobs — sexshop assigns
  the hash; SexFiles assigns the `object_id`.

**Open sub-question:** Who maintains the mapping `object_id → sexshop hash`?
Recommendation: SexFiles `SexObjectRecord.content_ref` field stores the hash when
content lives in sexshop. Not resolved until M2.

---

### OQ4 — Where does `rights_generation` live? [ANSWERED]

**Evidence:** `SexfilesObjectEntry.rights_generation: u64` [VERIFIED at diskfs.rs:29].
`CollarGrant.generation: u64` [VERIFIED at silk-shell/src/main.rs:1409] — separate
grant-level epoch.

**Recommendation:** SexFiles is the authoritative source for `rights_generation`.
Collar is the authority for granting/revoking. On revocation, Collar must send a PDX
op to SexFiles to bump `rights_generation` for the target `object_id`. SexFiles then
rejects all caps with stale `rights_generation`. The two generation counters (`CollarGrant.generation`
and `SexfilesObjectEntry.rights_generation`) are NOT the same value — they must
be kept in sync via an explicit revocation protocol (design in M3, not M1).

---

### OQ5 — `CollarGrant.object_id` vs `SexfilesObjectEntry.object_id`? [ANSWERED]

**Evidence:** `CollarGrant.object_id` [VERIFIED at silk-shell/src/main.rs:1407]
currently references silk-shell-local Linen object indices (not SexFiles `object_id`).
They are different ID spaces today.

**Recommendation:** Unification is M3 work, not M1. In V1 model crate, document this
gap. In M3, migrate Collar's `object_id` field to reference `SexfilesObjectEntry.object_id`
directly. Until then, treat `CollarGrant.object_id` as a shell-local handle.

---

### OQ6 — Can `SexObjectRef` fit existing PDX messages without ABI change? [ANSWERED — YES]

**Evidence:** `PdxMessage` has `arg0: u64`, `arg1: u64`, `arg2: u64`
[VERIFIED at `crates/sex-pdx/src/lib.rs:54`].

**Answer:** `SexObjectRef { object_id: u64, generation: u64 }` fits:
- `arg0` = `object_id`
- `arg1` = `generation`
- `arg2` free for operation-specific payload

**No PDX ABI change required for V1.** Existing `PdxMessage` carries a `SexObjectRef`
today if callers agree on the arg layout. Model crate defines the convention; no
`sex-pdx` edit needed.

---

### OQ7 — Negative tests for stale generation / revocation? [ANSWERED — plan]

**Existing gate** [VERIFIED]: `[sexfiles.caprec.proof.generation_deny]`
(`servers/sexfiles/src/proof.rs:178, 203, 243`) — covers stale-cap injection.

**Required future proof gates (not yet in repo):**

| Gate | Tests |
|---|---|
| `[sexobject.gen.write_after_revoke]` | Write to object whose `rights_generation` was bumped → denied |
| `[sexobject.gen.tombstone_access]` | Read/write to `FLAG_TOMBSTONED` object → denied |
| `[sexobject.gen.cross_pd_stale_ref]` | Cross-PD `SexObjectRef` with stale generation → denied by SexFiles |
| `[sexobject.gen.sealed_write]` | Write to `FLAG_SEALED` object → denied |

Add to M3 scope (Collar binding). Not needed before M1 model crate.

---

### OQ8 — `SceneSnapshot` in V1 kind list? [ANSWERED — EXCLUDED]

**Evidence:** `sex_pdx::SceneSnapshot` has `layers_ptr: u64`, `damage_rects_ptr: u64`
[VERIFIED at `crates/sex-pdx/src/lib.rs:167`]. These are raw pointer fields
(in-flight IPC, not durable values).

**Answer:** `SceneSnapshot` excluded from V1 `SexObjectKind`. It is a display-pipeline
IPC struct, not a durable stored object. Display subsystem design doc owns it.
Add `SceneSnapshot` kind only when a serialized/durable scene format is designed.

---

### OQ9 — `BellEvent` struct owner? [ANSWERED — already exists]

**Evidence:** `BellQueueEntry` [VERIFIED at `servers/sexbell/src/main.rs:23`]:

```rust
struct BellQueueEntry {
    event_id:         u64,    // monotonic, assigned by Bell on accept
    caller_pd:        u32,
    category:         u8,     // 0=Info .. 5=Error
    requested_lane:   u8,
    final_lane:       u8,
    final_urgency:    u8,
    privacy_level:    u8,
    redaction_class:  u8,
    action_count:     u8,
    action_id:        u8,
    object_ref_count: u8,
    object_ref:       u8,
    dismissed:        u8,
    _pad:             [u8; 2],
}
```

**Answer:** `BellQueueEntry.event_id` is the natural `object_id` for
`SexObjectKind::BellEvent`. No new struct needed. `BellQueueEntry` IS the V1
Bell notification record. Binding: M6 assigns `event_id` as `SexObjectHeader.object_id`
when `OP_BELL_NOTIFY` creates a persistent BellEvent object.

**Note:** `object_ref: u8` in `BellQueueEntry` is a marker-only field in V1.
In M6, this field would carry a `SexObjectRef` object_id (currently truncated to u8,
which is a V1 limitation to document).

---

### OQ10 — `SexObjectHeader.generation` maps to which existing field? [ANSWERED]

**Evidence:** `SexfilesObjectEntry` has two generation fields [VERIFIED]:
- `rights_generation: u64` — capability/revocation epoch
- `metadata_generation: u64` — metadata-write epoch

No `content_generation` field exists.

**Answer:** `SexObjectHeader.generation` (content/mutation epoch) maps to a
**new field** `content_generation: u64` to be added to `SexObjectRecord` in M2.
In V1, it is set to `metadata_generation` value as a temporary proxy.

Generation summary (all three are distinct):

| Field | Lives in | Semantics |
|---|---|---|
| `rights_generation` | `SexfilesObjectEntry` [VERIFIED] | Capability/revocation epoch. Bumped by Collar. |
| `metadata_generation` | `SexfilesObjectEntry` [VERIFIED] | Metadata-write epoch. Bumped on metadata update. |
| `fs_generation` | `SexfilesSuperblock` [VERIFIED] | Filesystem transaction epoch. Per tx-commit. |
| `CollarGrant.generation` | `CollarGrant` [VERIFIED] | Grant-level epoch. Must sync with `rights_generation` on revocation. |
| `content_generation` | PROPOSED — not in repo | Content/payload write epoch. Add in M2. |

---

### OQ11 — `DeviceRoute` in V1 kind list? [ANSWERED — EXCLUDED]

No existing device route type found in repo. Excluded from V1. Add after
hardware/device layer stabilizes.

---

## Updated Kind List (V1 Final — Post OQ Answers)

All [PROPOSED] kinds; [VERIFIED source] where underlying type confirmed.

```rust
/// PROPOSED. Not yet in repo. V1 discriminants must fit u16 (see §Kind Constraint).
#[repr(u32)]
pub enum SexObjectKind {
    RawBlob         = 0,   // Untyped content block
    AppManifest     = 1,   // Source: silk_shell::AppManifest [VERIFIED source]
    AppState        = 2,   // Persisted app state blob [PROPOSED]
    LinenProject    = 3,   // Source: LinenObjectKind::Project [VERIFIED source]
    QuilDocument    = 4,   // Source: LinenObjectKind::Document [VERIFIED source]
    SpindleSession  = 5,   // Terminal/session/log object [PROPOSED]
    BellEvent       = 6,   // Source: BellQueueEntry.event_id [VERIFIED source]
    CollarGrant     = 7,   // Source: CollarGrant struct [VERIFIED source]
    MeshFact        = 8,   // Source: MeshFact struct [VERIFIED source]
    CrashReport     = 9,   // Kernel/server crash dump [PROPOSED]
    Package         = 10,  // Source: StoreProtocol::ObjectPut [VERIFIED source]
    // SceneSnapshot  — EXCLUDED (raw ptr fields; not durable)
    // DeviceRoute    — EXCLUDED (no existing type; defer)
}
```

### Kind Constraint (New — Not in V3)

`SexfilesObjectEntry.kind` is `u16` [VERIFIED at diskfs.rs:27].
`SexObjectKind` is `repr(u32)` in the model crate.

**Rule:** All V1 `SexObjectKind` discriminants MUST fit `u16` (≤ 65535).
Current max discriminant: 10 (`Package`). Well within u16 range.
M2 extends `SexfilesObjectEntry.kind` to `u32` if discriminants exceed 65535.
Until then, model crate documents the u16 storage constraint; callers cast `kind as u16`
on write and widen `kind as u32` on read.

---

## Updated Object Header (Post OQ Answers)

```rust
/// PROPOSED. Not yet in repo.
/// 9 × u64 + 2 × u32 = 80 bytes. repr(C), no padding.
#[repr(C)]
pub struct SexObjectHeader {
    pub object_id: u64,          // SexFiles NEXT_OBJECT_ID [VERIFIED allocator]
    pub generation: u64,         // content/mutation epoch (= metadata_generation in V1)
    pub kind: u32,               // SexObjectKind; must fit u16 in V1 storage
    pub owner_pd: u32,           // creating PD; matches SexfilesObjectEntry.owner_pd
    pub rights_generation: u64,  // capability/revocation epoch; SexFiles authoritative
    pub content_ref: u64,        // SexFiles first_block OR sexshop hash; 0 = no content
    pub metadata_ref: u64,       // reserved; 0 in V1
    pub policy_ref: u64,         // reserved; 0 unless FLAG_REDACTED set
    pub parent_ref: u64,         // object_id of parent; 0 = root
    pub checksum: u64,           // XOR over all other fields
    pub flags: u64,              // see §Flags
}
// Size verification: object_id(8) + generation(8) + kind(4) + owner_pd(4) +
//   rights_generation(8) + content_ref(8) + metadata_ref(8) + policy_ref(8) +
//   parent_ref(8) + checksum(8) + flags(8) = 80 bytes.
```

### Flags

```
bit 0  FLAG_TOMBSTONED  0x01  — deleted; deny all writes; generation readable
bit 1  FLAG_SEALED      0x02  — immutable after seal; deny further content writes
bit 2  FLAG_REDACTED    0x04  — policy_ref active; filter metadata on stat
bit 3  FLAG_MIGRATING   0x08  — moving between backends; deny reads until clear
```

---

## Proposed `SexObjectRef`

```rust
/// PROPOSED. Not yet in repo.
/// Cross-PD opaque reference. Carried in PdxMessage.arg0 + arg1. No ABI change.
#[repr(C)]
pub struct SexObjectRef {
    pub object_id: u64,   // arg0
    pub generation: u64,  // arg1
}
// Size: 16 bytes. Fits PdxMessage without modification [VERIFIED PdxMessage fields].
```

---

## What Remains Blocked Before M1

M1 = `crates/sex-object-model/` crate with `SexObjectHeader`, `SexObjectKind`,
`SexObjectRef`, `SexObjectPolicy`, flag constants. **No storage integration in M1.**

| Blocker | Status after OQ answers |
|---|---|
| object_id allocator owner | UNBLOCKED — SexFiles, NEXT_OBJECT_ID |
| SexObjectRef PDX fit | UNBLOCKED — arg0+arg1, no ABI change |
| SceneSnapshot in kind list | UNBLOCKED — excluded |
| DeviceRoute in kind list | UNBLOCKED — excluded |
| BellEvent struct | UNBLOCKED — maps to BellQueueEntry.event_id |
| kind u16 vs u32 constraint | UNBLOCKED — discriminants ≤ 65535; cast on storage |
| `CollarGrant.object_id` unification | NOT blocking M1 — blocking M3 |
| `rights_generation` sync protocol | NOT blocking M1 — blocking M3 |
| `content_generation` new field | NOT blocking M1 — blocking M2 |
| sexshop `object_id → hash` mapping | NOT blocking M1 — blocking M2 |
| `SexObjectPolicy` filtering in SexFiles | NOT blocking M1 — blocking M4+ |

**M1 is unblocked.** All blocking OQs resolved.

---

## Migration Sequence (Updated)

| Step | What | Gate |
|---|---|---|
| **M0** | This doc (V4) — resolve OQs | Doc review ✓ |
| **M1** | `crates/sex-object-model/`: `SexObjectHeader`, `SexObjectKind`, `SexObjectRef`, `SexObjectPolicy`, flag constants. no_std, repr(C), no heap, no PDX ops. | `cargo check -p sex-object-model` |
| **M2** | Extend `SexfilesObjectEntry` → `SexObjectRecord`: add `content_generation`, `parent_ref`, `policy_flags`. Extend `SexfilesObjectEntry.kind` to u32. | `[diskfs.proof.stat_object]` still passes |
| **M3** | Collar: unify `CollarGrant.object_id` with SexFiles `object_id`. Add Collar→SexFiles PDX op to bump `rights_generation` on revocation. | `[collar.grant.match]` + `[sexfiles.caprec.proof.generation_deny]` + new `[sexobject.gen.write_after_revoke]` |
| **M4** | Linen: bind `LinenObjectKind` to `SexObjectKind` discriminants. Query by `object_id` + `kind`. | Visual proof (Linen panel) |
| **M5** | sexshop: `SexObjectKind::Package` — assign sexshop hash into `SexObjectRecord.content_ref`. | `ObjectGet` returns blob for typed object |
| **M6** | Bell: bind `BellQueueEntry.event_id` → `SexObjectHeader.object_id`; emit `SexObjectKind::BellEvent` on `OP_BELL_NOTIFY`. | New `[bell.event.object.emit]` gate |
| **M7** | Quil: bind document buffer → `SexObjectKind::QuilDocument` + `object_id`. | New `[quil.doc.object.bind]` gate |
| **M8** | Spindle: bind session → `SexObjectKind::SpindleSession` + `object_id`. | New `[spindle.session.object.bind]` gate |
| **M9** | Mesh: extend `MeshFactKind` with typed SexObject edge variants. | `[mesh.fact.write]` with new kinds |

---

## Next Implementation Prompt (M1 Only — Do Not Implement Here)

When ready for M1, use this mission:

```
MISSION: SEX_OBJECT_MODEL_CRATE_M1

Create crates/sex-object-model/src/lib.rs.
Do NOT implement. Read V4 handoff first.
Follow sexos_build_spec.toml for crate registration.

STOP FIRST if any of:
- kernel/ edit required
- crates/sex-pdx/ edit required
- servers/ edit required (M1 is model types only)
- heap alloc / std / libc / thread appears
- raw pointer appears in any public type

Contents:
- #![no_std] only
- SexObjectHeader (repr(C), 80 bytes, 9×u64 + 2×u32, see V4 §Updated Object Header)
- SexObjectKind (repr(u32), discriminants ≤ 65535, see V4 §Updated Kind List)
- SexObjectRef (repr(C), 16 bytes, object_id+generation, see V4 §Proposed SexObjectRef)
- SexObjectPolicy (repr(C), 32 bytes, see V3 §Proposed Privacy/Policy)
- FLAG_TOMBSTONED/SEALED/REDACTED/MIGRATING constants (see V4 §Flags)
- Checksum helper fn: XOR over SexObjectHeader fields
- NO pdx_call, NO storage ops, NO server imports

Proof gates required:
- cargo check -p sex-object-model passes
- rg "use std|extern crate std|libc|thread" crates/sex-object-model/ → empty
- rg "kernel|pku|gdt|interrupts" crates/sex-object-model/ → empty
- rg "framebuffer|FRAMEBUFFER" crates/sex-object-model/ → empty
- git diff crates/sex-pdx/ → empty
- git diff kernel/ → empty
- git diff servers/ → empty

Handoff: add build proof to this doc's §Proof Gates section.
```

---

## STOP FIRST Triggers (Unchanged from V3)

- Kernel object model change
- `crates/sex-pdx/` edit (ABI change)
- Broad SexFiles rewrite (not additive)
- sexshop replacement
- POSIX inode / path authority
- App raw disk or framebuffer authority
- Unbounded metadata / plugin execution
- Cross-PD raw pointer
- `sexdisplay` loses sole framebuffer writer status
- MPK/PKU/PKEY model change
- More than two major domains in one patch

---

## Handoff Path

- V4 (this): `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
- V3 (audit baseline): `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V3.md` — keep
- V2 (overclaimed): `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V2.md` — keep for diff
- Next: run M1 prompt above after any future approval
