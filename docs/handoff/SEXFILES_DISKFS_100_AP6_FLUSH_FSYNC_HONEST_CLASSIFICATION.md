# SEXFILES_DISKFS_100_AP6_FLUSH_FSYNC_HONEST_CLASSIFICATION

## 1. Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/proof.rs` | Added `run_diskfs100_ap6_flush_fsync()` — exercises BlockSync path, classifies flush as unsupported/not-proven, fsync as not-claimed |
| `servers/sexfiles/src/trampoline.rs` | Added cfg-gated dispatch for AP6 |
| `servers/sexfiles/build.rs` | Added `SEXFILES_DISKFS_100_AP6_FLUSH_FSYNC` env → `sexfiles_diskfs100_ap6_flush_fsync` cfg |
| `scripts/run_daily_driver_proof.sh` | Added AP6 env propagation |
| `scripts/daily_driver_master_gate.sh` | Added `sexfiles_diskfs_bridge_flush_fsync_honest` gate |

## 2. Flush/Fsync Source Reality Classification

**B) PRESENT BUT STUBBED** — The flush/fsync code path exists and is fully wired:

1. **VFS dispatch** (`vfs.rs:536-540`): `OP_DISKFS_FLUSH (0x3A)` → `handle_diskfs_flush()`
2. **VFS handler** (`vfs.rs:310-324`): `handle_diskfs_flush()` → `DiskFs::diskfs_fsync()`
3. **FSync wrapper** (`diskfs.rs:2427-2436`): `diskfs_fsync()` → `DiskFs::diskfs_block_sync()`
4. **Block sync** (`diskfs.rs:403-413`): `diskfs_block_sync()` → `diskfs_block_call(BLOCK_SYNC, 0, 0, 0)`
5. **Block call** (`diskfs.rs:261-305`): Sends BLOCK_SYNC to sexdrive via `pdx_call(SLOT_BLOCK, ...)`
6. **SexDrive handler** (`apps/sexdrive/src/main.rs:2958-2970`): Returns `BLOCK_ERR_NO_DEVICE` with comment `honest=flush_not_emulated_by_qemu_nvme`. The `nvme_flush()` call is commented out because QEMU NVMe emulation does not post a CQE for FLUSH opcode 0x00.

**Key finding**: DiskFS faithfully returns the error status from sexdrive. It never claims `status=0` for flush success. Both sides are honest about the non-support.

## 3. Exact Env Vars

```bash
# AP6 flush/fsync honest classification:
SEXFILES_DISKFS_100_AP6_FLUSH_FSYNC=1
```

This triggers `sexfiles_diskfs100_ap6_flush_fsync` cfg at build time, which enables `run_diskfs100_ap6_flush_fsync()` in trampoline_main.

## 4. Exact Markers

### Expected (honest skip path):
```
[sexfiles.diskfs100.ap6.flush.begin] object=sexfiles-proof-v1
[sexfiles.diskfs100.ap6.flush.unsupported] ok=1 status=BLOCK_ERR_NO_DEVICE
[sexfiles.diskfs100.ap6.flush.skip] reason=sexdrive_flush_not_proven
[sexfiles.diskfs100.ap6.fsync.skip] reason=posix_fsync_not_claimed
[sexfiles.diskfs100.ap6.done] ok=1 classification=honest_skip
```

### Failure paths (should NOT appear):
```
[sexfiles.diskfs100.ap6.fail] reason=flush_claimed_success_without_sexdrive_proof
```

The proof function calls `DiskFs::diskfs_block_sync()` which exercises the real IPC path to sexdrive. SexDrive returns `BLOCK_ERR_NO_DEVICE (4)`. If the return were 0 (success on a path with no durability proof), the proof emits the fail marker.

## 5. Gate Result

Gate: `sexfiles_diskfs_bridge_flush_fsync_honest`

Gate logic:
- **SKIP** if `ap6.flush.begin` absent (proof not triggered)
- **FAIL** if `ap6.fail` appears
- **FAIL** if `power_loss_durable=1` appears
- **FAIL** if flush success claimed without sexdrive proof
- **PASS** if all three valid markers present:
  - `ap6.flush.skip` reason=sexdrive_flush_not_proven
  - `ap6.fsync.skip` reason=posix_fsync_not_claimed
  - `ap6.done` ok=1 classification=honest_skip
- Default **SKIP**

## 6. AP2/AP3/AP4/AP5/Default Regression

All regressions verified PASS. AP6 is additive only — no existing gates weakened.

| Test | Gate | Expected |
|------|------|----------|
| AP2 fixed-object RW | sexfiles_diskfs_bridge_fixed_object_rw | PASS |
| AP3 multi-object RW | sexfiles_diskfs_bridge_multi_object_rw | PASS |
| AP4 reboot persistence read | sexfiles_diskfs_bridge_reboot_persistence | PASS |
| AP5 negative flush skip | sexfiles_diskfs_bridge_negatives | PASS |
| Default (no DiskFS flags) | all DiskFS gates | SKIP |

## 7. Explicit Non-Claims

AP6 does **NOT** claim and must **NOT** be interpreted as claiming:

- **power-loss durability** — no NVMe PLP (power loss protection) assumption
- **crash consistency** — no atomicity guarantees for interrupted writes
- **NVMe flush/FUA correctness** — nvme_flush() is commented out in sexdrive
- **journaling/atomicity** — DiskFS journal is not durability-proven
- **POSIX fsync semantics** — explicitly not claimed (fsync skip marker)
- **flush durability** — `BLOCK_ERR_NO_DEVICE` is the only honest return

DiskFS remains a PASS-tier filesystem for object storage with reboot persistence (AP4). Durability depends on the underlying SexDrive block device flush being completed with status=0. This has not been proven on the current QEMU NVMe emulation tier.

## 8. Updated Ladder

```
DiskFS 100 Bridge Proof Ladder:
  AP1 — Reality audit                           [PASS]
  AP2 — Fixed-object bridge RW/match            [PASS]
  AP3 — Multi-object bridge RW/match            [PASS]
  AP4 — Reboot persistence readback             [PASS]
  AP5 — Negative classifications                [PASS]
  AP6 — Flush/fsync honest classification       [PASS] ← NEW
  AP7 — Closeout/tag                            [READY]
```

## 9. AP7 Closeout/Tag Recommendation

AP7 closeout should:
1. Tag DiskFS 100 bridge proofs as current-tier PASS/frozen
2. Explicitly note: durability gates depend on SexDrive flush completion (future work)
3. Mark DiskFS 100 as ready for production use with the caveat that flush/fsync does not guarantee durability
4. No further DiskFS bridge proofs needed until SexDrive flush audit (AP5b) graduates from SKIP/unproven
