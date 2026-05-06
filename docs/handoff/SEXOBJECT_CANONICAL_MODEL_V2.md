# SexObject Canonical Model V2

**Date:** 2026-05-06
**Status:** CANONICAL — doc-only, no implementation
**Scope:** Schema/model layer above SexFiles + sexshop. No kernel edits. No PDX ABI edits.

---

## Canon (One Paragraph)

SexObject is the durable, typed, capability-scoped semantic unit of SexOS. Every
meaningful persistent thing in the OS — a Quil document, a Bell event, a Spindle
session, a Linen project, an app manifest, a Mesh fact, a Collar grant, a scene
snapshot, a crash report, a package — is a SexObject. Objects are stored by SexFiles
(disk/ramfs backends) and cached/indexed by sexshop. Authority over objects is governed
by Collar. Relationships between objects are visible in Mesh. Users browse objects in
Linen. Quil edits document objects. Spindle owns session/log objects. Bell emits event
objects. Silk renders scene objects. All cross-PD object references travel over PDX as
opaque `object_id` + generation pairs, never as POSIX paths, raw pointers, or
filesystem inodes.

---

## Non-Goals (MUST NOT drift into these)

| Non-Goal | Why |
|---|---|
| SexObject is NOT a kernel object | Kernel objects are `PD`, `cap_table`, scheduler state — microkernel domain only |
| SexObject does NOT require kernel edits | Schema lives in userland; no kernel ABI change in V1 |
| SexObject does NOT replace SexFiles | SexFiles is the storage backend; SexObject is the semantic type on top |
| SexObject does NOT replace sexshop | sexshop is package/cache store; SexObject is the model sexshop stores |
| SexObject is NOT POSIX inode/path semantics | No path authority, no filesystem hierarchy |
| SexObject does NOT require PDX ABI changes in V1 | object_id referenced as existing `u64` scalar in PDX messages |
| Apps do NOT gain raw disk/framebuffer authority via SexObject | Collar governs all; sexdisplay is sole framebuffer writer |
| No unbounded metadata/plugin/theme execution | Metadata refs are opaque `u64` content refs, not function pointers |
| No cross-PD raw pointers | Only `object_id` + `generation` pairs cross PD boundaries |

---

## Ownership Boundaries

```
Kernel (microkernel)
  └── PD isolation / PDX routing / capability table
        │
        ├── SexFiles (disk/ramfs/tmpfs backends)
        │     └── stores raw object content blocks
        │           SexfilesObjectEntry: object_id, kind, owner_pd,
        │                               rights_generation, checksum
        │
        ├── sexshop (package/object cache/store)
        │     └── ObjectPut / ObjectGet / ObjectExists (hash-addressed)
        │           StoreProtocol: hash, data_paddr, data_len
        │
        ├── SexObject model layer  ← defined here, no separate server yet
        │     └── SexObjectHeader: typed metadata above storage records
        │
        ├── Collar (silk-shell CollarGrant / CollarAuditEvent)
        │     └── governs authority over SexObjects by grant_id/generation
        │
        ├── Mesh (silk-shell MeshFact ring)
        │     └── records object/capability edges as typed facts
        │
        ├── Linen (project/session browser)
        │     └── surfaces SexObjects via LinenObjectKind
        │
        ├── Quil (document editor) — edits QuilDocument SexObjects
        ├── Spindle (terminal/session manager) — owns SpindleSession SexObjects
        ├── Bell (notification server) — emits BellEvent SexObjects
        └── Silk (compositor/shell) — renders SceneSnapshot SexObjects
```

---

## Object Header (Fixed-Size, No Heap)

This is the canonical schema. Lives in a shared `no_std` model crate when implementation
becomes safe. All fields are `u64`/`u32` scalars — no pointers, no strings, no slices.

```rust
/// Fixed-size SexObject header. repr(C), no_std, no heap.
/// Stored alongside SexfilesObjectEntry; content at content_ref block.
#[repr(C)]
pub struct SexObjectHeader {
    /// Globally unique object ID (monotonic, assigned by SexFiles on create).
    pub object_id: u64,
    /// Write generation: incremented on every mutation. 0 = never written.
    /// Cross-PD references carry generation; stale generation = denied access.
    pub generation: u64,
    /// Object semantic type. Maps to SexObjectKind discriminant.
    pub kind: u32,
    /// PD that owns this object. Only owner PD may grant/revoke Collar authority.
    pub owner_pd: u32,
    /// Generation of the Collar rights record last applied to this object.
    /// Collar uses this to detect stale-cap injection (matches SexfilesObjectEntry.rights_generation).
    pub rights_generation: u64,
    /// Block reference to object content in SexFiles. Opaque to model layer.
    pub content_ref: u64,
    /// Block reference to structured metadata (kind-specific, fixed schema per kind).
    pub metadata_ref: u64,
    /// Block reference to policy/privacy record (redaction flags, retention policy).
    pub policy_ref: u64,
    /// Block reference to parent object (e.g. LinenProject → QuilDocument, 0 = root).
    pub parent_ref: u64,
    /// XOR/CRC checksum over header fields (matches DiskFs journal checksum scheme).
    pub checksum: u64,
    /// Flags bitfield: bit 0 = deleted/tombstoned, bit 1 = sealed (immutable),
    /// bit 2 = redacted (policy_ref active), bit 3 = migrating.
    pub flags: u64,
}
```

**Checksum note:** Uses same scheme as `SexfilesObjectEntry.checksum` and
`JournalRecord.checksum`. Model layer verifies checksum on read; rejects mismatched
records (matches existing `[sexfiles.journal.proof.checksum_reject]` proof gate).

**generation note:** Matches `SexfilesObjectEntry.rights_generation` /
`sexstore KvSlot.generation` semantics. Cross-PD refs carry generation at bind time;
mismatch = access denied (matches existing `[sexfiles.caprec.proof.generation_deny]`).

---

## Object Kind List (Bounded V1)

Discriminant values are stable. Gaps intentional — reserved for future kinds without
renumbering.

```rust
#[repr(u32)]
pub enum SexObjectKind {
    RawBlob         = 0,   // Untyped content block
    AppManifest     = 1,   // silk-shell AppManifest (capability bits, ABI version)
    AppState        = 2,   // Persisted app state blob
    LinenProject    = 3,   // Linen LinenObjectKind::Project
    QuilDocument    = 4,   // Quil document buffer
    SpindleSession  = 5,   // Spindle terminal/session/log
    BellEvent       = 6,   // Bell OP_BELL_NOTIFY output
    SceneSnapshot   = 7,   // Silk sex-pdx SceneSnapshot
    CollarGrant     = 8,   // Collar CollarGrant record
    MeshFact        = 9,   // Mesh MeshFact edge record
    CrashReport     = 10,  // Kernel/server crash dump
    Package         = 11,  // sexshop package object
}
```

**Mapping to existing types:**

| SexObjectKind | Existing type | Location |
|---|---|---|
| AppManifest | `silk_shell::AppManifest` | `servers/silk-shell/src/lib.rs:95` |
| LinenProject | `LinenObjectKind::Project` | `servers/silk-shell/src/main.rs:280` |
| QuilDocument | `LinenObjectKind::Document` | `servers/silk-shell/src/main.rs:281` |
| BellEvent | `OP_BELL_NOTIFY` target | `crates/sex-pdx/src/lib.rs:106` |
| SceneSnapshot | `sex_pdx::SceneSnapshot` | `crates/sex-pdx/src/lib.rs:167` |
| CollarGrant | `CollarGrant` struct | `servers/silk-shell/src/main.rs:1404` |
| MeshFact | `MeshFact` struct | `servers/silk-shell/src/main.rs:1609` |
| Package | `StoreProtocol::ObjectPut/Get` | `servers/sexshop/src/pdx.rs:54` |

---

## Relation / Edge Model

Relations are first-class MeshFact records, not header pointers. `parent_ref` in the
header is the only structural hierarchy pointer; all semantic edges go through Mesh.

```
SexObject A  ──[MeshFact: kind=ObjectLinkedToBuffer]──►  SexObject B
                subject_id = A.object_id
                object_id  = B.object_id   (overloaded field name — Mesh internal)
                ref_id     = linked surface / session id
```

V1 Mesh only has `MeshFactKind::ObjectLinkedToBuffer`. Future kinds (e.g.
`GrantLinkedToObject`, `EventEmittedByApp`) extend the enum without schema change.

---

## Generation / Revocation Model

Matches existing sexfiles cap-record proof semantics exactly:

1. Object created → `generation = 1`, `rights_generation = 1`.
2. Every write → `generation` bumped (never 0, wraps 255→1 for `u8`, unbounded `u64`
   for header).
3. Cross-PD reference binds `(object_id, generation)` pair at grant time.
4. On access: SexFiles compares `stored.rights_generation` vs `cap.generation`. Mismatch
   → denied. (Proof gate: `[sexfiles.caprec.proof.generation_deny]`.)
5. Revocation: Collar marks `CollarGrant.state = Revoked` + bumps
   `rights_generation`. All stale caps denied on next access.
6. Deletion: `flags |= FLAG_TOMBSTONED` + generation bump. SexFiles journal writes
   tombstone record. (Matches `sexstore` tombstone semantics.)

```
FLAG_TOMBSTONED = 0x01
FLAG_SEALED     = 0x02   // immutable; further writes denied
FLAG_REDACTED   = 0x04   // policy_ref active; metadata reads filtered
FLAG_MIGRATING  = 0x08   // object being moved between backends
```

---

## Integrity / Checksum Model

Header checksum covers: `object_id ^ generation ^ kind ^ owner_pd ^ rights_generation ^
content_ref ^ metadata_ref ^ policy_ref ^ parent_ref ^ flags`. Same XOR scheme as
`JournalRecord.checksum`. On read, model layer recomputes and rejects mismatch.
Matches proof gate `[sexfiles.journal.proof.checksum_reject]`.

Content checksum (hash-addressed in sexshop): `StoreProtocol::ObjectPut { hash, ... }`
— content addressed by hash. SexObject `content_ref` points to SexFiles block; sexshop
holds the content-addressed blob. These are complementary, not competing.

---

## Privacy / Redaction Metadata

When `FLAG_REDACTED` set, `policy_ref` block contains a fixed-size policy record:

```rust
#[repr(C)]
pub struct SexObjectPolicy {
    pub redact_fields_mask: u64,  // bitmask of header fields suppressed in list/stat
    pub retention_epoch: u64,     // delete after this monotonic epoch (0 = never)
    pub visibility_mask: u32,     // which PDs may see object in Linen/Mesh listings
    pub _pad: u32,
    pub checksum: u64,
}
```

Apps never read `policy_ref` directly. SexFiles filters stat results before crossing
PDX. Collar decides `visibility_mask` from `AppCapabilityBits`.

---

## Capability Binding Rules

1. App requests capability via `AppManifest.capabilities` (e.g. `AppCapabilityBits::SEXFILES`).
2. Collar reviews manifest (`collar_review_manifest`) → creates `CollarGrant` with
   `operation_mask` covering allowed ops.
3. App receives `(object_id, grant_id, generation)` tuple over PDX — no raw pointer.
4. On every PDX op touching an object: SexFiles checks `rights_generation` matches
   active Collar grant generation. Mismatch → denied.
5. Revocation: Collar bumps grant generation → all extant caps stale on next use.
6. `AppCapabilityBits::BELL` → may emit BellEvent SexObjects.
   `AppCapabilityBits::SEXFILES` → may read/write SexObjects in granted scope.
   No bit → read-only access to own objects only.

---

## Migration Path (Server by Server)

No implementation required in V1. Migration is additive — existing types gain
`object_id` + `generation` fields; SexFiles backend gains `create_object_entry` calls.
Each step is independently provable via existing proof gates.

| Step | Server | Change | Proof gate |
|---|---|---|---|
| M1 | SexFiles | `SexfilesObjectEntry` already has `object_id`, `kind`, `owner_pd`, `rights_generation`, `checksum` — already aligned | `[diskfs.proof.stat_object]` |
| M2 | silk-shell | `LinenObjectKind` maps 1:1 to `SexObjectKind` subset — no change | `[linen.object.kind]` (existing) |
| M3 | silk-shell | `CollarGrant.object_id` + `generation` already exist — already aligned | `[collar.grant.revoke]` proof |
| M4 | silk-shell | `MeshFact` gets `SexObjectKind` typed `subject_kind`/`object_kind` fields | `[mesh.fact.record]` proof |
| M5 | sexshop | `ObjectPut` hash becomes `content_ref` in SexObject header | `[sexshop.object.put]` proof |
| M6 | Bell | BellEvent emits `SexObjectHeader` with `kind=BellEvent` instead of raw scalar | `[bell.event.emit]` proof |
| M7 | Quil | Document buffer gains `object_id` + `generation` binding to SexFiles entry | `[quil.doc.bind]` proof |
| M8 | Spindle | Session gains `object_id` binding; log is SexObject with `kind=SpindleSession` | `[spindle.session.bind]` proof |
| M9 | Linen | Project view queries by `object_id` + `kind` rather than name string | `[linen.project.query]` proof |

---

## Smallest Safe Implementation Location

When implementation becomes safe:

1. **New crate:** `crates/sex-object-model/src/lib.rs`
   - `#![no_std]`
   - Only contains: `SexObjectHeader`, `SexObjectKind`, `SexObjectPolicy`, flag constants
   - Fixed-size, `repr(C)`, no heap, no alloc
   - No PDX ops — model types only
   - Dependency: none (no sex-pdx import in V1)

2. **Size bound:** Header = 11 × u64 + 2 × u32 = 96 bytes. Policy = 3 × u64 + 2 × u32
   = 32 bytes. Both fit in single cache line or PDX message payload.

3. **Additive only:** Servers import model crate; no existing struct is removed.
   Existing `SexfilesObjectEntry` fields are a strict subset of `SexObjectHeader` —
   migration is field-by-field, not a rewrite.

---

## Proof Gates

| Gate | What it verifies |
|---|---|
| Build proof | `cargo check -p sex-object-model` passes with `no_std` |
| Grep proof | `rg "kernel\|pku\|gdt\|interrupts" crates/sex-object-model/` → empty |
| No std proof | `rg "use std\|extern crate std\|libc\|thread" crates/sex-object-model/` → empty |
| No renderer drift | `rg "framebuffer\|FRAMEBUFFER" crates/sex-object-model/` → empty |
| No PDX ABI edit | `git diff crates/sex-pdx/` → empty |
| No kernel edit | `git diff kernel/` → empty |
| Handoff updated | This file exists at `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V2.md` |

---

## STOP FIRST Triggers

Halt and require explicit user approval before any implementation if:

- Kernel object model changes suggested
- `sex-pdx` ABI edit required
- POSIX inode/path semantics appear in design
- SexFiles rewrite (not additive extension)
- sexshop replacement (not additive)
- App raw disk/framebuffer authority appears
- Unbounded metadata map / plugin execution
- Cross-PD raw pointers appear
- Scope touches more than two major domains simultaneously

---

## Why This Improves SexOS

Normal OS: file + app state + window + notification + permission + log + process =
scattered systems with inconsistent authority models, no shared revocation, no
capability genealogy.

SexOS with SexObject: every meaningful durable thing is a typed, capability-scoped,
generation-versioned record. Collar can revoke authority over any object class
uniformly. Mesh can explain any object relationship. Linen can browse any object kind.
Bell can reference any object as an event target. No object escapes capability
governance. No object requires a POSIX path. No object requires raw disk authority.

The OS becomes a living graph of capability-scoped objects — stored by SexFiles,
governed by Collar, explained by Mesh, browsed by Linen, edited by Quil, controlled by
Spindle, alerted from by Bell, displayed by Silk.
