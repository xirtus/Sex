# SexFiles DiskFS 100 Current Tier Closeout V1

**Tag:** `sexfiles-diskfs-100-current-tier-v1`
**Branch:** master
**HEAD:** `3a58bbed` — sexfiles: classify DiskFS flush fsync honestly
**Closeout date:** 2026-05-23

---

## 1. Baseline

| Field | Value |
|---|---|
| Branch | master |
| HEAD commit | `3a58bbedaff1627aa22b1cc9e575b9b9dc2abe21` |
| HEAD subject | sexfiles: classify DiskFS flush fsync honestly |
| Intended tag | `sexfiles-diskfs-100-current-tier-v1` |

### Recent commits in tier

```
3a58bbed sexfiles: classify DiskFS flush fsync honestly
3e80e734 sexfiles: prove DiskFS negative classifications
5c3c86dd sexfiles: prove DiskFS reboot persistence
1623a65c kernel: reserve boot frame allocations from global allocator
1e5553ce kernel: instrument allocator frame overlap
d6acfc7e docs: audit DiskFS AP3 VA PKU fault
3add9aea sexfiles: retry DiskFS multi-object bridge proof
81b0e56a fix(sexfiles): require explicit AP2 DiskFS proof profile
77349bb1 sexfiles: isolate DiskFS multi-object PKU fault
686984b0 sexfiles: prove fixed-object DiskFS bridge read write match
```

---

## 2. Proven Ladder

- **AP1** reality audit: PASS
- **AP2** fixed-object bridge RW/match: PASS
- **AP3** multi-object bridge RW/match: PASS
- **AP4** reboot persistence readback: PASS
- **AP5** negative classifications: PASS
- **AP6** flush/fsync honest classification: PASS / honest non-claim
- Kernel allocator overlap: fixed
- SexDrive storage current tier: frozen/proven

---

## 3. Replay Results (AP7 Closeout)

All lanes replayed on 2026-05-23. 0 FAIL gates, 0 faults in every lane.

### 3.1 AP2 — Fixed-object Bridge RW/match

```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_PROOF=1
Gate:  sexfiles_diskfs_bridge_fixed_object_rw  PASS  IOQ-ready + select.ok + read.match ok=1 + done ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.2 AP3 — Multi-object Bridge RW/match

```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP3_PROOF=1
Gate:  sexfiles_diskfs_bridge_multi_object_rw   PASS  linen+quil match ok=1 + proof intact + done ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.3 AP4 — Reboot Persistence Write Boot

```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_WRITE=1
Gate:  sexfiles_diskfs_bridge_reboot_persistence PASS  AP4 write boot: chunks written + readback match + done ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.4 AP4 — Reboot Persistence Read Boot

```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_READ=1
Runner: AP4 read boot: preserving existing nvme.img (no recreation)
Gate:  sexfiles_diskfs_bridge_reboot_persistence PASS  AP4 read boot: chunks read + byte match + done ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.5 AP5 — Negative: Intentional Mismatch

```
SEXFILES_DISKFS_100_AP5_NEGATIVE=1 SEXFILES_DISKFS_100_AP5_NEG_MISMATCH=1
Gate:  sexfiles_diskfs_bridge_negatives          PASS  neg mismatch: intentional mismatch detected ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.6 AP5 — Negative: Missing Image

```
SEXFILES_DISKFS_100_AP5_NEGATIVE=1 SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE=1
Gate:  sexfiles_diskfs_bridge_negatives          PASS  neg missing image: honest failure detected ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.7 AP6 — Flush/fsync Honest Classification

```
SEXFILES_DISKFS_100_AP6_FLUSH_FSYNC=1
Gate:  sexfiles_diskfs_bridge_flush_fsync_honest PASS  flush fsync honest classification: skip/unsupported ok=1
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

### 3.8 Default Daily

```
No env flags (default profile)
All sexfiles_diskfs_bridge_* gates             SKIP  (as expected)
All sexdrive_storage_* gates                   SKIP  (as expected)
Gate:  faults_zero                              PASS  0 fault markers
Result: FAIL gates: 0  FINAL: PASS
```

---

## 4. Exact Claim Boundary

### PROVEN

- DiskFS bridge reaches real SexDrive/NVMe storage.
- Fixed-object write/read/match works (128-byte object, 1 object).
- Multi-object write/read/match works (linen+quil, multiple objects).
- Reboot persistence across QEMU proof boots with same `nvme.img` works.
- Negative cases (intentional mismatch, missing image) classify honestly.
- Flush/fsync makes no false durability claim (classified `honest_skip`).
- No faults observed in any proof lane.

### NOT CLAIMED

- POSIX filesystem semantics.
- Arbitrary directories/path semantics.
- Linen user-facing persistence.
- Power-loss durability.
- Crash consistency.
- Journaling.
- NVMe flush/FUA correctness.
- Concurrent multi-PD write-lock/read-lock semantics.

---

## 5. Dependency Fixes

| Fix | Commit |
|---|---|
| Legacy IO read probe gated | `cfb8c8f9` |
| Flush audit gated behind explicit NVMe flush flag | `e7459039` |
| Kernel allocator overlap fixed | `1623a65c` |
| Require explicit AP2 DiskFS proof profile | `81b0e56a` |

---

## 6. Do-Not-Regress Markers

The following markers must remain stable across any future refactor:

| Marker | Expected |
|---|---|
| `sexfiles.diskfs100.ap2.read.match bytes=128 ok=1` | present |
| `sexfiles.diskfs100.ap3.done ok=1` | present |
| `sexfiles.diskfs100.ap4.read.match bytes=128 ok=1` | present |
| `sexfiles.diskfs100.ap5.neg.* detected ok=1` | present |
| `sexfiles.diskfs100.ap6.done ok=1 classification=honest_skip` | present |
| `faults_zero PASS` | always |

---

## 7. Recommended Future Tiers

- DiskFS concurrent access / locking
- Crash consistency (write-ahead log or CoW tree)
- True flush/FUA after SexDrive flush graduates from SKIP
- Linen/SexFiles user-facing persistence (user-visible save/load)
- Object allocator / directory tree
- DiskFS object deletion and space reclamation
