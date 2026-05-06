# SexObject SexFiles View Adapter — M2 Handoff

**Date:** 2026-05-06
**Status:** COMPLETE — build proof verified
**Spec:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md §M2

---

## What Was Added

| File | Change |
|---|---|
| `servers/sexfiles/Cargo.toml` | Added `sex-object-model = { path = "../../crates/sex-object-model" }` |
| `servers/sexfiles/src/sexobject.rs` | New module — pure adapter + size assertions |
| `servers/sexfiles/src/lib.rs` | Added `pub mod sexobject;` |

**Files NOT touched:** `kernel/`, `crates/sex-pdx/`, any other server, `sexos_build_spec.toml`,
any disk format, any PDX opcode, any SexFiles handler, any Collar/sexshop behavior.

---

## Why Adapter Lives in sexfiles, Not sex-object-model

The model crate must remain dependency-free (no knowledge of SexFiles internals).
`SexfilesObjectEntry` is an implementation detail of the SexFiles server.
`sex-object-model` depends on nothing; `sexfiles` depends on `sex-object-model`.
Dependency arrow: `sexfiles` → `sex-object-model`. Never the reverse.

---

## Adapter

**Location:** `servers/sexfiles/src/sexobject.rs`

**Signature:**
```rust
pub fn sexobject_header_from_entry(entry: &SexfilesObjectEntry) -> SexObjectHeader
```

Pure, no allocation, no mutation, no I/O, no disk write, no PDX op.

---

## Field Mapping Table

| `SexObjectHeader` field | Source | V1 Notes |
|---|---|---|
| `object_id` | `entry.object_id` | Direct |
| `content_generation` | `entry.metadata_generation` | **V1 proxy** — real `content_generation` field added in M2 disk extension |
| `rights_generation` | `entry.rights_generation` | Direct — SexFiles authoritative |
| `metadata_generation` | `entry.metadata_generation` | Direct |
| `object_size_bytes` | `entry.object_size_bytes` | Direct |
| `first_block` | `entry.first_block` | Direct — also sexshop hash for large blobs (M5) |
| `owner_pd` | `entry.owner_pd` | Direct |
| `kind` | `entry.kind as u32` | Widening cast; V1 discriminants ≤ 65535, fit u16 |
| `checksum` | `entry.checksum` | Direct — XOR scheme from SexFiles |
| `flags` | `0` if `entry.in_use`, else `FLAG_TOMBSTONED` | Logical; no new `in_use` semantics added |
| `reserved0` | `0` | Zero until M2 disk extension |
| `reserved1` | `0` | Zero until M2 disk extension |

---

## No Disk Format Change

`sexobject_header_from_entry` is a read-only view. It reads existing `SexfilesObjectEntry`
fields and maps them to `SexObjectHeader` fields. No write path, no journal record,
no superblock update, no new on-disk layout. The disk format is identical before and
after M2.

---

## Build Proof

```
$ cargo check -p sexfiles --target x86_64-sex.json -Z build-std=core,alloc ...
    Checking sex-object-model v0.1.0
    Checking sexfiles v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.54s
    (1 pre-existing warning: unused DISKFS_CHECKPOINT_MAGIC constant — not introduced by M2)

$ cargo check -p sex-object-model --target x86_64-sex.json -Z build-std=core ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
```

---

## Grep Proofs

```
$ rg "use std|extern crate std|alloc|libc|thread" crates/sex-object-model/ servers/sexfiles/src/sexobject.rs
  (hits are comments only — "No allocation", "no alloc" in doc comments)
  CLEAN

$ git diff -- kernel/ crates/sex-pdx/
  (0 lines — nothing touched)

$ git diff --name-only  [M2-owned files only]
  servers/sexfiles/Cargo.toml        ← dep added
  servers/sexfiles/src/lib.rs        ← pub mod sexobject added
  servers/sexfiles/src/sexobject.rs  ← new adapter module
```

---

## Compile-Time Size Assertions (sexfiles side)

`sexobject.rs` re-asserts from the sexfiles build context:

```rust
const _: () = assert!(core::mem::size_of::<SexObjectHeader>() == 80);
const _: () = assert!(core::mem::size_of::<SexObjectRef>() == 16);
```

These fire at compile time if M1 layout changes out from under M2.

---

## What Is NOT Done (Next Steps)

| Step | What |
|---|---|
| M2 disk extension | Add `content_generation` field to `SexfilesObjectEntry` (requires disk format design — not M2 scope) |
| M3 proof path | Add `[sexobject.view.from_entry]` proof gate to `servers/sexfiles/src/proof.rs` to exercise `sexobject_header_from_entry` at boot-time proof run |
| M3 Collar | Unify `CollarGrant.object_id` with SexFiles `object_id`; add revocation PDX op |
| M4+ | Linen, Bell, Quil, Spindle binding |

**Recommended next step: M3 proof path** — call `sexobject_header_from_entry` from an
existing proof function using a live `SexfilesObjectEntry`, emit a `[sexobject.view.*]`
proof marker, verify fields round-trip correctly. Proves SexFiles can derive
`SexObjectHeader` safely before any authority semantics depend on it.

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_SEXFILES_VIEW_M2.md`
- M1 doc: `docs/handoff/SEXOBJECT_MODEL_M1.md`
- Canonical model: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
- Adapter: `servers/sexfiles/src/sexobject.rs`
- Model crate: `crates/sex-object-model/src/lib.rs`
