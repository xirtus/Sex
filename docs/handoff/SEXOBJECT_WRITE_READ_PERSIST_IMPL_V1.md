# SEXOBJECT_WRITE_READ_PERSIST_IMPL_V1 Handoff

## A) Outcome: PASS

Gate `sexobject_write_read_persist` — PASS.
All 16 required markers confirmed in log. Pre-existing `linen_diskfs_direct` FAIL is unrelated and unchanged.

---

## B) Files Changed

- `servers/sexfiles/src/backends/diskfs.rs` — `sexobject_slot_for_id`, `sexobject_create`, `sexobject_write`, `sexobject_read`, `proof_sexobject_write_read_persist()`
- `servers/sexfiles/src/proof.rs` — `run_sexobject_write_read_persist_proofs()`
- `servers/sexfiles/src/trampoline.rs` — dispatch for `SEXOBJECT_WRITE_READ_PERSIST_PROOF`
- `servers/sexfiles/build.rs` — `rerun-if-env-changed=SEXOBJECT_WRITE_READ_PERSIST_PROOF`
- `scripts/run_daily_driver_proof.sh` — export env var, bump `PROBE_SECONDS` to 150 for 5-proof chain
- `scripts/daily_driver_master_gate.sh` — gate init, 15-sub-check FAIL block, ALL_GATES entry

---

## C) Create/Write/Read Proof

1. `sexfs_v0_format_to_disk()` — clean slate
2. `sexobject_create(kind=1, name_hash=fnv1a("test"))` — slot 0 → object_id=1, IN_USE, size=0, no extent
3. `sexobject_write(1, b"test")` — alloc block (LBA 128), write 8 sectors (data + zero pad), compute FNV-1a hash over 4096 bytes, persist freemap + table
4. Emit remount marker (all subsequent reads come from disk, no in-memory state)
5. `sexobject_read(1, &mut buf)` — reads table → validates → reads first data sector → returns 4
6. Verify `buf[0..4] == b"test"` and `read_size == 4`
7. Stat: size=4, extent_count=1
8. Hash: re-read table (stored hash) + re-read 8 sectors (disk hash) → match
9. Freemap: re-read freemap → block 16 (LBA 128) is marked used

---

## D) Negative Tests

| Test | Method | Result |
|------|--------|--------|
| read missing object_id=99 | `sexobject_read(99, ..)` — slot=98≥16 → ERR_NOT_FOUND | ok=1 |
| zero-length write | `sexobject_write(1, b"")` — len==0 check | ok=1 |
| oversize write (>4096) | `sexfs_v0_validate_extent_bounds` with size=4097, extent_count=1 | ok=1 |
| bad extent LBA | `sexfs_v0_validate_extent_bounds` with first_block=10 (<128) | ok=1 |
| hash mismatch | corrupt sector 0 (byte 0 → 0xFF), recompute hash → ≠ stored | ok=1 |

---

## E) Non-Claims

- No powerloss durability or journaling
- No POSIX semantics, no directories, no rename/delete
- No multi-extent objects
- No Linen direct SexDrive access
- Fixed-object proof region LBAs 2022–2047 untouched
- `sexobject_read` returns first 512 bytes only (adequate for objects ≤512B; for larger objects caller must read additional sectors)

---

## F) Gate Result

```
sexobject_write_read_persist PASS   native create/write/read/remount/negative all ok
```

Preserved gates:
- `sexfs_v0_superblock_format_mount` — PASS
- `sexobject_table_persist` — PASS
- `sexobject_table_extent_alloc` — PASS
- `sexobject_extent_write_full_block` — PASS
- `linen_sexfiles_100_current_tier_release` — PASS

Pre-existing FAIL:
- `linen_diskfs_direct` — FAIL, unchanged, unrelated

---

## G) Fault Scan

```
faults_zero PASS   0 fault markers
```

No new faults introduced.

---

## H) Commit Hash

`ef0677c8`

---

## I) Next Phase Recommendation

**SEXOBJECT_MULTI_OBJECT_V1**: Prove that two SexObjects can coexist in the same table and each reads back its own data independently. Requires:
- Allocate slot 0 and slot 1 simultaneously
- Write different payloads (e.g., "foo" and "bar")
- Verify each reads back its own bytes
- Verify freemap marks two separate blocks used
- Negative: read slot 0 does not return slot 1's data

This is the minimal proof before Linen can treat SexFiles as a real object store with multiple live objects.
