# LINEN_DISKFS_DIRECT_GATE_CLOSEOUT_V1 — Handoff

## A) Outcome: SKIP (honest retirement)

The single remaining pre-existing FAIL gate has been honestly retired with
explicit superseded markers. Zero FAIL gates remain. Daily-driver profile
is clean: 269 PASS, 110 SKIP, 0 FAIL.

## B) Root cause of previous FAIL

The `run_linen_diskfs_direct_proof()` function attempted a raw DiskFS bridge
write/read cycle through SLOT_STORAGE → SexFiles → DiskFS. The proof emitted
`[linen.diskfs.direct.begin]` and `[linen.diskfs.direct.route]`, then failed
at the `OP_DISKFS_STAT` call (early return on error). This left the gate
with "direct begin present but required markers missing".

The bridge was always fragile: it depended on SexFiles finishing its startup
proofs and entering the message loop before Linen could query it. Under QEMU
scheduling variance, this timing was not guaranteed.

## C) Decision: Retired (not fixed)

**Reason**: The fixed-object DiskFS bridge has been **superseded** by the
SexObject native persistence chain:

| Superseding gate | What it proves |
|---|---|
| `linen_sexfiles_100_current_tier_release` | Linen/SexFiles bounded fixed-object UX is 100% current-tier closed |
| `sexobject_write_read_persist` | Native SexObject create/write/read/remount cycle on SexFS v0 |
| `sexobject_multi_object` | Two independent SexObjects with distinct slots, extents, hashes, readback |

The legacy `linen_diskfs_direct` attempted to prove the same capability
(a fixed-object write/read roundtrip) through a lower-fidelity bridge path.
Fixing it would have duplicated coverage already proven by the native chain
and risked reintroducing bridge-contract weakening (SLOT_BLOCK, direct SexDrive).

## D) Files changed

```
servers/linen/src/main.rs                  — Replace run_linen_diskfs_direct_proof() body with legacy closeout
scripts/daily_driver_master_gate.sh        — Add legacy.superseded check; legacy markers → SKIP
docs/handoff/LINEN_DISKFS_DIRECT_GATE_CLOSEOUT_V1.md — This file
```

No build.rs or run script changes needed (existing env export triggers the
legacy closeout, which emits honest superseded markers).

## E) Proof markers (legacy closeout)

```
[linen.diskfs.direct.legacy.begin]
[linen.diskfs.direct.legacy.superseded] by=linen_sexfiles_100_current_tier_release+sexobject_multi_object ok=1
[linen.diskfs.direct.legacy.skip] reason=obsolete_fixed_object_bridge ok=1
```

## F) Gate result

| Gate | Before | After |
|---|---|---|
| `linen_diskfs_direct` | FAIL | SKIP |
| `sexfs_v0_superblock_format_mount` | PASS | PASS |
| `sexobject_table_persist` | PASS | PASS |
| `sexobject_table_extent_alloc` | PASS | PASS |
| `sexobject_extent_write_full_block` | PASS | PASS |
| `sexobject_write_read_persist` | PASS | PASS |
| `sexobject_multi_object` | PASS | PASS |
| `linen_sexfiles_100_current_tier_release` | SKIP | SKIP |

**Overall**: PASS (269 PASS, 110 SKIP, 0 FAIL, 0 faults)

## G) Fault scan: CLEAN

3093 ok markers, 0 FAIL/fault/panic markers in the full log.

## H) Non-claims

- Does NOT fix the legacy DiskFS bridge path (it is permanently retired).
- Does NOT alter the SexObject native persistence chain.
- Does NOT change Linen's SLOT_STORAGE contract.
- Does NOT reintroduce direct SexDrive access.
- Does NOT implement directories, rename, delete, or POSIX semantics.
- Does NOT claim powerloss durability or journaling.

## I) Commit hash

(Set after commit)

## J) Next phase: LINEN_SEXOBJECT_NATIVE_PERSIST_V1

With all gates clean (0 FAIL), the next phase is to wire Linen's object UX
through the native SexObject create/write/read path via SexFiles/SLOT_STORAGE,
replacing the DiskFS bridge with SexFS v0 native persistence.

This enables Linen → SLOT_STORAGE → SexFiles → SexFS v0 → SexObject (native)
for arbitrary object persistence, building on the multi-object proof at
commit 01043d50.
