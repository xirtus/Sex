# SexObject M5 — Collar rights_generation Binding Handoff

**Date:** 2026-05-06
**Status:** COMPLETE — stub binding with honest source labelling
**Spec:** docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md §M3 Collar / OQ4

---

## Investigation Result: Source = stub

**No Collar server exists.** All Collar logic lives in `servers/silk-shell/src/main.rs`
as private static state. It is not a standalone server and has no PDX interface
for cross-server authority queries.

**Real Collar generation found but not bridged:**

| Symbol | Location | Accessible cross-PD? |
|---|---|---|
| `COLLAR_GRANT_GENERATION: u64` | `silk-shell/src/main.rs:1430` | No — private static mut |
| `CollarGrant.generation: u64` | `silk-shell/src/main.rs:1409` | No — private struct field |
| `collar_check_operation()` | `silk-shell/src/main.rs:1245` | No — private fn |

**What Collar revocation does today:**
- Sets `CollarGrant.state = Revoked` locally in silk-shell
- Records `[collar.audit.write]` serial marker
- **Does NOT send a PDX op to SexFiles**
- **Does NOT bump `SexfilesObjectEntry.rights_generation`**

**SexFiles' `rights_generation`:**
- Set to `1` on `create_object_entry` (verified in diskfs.rs)
- Never subsequently bumped by any external authority
- Is the authoritative store per OQ4 recommendation — but has no write path from Collar

**Conclusion:** A `source=real` binding requires a new Collar→SexFiles PDX opcode that
silk-shell calls on grant revocation. That is a future step, not M5 scope.

---

## What Was Added

| File | Change |
|---|---|
| `servers/sexfiles/src/sexobject.rs` | `collar_rights_generation()` stub helper added |
| `servers/sexfiles/src/proof.rs` | `[sexobject.collar.rights_generation]` marker added to `run_sexobject_view_proof()` |

**Files NOT touched:** `kernel/`, `crates/sex-pdx/`, `crates/sex-object-model/`,
`servers/silk-shell/`, any other server, any disk format, any PDX opcode.

---

## Stub Helper

**Location:** `servers/sexfiles/src/sexobject.rs`

```rust
#[allow(dead_code)]
pub fn collar_rights_generation(entry: &SexfilesObjectEntry) -> u64 {
    entry.rights_generation   // stub: SexFiles value; not yet bumped by Collar
}
```

**Purpose of the named stub:**
This function is the single call site to update when the real Collar→SexFiles
revocation bridge is wired. Its name distinguishes "Collar-intended rights_generation
source" from the generic `entry.rights_generation` field access. Changing
`source=stub` to `source=real` in the marker requires only wiring this function.

---

## Serial Marker (runtime, `SEXOS_SEXOBJECT_VIEW_PROOF=1`)

```
[sexobject.collar.rights_generation] object_id=<N> rights_generation=1 source=stub
```

- `rights_generation=1` — SexFiles initial value; never bumped by Collar today
- `source=stub` — honest label; real source is `silk-shell::COLLAR_GRANT_GENERATION`

Emitted inside `run_sexobject_view_proof()`, immediately after the
`[sexobject.view.from_entry]` marker. Same env-var gate: `SEXOS_SEXOBJECT_VIEW_PROOF`.

---

## Build Proof

```
$ cargo check -p sexfiles --target x86_64-sex.json -Z build-std=core,alloc ...
    Checking sexfiles v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s   (zero warnings)
```

---

## Grep Proofs

```
$ rg "collar|sexfiles|linen" crates/sex-object-model/src/lib.rs
  CLEAN — no reverse dep

$ git diff -- kernel/ crates/sex-pdx/
  0 lines — nothing touched

$ cat crates/sex-object-model/Cargo.toml
  [dependencies]   ← empty; model crate still zero deps
```

---

## What Blocks `source=real`

To change from `source=stub` to `source=real`, all three must be true:

| Requirement | Status |
|---|---|
| Collar→SexFiles PDX opcode defined (e.g. `OP_COLLAR_REVOKE_RIGHTS`) | **Not yet** |
| silk-shell calls that opcode when `CollarGrant.state → Revoked` | **Not yet** |
| SexFiles handler bumps `rights_generation` for the target `object_id` | **Not yet** |
| `CollarGrant.object_id` unified with `SexfilesObjectEntry.object_id` (OQ5) | **Not yet** |

OQ5 is the hardest: `CollarGrant.object_id` is currently a silk-shell-local Linen
object index, not the global `SexfilesObjectEntry.object_id`. Without ID unification,
silk-shell cannot tell SexFiles which `object_id` to bump.

---

## Dependency Graph (Unchanged)

```
sex-object-model (zero deps)
        ↑
        ├── servers/sexfiles   (M2 adapter, M3 proof gate, M5 collar stub)
        └── servers/linen      (M4 adapter)
```

No new edges. `silk-shell` not touched. No reverse dependency.

---

## Next Recommended Step

**OQ5 resolution** — the prerequisite for `source=real`:

Option A (minimal): Assign SexFiles-allocated `object_id` at Linen object creation
time. Linen calls `create_object_entry` in SexFiles on `linen.session.create`;
SexFiles returns the global `object_id`. Linen stores it. CollarGrant then carries
the SexFiles `object_id`, enabling the revocation PDX bridge.

Option B (deferred): Keep ID spaces separate; add a translation table in silk-shell
mapping local Linen IDs → SexFiles object_ids. More complex, defers ID unification.

Recommendation: Option A. It is additive, closes OQ5, and unblocks both Collar
revocation and the Bell/Quil/Spindle bindings (M6/M7/M8) which all need a stable
`object_id` namespace.

---

## Handoff Path

- This doc: `docs/handoff/SEXOBJECT_M5_COLLAR_RIGHTS_GENERATION_BINDING.md`
- M3 proof gate: `docs/handoff/SEXOBJECT_M3_SEXFILES_PROOF_GATE.md`
- M4 Linen view: `docs/handoff/SEXOBJECT_LINEN_VIEW_M4.md`
- Stub location: `servers/sexfiles/src/sexobject.rs::collar_rights_generation()`
- Canonical model: `docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md`
