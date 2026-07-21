# DISKFS_V4 crash injection — Lane 2, first slice

## Status: PASS — grow ordering empirically proven at both commit boundaries

Every prior DISKFS_V4 reboot test (Lane 1) killed QEMU cleanly *between*
operations. That never exercised the crash-safety ordering documented in
`DISKFS_V4_GROWTH_V1.md` (grow: content -> indirect -> manifest) — an
ordering that only matters if a crash can land *inside* an operation. This
gate (`scripts/diskfs_v4_crash_injection_gate.sh`) does that.

## Mechanism: deterministic markers, not timing guesses

Two log lines were added to `servers/sexfiles/src/vfs.rs`, not present
before this slice:

- `[sexfiles.diskfs.v4.crash_point.extent_committed] slot={} new_count={}`
  — in `handle_diskfs_write`'s grow path, right after `v4_indirect_write`
  succeeds and *before* `v4_persist` runs. The new extent is on disk and
  the derived bitmap would see it on a rebuild, but the manifest doesn't
  know about it yet.
- `[sexfiles.diskfs.v4.crash_point.manifest_committed] slot={} size={}`
  — in *both* `handle_diskfs_write`'s grow path (fires once per WRITE
  call whose size increases) and `handle_diskfs_truncate`'s shrink path
  (fires once, after `v4_persist`, at the operation's real atomicity
  boundary). The gate targets the truncate-path occurrence for crash
  point B, since that's the boundary a real WRITE-loop-then-TRUNCATE save
  (spindle's `filldoc`, quil's `persist_save`) actually relies on for
  "the complete new version is durable" — not an intermediate WRITE-path
  commit that's still padded to a 16-byte boundary the caller intends to
  trim away (see "False start" below).

The gate's kill trigger is a tight busy-wait `grep` for these exact
strings, not an LBA or timing inference.

## Two crash points, both required by the core invariant

> Recovery exposes either the complete old version or the complete new
> version — never a mixture.

**Crash point A** — kill immediately after `extent_committed`, before
`manifest_committed`. Required: reboot resolves to the OLD version
exactly.

**Crash point B** — kill immediately after `manifest_committed` (the
truncate-path occurrence, i.e. after a real save's final commit).
Required: reboot resolves to the COMPLETE NEW version exactly.

## Authoritative result

Run provenance (`/tmp/sexos_crash_inj/RUN_PROVENANCE.txt` at run time):
gate script and ISO/kernel content hashes, git commit, working-tree
status, start/end timestamps, exit code — recorded before interpreting
results, per the requirement that evidence be reproducible from a known
artifact set, not just a log.

- gate_script_sha256: `a824ae53dae1ccdbb8550ee8276e5f55c6e6e70c73093b2c92d2072c4ca8cd0e`
- iso_sha256: `2648a149d914350bc2859c42afd77900b0a18135beff4888ddbe6023eea8fdea`
- git_commit: `8f8f42ce351607b5902862ae7c7078c798fce5e9` (+ uncommitted vfs.rs crash-point markers and the new gate script, committed alongside this doc)
- EXIT_MARKER=0, START 2026-07-21T09:30:45+02:00, END 2026-07-21T09:42:51+02:00

| Row | Result |
|---|---|
| A_crash_injected | PASS — killed on `[...crash_point.extent_committed] slot=3 new_count=1` |
| A_fault_free_boot | PASS |
| A_landed_before_manifest_commit | PASS (0 manifest_committed lines in A_b1.log before kill) |
| A_old_version_exact | PASS — `size=0 hash=0xcbf29ce484222325` (empty, exact FNV seed) |
| A_no_layout_overlap | PASS |
| A_later_grow_succeeds | PASS — `size=400 hash=0x8e3e43066d3b8995` |
| B_crash_injected | PASS — killed on `[...crash_point.manifest_committed] slot=3 size=4200` |
| B_fault_free_boot | PASS |
| B_new_version_exact | PASS — `size=4200 hash=0xd4c5a06deed8e2a5 ok=1` (spindle's own per-byte pattern check found zero mismatches) |
| B_no_layout_overlap | PASS |
| negative_control_detects_mismatch | PASS — the same comparison function used for `A_old_version_exact` correctly reports FAIL against a deliberately wrong size (7) and reproduces the real PASS against the actual observed value |

Zero stray QEMU processes after the run (verified by `ps` and the
script's own `trap ... EXIT` safety net).

## Abandoned extent: not a leak, in either follow-up path

Crash point A allocates and zero-fills one block, writes its indirect
descriptor, then crashes before the manifest ever reflects it. That
extent is provably not orphaned:

- **Write path (live-tested above)**: `A_later_grow_succeeds` reuses it
  directly — the next real WRITE to slot 3 reads the (unchanged, correct)
  indirect descriptor via `v4_cache_get`, sees capacity already covering
  1 block, and never re-allocates. The abandoned block becomes the first
  block of the next real save with no wasted space and no failure.
- **Delete path (verified by code inspection, not separately live-tested
  this session)**: `handle_diskfs_delete` reads the indirect descriptor
  unconditionally before freeing (`v4_indirect_read` -> extents used by
  `v4_free_pool_only`) regardless of what the manifest's `size_bytes`
  says. Since the abandoned extent genuinely exists in the indirect
  sector, a delete of slot 3 without ever rewriting it would free it via
  the same mechanism. Both paths the derived bitmap can reach — reuse and
  delete — already account for it. There is no third path (e.g. a
  "list free space" report) that could see it as available while it's
  still indirect-referenced.

## False start: what crash point B looked like before this was fixed

The first two attempts killed on the *first* `manifest_committed` for
slot 3 (any size), not specifically the truncate-path occurrence at the
save's real final size. `filldoc`'s WRITE loop increments in fixed
16-byte steps and only checks `off < n` — for `n=4200` (not a multiple of
16) the last WRITE call lands at `off=4192`, publishing `size=4208`, one
step past the caller's intended length. `filldoc` then issues
`OP_DISKFS_TRUNCATE(4200)` to trim it back — a separate call, which is
why the truncate path needed its own marker rather than reusing the
WRITE-path one. Landing on an early WRITE-path commit instead (observed:
`size=992` after a too-short 90s budget) produced a technically-correct
but unintended result — a smaller, self-consistent, non-torn WRITE-path
checkpoint, confirmed via spindle's own per-byte verification (`ok=1`).
That's real evidence *for* the invariant (every commit is atomic, never
torn), just not evidence about the specific truncate-boundary this crash
point was designed to test. Reclassified as an injection/harness timing
failure (90s budget for a ~263-round-trip, ~380s operation), not a
storage defect, and fixed by raising the busy-wait budget to 600s and
targeting the exact final size in the marker regex.

## Gate

`scripts/diskfs_v4_crash_injection_gate.sh` — two independent crash
points, each its own fresh disk image, each followed by a clean reboot
and recovery verification, plus a negative control on the gate's own
comparison logic and a post-recovery regrow to prove the allocator stays
usable. Includes a script-level `trap ... EXIT` safety net (in addition
to per-crash-point unconditional kill-on-timeout) so a future bug can't
leak an orphaned QEMU instance the way two earlier harness-development
attempts did (documented in git history of this file's development, not
carried forward as open issues).
