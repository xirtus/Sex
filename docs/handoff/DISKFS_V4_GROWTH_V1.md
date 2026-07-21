# DISKFS_V4 — variable-length dynamic object store

## Status: Lane 1 COMPLETE — base=400, full gate passing, reboot-exact

Earlier checkpoints of this document reported reboot-content persistence
as blocked by an apparently driver-level bug. That diagnosis was wrong in
one specific way: it correctly ruled out DiskFS's own logic (extent
resolution, write payload) but incorrectly concluded the remaining
culprit was `apps/sexdrive`'s NVMe queue handling. The actual cause was
simpler and entirely within this lane: the content pool's base LBA
collided with fixed self-test regions `apps/sexdrive` writes on every
boot. Full root-cause writeup and byte-for-byte proof:
`docs/handoff/SEXDRIVE_NVME_QUEUE_WRAP_V1.md` (now marked DISPROVEN at
the top, kept for methodology).

Two real collisions were found and fixed, in order:
1. Pool base LBA 128 collided with `apps/sexdrive`'s unconditional
   `AP4_MULTI` self-test (LBA 128-131, runs every boot). Moved to 136.
2. Pool base 136 still collided with `AP5A`/`AP6` self-test regions
   (256, 384 — gated off in normal builds, so manual testing never hit
   them). Caught immediately by the new compile-time
   `const _: () = assert!(!ranges_overlap(...))` checks in
   `crates/sex-pdx/src/lib.rs`'s canonical disk-layout module — the
   build failed before any test ran. Moved to final base **400**. See
   `docs/handoff/DISK_LAYOUT_V1.md` for the authoritative disk map.

### Root cause of the "no_stale_trailing_bytes" flakiness: a real perf bug, not gate timing

The first two base=400 attempts intermittently failed
`no_stale_trailing_bytes` on a keystroke-garbling symptom
(`ccaatdoc`, gate `kv()` resending into an in-flight command) and were
worked around at the time by raising `kv()`'s per-keystroke timeout.
**That explanation was incomplete and has been superseded.** The
regression sweep run afterward caught the real defect directly:
`quil_cursor_gate`'s single save (shrinking a doc to 3 bytes) never
completed inside its 120s timeout at all, on any keystroke-timing
grounds — no amount of retiming would have fixed it, because the save
genuinely hadn't finished.

**Confirmed root cause**: `v4_zero_tail` (`servers/sexfiles/src/vfs.rs`,
called from `handle_diskfs_truncate` to clear stale trailing bytes on
shrink) zeroed its target range 16 bytes at a time, routing every
16-byte chunk through the full content-write path
(`DiskFs::diskfs_write_object_entry`) — one real disk round-trip per
chunk. For a shrink that leaves most of a 4096-byte block to be
zeroed, that's up to ~256 round-trips (`4093 / 16`) just to clear one
block's tail. This is what generated the timing pressure the earlier
fix papered over: `truncdoc` in the growth gate and `save` in
`quil_cursor_gate` both trigger this path, and the latter's 120s
budget wasn't enough to survive it under load.

**Fix**: rewrote `v4_zero_tail` to zero at 512-byte sector
granularity — direct sector writes for every fully-cleared sector, and
a single read-modify-write only for the sector straddling the keep
boundary (mirrors `v4_zero_block`'s approach). ~32x fewer round-trips.

**Before/after evidence** (same interaction — create doc, edit to 3
bytes, save — captured from `/tmp/sexos_qc/r.log` across the two
`quil_cursor_gate` runs):

| | old `v4_zero_tail` | new `v4_zero_tail` |
|---|---|---|
| NVMe write submits (create→save) | ≥200 (still climbing; run hit the 120s timeout before `[quil.persist.save.ok]` ever appeared) | 22 |
| NVMe block reads in the zero-tail step | n/a (chunked writes don't distinguish a boundary sector) | 1 (exactly the boundary sector, LBA 400) |
| LBAs touched | climbing sequentially past LBA 405+ when the timeout hit | exactly the owned block's 8 sectors (400-407) + unrelated indirect/manifest bookkeeping (1922, 2046) |
| Save completion | never (WARN, no result) | ~39s wall-clock (07:31:31 → 07:32:10), well inside the 120s budget |
| Final content | n/a — save never completed | `[quil.persist.save.ok] bytes=3 hash=0xe71fab1905416696` — exact match to the gate's independently-computed FNV-1a("abx") expectation |

### Evidence tiers (do not conflate)

- **Base=136 diagnostic run** — superseded, historical only. Proved the
  reboot-hash fix worked in principle but used a pool base later found
  to collide with gated self-tests.
- **First two base=400 attempts** — superseded, historical only. Real
  correctness passes (reboot hash, growth, shrink, etc.) were already
  clean; the intermittent `no_stale_trailing_bytes` FAIL and the
  `kv()` timeout workaround are retained here only as the trail that
  led to the real defect, not as acceptance evidence.
- **Fresh authoritative run, fixed binary** (`scripts/diskfs_v4_growth_gate.sh`,
  full two-boot run, `v4_zero_tail` fix applied, `SKIP_BUILD=1` against
  the rebuilt ISO) — **this is the acceptance record.** All 15 rows
  PASS cleanly, zero `kv()` retries logged (`grep -c "key miss"` = 0),
  including `no_stale_trailing_bytes PASS` with no workaround needed.
  `reboot_survival_exact_hash PASS hash=0x8e3e43066d3b8995` — identical
  to every prior run, confirming the fix changed only the zeroing
  mechanism, not the resulting bytes.

Supersedes DISKFS_V3 (commit `e6f2ef2e`), which capped every object at a
fixed 4096 bytes (one manifest slot = one hardcoded 8-sector LBA range).
V4 keeps V3's manifest location, slot count, and enumeration model, and
replaces only the per-object storage representation.

## On-disk layout

**Manifest** — one 512-byte sector at LBA 2046 (unchanged location/magic):

```
[0..8)   magic  "SDISKMV1"
[8..10)  version = 4
[10..12) entry_count = 15
[12..16) generation (u32, bumped on every metadata change)
16 + i*32 ..: entry i (15 entries x 32 bytes = 480):
   name[24]       zero-padded ASCII; name[0]==0 -> slot free
   +24 size_bytes u32  exact logical content length (0 = empty)
   +28 checksum   u16  FNV-ish over (name || size_bytes || gen)
   +30 gen        u16  bumped on create/delete/rename
```

**Indirect extent descriptor** — one 512-byte sector per slot, at a fixed
LBA (`1925 - slot_index`, so slots occupy LBA 1911..1925 — no allocator
needed just to place it):

```
[0..4)  magic "SFEX"
[4..6)  version = 1
[6..8)  extent_count (0..8)
[8..12) checksum (FNV-1a over the live extent bytes)
12+k*4..: extent k: start_lba u16 (raw sector number), sector_count u16
```

Extents store raw LBA/sector-count, not pool-relative block indices, so a
migrated legacy object (fixed LBA, not aligned to the pool's origin) uses
the exact same representation as a freshly allocated one.

**Content pool** — 176 blocks of 4096 bytes at LBA `[0, 1408)`. Free/used
state is derived, not persisted: rebuilt every mount by scanning every
in-use slot's indirect descriptor and marking its extents allocated. A
block claimed twice marks the later slot corrupt and drops it (same
"drop, don't trust" policy V3 used for bad slot geometry).

**Cap**: 16 blocks (64 KiB) per object, explicit and enforced
(`ERR_OVERFLOW`), never silently truncated.

## Crash-safety ordering

- **Grow**: allocate (in-memory bitmap) -> zero-fill new block(s) on disk
  -> write requested content -> commit indirect descriptor -> commit
  manifest size_bytes. A crash before the indirect commit leaves new
  blocks written but unreferenced (reclaimed as free next mount). A crash
  before the manifest commit leaves the object at its old, smaller,
  fully-valid size.
- **Shrink (TRUNCATE)**: commit manifest size_bytes DOWN first, then drop
  now-unused trailing extents from the indirect descriptor. Reads are
  always bounds-checked against size_bytes, so no stale trailing bytes are
  ever visible regardless of which side of the crash a shrink lands on.
- **Delete**: clear the manifest name (object disappears from every
  listing/read/write immediately) -> free its extents from the live
  bitmap -> zero the indirect descriptor.

## Migration from V3

The 3 legacy system objects (`sexfiles-proof-v1`, `linen-object-v1`,
`quil-object-v1`) already live at fixed LBAs (2038/2030/2022) outside the
new content pool. Migration wraps each as a single-extent V4 entry
pointing at its EXISTING physical location — no data copy. Their reported
size_bytes becomes 4096, matching what V3 always exposed. Verified intact
post-migration via `disk_persistence_gate.sh` (quil_persist_save/load
round-trip through the legacy slot).

## Protocol changes

- `OP_DISKFS_WRITE`/`OP_DISKFS_READ`: same wire shape (16B/8B chunks), but
  no longer bounded by a fixed 4096-byte object — bounded by the object's
  actual variable size_bytes, and a write past the current end grows it
  (up to the 64 KiB cap).
- `OP_DISKFS_STAT`: now reports the object's real current length, not a
  constant.
- **New**: `OP_DISKFS_TRUNCATE` (0x49) — `arg0 = new_length_bytes`,
  shrink/no-op only (growth is implicit via WRITE). Frees now-unused
  extents and zeroes the tail of the last kept block.

## Perf: extent cache

A save loop issues ~len/16 WRITE calls against the same selected object.
The first cut of this work re-read the 512-byte indirect descriptor from
disk on every single call (V3 had zero such indirection). That was slow
enough in practice to blow through `quil_editor_gate.sh`'s scripted
keystroke timing windows and drop keystrokes sent while quil was still
waiting on a save. Fixed with a single-slot in-memory extent cache
(`V4_EXTENT_CACHE`), refilled on miss and updated on every mutation —
correctness doesn't depend on the cache (a miss just means a slower disk
read, never wrong data), it only removes redundant round-trips on the hot
path.

## Debug tooling

`apps/spindle` gained three commands for exercising the store without
typing thousands of characters through the editor UI:

- `filldoc <id> <bytes>` — SELECT + loop WRITE with a deterministic
  pattern (`byte[i] = (i*37+11) & 0xFF`) + TRUNCATE to the exact length.
- `truncdoc <id> <bytes>` — SELECT + TRUNCATE alone.
- `catdoc <id>` — SELECT + STAT + read back every byte, verify it matches
  the deterministic pattern for the object's current length (not just a
  hash match — an actual per-byte check), report an FNV-1a hash for
  reboot-survival gates to assert exact reproducibility.

## Quil changes

- `QUIL_BUFFER_MAX_LEN`: 512 -> 12288 (spans 3 disk blocks, clears the
  ">4KiB, grows across multiple storage units" bar with headroom under
  the 64 KiB backend cap).
- `QUIL_MAX_DISPLAY_LINES` decoupled from `QUIL_BUFFER_MAX_LEN` (was
  `MAX_LEN + 1`, sizing a stack-local `[u16; N]` line-start table). At
  12 KiB that would have been a ~24 KB stack array against a 64 KB
  per-PD user stack — fixed at a flat 768 lines instead.
- `UNDO_DEPTH`: 16 -> 6 (one full buffer copy per depth level; keeps
  UNDO_RING's BSS footprint from scaling 24x with the buffer).
- `quil_persist_save` now issues `OP_DISKFS_TRUNCATE` after its write
  loop, committing the exact final length so a shorter re-save never
  leaves stale trailing bytes (the write loop always sends full 16-byte
  chunks, which can overshoot the true length by up to 15 bytes).
- `quil_persist_load`'s read loop now requests only the bytes actually
  remaining on its final chunk instead of a flat 8 — with V3's generous
  fixed 4096-byte bound this slack never mattered; V4's tight per-object
  bound would reject the overrun with `ERR_OVERFLOW`.
- Added `memcpy`/`memset`/`memmove`/`memcmp` (quil and spindle both) — the
  larger buffers push codegen past the size where LLVM inlines array
  copies/zeroing and starts emitting real libc calls, which don't exist
  in this freestanding target. Same pattern already used by
  `apps/sexdrive`, `kernel`, `sex-rt`, `crates/silk-client`.

## Two real bugs found while building the growth gate

Both predate V4 and are documented here rather than fixed in full, because
fixing them properly is Lane 3 (streaming/chunked IPC) scope, not Lane 1 —
but they were found *by* Lane 1's own growth gate, so recording them here
is where the next session will look first.

- **sexdrive's real NVMe write path only ever accepts exactly 512 bytes
  per call.** `nvme_write_one_block` (apps/sexdrive/src/main.rs) hardcodes
  `nlb=0` (one LBA) into the submitted NVMe command and gates on
  `size == WRITE_PROOF_LEN` (512) before even reaching that point — a
  4096-byte single-shot write is rejected with `BLOCK_ERR_NO_DEVICE`
  regardless of offset. `v4_zero_block` originally tried to zero a whole
  4096-byte pool block in one `diskfs_block_write` call; every DISKFS_V4
  write that needed to grow an object silently died at that point (see
  the "silent failure" bug below for why it took a while to find). Fixed
  by zero-filling 8 sectors of 512 bytes each, matching every other write
  in this module.

  A related, narrower issue: `write_guard_allows` (same file) is a
  hardcoded allowlist of writable LBA ranges — a deliberate bring-up
  safety guard, not a bug on its own — covering the manifest, the 3
  legacy object slots, and (usefully) a large pre-existing SexFS v0
  range, LBA `[128, 2019]`. The content pool was moved to start at LBA
  128 (`DISKFS_V4_POOL_BASE_LBA`) specifically to land inside that
  already-allowed range rather than needing a guard change.

- **`OP_DISKFS_READ`'s reply protocol can't distinguish real data from an
  error code.** The reply packs up to 8 raw content bytes directly into
  the u64 return value; `pdx_storage_call_bounded`-style callers (spindle,
  quil) then do `if (reply as i64) < 0 { Err(...) }`. A content byte
  >= 0x80 landing in the top position of an 8-byte chunk makes the whole
  reply look negative, so legitimate data gets rejected as a phantom
  error. This is exactly the "reply bit 63 interpreted through sign
  checks" class the sprint's cross-cutting audit calls out — it predates
  V4 and would affect any binary-ish content read through this path, it
  just never manifested because V3's real content stayed within ASCII.
  Not fixed here (needs a real status/data separation in the wire
  protocol — Lane 3). Worked around in the growth gate's test pattern
  (`& 0x7F` instead of `& 0xFF`) so it doesn't block proving growth itself
  is correct; flagged clearly rather than silently avoided.

- **Debugging note**: tracking down the first bug took several rebuild
  cycles because most of `handle_diskfs_write`'s early-return paths had
  no log line at all (silent `return err;`), so a failing call was
  indistinguishable from a call that was never received. Every return
  path in `handle_diskfs_write`/`handle_diskfs_truncate` now logs a
  reason on the way out.

## Also fixed during verification (unrelated to V4, found while re-running
baseline gates before starting this lane)

- `quil`: "New Buffer" cleared `QUIL_DOC_ID` to untitled, which also
  destroyed the only handle "Load" had to reload the previously-saved
  document. Added `QUIL_LAST_DOC_ID`, tracked separately, that survives
  the untitled reset.
- 6 gate scripts (`app_data`, `desktop_cycles`, `dynamic_object`,
  `disk_persistence`, `quil_editor`, `ipc_defer`) had a `r()` helper that
  compared a detail-suffixed failure message (`"FAIL load=none save=241"`)
  against the literal string `"FAIL"` with `==`, which never matches — so
  a failing row never set `FAILED`, and the gate printed an overall PASS
  regardless. Swept to a `FAIL*` glob match.

## Gate

`scripts/diskfs_v4_growth_gate.sh` — two-boot gate proving: create empty,
grow across multiple blocks with exact-content verification, shrink below
a block boundary with no stale trailing bytes, truncate to zero, regrow
after truncate-to-zero (reusing freed blocks), delete + slot reuse, and
(intended) reboot survival with an identical FNV-1a hash before and after
reboot.

**Final result, fresh authoritative run against the `v4_zero_tail`-fixed
binary: 15/15 rows PASS**, including `no_stale_trailing_bytes PASS`,
`exact_content_4112 PASS` (hash `0x2daf16aecc63f795`), and
`reboot_survival_exact_hash PASS hash=0x8e3e43066d3b8995`. Zero `kv()`
keystroke-retry lines in this run's log — a clean pass, not a
timing-masked one. See "Root cause of the no_stale_trailing_bytes
flakiness" above for why earlier runs needed a workaround and why this
run doesn't.

Regression suite (`SKIP_BUILD=1`, run serially — one QEMU instance at a
time, no shared nvme.img/sockets/ports/log paths across gates — against
the same rebuilt ISO): `window_lifecycle_gate.sh`,
`dynamic_desktop_convergence_gate.sh`, `quil_editor_gate.sh`,
`quil_cursor_gate.sh`, `quil_viewport_gate.sh`,
`disk_persistence_gate.sh`, `dynamic_object_gate.sh` — all 8 gates
(growth gate + 7 regression gates) PASS. `quil_cursor_gate` in
particular — which failed outright (timeout, no result) against the
pre-fix binary — now completes its save in ~39s. No regressions from
the DISKFS_V4 base-address change, the canonical disk-layout refactor,
or the `v4_zero_tail` fix.
