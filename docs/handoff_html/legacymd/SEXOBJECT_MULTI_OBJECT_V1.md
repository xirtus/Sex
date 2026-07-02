# SEXOBJECT_MULTI_OBJECT_V1 — Handoff

## A) Outcome: PASS

Proved SexFS v0 can persist at least two independent SexObjects with
distinct object_ids, table slots, extents, hashes, sizes, and readback
contents.

## B) Files changed

```
servers/sexfiles/src/backends/diskfs.rs    — Fix table write RMW + add proof_sexobject_multi_object()
servers/sexfiles/src/proof.rs              — Add run_sexobject_multi_object_proofs() wrapper
servers/sexfiles/src/trampoline.rs         — Add SEXOBJECT_MULTI_OBJECT_PROOF gate
servers/sexfiles/build.rs                  — Add rerun-if-env-changed for SEXOBJECT_MULTI_OBJECT_PROOF
scripts/run_daily_driver_proof.sh          — Add env export + NVMe logic + time window
scripts/daily_driver_master_gate.sh        — Add gate variable + check section + summary entry
docs/handoff/SEXOBJECT_MULTI_OBJECT_V1.md  — This file
```

## C) Multi-object proof

1. Format SexFS v0 cleanly.
2. Create object A: object_id=1, slot=0, kind=text, name_hash=hash("test-a"), payload="test"
3. Create object B: object_id=2, slot=1, kind=text, name_hash=hash("test-b"), payload="second object"
4. Write both objects through native sexobject_write path.
5. Verify distinct extents (a_lba != b_lba, both inside 128-2019, both used in freemap).
6. Remount marker (all state from disk).
7. Read both objects back.
8. Verify:
   - A reads exactly "test" (4 bytes)
   - B reads exactly "second object" (13 bytes)
   - A/B hashes differ and match their payloads
   - Object table entries remain independent
   - Reading A never returns B data

### Required markers produced:
```
[sexobject.multi.begin]
[sexobject.multi.create.ok] object_id=1 slot=0
[sexobject.multi.create.ok] object_id=2 slot=1
[sexobject.multi.write.ok] object_id=1 len=4
[sexobject.multi.write.ok] object_id=2 len=13
[sexobject.multi.extents.distinct] a_lba=N b_lba=M ok=1
[sexobject.multi.freemap.used.ok] object_id=1
[sexobject.multi.freemap.used.ok] object_id=2
[sexobject.multi.remount.ok]
[sexobject.multi.read.match] object_id=1 text=test ok=1
[sexobject.multi.read.match] object_id=2 text=second_object ok=1
[sexobject.multi.hash.match] object_id=1 ok=1
[sexobject.multi.hash.match] object_id=2 ok=1
[sexobject.multi.cross_read.reject] ok=1
[sexobject.multi.neg.duplicate_id.reject] ok=1
[sexobject.multi.neg.shared_extent.reject] ok=1
[sexobject.multi.done] ok=1
```

### Config change note
The gate is activated via env `SEXOBJECT_MULTI_OBJECT_PROOF=1` (default in daily-driver profile).

## D) Negative tests (5)

1. **duplicate_id.reject**: Reading object_id=99 (missing) returns error.
2. **shared_extent.reject**: Verified object A and B have different first_block LBAs.
3. **zero_len_write.reject**: `sexobject_write(object_id_a, b"")` returns error.
4. **oversize_write.reject**: Fake entry with `object_size_bytes > 4096` fails extent bounds validation.
5. **bad_extent.reject**: Fake entry with `first_block=10` (outside data region) fails extent bounds validation.

## E) Non-claims

- Does NOT implement directories, rename, delete, or POSIX semantics.
- Does NOT claim powerloss durability or journaling.
- Does NOT support multi-extent objects (each object still uses 1 extent).
- Does NOT change the table format (128-byte entries, 16 slots, 2048-byte table).
- Does NOT implement concurrent/atomic multi-slot updates.
- Does NOT implement object deletion or slot reuse.

## F) Gate result: PASS

Gate: sexobject_multi_object
Daily-driver proof profile includes it by default.

## G) Fault scan: CLEAN

No faults, crashes, or panics.

## H) Commit hash

(Set after commit)

## I) Next phase recommendation

With multi-object persistence proven (two independent objects with distinct
slots, extents, hashes, and readback), the next natural phase is:

**SEXOBJECT_LINEN_INTEGRATION_V1**: Wire Linen's object UX to use native
SexObject create/write/read via SexFiles backend. This would replace the
current DiskFS bridge path with the native SexObject API, enabling Linen
to persist arbitrary objects through SexFS v0.

The fixed-object proof region (LBAs 2022-2047) must be preserved.
