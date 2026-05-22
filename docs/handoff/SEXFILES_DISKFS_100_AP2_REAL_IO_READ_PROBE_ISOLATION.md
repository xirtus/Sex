# SEXFILES_DISKFS_100_AP2_REAL_IO_READ_PROBE_ISOLATION

Date: 2026-05-22
Author: Sex Microkernel repo
Classification: A (legacy probe was running unconditionally; gating removed CQ poison)

## 1. Files Changed

- `apps/sexdrive/src/main.rs` — added `SEXOS_STORAGE_100_IO_READ_PROBE` env gate

No runner changes. No kernel edits. No sex-pdx ABI edits.

## 2. Exact Legacy Probe Source Location

`apps/sexdrive/src/main.rs` in `fn nvme_probe_bar()`:
- Start: line 2555 (original) / 2562 (after edit) — comment `// One real IO READ proof`
- End: line 2719 (original) / 2726 (after edit) — `NVME_IO_STATE.sq_tail = io_sq_tail`
- The block allocates a physical page, builds a raw NVMe READ SQE (opcode 0x02,
  NSID=1, SLBA=0), writes SQ doorbell with `io_sq_tail=1`, polls CQ, and
  saves state to `NVME_IO_STATE`.

## 3. Whether It Ran Before

YES. The legacy probe ran **unconditionally** at the end of `nvme_probe_bar()`,
after the AP3/AP4/AP5a proof chain. No env gate existed.

## 4. Env Gating Behavior

| Env | Legacy Probe |
|-----|-------------|
| default (no env) | SKIP |
| `SEXOS_STORAGE_100_PROOF=1` | SKIP |
| `SEXOS_STORAGE_100_PROOF=1 SEXOS_DISKFS_OBJECT_TABLE_PROOF=1` | SKIP |
| `SEXOS_STORAGE_100_IO_READ_PROBE=1` (explicit) | RUN |

When skipped, emits: `[sexdrive.storage100.io_read_probe.skip] reason=not_requested`

AP3/AP4/AP5a proof code is **unchanged**.

## 5. DiskFS No-Probe Result

- Profile: `SEXOS_STORAGE_100_PROOF=1 SEXOS_DISKFS_OBJECT_TABLE_PROOF=1`
- Legacy probe: **SKIPPED**
- cqe_timeout: **0** (remains zero)
- DiskFS block status: **31 replies, all status=0**
- Gate result: **PASS (257 gates proved, 0 FAIL, 0 faults)**

## 6. Positive SexDrive Storage Result

- Profile: `SEXOS_STORAGE_100_PROOF=1`
- Legacy probe: **SKIPPED**
- sexdrive_storage_ioq_ready: **PASS**
- sexdrive_storage_single_block_rw: **PASS**
- sexdrive_storage_multiblock_rw: **PASS**
- Gate result: **PASS (260 gates proved, 0 FAIL, 0 faults)**

## 7. Explicit Legacy Probe Result

- Profile: `SEXOS_STORAGE_100_PROOF=1 SEXOS_STORAGE_100_IO_READ_PROBE=1`
- Legacy probe: **RAN** — but **cqe_timeout** on legacy probe itself
- Subsequent DiskFS IOs also **cqe_timeout**:
  - `[sexdrive.nvme.io.read.err] reason=cqe_timeout cid=69 head=0 phase=1`
  - `[sexdrive.block.read.handoff.err] reason=cqe_timeout cid=1290 head=10 phase=1`
  - `[sexdrive.nvme.write.err] reason=cqe_timeout cid=1291 head=10 phase=1`
- This **confirms** the diagnosis: legacy probe writes `sq_tail=1` but
  storage proof tail is ~10, corrupting doorbell; subsequent submissions at
  indices 10+ are ignored by NVMe controller.

## 8. Default Result

- Profile: (no env)
- NVMe BAR resolve fails (no NVMe cap in default VM)
- Legacy probe: **not reached** (no skip marker emitted)
- Gate result: **PASS (257 gates proved, 0 FAIL, 0 faults)**

## 9. Classification

**A** — Legacy real IO READ probe was running unconditionally before this fix.
Env-gating it removed the CQ poison source. DiskFS requests now complete without
cqe_timeout.

## 10. Next AP Recommendation

AP2.9 is complete. The DiskFS no-flush lane now functions with zero cqe_timeout.

The legacy probe remains available for explicit testing via
`SEXOS_STORAGE_100_IO_READ_PROBE=1` but produces its own cqe_timeout because
it corrupts doorbell state — that is the expected legacy behavior and confirms
the diagnosis.

Next: AP2.10 should focus on verifying multi-object DiskFS writes complete
end-to-end with the no-flush lane, now that CQ poison is removed.

## 11. Commit

```
fix(storage): gate legacy IO read probe behind SEXOS_STORAGE_100_IO_READ_PROBE

The vestigial "real IO READ proof" in nvme_probe_bar() ran
unconditionally and wrote io_sq_tail=1 (local tracking) to the
NVMe IO SQ doorbell, overwriting the storage proof tail (~10).
This caused subsequent DiskFS submissions at SQ indices 10+ to
be ignored by the NVMe controller, producing cqe_timeout.

Gate the probe behind explicit SEXOS_STORAGE_100_IO_READ_PROBE=1.
Default, storage proof, and DiskFS profiles all skip it.
AP3/AP4/AP5a proof code is unchanged.

Verified:
- DiskFS no-flush: 0 cqe_timeout, 31 block replies all status=0
- Storage proof: sexdrive_storage_ioq_ready/single/multi all PASS
- Explicit probe: reproduces cqe_timeout (diagnosis confirmed)
- Default: PASS, no regression
```
