# SEXFILES DISKFS AP2.8 FLUSH POISON ISOLATION

Date: 2026-05-22
Classification: **B** (flush audit was not running; CQ poison has another source)
Secondary: C (nvme_flush timeout path doesn't recover queue state)

## Files Changed

| File | Change |
|------|--------|
| `scripts/run_daily_driver_proof.sh` | Remove auto-enable of `SEXOS_STORAGE_100_FLUSH_AUDIT`; add explicit `FLUSH_AUDIT` env read |

Minimal diff (2 lines added, 1 line changed):

```
+FLUSH_AUDIT="${SEXOS_STORAGE_100_FLUSH_AUDIT:-0}"
-if [ "$SEXOS_STORAGE_100_PROOF" = "1" ] && [ "$PERSIST_WRITE" != "1" ] ... ]; then
+if [ "$FLUSH_AUDIT" = "1" ]; then
```

## Was Flush Audit Running Before DiskFS Lane?

**YES** — before this fix, flush audit auto-ran whenever `SEXOS_STORAGE_100_PROOF=1`
was set without persist/negative sub-modes. After fix, flush audit requires explicit
`SEXOS_STORAGE_100_FLUSH_AUDIT=1`.

## Gating Changed

YES — `run_daily_driver_proof.sh` lines 418-420.

Before: auto-enable flush audit when `SEXOS_STORAGE_100_PROOF=1` and no sub-mode.
After: only enable when user explicitly sets `SEXOS_STORAGE_100_FLUSH_AUDIT=1`.

Gate script (`daily_driver_master_gate.sh`) already correctly handles SKIP when
flush audit not triggered (line 4726-4728: "AP5b flush audit not triggered in this log").

No new markers added per mission constraints.

## DiskFS No-Flush Result

| Metric | Value |
|--------|-------|
| cqe_timeout | **YES** — still present |
| DiskFS READ (lba=2046) | status=4 (no_device_other), cid=1290, head=10 |
| DiskFS WRITE (lba=2046) | status=4 (no_device_other), cid=1291, head=10 |
| Gate result | PASS (260 gates, 0 fails) |

**Key finding**: Even without flush audit, CQ timeouts persist at head=10.
The flush audit removal was necessary but insufficient.

### Second Poison Source Identified

The "real IO READ proof" at `apps/sexdrive/src/main.rs` lines 2555+ uses:
- Hardcoded `read_cid: u16 = 0x0045` (CID 69)
- Local `io_sq_tail=0`, `io_cq_head=0`, `io_cq_phase=1`
- Writes `sq_tail=1` to IO SQ doorbell at line 2621, **overwriting** the
  storage proof's tail=10 that was set via `NVME_IO_STATE`

This corrupts hardware SQ tracking: hardware sees tail=1 and ignores
subsequent DiskFS submissions at SQ index 10 and beyond. Result: no
completions posted, CQ head stuck at 10, all DiskFS ops timeout.

Log evidence:
```
1711:[sexdrive.nvme.io.read.err] reason=cqe_timeout cid=69 head=0 phase=1
...
5749:[sexdrive.block.nvme.submit] op=READ lba=2046 bytes=512 cid=1290 tail=10 ready=1
5757:[sexdrive.block.read.handoff.err] reason=cqe_timeout cid=1290 head=10 phase=1
```

## Explicit Flush Audit Result

| Metric | Value |
|--------|-------|
| Flush submit | CID=1290, nsid=1, sq_tail=10 |
| Flush status | cqe_timeout, returns BLOCK_ERR_NO_DEVICE |
| Gate classification | SKIP "flush/FUA not completed or not supported" |
| Gate result | PASS (260 gates, 0 fails) |

Honest AP5b SKIP is preserved. Flush audit never returns false success.

## Default Result (no storage proof)

| Metric | Value |
|--------|-------|
| Storage gates | All SKIP |
| DiskFS ops | Timeout (ready=0 — NVMe never initialized) |
| Gate result | PASS (257 gates, 0 fails) |

## Classification: **B** (primary) + **C** (secondary)

**B) Flush audit was not running; CQ poison has another source.**
After gating fix, flush audit correctly does not auto-run. Yet CQ timeouts
persist from the "real IO READ proof" (lines 2555+) which independently
corrupts the SQ doorbell and NVME_IO_STATE tracking.

**C) Flush audit still needed but queue recovery missing after timeout.**
`nvme_flush()` (line 1415) does NOT update `NVME_IO_STATE.{sq_tail,cq_head,cq_phase}`
on the timeout path (lines 1491-1497). If flush audit is explicitly requested,
the timeout leaves NVME_IO_STATE.cq_head stuck, poisoning ALL subsequent
block operations that share the same IOQ tracking.

## Dual Poison Mechanism Summary

```
Storage Proof (AP3/AP4): uses CIDs 1280-1289, leaves cq_head=10, sq_tail=10, phase=1
        |
        v
[PATH 1: Flush audit (if enabled)]
  nvme_flush() at tail=10, CID=1290
  QEMU doesn't complete FLUSH -> cqe_timeout
  NVME_IO_STATE.{sq_tail,cq_head,cq_phase} NOT updated on error
  head stuck at 10 -> all subsequent ops timeout
        |
[PATH 2: "Real IO READ proof" (always runs)]
  Local io_sq_tail=0, io_cq_head=0, io_cq_phase=1
  Writes tail=1 to SQ doorbell -> hardware ignores submissions beyond index 1
  DiskFS submits at index 10 -> never processed -> timeout
        |
        v
DiskFS bridge: cqe_timeout at head=10, status=4
```

Both paths independently poison the shared NVME_IO_STATE / hardware SQ state.
Path 2 runs unconditionally (lines 2555+), making it the dominant poison.

## Next AP Recommendation

AP2.9: **Remove or isolate the "real IO READ proof"** at lines 2555+.
Three options:
1. **Remove**: Delete the proof block entirely (it's labeled "no BLOCK API
   wiring in this mission" — may be vestigial).
2. **Gate behind env var**: Only run under explicit proof flag so it
   doesn't corrupt the default DiskFS bridge lane.
3. **Fix tracking**: Use `NVME_IO_STATE` instead of local head/tail/phase,
   and resynchronize after completion/timeout.

Option 1 or 2 preferred. Option 3 risks introducing new bugs in the
shared tracking code that could affect proven AP2/AP3/AP4 behavior.

After fixing Path 2, test DiskFS bridge again with no flush:
```
SEXOS_STORAGE_100_PROOF=1 ./scripts/run_daily_driver_proof.sh
```
Expected: DiskFS block ops (READ/WRITE at lba=2046) complete with status=0.
