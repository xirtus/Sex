# Linen DiskFS Persistence — 100% Current Tier Closeout V1

**Date:** 2026-05-25
**Tag:** `linen-diskfs-persistence-100-current-tier-v1`
**Branch:** `master`
**HEAD:** `5a6eb4f2d637448096b921b2e12a96de2bcb6106`

---

## 1. Baseline

| Item | Value |
|---|---|
| Branch | `master` |
| HEAD | `5a6eb4f2` — linen: fix DiskFS reboot restore gate false-positive |
| Intended tag | `linen-diskfs-persistence-100-current-tier-v1` |
| SexFiles DiskFS tag | `sexfiles-diskfs-100-current-tier-v1` (frozen) |

### Recent Commits

```
5a6eb4f2 linen: fix DiskFS reboot restore gate false-positive
e7978616 linen: prove DiskFS negative classifications
9b34ee34 gate: require explicit Atlas final closeout proof
aa0d5725 linen: classify DiskFS metadata persistence
4feefdb9 linen: prove DiskFS reboot restore
a1c1ad72 linen: prove fixed-object DiskFS save load
8ffed127 docs: audit Linen DiskFS persistence reality
025f7515 docs: close SexFiles DiskFS current tier
```

---

## 2. Proven Ladder

| Proof Point | Description | Status |
|---|---|---|
| AP1 | Linen DiskFS persistence reality audit | PASS |
| AP2 | Fixed-object DiskFS content save/load | PASS |
| AP3 | DiskFS reboot content restore (write + read) | PASS |
| AP3.1 | Gate false-positive fix (reboot restore gate hygiene) | PASS |
| AP4 | Metadata honest classification (RamFS-only) | PASS (honest-skip) |
| AP5 | Negative classifications (mismatch / metadata false-claim / flush skip) | PASS |

---

## 3. Replay Results

All replays executed on 2026-05-25 with `SEXOS_STORAGE_100_PROOF=1` and
`DAILY_DRIVER_PROBE_SECONDS=180` (default: 30s).

### 3.1 AP2 — Fixed-Object Save/Load

```
linen_diskfs_fixed_object_save_load PASS   content match ok=1 bytes=128 (metadata skipped — RamFS-only)
sexfiles_diskfs_bridge             PASS   bridge op success markers complete
faults_zero                        PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.2 AP3 Write — Reboot Restore (Write Boot)

```
linen_diskfs_reboot_restore  PASS   AP3 write boot: chunks written + readback match + all_done ok=1
sexfiles_diskfs_bridge       PASS   bridge op success markers complete
faults_zero                  PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.3 AP3 Read — Reboot Restore (Read Boot, preserved nvme.img)

```
linen_diskfs_reboot_restore  PASS   AP3 read boot: chunks read + byte match + done ok=1
sexfiles_diskfs_bridge       PASS   bridge op success markers complete
faults_zero                  PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.4 AP4 — Metadata Honest Classification

```
linen_diskfs_metadata_persistence PASS   honest skip: metadata is RamFS/session-only, not DiskFS-backed
faults_zero                        PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.5 AP5 — Mismatch Negative

```
linen_diskfs_negative_classifications PASS   negative detection/guard marker(s) present
faults_zero                            PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.6 AP5 — Metadata False-Claim Guard

```
linen_diskfs_negative_classifications PASS   negative detection/guard marker(s) present
faults_zero                            PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.7 AP5 — Flush Skip / Durability Non-Claim

```
linen_diskfs_negative_classifications PASS   negative detection/guard marker(s) present
faults_zero                            PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

### 3.8 Default Daily (no storage flags)

```
linen_diskfs_fixed_object_save_load    SKIP   AP2 fixed-object save/load proof not triggered
linen_diskfs_reboot_restore            SKIP   AP3 reboot restore proof not triggered
linen_diskfs_metadata_persistence      SKIP   AP4 metadata persistence proof not triggered
linen_diskfs_negative_classifications  SKIP   AP5 negative classifications not triggered
faults_zero                            PASS   0 fault markers
FAIL gates: 0
FINAL: PASS
```

---

## 4. Exact Claim Boundary

### PROVEN

- Linen bounded object content can be **saved** through SexFiles DiskFS.
- Linen bounded object content can be **loaded** through SexFiles DiskFS.
- Linen bounded object content **survives reboot** with preserved QEMU NVMe image (write boot → read boot).
- Linen negative classifications detect:
  - Content **mismatch** after tampered data.
  - **Metadata false claim** (claiming DiskFS persistence when metadata is RamFS-only).
  - **Flush skip** / durability non-claim (Linen does not issue durable flush).
- Linen **metadata persistence** is honestly classified as **not DiskFS-backed** (session-only RamFS).
- Default daily profile does **not fake PASS** — all Linen DiskFS gates are SKIP by default.

### NOT CLAIMED

- DiskFS-backed Linen metadata (object names, indices, schema — RamFS only).
- Quil edit persistence through Linen → DiskFS.
- UI object list restore proof.
- folders / directories / path semantics.
- POSIX semantics (open/close/fd/seek, etc.).
- Concurrent multi-PD locking.
- True NVMe flush/FUA.
- Power-loss durability.
- Crash consistency.
- Journaling.

---

## 5. Dependency Stack

| Dependency | Status |
|---|---|
| SexDrive storage current tier | PASS (frozen) |
| SexFiles DiskFS current tier | PASS (frozen, tagged `sexfiles-diskfs-100-current-tier-v1`) |
| Kernel allocator frame overlap fix | Committed (`1623a65c`) |

---

## 6. Do-Not-Regress Markers

The following kernel/user-space markers must remain present in any future replay:

| Marker | Expected Value |
|---|---|
| `[linen.diskfs100.ap2.content.match]` | bytes=128 ok=1 |
| `[linen.diskfs100.ap3.read.match]` | bytes=128 ok=1 |
| `[linen.diskfs100.ap4.meta.done]` | ok=1 classification=honest_skip |
| `[linen.diskfs100.ap5.neg.*]` | ok=1 (mismatch, metadata_false_claim, flush_skip) |
| `faults_zero` | PASS (0 fault markers) |
| Default SKIP hygiene | All four Linen DiskFS gates SKIP when no storage proof flags set |

---

## 7. Future Tiers

Items deferred for future proof tiers:

| Item | Dependency |
|---|---|
| Quil edit → Linen save → DiskFS pipeline | DiskFS content persistence (this tier) |
| UI object list restore proof | DiskFS content persistence + metadata model |
| Metadata model backed by DiskFS | DiskFS content persistence |
| Arbitrary path/folder semantics | Filesystem model above SexFiles |
| Concurrent edit/locking | Multi-PD lock model |
| Crash consistency / journaling | SexDrive flush/FUA |
| True NVMe flush/FUA durability | SexDrive storage tier advancement |
| Power-loss durability | Hardware NVMe power-loss protection |
