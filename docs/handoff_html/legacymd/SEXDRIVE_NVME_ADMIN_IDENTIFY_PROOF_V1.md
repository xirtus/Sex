# SEXDRIVE_NVME_ADMIN_IDENTIFY_PROOF_V1

- date: 2026-05-07
- scope: submit one NVMe Admin Identify command via preconfigured admin queue
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no kernel/syscall/ABI/storage changes

## Result

- Identify command **submission path proved**.
- Completion path did **not** prove: bounded poll timed out.
- Outcome is **STOP FIRST blocker** (exact reason below), not fake success.

## Command Layout Used

64-byte admin command (SQE):
- `CDW0`:
  - `OPC = 0x06` (Identify)
  - `CID = 0x5aa5` (`23205`)
- `DPTR/PRP1`:
  - `DW6/DW7 = identify_buffer_phys`
- `CDW10`:
  - `CNS = 1` (Identify Controller)

Doorbell:
- `CAP.DSTRD = 0` (already known), stride=4 bytes
- SQ0 tail doorbell offset `0x1000`

## ASQ/ACQ Access Method

Used preconfigured queue base registers from BAR0:
- `ASQ @ 0x0028`
- `ACQ @ 0x0030`

Then mapped those physical addresses into sexdrive VA using existing syscall path:
- syscall `30` (`MAP_MEMORY`) for `ASQ` page and `ACQ` page

No queue reprovisioning or reset done.

## Identify Buffer Allocation/Physical Address Method

- Allocated one page via syscall `31` (`ALLOCATE_MEMORY`)
- Mapped it via syscall `30` (`MAP_MEMORY`)
- Used returned physical address directly in PRP1 (`DW6/DW7`)

## Runtime Evidence

From `.gate_master/serial.log`:
- `[sexdrive.nvme.admin.identify.begin] asq_phys=0x1ffdd000 acq_phys=0x1ffde000 asq_va=0x400000005000 acq_va=0x400000006000 id_phys=0x102ab000 id_va=0x400000007000 sq_tail=0 cq_head=0 sqe=64 cqe=256`
- `[sexdrive.nvme.admin.identify.cmd.submit] cid=23205 opc=0x6 cns=1 prp1=0x102ab000`
- `[sexdrive.nvme.admin.identify.doorbell] sq0_tail_old=0 sq0_tail_new=1 db_off=0x1000`
- `[sexdrive.nvme.admin.identify.err] reason=cqe_timeout cid=23205`

No `#PF`, `#GP`, or `panic` lines in this run.

## STOP FIRST Blocker

`sexdrive` can submit into preconfigured queues, but completion ownership/state is not safely determined with current primitives:
- Preconfigured admin queue appears controller/firmware-owned and already enabled.
- `sexdrive` lacks authoritative queue epoch/phase ownership and command-stream coordination for that preowned queue context.
- Bounded polling (including CQ-wide CID scan) still observes no matching completion for submitted CID.

Proceeding further without an explicit queue-ownership contract risks writing into a live queue context not owned/coordinated by sexdrive.

## What Changed in Code

`apps/sexdrive/src/main.rs`:
- Refactored queue branch so preconfigured queue state continues into Identify path.
- Added Admin Identify markers:
  - `[sexdrive.nvme.admin.identify.begin]`
  - `[sexdrive.nvme.admin.identify.cmd.submit]`
  - `[sexdrive.nvme.admin.identify.doorbell]`
  - `[sexdrive.nvme.admin.identify.cqe]` (emits only if seen)
  - `[sexdrive.nvme.admin.identify.ok]` (not reached)
  - `[sexdrive.nvme.admin.identify.err]`
- Added bounded CQ poll with full CQ entry scan for matching CID.

Block API behavior unchanged: still honest `ERR_NO_DEVICE` pre-IO.

## Final Proof Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(bar\.resolve|identity|queue|admin\.identify)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

STOP FIRST blocker continuation:
- `SEXDRIVE_NVME_ADMIN_QUEUE_OWNERSHIP_PROOF_V1`

After ownership proof resolves:
- `SEXDRIVE_NVME_IO_QUEUE_CREATE_PROOF_V1`
