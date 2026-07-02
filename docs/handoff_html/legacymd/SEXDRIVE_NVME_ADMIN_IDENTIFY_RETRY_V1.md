# SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V1

- date: 2026-05-07
- scope: retry Identify Controller on SexDrive-owned admin queue
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no IO queue creation, no block success claim

## Result

- Reprovision baseline still passes.
- Identify retry command submit/doorbell on owned queue is proven.
- Completion not observed within bounded polling window.
- Outcome: **STOP FIRST blocker** (`cqe_timeout`), no fake success.

## Queue State Used

From reprovision readback markers in same run:
- `AQA=0x000f000f` (16-entry SQ/CQ)
- `ASQ phys=0x1f91c000`, `ASQ va=0x400000005000`
- `ACQ phys=0x102ab000`, `ACQ va=0x400000006000`
- start state used by retry path: `sq_tail=0`, `cq_head=0`, `cq_phase=1`

## Command/CID/PRP1

- `OPC=0x06` (Identify)
- `CNS=1` (Identify Controller)
- `CID=0x0042` (`66`)
- `PRP1=0x1f918000`
- SQ write done with volatile stores; `compiler_fence(Ordering::SeqCst)` issued before SQ0 tail doorbell write.
- SQ0TDBL written at `BAR+0x1000` with `tail=1`.

## Completion Outcome

Observed markers:
- `[sexdrive.nvme.admin.identify.retry.begin] ...`
- `[sexdrive.nvme.admin.identify.retry.submit] cid=66 ...`
- `[sexdrive.nvme.admin.identify.retry.doorbell] sq0tdbl=0x1000 new_tail=1 ...`
- `[sexdrive.nvme.admin.identify.retry.err] reason=cqe_timeout cid=66 head=0 phase=1 polls=1000000`

No `#PF`, `#GP`, `panic` in this run.

## STOP FIRST Blocker

Completion still times out on owned queue with current head/phase assumption.
The next safe step is to prove/fix CQ phase/head progression semantics (including whether initial expected phase should be 0 in this lane and whether CQE visibility needs additional ordering/consumption discipline).

## Files Changed

- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V1.md`

## Final Proof Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(reprovision|admin\.identify\.retry)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_NVME_ADMIN_CQ_PHASE_FIX_V1`
