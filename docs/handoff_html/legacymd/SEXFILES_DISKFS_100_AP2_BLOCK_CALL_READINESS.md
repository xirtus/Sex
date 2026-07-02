# SEXFILES_DISKFS_AP2.3 Ordering Instrumentation

## 1) Files changed
- `servers/sexfiles/src/backends/diskfs.rs`
- `apps/sexdrive/src/main.rs`

## 2) Diagnostic markers added
- DiskFS block call/reply markers:
  - `[sexfiles.diskfs.block.call] op=READ/WRITE lba=L bytes=B slot=S buffer_cap=C device_cap=D`
  - `[sexfiles.diskfs.block.reply] op=READ/WRITE status=N bytes=B`
- DiskFS manifest ensure markers:
  - `[sexfiles.diskfs.manifest.ensure.begin] lba=L`
  - `[sexfiles.diskfs.manifest.ensure.err] status=N reason=...`
- SexDrive typed block handler markers:
  - `[sexdrive.block.req] op=READ/WRITE ready=R lba=L bytes=B buffer_cap=C device_cap=D`
  - `[sexdrive.block.reply] op=READ/WRITE status=4 reason=... ready=R`
  - `[sexdrive.block.reply] op=READ/WRITE status=0 bytes=B ready=1`

## 3) Extracted ordering from log
Source log: `/tmp/sexfiles_ap23_ordering.log`

- IOQ ready:
  - `1571:[sexdrive.nvme.ioq.ready] ...`
- First DiskFS manifest block call:
  - `5306:[sexfiles.diskfs.block.call] op=READ lba=2046 bytes=512 ...`
- SexDrive block request receive:
  - `5694:[sexdrive.block.req] op=READ ready=1 lba=2046 bytes=512 ...`
- Reply status path:
  - `5704:[sexdrive.block.reply] op=READ status=4 reason=no_device_other ready=1`
  - `5796:[sexfiles.diskfs.block.reply] op=READ status=4 bytes=512`

Additional reason evidence in same request path:
- `5701:[sexdrive.block.read.handoff.err] reason=cqe_timeout cid=1291 head=10 phase=1`

Write path mirrors same pattern:
- `5864:[sexdrive.block.req] op=WRITE ready=1 lba=2046 bytes=512 ...`
- `5877:[sexdrive.block.reply] op=WRITE status=4 reason=no_device_other ready=1`
- `5960:[sexfiles.diskfs.block.reply] op=WRITE status=4 bytes=512`

## 4) Conclusion
- Not a before-ready race in this run.
- Ordering proves IOQ was ready before first DiskFS manifest block request (`1571` before `5306`).
- During failing request, SexDrive `ready=1` and status `4` comes from another unavailable/failure branch.
- Closest concrete cause marker in failing read path is `cqe_timeout` (`5701`).

## 5) Recommended next AP (AP2.4)
- Recommended lane: **AP2.4 status reason split** (and tighten no-device reason taxonomy).
- Specifically separate `BLOCK_ERR_NO_DEVICE` causes in SexDrive block paths (e.g. `no_ioq_ready`, `map_failed`, `cqe_timeout`, other transport/backend unavailable branches) so DiskFS can distinguish readiness vs runtime IO failure without changing current semantics yet.
