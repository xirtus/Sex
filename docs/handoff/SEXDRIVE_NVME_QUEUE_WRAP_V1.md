# DISPROVEN: this was never an NVMe queue-wrap bug — see resolution below

## FINAL STATUS: CLOSED. Confirmed root cause was an LBA collision inside
## DISKFS_V4 itself. `apps/sexdrive` was never modified and did not need
## to be. Do not read this document's title or the sections below as
## describing an open driver defect — none exists. If you are looking for
## why DISKFS_V4 content didn't survive reboot, the answer is the
## "Actual root cause" paragraph immediately below, not queue wraparound.

**Actual root cause (confirmed, fixed, verified)**: `apps/sexdrive`'s
boot-time self-test `nvme_multiblock_write_readback_proof` runs
**unconditionally, on every boot, with no gate**, and unconditionally
overwrites LBA 128..131 (`AP4_MULTI_BASE_LBA = 128`, `AP4_MULTI_BLOCKS =
4`) with its own deterministic pattern (`0xA5 ^ i ^ (block*0x33) ^ 0x3C`)
as part of sexdrive's own bring-up self-verification — this is correct,
intentional, expected sexdrive behavior, not a bug in sexdrive.
DISKFS_V4's content pool (`servers/sexfiles/src/vfs.rs`,
`DISKFS_V4_POOL_BASE_LBA`) was originally set to start at LBA 128 too —
the same address, chosen independently because it was the start of the
already-allowlisted SexFS v0 write range (`write_guard_allows`, LBA
[128,2019]). Every object whose first-fit allocation landed in the
pool's first block (LBA 128, by far the most common allocation —
first-fit always tries there first) got silently overwritten by
sexdrive's self-test on the *next* boot, before DiskFS or any client had
a chance to read it.

The exact byte pattern match was conclusive: for block 0 (`b=0`), byte 0
is `0xA5^0^0^0x3C = 0x99`, byte 1 is `0x98`, byte 2 is `0x9B` — an exact,
byte-for-byte match to the "garbage" observed after every failing reboot.

**Fix, final form**: `DISKFS_V4_POOL_BASE_LBA` now lives in
`crates/sex-pdx/src/lib.rs`'s canonical disk-layout module (single source
of truth for every fixed on-disk LBA region in the system — see
`docs/handoff/DISK_LAYOUT_V1.md`) at **LBA 400**, clear of every sexdrive
self-test region including the ones gated off in normal builds (AP5A at
256, AP6 at 384 — these were a SECOND, independent collision the
compile-time `const _: () = assert!(...)` checks in that module caught
before this could ship, which manual testing alone would never have
found since those self-tests never run in a default build). Both
`apps/sexdrive` and `servers/sexfiles` now reference the same canonical
constants rather than each independently hardcoding a value — that
absence of a shared source of truth is what let the original 128/128
collision happen silently in the first place.

Verified: isolated create/fill/verify/hard-reboot/verify cycle produces
an **identical content hash** before and after reboot. The full
`scripts/diskfs_v4_growth_gate.sh` — grow, shrink, truncate, regrow,
slot reuse, migration, reboot — passes end to end from a clean disk
image built at LBA 400. Exact results recorded in
`docs/handoff/DISKFS_V4_GROWTH_V1.md`.

**`apps/sexdrive` was never modified.** The "queue wrap" correlation with
"~16 writes" was a coincidence of test design, not the actual trigger:
every failing test in this investigation happened to use the pool's
first block (block 0 = LBA 128) as its first or only allocation,
regardless of how many total writes it issued — a fresh single-block
32-byte object failed exactly the same way a 257-write 4112-byte object
did, because both landed on LBA 128 first. Do not resume the "audit
apps/sexdrive's SQ/CQ handling" investigation described below unless a
NEW, independent, focused reproducer demonstrates an actual queue
failure unrelated to LBA placement — the analysis below was a reasonable
hypothesis at the time but is now known to have been the wrong layer.

## Original investigation (kept for methodology reference only)

The section below is the analysis as it stood before the LBA collision
was found. The rule-outs (not caching, not shutdown method, not DiskFS's
own write payload, not extent resolution) are still accurate and were
useful in narrowing the search — they just correctly pointed at "not
sexfiles" without yet finding the true "not sexdrive either, a coordinate
collision" answer.

## Symptom

Any object whose content required more than ~16 real `OP_DISKFS_WRITE`
round-trips (each round-trip becomes one `BLOCK_WRITE` NVMe command) in a
single boot session reads back as garbage after a reboot — even though:

- the SAME content reads back correctly within that same boot session,
  every time, immediately after writing;
- every individual NVMe write command reports success (status 0, CQE
  observed) at submission time;
- the manifest/metadata layer (object name, size, generation, checksum)
  is completely unaffected — `OP_DISKFS_STAT` after reboot reports the
  correct name and size every time.

The garbage value is deterministic: reading the first 8 bytes of the
corrupted region consistently returns the byte sequence
`99 98 9b 9a 9d 9c 9f 9e` (and further reads continue in the same
byte-pair-swapped, arithmetically-structured pattern — see raw dump
below). This is NOT random corruption; it looks like leftover/synthetic
data from somewhere in the NVMe emulation or queue-management path, not
zeros, not the correct content, and not any other object's content
either.

## What has been ruled out

- **Host/QEMU block-layer write caching.** Reproduces identically with
  `cache=writethrough` on the `-drive` (forces synchronous writes,
  bypassing the host page cache). Ruled out.
- **Shutdown method / unflushed writes at kill time.** Reproduces
  identically whether the VM is stopped with `kill -9` immediately,
  `kill` (SIGTERM) with a 1s grace period before `kill -9`, or a fully
  clean QMP `quit` command with an 8s settle period beforehand. Ruled
  out.
- **DISKFS_V4's own write payload.** Added a temporary diagnostic that
  dumps the shared MemLend buffer's content immediately before and after
  every sector write inside `v4_zero_block` (`servers/sexfiles/src/vfs.rs`,
  removed again once confirmed). The buffer is provably all-zero at
  write-time, every single time, for every object, including the ones
  that end up corrupted after reboot. sexfiles is sending correct data on
  every call.
- **Object/slot reuse specifically.** Originally suspected (see below),
  but a fresh object that was NEVER deleted/recreated — just one object,
  created once, filled once, with enough writes (32) to cross the queue
  wrap boundary — reproduces the exact same corruption. This rules out
  anything in DISKFS_V4's delete/create/cache-invalidation logic.
- **DiskFS_V4's extent resolution / bitmap logic.** A diagnostic dump of
  the resolved extent (slot, LBA, sector count) immediately before every
  read showed the CORRECT extent (matching what was allocated and
  written) in every failing case. The read path in `vfs.rs` is asking for
  the right bytes at the right physical location; what comes back is
  wrong.

**Conclusion: the bug is below `servers/sexfiles`, in `apps/sexdrive`'s
NVMe command/queue handling, or in QEMU's NVMe device emulation itself.**
Given `apps/sexdrive`'s SQ/CQ ring is a fixed 16-entry queue
(`cq_head = (cq_head + 1) % 16` and equivalent SQ-tail wraparound present
throughout `nvme_write_one_block`/`nvme_read_into_mapped_va`), and the
failure threshold empirically sits right around 16-17 cumulative NVMe
commands per boot session, queue-depth wraparound handling is the prime
suspect.

## Why this was never caught before

DISKFS_V3 objects were always exactly one 4096-byte block, written via a
handful of proof-style calls — no real workload before DISKFS_V4 ever
issued more than a few real NVMe commands in one boot session. DISKFS_V4
is the first feature to do real, sustained multi-sector I/O, and it
immediately exposed this pre-existing infrastructure limit.

## Minimal reproduction (no Quil, no dynamic-object logic)

1. Boot from a blank NVMe image.
2. Create one object (`mkdoc`), select it, issue enough real
   `OP_DISKFS_WRITE` calls to exceed ~16 total NVMe write commands in the
   session (32 sixteen-byte writes for a 512-byte object is enough).
3. Read the object back — correct, every time, in-session.
4. Kill and reboot (any shutdown method — all confirmed equivalent).
5. Read the object back again — garbage, deterministic value, starting
   `99 98 9b 9a 9d 9c 9f 9e ...`.

Confirmed directly against the raw backing file with `xxd` — the WRONG
bytes are genuinely what's physically on disk, not a read-path artifact:

```
$ xxd -s 65536 -l 64 nvme.img
00010000: 9998 9b9a 9d9c 9f9e 9190 9392 9594 9796
00010010: 8988 8b8a 8d8c 8f8e 8180 8382 8584 8786
00010020: b9b8 bbba bdbc bfbe b1b0 b3b2 b5b4 b7b6
00010030: a9a8 abaa adac afae a1a0 a3a2 a5a4 a7a6
```

Note the structure: each 8-byte group is an ascending run with adjacent
pairs byte-swapped (`98 99` -> `99 98`, `9a 9b` -> `9b 9a`, ...), and
successive 16-byte lines start at `0x98`, `0x88`, `0xb8`, `0xa8` — not an
obviously "random garbage" pattern. Worth keeping in mind when
root-causing: this looks like it could be misinterpreted queue/command
state (CID, SQ/CQ indices, or similar) landing in the data path, not
memory poison or literal uninitialized RAM.

## Required driver-focused audit (apps/sexdrive)

Audit at minimum:

- SQ tail advancement and modulo queue depth
- CQ head advancement and modulo queue depth
- completion phase-bit handling across wrap
- doorbell writes
- queue-full detection
- outstanding-command accounting
- command identifier (CID) allocation and reuse
- CID-to-request correlation
- stale completion handling
- memory barriers before ringing the SQ doorbell
- DMA visibility of command entries and completion entries
- whether the controller queue size is encoded as entries-minus-one
- initial CQ phase value
- phase toggling only when CQ head wraps
- whether SQ head is learned from completions or tracked incorrectly
- reuse of SQ entries before their commands complete
- assumptions that only one request can be outstanding
- timeout paths that leave queue state inconsistent
- reset/reboot behavior

## Required driver-focused gate (not yet written)

A minimal, sexfiles/DiskFS-independent gate exercising `apps/sexdrive`
directly:

- at least 64 sequential sector writes
- at least 64 sequential sector reads
- exact per-sector data verification
- crossing SQ/CQ wrap multiple times
- mixed read/write sequence
- repeated use after the first wrap
- reboot and exact verification
- no timeout, stale completion, duplicate CID, or queue reset
- zero faults, traps, or panics
- distinguishable per-sector payloads (not a repeating pattern) so
  reordered, duplicated, stale, or missing operations are identifiable
  precisely — unlike this handoff's repro pattern, which was closer to
  the wrap boundary than ideal for full diagnosis

## What to do once fixed

1. Commit the sexdrive fix on its own, separately from any DiskFS
   changes.
2. Rerun `scripts/diskfs_v4_growth_gate.sh` UNCHANGED — do not weaken any
   assertion to make it pass.
3. Confirm exact content hashes and lengths match before and after
   reboot for every row, including `reboot_survival_exact_hash`.
4. Rerun `disk_persistence_gate.sh`, `dynamic_object_gate.sh`,
   `ipc_defer_gate.sh`, `quil_editor_gate.sh`, `window_lifecycle_gate.sh`,
   `dynamic_desktop_convergence_gate.sh` for regressions.
5. Only then mark DISKFS_V4 Lane 1 complete and proceed to Lane 2
   (crash-aware metadata publication and recovery) — Lane 2's own
   recovery-under-interruption gates would be meaningless on top of an
   unreliable block device.

## Related, separate, lower-priority bug

`docs/handoff/DISKFS_V4_GROWTH_V1.md` documents a second, unrelated
pre-existing bug (`OP_DISKFS_READ` reply sign-bit vs. data-byte
ambiguity) — do not conflate the two. That one is real but narrower in
impact (Lane 3 scope, doesn't block basic reboot durability) and has a
working test-pattern workaround already in place
(`scripts/diskfs_v4_read_signbit_regression_gate.sh`). This queue-wrap
bug has no workaround — it blocks exact content persistence outright once
a session does enough real I/O.
