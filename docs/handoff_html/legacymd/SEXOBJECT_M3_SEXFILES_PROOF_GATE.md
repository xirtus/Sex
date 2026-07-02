# SexObject M3 — SexFiles Proof Gate Handoff

**Date:** 2026-05-06
**Status:** COMPLETE — build proof verified, runtime gate wired
**Spec:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md §M3 proof step

---

## What Was Added

| File | Change |
|---|---|
| `servers/sexfiles/src/proof.rs` | `run_sexobject_view_proof()` appended at end |
| `servers/sexfiles/src/trampoline.rs` | Gate added before `[sexfiles.ready]` |
| `servers/sexfiles/src/sexobject.rs` | Trimmed unused `SexObjectKind` re-export |
| `servers/sexfiles/src/main.rs` | Added `mod sexobject;` (binary crate has its own module tree) |
| `docs/handoff/SEXOBJECT_M3_SEXFILES_PROOF_GATE.md` | This file |

**Files NOT touched:** `kernel/`, `crates/sex-pdx/`, `crates/sex-object-model/`,
any other server, any disk format, any PDX opcode, any Collar/Linen behavior.

---

## Why `mod sexobject` Had to Be in Both `lib.rs` and `main.rs`

`sexfiles` is a Rust package with both a library crate (`lib.rs`) and a binary
crate (`main.rs`). Each has its own independent module tree. `lib.rs` had
`pub mod sexobject;` but `main.rs` did not — so `proof.rs`, compiled under the
binary tree, could not resolve `crate::sexobject`. Fix: `mod sexobject;` added to
`main.rs`. Both crate roots now declare it.

---

## Proof Function

**Location:** `servers/sexfiles/src/proof.rs` (appended, line ~1243)

**Activated by:** `SEXOS_SEXOBJECT_VIEW_PROOF=1` environment variable at build time.

**What it does:**
1. `DiskFs::new()` + `format_init_empty()` + `mount()` — standard diskfs setup
2. `create_object_entry(kind=4, owner_pd=42)` — creates a live entry (kind 4 = QuilDocument)
3. `stat_object_entry(oid)` — retrieves the live `SexfilesObjectEntry`
4. `sexobject_header_from_entry(&entry)` — derives `SexObjectHeader` via M2 adapter
5. Invariant checks: `object_id`, `owner_pd`, `kind`, `rights_generation`, `object_size_bytes`
6. Emits marker

**Serial marker emitted:**
```
[sexobject.view.from_entry] ok=1 object_id=<u64> kind=4 size=0 flags=0 rights_generation=1 checksum=<u32>
```

- `ok=1` — all invariant checks passed
- `kind=4` — QuilDocument discriminant, round-tripped correctly
- `rights_generation=1` — SexFiles sets this to 1 on create (verified via M2 field mapping)
- `size=0` — new entry has no content yet
- `flags=0` — entry is in_use; FLAG_TOMBSTONED not set
- `checksum` — XOR checksum from `SexfilesObjectEntry.checksum`

**Grep proof (runtime):**
```bash
rg "sexobject.view.from_entry" /tmp/*.log
```

---

## Trampoline Gate

```rust
const SEXOBJECT_VIEW_PROOF_ENABLED: bool =
    option_env!("SEXOS_SEXOBJECT_VIEW_PROOF").is_some();
if SEXOBJECT_VIEW_PROOF_ENABLED {
    crate::proof::run_sexobject_view_proof();
}
```

Pattern matches all existing sexfiles proof gates. Zero runtime cost when env var absent.

---

## Build Proof

```
$ cargo check -p sexfiles --target x86_64-sex.json -Z build-std=core,alloc ...
    Checking sex-object-model v0.1.0
    Checking sexfiles v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s   (zero warnings)

$ cargo check -p sex-object-model --target x86_64-sex.json -Z build-std=core ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s   (zero warnings)
```

---

## Grep Proofs

```
$ rg "use std|extern crate std|libc" crates/sex-object-model/ servers/sexfiles/src/sexobject.rs proof.rs
  CLEAN

$ cat crates/sex-object-model/Cargo.toml
  [dependencies]   ← empty; model crate still zero deps

$ rg "sexfiles|linen|collar" crates/sex-object-model/src/lib.rs
  CLEAN   ← no reverse dependency

$ git diff -- kernel/ crates/sex-pdx/
  0 lines
```

---

## Dependency Graph After M3

```
sex-object-model (zero deps)
        ↑
        ├── servers/sexfiles   (M2 adapter + M3 proof gate)
        └── servers/linen      (M4 adapter)
```

---

## What Is NOT Done

| Remaining | Notes |
|---|---|
| Runtime log verification | Requires boot with `SEXOS_SEXOBJECT_VIEW_PROOF=1` set in build env |
| Collar M3 binding | Unify `CollarGrant.object_id` with SexFiles `object_id`; add revocation PDX op |
| M4+ Linen kind refinement | Adopt 11-variant kind mapping from silk-shell |
| M4+ ID unification | Linen requests `object_id` from SexFiles on create |
| Bell / Quil / Spindle binding | M6/M7/M8 |

---

## Next Recommended Step

Collar binding (M3 Collar from V4 §M3): unify `CollarGrant.object_id` with
`SexfilesObjectEntry.object_id`. Add a Collar→SexFiles PDX op to bump
`rights_generation` on revocation. Add `[collar.revoke.rights_gen.bump]` proof gate.
This closes OQ5 and makes `rights_generation = 0` in Linen/Bell headers accurate
rather than a V1 placeholder.

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_M3_SEXFILES_PROOF_GATE.md`
- M4 doc: `docs/handoff/SEXOBJECT_LINEN_VIEW_M4.md`
- M2 doc: `docs/handoff/SEXOBJECT_SEXFILES_VIEW_M2.md`
- M1 doc: `docs/handoff/SEXOBJECT_MODEL_M1.md`
- Canonical model: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
