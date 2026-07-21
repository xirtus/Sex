# Authoritative disk layout

**Single source of truth: `crates/sex-pdx/src/lib.rs`, the disk-layout
module** (search for `DISK LAYOUT` in that file). Every fixed on-disk LBA
region any component reserves — a self-test, a manifest sector, an
object-content pool, anything — MUST be declared there before it's used
anywhere else, and gets a `const _: () = assert!(!ranges_overlap(...))`
check added against every region that already exists. An overlapping
reservation fails the build, not a reboot months later.

**Do not hardcode a new fixed LBA or LBA range in any crate without
adding it here first.** This rule exists because of a real incident: two
crates each independently picked LBA 128 as the base of a region they
needed, both for the same reason (it was the start of the SexFS v0
write-allowed range), neither aware of the other. One of them ran on
every boot and silently overwrote the other's data. See
`docs/handoff/SEXDRIVE_NVME_QUEUE_WRAP_V1.md` for the full incident and
how long it took to find because there was no shared source of truth to
check against.

## Current map (LBA = 512-byte sector; disk image is 2048 sectors / 1 MiB
in the gate test harness)

| Range (LBA)   | Sectors | Owner                        | Notes |
|---------------|---------|-------------------------------|-------|
| 0 – 47        | 48      | SexFS v0 metadata (unused)   | Allowed but not currently written by anything live |
| 48 – 127      | 80      | **Nothing — not allowlisted** | `write_guard_allows` (apps/sexdrive) does NOT permit real writes here at all |
| 128 – 131     | 4       | apps/sexdrive AP4 self-test  | **Unconditional, every boot, no gate** |
| 132 – 255     | 124     | Free (inside SexFS v0 allowed range, unclaimed) | |
| 256 – 259     | 4       | apps/sexdrive AP5A self-test | Gated behind `SEXOS_STORAGE_100_PERSIST_WRITE`/`_READ` — off by default, still reserved |
| 260 – 383     | 124     | Free | |
| 384           | 1       | apps/sexdrive AP6 self-test  | Gated behind `SEXOS_STORAGE_100_NEGATIVE` + `SEXOS_STORAGE_100_NEG_MISMATCH` — off by default, still reserved |
| 385 – 399     | 15      | Free | |
| **400 – 1807**| **1408**| **DISKFS_V4 content pool**   | 176 blocks × 4096 bytes (8 sectors/block); variable-length object content |
| 1808 – 1910   | 103     | Free (margin) | |
| 1911 – 1925   | 15      | DISKFS_V4 indirect extent descriptors | One 512-byte sector per object slot (15 slots) |
| 1926 – 2019   | 94      | Free (inside SexFS v0 allowed range, unclaimed) — note the legacy V3 slots below start at 1926 too; see next row | |
| 1926 – 2045   | 120     | DISKFS_V4 legacy-migrated V3 object slots | 15 × 8-sector slots; slots 0-2 (sexfiles-proof-v1, linen-object-v1, quil-object-v1) are system objects migrated in place from V3, slots 3-14 available for V4-native objects that happen to reuse this legacy layout during migration bootstrap |
| 2046          | 1       | DISKFS_V4 / DISKFS_V3 manifest | |
| 2047          | 1       | apps/sexdrive AP3 self-test / write-proof LBA | **Unconditional, every boot, no gate** |

Note the overlap in the table between "1926-2019 free" and "1926-2045
legacy slots" is not a real conflict — the legacy slot region physically
begins at 1926 and runs to 2045; the "free" row is listing the same
address range from a different framing (SexFS v0's allowed envelope) and
is superseded by the more specific legacy-slots row. The compile-time
checks in sex-pdx only reason about the actual reserved regions
(manifest, legacy slots, indirect descriptors, sexdrive self-tests, the
V4 pool) — not the broader SexFS v0 envelope, which is an outer bound the
pool must stay inside, not a region anything else owns.

## Adding a new fixed-LBA region

1. Add the new region's base LBA and sector count as `pub const`s in
   `crates/sex-pdx/src/lib.rs`'s disk-layout module.
2. Add a `const _: () = assert!(!ranges_overlap(new_start, new_len,
   existing_start, existing_len), "...")` against every existing region
   in that file.
3. Update the table above.
4. If the new region is claimed by `servers/sexfiles` or
   `apps/sexdrive`, make that crate's local constant equal to the
   sex-pdx canonical value (`= sex_pdx::YOUR_NEW_CONST;`) rather than
   redefining the number locally.
