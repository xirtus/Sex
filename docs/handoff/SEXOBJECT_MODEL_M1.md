# SexObject Model Crate — M1 Handoff

**Date:** 2026-05-06
**Status:** COMPLETE — build proof verified
**Spec:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md §M1

---

## What Was Added

| File | Role |
|---|---|
| `crates/sex-object-model/Cargo.toml` | No dependencies. edition 2021. |
| `crates/sex-object-model/src/lib.rs` | Model types only — no_std, repr(C), no heap |
| `Cargo.toml` | Added `"crates/sex-object-model"` to workspace members |

**Files NOT touched:** `kernel/`, `crates/sex-pdx/`, any server, `sexos_build_spec.toml`.

---

## Types Defined

### `SexObjectKind` — `repr(u32)`, 12 variants

All V1 discriminants fit `u16` (max = 11) — compatible with
`SexfilesObjectEntry.kind: u16`. Discriminant 7 reserved (SceneSnapshot deferred).

```
RawBlob=0  AppManifest=1  AppState=2  LinenProject=3  QuilDocument=4
SpindleSession=5  BellEvent=6  [7=reserved]  CollarGrant=8  MeshFact=9
CrashReport=10  Package=11
```

Helpers: `from_u16(u16) -> Option<Self>` (const), `as_u16(self) -> u16` (const).

### `SexObjectRef` — 16 bytes, repr(C)

```
object_id:  u64  →  PdxMessage.arg0
generation: u64  →  PdxMessage.arg1
```

Zero PDX ABI change. Helpers: `new()`, `is_null()`, `NULL` const.

### `SexObjectHeader` — 80 bytes, repr(C)

```
offset  0  object_id           u64
offset  8  content_generation  u64   (content/payload write epoch)
offset 16  rights_generation   u64   (capability/revocation epoch; SexFiles authoritative)
offset 24  metadata_generation u64   (metadata-write epoch)
offset 32  object_size_bytes   u64
offset 40  first_block         u64   (SexFiles block ref; sexshop hash for large blobs)
offset 48  owner_pd            u32
offset 52  kind                u32   (SexObjectKind discriminant)
offset 56  checksum            u32
offset 60  flags               u32   (FLAG_* bits)
offset 64  reserved0           u64   (zero in V1)
offset 72  reserved1           u64   (zero in V1)
```

Size: 6×u64 + 4×u32 + 2×u64 = 48+16+16 = **80 bytes**.
Compile-time `assert!` enforces this.

Helpers: `cap_ref()` (uses rights_generation), `object_ref()` (uses content_generation),
`is_tombstoned()`, `is_sealed()`, `is_redacted()`, `kind() -> Option<SexObjectKind>`.

### Flag Constants — `u32` bits

```
FLAG_TOMBSTONED = 0x01
FLAG_SEALED     = 0x02
FLAG_REDACTED   = 0x04
FLAG_MIGRATING  = 0x08
```

---

## Why SexObject Is a Concept, Not a Struct

`SexObject` does not appear as a type name anywhere in this crate. The concept —
a durable typed capability-scoped OS unit — is the semantic idea. The concrete types
are `SexObjectHeader` (the fixed record), `SexObjectKind` (the type tag),
`SexObjectRef` (the cross-PD handle). Naming the concept directly as a struct would
imply a single concrete representation, which would conflict with the storage layer's
`SexObjectRecord` (future) and the in-flight IPC `SexObjectRef`. The three names
cover three distinct roles without collision.

---

## Why This Is Zero ABI Change

- No file in `crates/sex-pdx/` was modified.
- `PdxMessage` struct unchanged.
- `SexObjectRef` fields (`object_id`, `generation`) map to existing `arg0`/`arg1`
  slots by convention — callers agree on the arg layout; no new opcode or message
  type was added.
- No server was modified. No kernel was modified.

---

## Build Proof

```
$ cargo check -p sex-object-model --target x86_64-sex.json -Z build-std=core -Z json-target-spec
   Checking sex-object-model v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
```

---

## Grep Proofs

```
$ rg "use std|extern crate std|alloc|libc|thread" crates/sex-object-model/ -n
CLEAN

$ rg "kernel|pku|gdt|interrupts|framebuffer" crates/sex-object-model/ -n
crates/sex-object-model/src/lib.rs:5: // no kernel edits ...
  (comment only — explains what crate does NOT do)

$ git diff -- kernel/ crates/sex-pdx/ servers/
  (pre-existing diffs from prior sessions — M1 touched none of these)

$ git diff --name-only | grep sex-object
  Cargo.toml  (workspace member added)
  crates/sex-object-model/Cargo.toml  (new)
  crates/sex-object-model/src/lib.rs  (new)
```

---

## What Is NOT Done (Next Steps)

| Step | What | Blocking |
|---|---|---|
| M2 | SexFiles logical view adapter: construct `SexObjectHeader` from `SexfilesObjectEntry` | Needs `sex-object-model` dep in sexfiles |
| M2 | Extend `SexfilesObjectEntry` → `SexObjectRecord`: add `content_generation`, `parent_ref`, `policy_flags` | Disk format design |
| M3 | Collar: unify `CollarGrant.object_id` with SexFiles `object_id`; add revocation PDX op | OQ5 |
| M4+ | Linen, Bell, Quil, Spindle, Mesh binding | After M2+M3 |

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_MODEL_M1.md`
- Canonical model: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
- Crate: `crates/sex-object-model/src/lib.rs`
- Next prompt: M2 — SexFiles logical view adapter (additive only, no disk format change)
