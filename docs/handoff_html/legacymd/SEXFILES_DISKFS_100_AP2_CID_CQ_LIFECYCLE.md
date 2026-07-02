# SEXFILES_DISKFS_AP2_5_CID_CQ_LIFECYCLE

## 1. Files changed
- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXFILES_DISKFS_100_AP2_CID_CQ_LIFECYCLE.md`
- backup created: `apps/sexdrive/src/main.rs.ap25_cid_cq_lifecycle.bak`

## 2. Markers added
- `[sexdrive.nvme.submit.detail] path=typed/selftest ...`
- `[sexdrive.nvme.doorbell] path=typed/selftest ...`
- `[sexdrive.nvme.poll.begin] path=typed/selftest ...`
- `[sexdrive.nvme.poll.end] path=typed/selftest ...`
- `[sexdrive.nvme.poll.timeout.detail] path=typed/selftest ...`

## 3. Self-test submit/poll shape
From `/tmp/sexfiles_diskfs_ap25_cid_cq.log`:
- selftest WRITE cid=1280: sq_tail 0->1, cq_head=0, cq_phase=1, polls=1, found=1, status=0
- selftest READ cid=1281: sq_tail 1->2, cq_head=1, cq_phase=1, polls=1, found=1, status=0
- selftest lifecycle continues through typed AP4 lanes with normal CQ progress (cq_head increments, tails increment).

## 4. Typed submit/poll shape (DiskFS bridge lane)
From `/tmp/sexfiles_diskfs_ap25_cid_cq.log`:
- DiskFS READ call enters typed path at lba=2046, bytes=512, ready=1.
- typed READ cid=1291: sq_tail 10->11, cq_head=10, cq_phase=1, polls=1_000_000, found=0.
- timeout detail: expected_cid=1291, seen_cid=0, seen_phase=1, seen_status=16395.
- DiskFS WRITE call enters typed path at lba=2046, bytes=512, ready=1.
- typed WRITE cid=1292: sq_tail 10->11, cq_head=10, cq_phase=1, polls=1_000_000, found=0.
- timeout detail: expected_cid=1292, seen_cid=0, seen_phase=1, seen_status=16395.

## 5. CID/CQ conclusion (A-G)
- **E) Completion never arrives for typed command**.
- Evidence: typed path submits and doorbells correctly (tail advances), but poll never finds matching CID, and CQ slot at head appears unchanged/stale across full poll window.

## 6. AP2 blockage status
- AP2 is still blocked.
- `sexfiles.diskfs100.ap2.*` markers are still absent in this route and typed block bridge returns timeout status.

## 7. Next fix recommendation
- Keep these diagnostics.
- Next safe fix should isolate why typed lane targets LBA 2046/offset `0xffc00` but CQ entry at head is stale (`seen_cid=0` with fixed phase), likely by auditing queue ownership/command visibility for this specific lane only, without changing ABI or success semantics.
