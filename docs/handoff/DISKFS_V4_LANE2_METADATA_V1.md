# DISKFS_V4 Lane 2 — crash-aware metadata publication and recovery

## Status: COMPLETE

This closes Lane 2 in full. The first slice (grow/shrink crash ordering)
was proven and committed earlier (`f905780a`,
`docs/handoff/DISKFS_V4_CRASH_INJECTION_V1.md`). This document covers the
remaining metadata operations: create, delete, rename, truncate/replace
boundaries, mount-time corruption handling, and recovery idempotence.

## Slice A — object creation crash ordering

`handle_diskfs_create` has exactly ONE commit phase: a single `v4_persist()`
call. Unlike grow (content -> indirect -> manifest, two phases) there is
nothing else to zero or allocate at create time — a new object starts with
`size_bytes=0` and no extents, so "content sector zeroed but manifest not
published" (one of the three boundaries originally asked for) does not
actually exist as a distinct state in this design: there is no backing
storage until the first WRITE. Two real crash points, both deterministic
markers:

- `crash_point.create_pending` — before persist. Required: object doesn't exist.
- `create.ok` (pre-existing line, fires only after persist succeeds) — after
  persist. Required: complete empty object exists.

Result (`scripts/diskfs_v4_metadata_crash_gate.sh`, both PASS):
`CREATE_PENDING_object_absent`, `CREATE_COMMITTED_object_complete`
(`size=0 hash=0xcbf29ce484222325`, the empty-read FNV seed — exact).

## Slice B — rename crash ordering

Same single-phase shape as create (`gen` untouched — rename doesn't
represent a new identity, only `name` and the manifest generation change).
Markers: `crash_point.rename_pending` (before persist) and the pre-existing
`rename.ok` (after). Both crash points tested with a 64-byte pre-filled
seed object so content identity could be checked, not just presence:

- `RENAME_PENDING`: old name (`origdoc`) present, new name (`renamed`)
  absent, content hash unchanged.
- `RENAME_COMMITTED`: new name present, old name absent, content hash
  **still unchanged** — rename never touches content or object identity.

Max-length names: `v4_unpack_name` only ever fills 16 of the 24-byte name
field (lo/hi pack 8 bytes each) — the bounded `while n<16` loop is safe by
construction even for a full 16 non-null bytes (loop condition fails
cleanly at n=16, remaining 8 field bytes get zeroed). No overflow is
reachable regardless of what a caller sends; not separately live-tested,
verified by code inspection since the boundary can't be exercised through
the client-side 16-byte cap anyway (`mkdoc`/`mvdoc` already truncate to 16
before packing).

## Slice C — delete crash ordering

Delete has TWO phases, but only the first (`v4_persist`) touches the
manifest — the second (bitmap free + `v4_indirect_write` clearing the
descriptor) is local bookkeeping the manifest never records. Three
markers: `crash_point.delete_pending` (before persist),
`crash_point.delete_committed` (after persist, before free/clear).

- `DELETE_PENDING`: crash before persist → object fully intact, exact
  content hash unchanged.
- `DELETE_COMMITTED`: crash after persist, before the local cleanup phase →
  object gone from the manifest/listing immediately (`v4_bitmap_rebuild`
  skips non-in-use slots regardless of what's still sitting in the on-disk
  indirect descriptor, so even an un-cleared stale descriptor doesn't
  resurrect it or leak the blocks past the next mount).

**Stale-content check (the interesting one):** after `DELETE_COMMITTED`,
the freed slot's on-disk indirect descriptor is provably NOT cleared (the
crash landed before that step) — reusing the slot immediately reads that
stale, valid-looking descriptor back via `v4_indirect_read` and correctly
treats its capacity as already-available (same reuse path
`A_later_grow_succeeds` proved for grow). Live-tested: create a new object
in the freed slot with different, smaller content (20 bytes vs the
original 64) and verify byte-for-byte — `DELETE_COMMITTED_no_stale_content`
PASS. This works because reads are always bounded by the manifest's
`size_bytes`, never by physical extent capacity — leftover bytes past the
new object's own writes are simply never reachable through any read path,
independent of whatever physical content sits in the reused block.

## Slice D — truncate and replacement boundaries

- **Truncate to zero / shrink across an extent boundary**: no special-cased
  code path for `new_len == 0` — same shrink logic as any other value.
  Already exercised at scale by the existing Lane 1 growth gate (15/15
  PASS, `docs/handoff/DISKFS_V4_GROWTH_V1.md`) and Lane 2's own crash
  gate's grow/shrink cycles across the 2-block (4096B) boundary; not
  re-tested here as a separate live scenario since the mechanism is
  identical.
- **Replace contents without changing final length**: `handle_diskfs_write`
  only touches the indirect descriptor and manifest when
  `need_end > entry.size_bytes` — an in-place overwrite within the current
  size never calls `v4_persist` or `v4_indirect_write` at all. There is no
  crash-ordering boundary to test here because nothing gets published;
  verified by code inspection (`servers/sexfiles/src/vfs.rs`, the
  `if need_end > entry.size_bytes` gate around both commit calls).
- **Grow from zero**: exactly what `CREATE_COMMITTED` + a subsequent WRITE
  already exercises.
- **Failed growth (insufficient space)**: `v4_allocate`'s `Err(ERR_FULL)`
  paths return before `V4_TABLE[i]` (the local copy `entry`) is ever
  written back and before either `v4_persist` or `v4_indirect_write` runs
  — old content, length, and metadata are provably untouched on failure
  (nothing was mutated on persistent state at all), and the caller gets a
  real error, never a false success. Verified by code inspection; not
  live-tested — a real pool-exhaustion scenario needs ~176 blocks (704KB)
  filled through the 16-byte-per-round-trip `filldoc` mechanism, which at
  the ~11B/s observed application-level throughput would take on the order
  of hours per object. Disproportionate for what code inspection already
  settles unambiguously.

  **Real bug found and fixed along the way**: `v4_allocate` satisfies a
  request that needs multiple non-contiguous runs (pool fragmented) by
  finding and committing several runs in sequence. If a LATER run in the
  same call fails (pool exhausted or `DISKFS_V4_MAX_EXTENTS` reached), the
  EARLIER runs' bitmap bits were never rolled back — a same-session,
  self-healing (next reboot's bitmap rebuild recomputes from committed
  indirect descriptors only) but real leak: concurrent requests in the
  same boot would see less free space than genuinely exists. Fixed by
  tracking `start_n` and calling `v4_free_pool_only` on `out[start_n..n]`
  before both `Err` returns.

## Slice E — manifest validation and corruption behavior

**Real bug found and fixed.** Before this slice, `v4_ensure()` treated ANY
manifest sector that didn't match the current magic+version as "recognized
V3 or blank" and unconditionally bootstrapped a fresh V4 manifest over it
— including a manifest that was genuinely corrupted (torn write, bad
sector, foreign data), not actually blank. That's exactly the unacceptable
behavior this slice was scoped to catch: treating corrupt storage as empty
and silently overwriting it.

Fixed by distinguishing three cases instead of two: magic-matches-with-an-
older-version (real migration, unchanged), all-zero sector (genuinely
unformatted, unchanged), and anything else (new `ERR_CORRUPT`, mount
refuses). Every `v4_ensure()` caller already propagates `Err(e)` as a hard
error, so this fails visibly — no panic, no silent wipe, no partially
valid state exposed.

Per-entry checksum isolation (pre-existing, not changed by this slice) was
verified live rather than just asserted: flipping only one entry's stored
checksum bytes (magic/version/other entries untouched) still loads the
whole manifest normally, drops only that one entry
(`[sexfiles.diskfs.v4.load.drop] slot=N reason=checksum`), and the other
real object survives with its exact original content hash.

Duplicate/overlapping allocation defense (also pre-existing): `v4_bitmap_
rebuild` drops any slot whose indirect descriptor's extents collide with
an already-claimed block (`reason=extent_overlap_or_short`), verified by
code inspection of the per-block `bitmap_test` check inside the rebuild
loop, not separately live-tested this session (would need hand-crafting a
colliding on-disk indirect sector; the failure mode it guards against is
already covered in spirit by the corrupt-manifest and bad-checksum tests
above).

Sector-atomic write assumption, made explicit per the requirement to
distinguish it from the higher-level publication ordering already proven
in the first Lane 2 slice: `v4_persist()` issues exactly one 512-byte
`DiskFs::diskfs_block_write` to `DISKFS_MANIFEST_LBA`, followed by a
read-back verify. The crash-safety ordering proven in Lane 2's first slice
(content -> indirect -> manifest) assumes that single sector write is
itself atomic at the storage layer — this codebase has no mechanism to
prove or enforce that (it's a property of the underlying block device, not
something a guest OS can control), so it's a documented assumption, not an
additional guarantee this slice adds.

Deliberately NOT built: dual manifests or a journal. The single-sector
protocol, tested honestly here, already meets every acceptance requirement
(mount fails visibly on real corruption, isolates per-entry corruption,
never silently overwrites) without needing them.

Live results (`scripts/diskfs_v4_manifest_validation_gate.sh`, all PASS):
`A_mount_refused`, `A_did_not_silently_load`, `A_manifest_not_overwritten`
(raw manifest bytes hash-compared before/after the failed boot — byte-for-
byte identical, the strongest possible proof of "did not overwrite"),
`B_blank_bootstraps` (positive control), `C_manifest_still_loads`,
`C_bad_entry_dropped`, `C_other_object_survives_exact`,
`C_dropped_object_absent`.

## Slice F — recovery idempotence

By construction, `v4_ensure()` only calls `v4_persist()` (the only thing
that bumps `V4_GENERATION`) on an actual mutation or a dropped-entry
repair — a clean load does neither. Confirmed end to end, not just
reasoned about: two consecutive clean boots of the same valid image with
no mutation in between produce byte-identical raw manifest bytes
(`D_no_write_on_clean_load`), the same generation number
(`D_generation_stable`), and the same content hash
(`D_content_stable`). A third boot with one real mutation in between still
succeeds normally (`D_post_idempotence_mutation_succeeds`) — idempotence
isn't masking a stuck/frozen state.

## Gates

- `scripts/diskfs_v4_metadata_crash_gate.sh` — Slices A/B/C. Six crash
  points (create/delete/rename × pending/committed), each own fresh disk
  image, deterministic markers, reused the proven `tw`/`sp`/`num`/`sel3`/
  `kv` helpers from `diskfs_v4_crash_injection_gate.sh`.
- `scripts/diskfs_v4_manifest_validation_gate.sh` — Slice E (direct
  corruption injection into the raw disk image between boots, not a
  crash-timing race — there's no meaningful "crash mid-corruption" to
  simulate) and Slice F (idempotence, appended to the same gate since it
  reuses the same seeded image).

Both green: metadata gate 20/20 rows PASS, manifest validation gate 17/17
rows PASS. Full regression sweep after these changes landed also clean:
`diskfs_v4_growth_gate.sh` (Lane 1, 15/15, reboot-exact hash unchanged from
the prior authoritative run — `0x8e3e43066d3b8995`) and
`diskfs_v4_crash_injection_gate.sh` (Lane 2 first slice, 11/11) both still
fully pass — the mount-corruption fix and the `v4_allocate` rollback fix
introduced no regressions.

## False starts in THIS session's gate development (harness bugs, not storage bugs)

Documented so they aren't rediscovered:

- **Focus-toggle race**: `kv()`'s own retry-on-miss can press `scroll_lock`
  twice if the first press's log line lands slower than its budget (real
  risk right after a fresh boot). Since `ToggleSpindle` is a literal
  toggle and the old check only looked for "any ToggleSpindle line
  appeared" (direction-blind), two presses cancel out and the check
  reports success while spindle is actually still closed. Fixed by
  checking the ACTUAL resulting focus (`new=` sid != quil's 201) and
  retrying the press itself, not just the check, until it lands.
- **Vacuous disk-listing checks**: the `disk` command does a synchronous
  IPC loop over every slot (up to ~60 round trips) — a fixed `sleep 2`
  after pressing Enter isn't enough for it to actually produce output. The
  DELETE_PENDING verification block was accidentally checking
  `disk_has_name` without ever having run `disk` at all in that specific
  code path, so the absence check trivially "passed" for the wrong reason.
  Both bugs were caught by noticing the check reported PASS through a
  storage flow that, on direct log inspection, showed no evidence of
  having done what the check claimed to verify (silk-shell's OWN internal
  "stargate" terminal-emulation dispatcher — real per project memory as
  dead code for input, but still logging on every keypress since it
  listens independently — briefly looked like the misbehaving code path
  before the real cause was traced to the gate script itself).
