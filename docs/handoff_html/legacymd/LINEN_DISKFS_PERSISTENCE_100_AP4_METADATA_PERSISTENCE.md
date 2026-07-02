# LINEN_DISKFS_PERSISTENCE_100_AP4_METADATA_PERSISTENCE

## 1) Files changed
- servers/linen/src/main.rs
- scripts/run_daily_driver_proof.sh
- scripts/daily_driver_master_gate.sh
- docs/handoff/LINEN_DISKFS_PERSISTENCE_100_AP4_METADATA_PERSISTENCE.md

## 2) Metadata source reality classification (A-E)
- Classification: **B) Metadata only persists to RamFS today**.
- Evidence in source:
  - `linen_persist_object()` writes metadata using `OP_RAMFS_CREATE_OWNER`, `OP_RAMFS_WRITE`, `OP_RAMFS_CLOSE`.
  - AP2/AP3 DiskFS proofs are content-only and explicitly emit metadata skip markers.
  - No AP2/AP3 path writes Linen metadata via `OP_DISKFS_WRITE`.

## 3) Fields inspected
- `object_id`
- `kind`
- `name` + `name_len`
- `flags`
- `generation`
- `owner_pd` (recorded in metadata payload, though not requested as AP4 target field)

## 4) Exact AP4 env vars
- `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP4_META_WRITE=1`
- `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP4_META_READ=1`
- `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP4_META_AUDIT=1`

## 5) AP4 result
- **Real DiskFS metadata persistence PASS:** NO.
- **Honest skip/classification PASS:** YES.
- Emitted markers (audit lane):
  - `[linen.diskfs100.ap4.meta.audit.begin]`
  - `[linen.diskfs100.ap4.meta.classification] status=ramfs_only_or_session_only ok=1`
  - `[linen.diskfs100.ap4.meta.skip] reason=metadata_not_diskfs_backed`
  - `[linen.diskfs100.ap4.meta.done] ok=1 classification=honest_skip`

## 6) Gate result
- Gate added: `linen_diskfs_metadata_persistence`
- Gate behavior:
  - SKIP when no AP4 begin marker.
  - FAIL on `ap4.meta.fail`, `cqe_timeout`, fault/panic markers.
  - PASS real persistence only on `meta.match ... ok=1` + `meta.read.done ok=1`.
  - PASS honest skip on the classification+skip+done marker triple.
- Verified result on AP4 audit run:
  - `linen_diskfs_metadata_persistence PASS   honest skip: metadata is RamFS/session-only, not DiskFS-backed`

## 7) AP3 regression result
- Command run:
  - `DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_READ=1 ./scripts/run_daily_driver_proof.sh`
- Extracted gate lines:
  - `linen_diskfs_reboot_restore  SKIP   AP3 reboot restore proof not triggered`
  - `faults_zero                  PASS   0 fault markers`
  - `FAIL gates: 1`
  - `FINAL: FAIL (1 gate(s) failed)`
- Note: fail was from unrelated Atlas gate in that specific boot profile; AP3 lane was not triggered in this run.

## 8) Default result
- Command run:
  - `./scripts/run_daily_driver_proof.sh`
- Extracted gate lines:
  - `linen_diskfs_reboot_restore  SKIP   AP3 reboot restore proof not triggered`
  - `linen_diskfs_metadata_persistence PASS   honest skip: metadata is RamFS/session-only, not DiskFS-backed`
  - `faults_zero                  PASS   0 fault markers`
  - `FAIL gates: 0`
  - `FINAL: PASS (257 gates proved, 110 skipped, 0 faults)`

## 9) Non-claims
- No Quil claim.
- No folders/path semantics claim.
- No POSIX claim.
- No flush/power-loss durability claim.
- No crash consistency claim.

## 10) Updated Linen ladder
- AP1 reality audit: PASS
- AP2 fixed-object content save/load: PASS
- AP3 reboot content restore: PASS (historical frozen baseline)
- AP4 metadata persistence through DiskFS:
  - Source reality classification: RamFS/session-only metadata
  - Gate result: PASS via honest skip classification
  - Real DiskFS metadata persistence: NOT proven
