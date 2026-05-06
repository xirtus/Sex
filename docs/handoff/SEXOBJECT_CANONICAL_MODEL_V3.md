# SexObject Canonical Model V3

**Date:** 2026-05-06
**Status:** PROPOSED / DOC-ONLY / AUDITED CLAIMS ONLY
**Replaces:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V2.md (overclaimed)
**Scope:** Schema/model layer above SexFiles + sexshop. No kernel edits. No PDX ABI edits.
**Implementation:** None yet. Model crate not created.

---

## What This Document Is

A design intent document. Every claim is marked:

- **[VERIFIED]** — confirmed by `rg` in this session against current repo state
- **[PROPOSED]** — directional intent not yet implemented
- **[OPEN]** — unresolved before implementation can proceed

This document is NOT canonical until all [OPEN] questions resolve and a
model crate build proof exists.

---

## V2 Overclaims Corrected

| V2 Claim | Correction |
|---|---|
| Status: "CANONICAL" | Status: PROPOSED. No proof yet. |
| `content_ref`, `metadata_ref`, `policy_ref`, `parent_ref` exist | **[PROPOSED only]** — not present in any file |
| `FLAG_TOMBSTONED/SEALED/REDACTED/MIGRATING` exist | **[PROPOSED only]** — not present in any file |
| `SexObjectHeader`, `SexObjectKind`, `SexObjectPolicy` exist | **[PROPOSED only]** — not present in any file |
| `[collar.grant.revoke]` proof gate exists | **False.** Real gates: `[collar.grant.match]`, `[collar.audit.write]` |
| `[bell.event.emit]`, `[quil.doc.bind]`, `[linen.object.kind]`, `[sexshop.object.put]` exist | **False.** None present in repo |
| `BellEvent` struct exists | **False.** Only `OP_BELL_NOTIFY = 0xC0` opcode at `crates/sex-pdx/src/lib.rs:106` |
| `SceneSnapshot` is a clean value SexObject | **Caution.** `SceneSnapshot` has `layers_ptr:u64`, `damage_rects_ptr:u64` raw ptr fields. Not a pure value type. |
| "Every meaningful persistent thing is a SexObject" | **Future intent, not current truth.** |
| Header size: 11 × u64 + 2 × u32 = 96 bytes | **Wrong.** 9 × u64 + 2 × u32 = 80 bytes (see below) |
| "already aligned" migration claims for all servers | Partially true for SexFiles; unverified for others |
| generation model unified | V2 conflated three distinct generation counters (see §Generation Split) |

---

## Concept (Intent)

SexObject is the **planned** canonical durable typed semantic unit of SexOS. The intent:
every meaningful persistent thing — a document, an event, a session, a manifest, a
grant, a scene — is a SexObject with a typed header, a capability-scoped identity, and
a revocable capability binding. Objects are stored by SexFiles, cached/indexed by
sexshop, governed by Collar, visible in Mesh, browsed by Linen, referenced over PDX by
`(object_id, generation)` — never by POSIX path or raw pointer.

This is a design goal, not current OS state.

---

## Non-Goals (Hard)

| Non-Goal | Reason |
|---|---|
| SexObject is NOT a kernel object | PD/cap_table are microkernel-only; STOP FIRST if kernel edit required |
| No PDX ABI change in V1 | `object_id` referenced as existing `u64` scalar; no new message fields |
| Does NOT replace SexFiles | SexFiles is storage; SexObject is the semantic model above it |
| Does NOT replace sexshop | sexshop is package/content-addressed store; role is [OPEN] (see §Open Questions) |
| NOT POSIX inode/path semantics | No path authority; no filesystem hierarchy |
| No raw disk/framebuffer authority via SexObject | Collar governs all; sexdisplay is sole framebuffer writer |
| No unbounded metadata execution | `metadata_ref` is a storage block ref, not a function pointer |
| No cross-PD raw pointers | Only `(object_id, generation)` pairs cross PD boundaries |
| No broad SexFiles rewrite | Extensions are additive only |

---

## Verified Existing Foundation

### SexFiles — `servers/sexfiles/src/backends/diskfs.rs:25` [VERIFIED]

```rust
pub struct SexfilesObjectEntry {
    pub object_id: u64,
    pub kind: u16,              // currently untyped — matches SexObjectKind discriminant space
    pub owner_pd: u32,
    pub rights_generation: u64, // capability/revocation epoch
    pub object_size_bytes: u64,
    pub first_block: u64,
    pub metadata_generation: u64, // metadata-write epoch (separate from rights_generation)
    pub checksum: u32,
    pub in_use: bool,
}
```

**Checksum scheme** `[VERIFIED]`: XOR over `object_id`, `kind`, `owner_pd`,
`rights_generation`, `object_size_bytes`, `first_block`, `metadata_generation`.
Gate: `[sexfiles.journal.proof.checksum_reject]` (`servers/sexfiles/src/proof.rs:130`)

**Object create/stat gates** `[VERIFIED]`:
- `[diskfs.proof.create_object]` (`proof.rs:73`)
- `[diskfs.proof.stat_object]` (`proof.rs:77`)

**Cap revocation gate** `[VERIFIED]`:
- `[sexfiles.caprec.proof.generation_deny]` (`proof.rs:178, 203, 243`)

### Collar — `servers/silk-shell/src/main.rs:1404` [VERIFIED]

```rust
struct CollarGrant {
    grant_id: u64,
    subject_id: u64,
    object_id: u64,       // references a Linen/SexObject local ID (not yet SexfilesObjectEntry.object_id)
    operation_mask: u64,
    generation: u64,      // collar-grant epoch; distinct from SexFiles rights_generation
    state: CollarGrantState, // Active / Revoked / Expired / Tombstoned
}
```

Real proof gates `[VERIFIED]`: `[collar.grant.match]`, `[collar.audit.write]`,
`[collar.audit.overwrite]`. No `[collar.grant.revoke]` gate exists.

**[OPEN]:** `CollarGrant.object_id` currently references silk-shell-local Linen object
indices, NOT `SexfilesObjectEntry.object_id`. Binding these is a required migration step.

### Mesh — `servers/silk-shell/src/main.rs:1609` [VERIFIED]

```rust
struct MeshFact {
    fact_id: u64,
    kind: MeshFactKind,           // V1: only ObjectLinkedToBuffer
    subject_id: u64,
    object_id: u64,               // overloaded field name — Mesh-internal semantics
    ref_id: u64,
    sequence: u64,
}
```

Real proof gates `[VERIFIED]`: `[mesh.fact.write]`, `[mesh.fact.done]`,
`[mesh.fact.overwrite]`.

**[OPEN]:** `MeshFact.subject_id` / `object_id` currently reference silk-shell-local
indices. Binding to `SexfilesObjectEntry.object_id` requires migration.

### LinenObjectKind — `servers/silk-shell/src/main.rs:280` [VERIFIED]

```rust
enum LinenObjectKind {
    Project = 0, Document = 1, CodeFile = 2, MediaAsset = 3,
    BuildArtifact = 4, Folder = 5, Reference = 6, ImportPlaceholder = 7,
    BellEventReference = 8, QuilWorkspaceReference = 9, MeshDiagnosticReference = 10,
}
```

### AppManifest — `servers/silk-shell/src/lib.rs:95` [VERIFIED]

Fields include `capabilities: AppCapabilityBits`. Bits `BELL` and `SEXFILES` verified.

### Bell — `crates/sex-pdx/src/lib.rs:106` [VERIFIED]

`OP_BELL_NOTIFY = 0xC0` exists. **No `BellEvent` struct exists.** Comment says "request
to create a BellEvent" — the struct is planned, not present.

### sexshop — `servers/sexshop/src/pdx.rs:54` [VERIFIED]

`StoreProtocol::ObjectPut { hash, data_paddr, data_len }`, `ObjectGet`, `ObjectExists`,
`ObjectMove` are real ops. Also has `FetchPackage`, `CacheBinary`, `KVGet`, `KVSet`,
`KVDelete`, `TransactionBegin/Commit/Abort`, `SyncFilesystem`, `Stats`. **[OPEN]:**
Whether sexshop is content-addressed blob store, package cache, or KV store is
ambiguous — it implements all three. Role relative to SexObject is unresolved.

### SceneSnapshot — `crates/sex-pdx/src/lib.rs:167` [VERIFIED — caution]

```rust
pub struct SceneSnapshot {
    pub layers_ptr: u64,       // raw pointer field (as u64 in PDX)
    pub layers_len: u32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub is_incremental: u32,
    pub damage_rects_ptr: u64, // raw pointer field
    pub damage_rects_len: u32,
}
```

**[OPEN]:** `SceneSnapshot` is NOT a clean value-type SexObject. It carries raw pointer
fields. It cannot be stored as a SexObject without serializing out the pointer content.
`SceneSnapshot` as a SexObjectKind requires design work before inclusion.

---

## Proposed Object Header [PROPOSED — does not exist yet]

This is a design sketch, not a type in the repo. All fields are proposed.

```rust
/// PROPOSED. Not yet in repo.
/// Fixed-size, repr(C), no_std, no heap, no pointers.
#[repr(C)]
pub struct SexObjectHeader {
    pub object_id: u64,          // globally unique; assigned by SexFiles on create
    pub generation: u64,         // content/mutation epoch (see §Generation Split)
    pub kind: u32,               // SexObjectKind discriminant
    pub owner_pd: u32,           // PD that created/owns this object
    pub rights_generation: u64,  // capability/revocation epoch (matches SexfilesObjectEntry.rights_generation)
    pub content_ref: u64,        // opaque storage block ref (SexFiles first_block or sexshop hash)
    pub metadata_ref: u64,       // opaque ref to kind-specific metadata block
    pub policy_ref: u64,         // opaque ref to policy/privacy record (0 = none)
    pub parent_ref: u64,         // object_id of parent (0 = root object)
    pub checksum: u64,           // XOR over all other fields (not u32 — wider than SexfilesObjectEntry)
    pub flags: u64,              // tombstone/sealed/redacted/migrating bits (see §Flags)
}
// Size: 9 × u64 + 2 × u32 = 72 + 8 = 80 bytes. repr(C), no padding (u32 pair is 8 bytes aligned).
```

**Relation to `SexfilesObjectEntry`:** `SexObjectHeader` is a proposed superset.
`SexfilesObjectEntry` already provides `object_id`, `kind`, `owner_pd`,
`rights_generation`, `checksum` — these align. `content_ref` maps to `first_block`.
`metadata_ref` maps to a proposed new block ref. `metadata_generation` exists in
`SexfilesObjectEntry` but has no parallel in proposed header yet — this is a gap.

---

## Generation Split [PROPOSED — clarifies V2 conflation]

Three distinct generation counters exist or are proposed:

| Counter | Location | Semantics |
|---|---|---|
| `SexfilesObjectEntry.rights_generation` | **[VERIFIED]** SexFiles | Capability/revocation epoch. Bumped on Collar revocation. Stale cap → deny. |
| `SexfilesObjectEntry.metadata_generation` | **[VERIFIED]** SexFiles | Metadata-write epoch. Bumped on metadata update. |
| `JournalRecord.generation` / `fs_generation` | **[VERIFIED]** SexFiles superblock | Filesystem-level write epoch. Per-transaction. |
| `CollarGrant.generation` | **[VERIFIED]** silk-shell | Grant-level epoch. Distinct from SexFiles rights_generation. |
| `SexObjectHeader.generation` | **[PROPOSED]** | Content/mutation epoch. Would track object payload version. |

**V2 error:** called all of these "generation" and implied they were unified.
They are not. Binding `SexObjectHeader.generation` to `SexfilesObjectEntry` requires
deciding which counter maps to which field. **[OPEN]**

---

## Proposed Kind List [PROPOSED — no SexObjectKind type exists yet]

Stable discriminant values; gaps reserved.

| Kind | Value | Source type | Status |
|---|---|---|---|
| `RawBlob` | 0 | none | [PROPOSED] |
| `AppManifest` | 1 | `silk_shell::AppManifest` (`silk-shell/src/lib.rs:95`) | [VERIFIED source; binding PROPOSED] |
| `AppState` | 2 | none | [PROPOSED] |
| `LinenProject` | 3 | `LinenObjectKind::Project` (`silk-shell/src/main.rs:280`) | [VERIFIED source; binding PROPOSED] |
| `QuilDocument` | 4 | `LinenObjectKind::Document` | [VERIFIED source; binding PROPOSED] |
| `SpindleSession` | 5 | none | [PROPOSED] |
| `BellEvent` | 6 | opcode `OP_BELL_NOTIFY` only — **no struct** | [PROPOSED struct] |
| `SceneSnapshot` | 7 | `sex_pdx::SceneSnapshot` — **has raw ptr fields** | [OPEN — needs design] |
| `CollarGrant` | 8 | `CollarGrant` (`silk-shell/src/main.rs:1404`) | [VERIFIED source; binding PROPOSED] |
| `MeshFact` | 9 | `MeshFact` (`silk-shell/src/main.rs:1609`) | [VERIFIED source; binding PROPOSED] |
| `CrashReport` | 10 | none | [PROPOSED] |
| `Package` | 11 | `StoreProtocol::ObjectPut/Get` | [VERIFIED protocol; binding PROPOSED] |

**`DeviceRoute` omitted from V1 kind list** — no existing device route type found. Add after hardware layer stabilizes.

---

## Proposed Flags [PROPOSED — none exist yet]

```
bit 0  FLAG_TOMBSTONED  — deleted; deny all writes; generation still readable
bit 1  FLAG_SEALED      — immutable after seal; deny further content writes
bit 2  FLAG_REDACTED    — policy_ref active; filter metadata on stat
bit 3  FLAG_MIGRATING   — object being moved between backends; deny reads until clear
```

---

## Proposed Relation / Edge Model [PROPOSED]

Extends `MeshFact` with typed kind-to-kind edges. V1 Mesh only has
`MeshFactKind::ObjectLinkedToBuffer` (`[VERIFIED]`). Proposed extension: additional
`MeshFactKind` variants with `SexObjectKind`-typed subject/object fields. No
`MeshFact` struct change required in V1 — discriminants suffice.

`parent_ref` in header is the only structural hierarchy link; all semantic edges go
through Mesh facts.

---

## Proposed Capability Binding [PROPOSED]

1. App declares `AppCapabilityBits` in `AppManifest` `[VERIFIED]`.
2. Collar reviews and creates `CollarGrant` with `operation_mask` `[VERIFIED]`.
3. App receives `(object_id, grant_id, generation)` over PDX — no raw pointer.
4. On PDX op: SexFiles checks `rights_generation` vs Collar grant epoch.
   Gate: `[sexfiles.caprec.proof.generation_deny]` `[VERIFIED]`.
5. Revocation: Collar sets `grant.state = Revoked`, bumps `rights_generation`.
   Gate: `[collar.audit.write]` `[VERIFIED]` — no dedicated revoke gate yet.

**[OPEN]:** `CollarGrant.object_id` must be unified with `SexfilesObjectEntry.object_id`
before step 4 is meaningful. Currently they are separate ID spaces.

---

## Proposed Privacy / Policy [PROPOSED — SexObjectPolicy does not exist]

When `FLAG_REDACTED` set, `policy_ref` block contains a proposed fixed-size record:

```rust
/// PROPOSED. Not yet in repo.
#[repr(C)]
pub struct SexObjectPolicy {
    pub redact_fields_mask: u64,  // bitmask: which header fields suppressed in list/stat
    pub retention_epoch: u64,     // delete after this monotonic epoch; 0 = never
    pub visibility_mask: u32,     // which PDs may see object in Linen/Mesh listings
    pub _pad: u32,
    pub checksum: u64,
}
// Size: 3 × u64 + 2 × u32 = 24 + 8 = 32 bytes.
```

Apps never read `policy_ref` directly. SexFiles would filter stat results before
crossing PDX. **[OPEN]:** No SexFiles filtering path exists yet.

---

## Open Questions Before Implementation

| # | Question | Blocking |
|---|---|---|
| OQ1 | Who allocates `object_id`? SexFiles on `create_object_entry`? Monotonic counter per PD? Global? | Model crate |
| OQ2 | Where is `SexObjectHeader` stored? Alongside `SexfilesObjectEntry`? Separate block? Embedded? | SexFiles binding |
| OQ3 | Does sexshop store typed SexObject blobs, content-addressed opaque blobs, packages, or all three? What is its V1 role? | sexshop binding |
| OQ4 | Does `rights_generation` live in SexFiles only, Collar only, or both with a sync protocol? | Collar binding |
| OQ5 | `CollarGrant.object_id` vs `SexfilesObjectEntry.object_id` — same ID space or mapped? | Collar binding |
| OQ6 | Can `(object_id, generation)` fit in existing PDX message fields without ABI change? | PDX V1 |
| OQ7 | What are the exact negative tests for stale generation / revocation? Need proof gates beyond `[sexfiles.caprec.proof.generation_deny]`. | Collar binding |
| OQ8 | `SceneSnapshot` has raw ptr fields — how is it serialized as a SexObject? Or is it excluded from V1? | Kind list |
| OQ9 | `BellEvent` struct is mentioned in `OP_BELL_NOTIFY` comment but does not exist. Who owns it? sexbell server? | Bell binding |
| OQ10 | Does `SexObjectHeader.generation` (content mutation) map to `metadata_generation` or a new field in `SexfilesObjectEntry`? | SexFiles binding |
| OQ11 | `DeviceRoute` — include in V1 kind list or defer? No existing device route type found. | Kind list |

---

## Migration Sequence [PROPOSED — starts docs-only]

Each step is independent and gated. No step starts until prior step has a build proof.

| Step | What | Output | Gate |
|---|---|---|---|
| M0 | Resolve OQ1–OQ11 | Answers in V4 of this doc | Doc review |
| M1 | Create `crates/sex-object-model/` with `SexObjectHeader`, `SexObjectKind`, `SexObjectPolicy` | Crate compiles `no_std` | `cargo check -p sex-object-model` |
| M2 | SexFiles: extend `SexfilesObjectEntry` with header block ref or embed header fields | Extended entry compiles | `[diskfs.proof.stat_object]` still passes |
| M3 | Collar: unify `CollarGrant.object_id` with `SexfilesObjectEntry.object_id` | Collar check passes | `[collar.grant.match]` + `[sexfiles.caprec.proof.generation_deny]` |
| M4 | Linen: bind `LinenObjectKind` to `SexObjectKind` discriminants | Linen renders typed objects | Visual proof (Linen panel) |
| M5 | sexshop: assign `SexObjectKind::Package` to `ObjectPut` blobs | Package object retrievable | `ObjectGet` returns typed blob |
| M6 | Bell: define `BellEvent` struct; bind `OP_BELL_NOTIFY` handler to emit `SexObjectKind::BellEvent` | Bell proof gate | New `[bell.event.object.emit]` gate |
| M7 | Quil: bind document buffer to `SexObjectKind::QuilDocument` + `object_id` | Quil proof gate | New `[quil.doc.object.bind]` gate |
| M8 | Spindle: bind session to `SexObjectKind::SpindleSession` + `object_id` | Spindle proof gate | New `[spindle.session.object.bind]` gate |
| M9 | Mesh: extend `MeshFactKind` with typed SexObject edge variants | Mesh shows typed edges | `[mesh.fact.write]` with new kinds |

---

## Proposed Proof Gates (Required Future, Not Yet Verified)

| Gate | What it verifies |
|---|---|
| `cargo check -p sex-object-model` | Model crate compiles `no_std`, no `std`/`libc`/`thread` import |
| `rg "kernel\|pku\|gdt\|interrupts" crates/sex-object-model/` → empty | No kernel drift |
| `rg "framebuffer\|FRAMEBUFFER" crates/sex-object-model/` → empty | No renderer ownership drift |
| `git diff crates/sex-pdx/` → empty | No PDX ABI change |
| `git diff kernel/` → empty | No kernel edit |
| `[diskfs.proof.stat_object]` still emits `ok=1` | SexFiles extension didn't break stat |
| `[sexfiles.caprec.proof.generation_deny]` still emits `ok=1` | Revocation still works |
| `[collar.grant.match]` still emits for valid ops | Collar unification didn't break grants |

**Already-verified gates** (existing, not new):
`[diskfs.proof.create_object]`, `[diskfs.proof.stat_object]`,
`[sexfiles.journal.proof.checksum_reject]`, `[sexfiles.caprec.proof.generation_deny]`,
`[collar.grant.match]`, `[collar.audit.write]`, `[mesh.fact.write]`

---

## STOP FIRST Triggers

Halt before any implementation step if:

- Kernel object model change required
- `crates/sex-pdx/` edit required (ABI change)
- Broad `SexFiles` rewrite (not additive extension)
- `sexshop` replacement (not additive)
- POSIX inode / path authority appears in design
- App gains raw disk or framebuffer authority
- Unbounded metadata map / plugin execution appears
- Cross-PD raw pointer appears
- `sexdisplay` is no longer sole framebuffer writer
- MPK/PKU/PKEY model changes
- More than two major domains touched in one patch

---

## Why SexObject Matters (Intent)

Normal OS: file + app state + window + notification + permission + log + process =
scattered, authority-inconsistent systems. Revocation hits one system; other systems
keep stale state.

SexOS intent: every meaningful durable thing is a `(object_id, kind, generation)`
triple. Collar revokes authority uniformly across all kinds. Mesh explains any
object relationship. Linen browses any kind. One revocation mechanism, one identity
space, one capability model — no POSIX path escapes, no raw disk authority, no
framebuffer ownership drift.

This is worth building. It is not built yet.

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V3.md`
- Previous (overclaimed): `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V2.md` — keep for diff reference
- Next step: resolve OQ1–OQ11, produce V4 with answers, then M1 model crate prompt
- Do NOT start M1 until OQ1–OQ4 answered
