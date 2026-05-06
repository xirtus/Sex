# SexObject Linen View Adapter — M4 Handoff

**Date:** 2026-05-06
**Status:** COMPLETE — build proof verified
**Spec:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md §M4

---

## What Was Added

| File | Change |
|---|---|
| `servers/linen/Cargo.toml` | Added `sex-object-model = { path = "../../crates/sex-object-model" }` |
| `servers/linen/src/sexobject.rs` | New module — kind map + view adapter + size assertions |
| `servers/linen/src/main.rs` | Added `mod sexobject;` |

**Files NOT touched:** `kernel/`, `crates/sex-pdx/`, any other server, any disk format,
any PDX opcode, any Linen handler, any Collar/SexFiles/sexshop behavior.
`servers/linen/src/session.rs` modification is pre-existing (before this session).

---

## Adapter

**Location:** `servers/linen/src/sexobject.rs`

**Functions (all `#[allow(dead_code)]` — no callers yet):**

```rust
pub const fn linen_kind_to_sex(kind: ObjectKind) -> SexObjectKind
pub fn sexobject_header_from_linen(obj: &LinenObject) -> SexObjectHeader
pub fn linen_object_ref(obj: &LinenObject) -> SexObjectRef
```

Pure, no allocation, no mutation, no I/O, no disk write, no PDX op.

---

## Kind Mapping (Approximate — Documented)

Linen's `ObjectKind` has 3 coarse variants. `SexObjectKind` has 12.
Mapping is intentionally approximate in V1.

| `ObjectKind` (Linen) | `SexObjectKind` | Rationale |
|---|---|---|
| `Document = 0` | `QuilDocument = 4` | Linen documents are Quil-editable content |
| `Session = 1` | `SpindleSession = 5` | Linen sessions map to terminal/log sessions |
| `Unknown = 2` | `RawBlob = 0` | Fallback for untyped content |

This mapping will refine when Linen adopts the richer `LinenObjectKind` from
silk-shell (11 variants), which maps 1:1 to `SexObjectKind`. That is M4+ scope.

---

## Field Mapping Table

| `SexObjectHeader` field | Source | V1 Gap |
|---|---|---|
| `object_id` | `obj.object_id` | **Linen-local monotonic ID — NOT SexFiles NEXT_OBJECT_ID.** Two separate namespaces. Unification is future work (same gap as OQ5 for Collar). |
| `content_generation` | `obj.generation` | Direct — `LinenObject` already tracks `generation: u64` |
| `rights_generation` | `0` | Collar not bound yet (M3) |
| `metadata_generation` | `obj.generation` | Direct proxy |
| `object_size_bytes` | `0` | Linen doesn't track content size |
| `first_block` | `0` | `ramfs_handle` is a RamFS file handle, NOT a block ref. Would be wrong to put here. |
| `owner_pd` | `obj.owner_pd` | Direct |
| `kind` | `linen_kind_to_sex(obj.kind) as u32` | Approximate kind mapping (see §Kind Mapping) |
| `checksum` | `0` | Linen doesn't compute XOR checksums — SexFiles does |
| `flags` | `0` | `LinenObject.flags: u8` bit 0 (persisted) ≠ `SexObjectHeader.flags: u32` FLAG_* bits. Not mapped to avoid semantic confusion. |
| `reserved0` | `0` | V1 |
| `reserved1` | `0` | V1 |

---

## ID Space Gap (Important)

`LinenObject.object_id` is allocated by Linen's own `Session.next_id: u64` counter.
`SexfilesObjectEntry.object_id` is allocated by `NEXT_OBJECT_ID: AtomicU64` in SexFiles.
These are independent. A Linen `object_id = 5` does NOT refer to the same object as a
SexFiles `object_id = 5`. This is the same fundamental gap as `CollarGrant.object_id`
(OQ5 in V4 doc). Unification requires Linen to request `object_id` from SexFiles on
object creation — deferred to M4+ disk extension work.

`linen_object_ref()` returns a `SexObjectRef` with the Linen-local `object_id`.
Callers must not pass this ref to SexFiles without ID-space translation.

---

## Build Proof

```
$ cargo check -p linen --target x86_64-sex.json -Z build-std=core,alloc ...
    Checking sex-object-model v0.1.0
    Checking linen v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
    (zero warnings)
```

---

## Grep Proofs

```
$ rg "use std|extern crate std|libc" crates/sex-object-model/ servers/linen/src/sexobject.rs
  CLEAN

$ git diff -- kernel/ crates/sex-pdx/
  0 lines — nothing touched

$ git diff --name-only  [M4-owned files]
  servers/linen/Cargo.toml          ← dep added
  servers/linen/src/main.rs         ← mod sexobject added
  servers/linen/src/sexobject.rs    ← new adapter (untracked → new file)
```

---

## Compile-Time Size Assertions (linen build context)

```rust
const _: () = assert!(core::mem::size_of::<SexObjectHeader>() == 80);
const _: () = assert!(core::mem::size_of::<SexObjectRef>() == 16);
```

Third independent build context verifying the layout — after model crate (M1) and
sexfiles (M2).

---

## What Is NOT Done (Next Steps in Order)

| Step | What |
|---|---|
| M3 proof gate | `[sexobject.view.from_entry]` in sexfiles proof.rs — call `sexobject_header_from_entry` on live entry, verify fields |
| M4+ kind refinement | Adopt silk-shell's 11-variant `LinenObjectKind` in linen server for 1:1 `SexObjectKind` mapping |
| M4+ ID unification | Linen requests `object_id` from SexFiles on create; closes Linen/SexFiles ID space gap |
| M4+ `first_block` binding | Linen stores `SexfilesObjectEntry.first_block` in `LinenObject` after SexFiles create |
| M3 Collar | Unify `CollarGrant.object_id` with SexFiles `object_id` |

---

## Dependency Graph After M4

```
sex-object-model (no deps)
        ↑
        ├── servers/sexfiles (M2)
        └── servers/linen   (M4)
```

`sex-object-model` remains dependency-free. Servers depend on it; it depends on nothing.

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_LINEN_VIEW_M4.md`
- M2 doc: `docs/handoff/SEXOBJECT_SEXFILES_VIEW_M2.md`
- M1 doc: `docs/handoff/SEXOBJECT_MODEL_M1.md`
- Canonical model: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
- Adapter: `servers/linen/src/sexobject.rs`
