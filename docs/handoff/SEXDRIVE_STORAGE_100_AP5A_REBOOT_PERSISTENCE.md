# SEXDRIVE_STORAGE_100_AP5A_REBOOT_PERSISTENCE

## 1) Files changed
- `apps/sexdrive/src/main.rs`
- `scripts/run_daily_driver_proof.sh`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/SEXDRIVE_STORAGE_100_AP5A_REBOOT_PERSISTENCE.md`

## 2) Exact env vars
- `SEXOS_STORAGE_100_PROOF=1`
- `SEXOS_STORAGE_100_PERSIST_WRITE=1`
- `SEXOS_STORAGE_100_PERSIST_READ=1`

## 3) Exact two commands used
```bash
SEXOS_STORAGE_100_PROOF=1 SEXOS_STORAGE_100_PERSIST_WRITE=1 ./scripts/run_daily_driver_proof.sh
cp /tmp/sexos_daily_driver_proof.log /tmp/sexos_storage_ap5a_write.log

SEXOS_STORAGE_100_PROOF=1 SEXOS_STORAGE_100_PERSIST_READ=1 ./scripts/run_daily_driver_proof.sh
cp /tmp/sexos_daily_driver_proof.log /tmp/sexos_storage_ap5a_read.log
```

## 4) Same image confirmation
- `.gate_master/nvme.img` was not deleted or recreated between write/read boots.
- Runner behavior is explicit for AP5a read mode: if image is missing it fails; it does not create/zero the image in read mode.

## 5) AP5a persistence lane parameters
- `base_lba = 256`
- `blocks = 4`
- `block_size = 512`
- Pattern formula:
  - `byte[i] = (0x5A ^ i ^ (b * 0x21) ^ 0xC3) & 0xFF`

## 6) Write log result
- Log: `/tmp/sexos_storage_ap5a_write.log`
- Markers:
  - `[sexdrive.storage100.persist.write.begin] base_lba=256 blocks=4 bytes_per_block=512`
  - Four `[sexdrive.storage100.persist.write.block] ... status=0 bytes=512`
  - `[sexdrive.storage100.persist.write.done] blocks=4 ok=1`

## 7) Read log result
- Log: `/tmp/sexos_storage_ap5a_read.log`
- Markers:
  - `[sexdrive.storage100.persist.read.begin] base_lba=256 blocks=4 bytes_per_block=512`
  - Four `[sexdrive.storage100.persist.read.block] ... status=0 bytes=512`
  - Four `[sexdrive.storage100.persist.read.match] ... bytes=512 ok=1`
  - `[sexdrive.storage100.persist.read.done] blocks=4 ok=1`

## 8) Match evidence
- `idx=0 lba=256 ok=1`
- `idx=1 lba=257 ok=1`
- `idx=2 lba=258 ok=1`
- `idx=3 lba=259 ok=1`

## 9) Gate result
Per-log gate (`sexdrive_storage_reboot_persistence`):
- Write log gate: PASS (`write boot persistence blocks recorded`)
- Read log gate: PASS (`read boot persistence match verified`)

AP5a full acceptance requires:
- write log PASS
- read log PASS
- same `.gate_master/nvme.img`
- no image recreation between runs

## 10) Explicit limitation / non-claims
AP5a proves reboot persistence only across QEMU boots using the same backing image.

It does not prove:
- power-loss durability
- NVMe flush/FUA correctness
- filesystem durability

## 11) Updated ladder
- AP2 PASS
- AP3 PASS
- AP4 PASS
- AP5a reboot persistence PASS
- AP5b flush/durability audit pending
- AP6 negatives pending
- AP7 closeout/tag pending
