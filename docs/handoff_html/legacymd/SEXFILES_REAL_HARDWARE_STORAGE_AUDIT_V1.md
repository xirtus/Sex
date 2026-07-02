# SEXFILES_REAL_HARDWARE_STORAGE_AUDIT_V1

**Date:** 2026-05-06
**Scope:** Real-hardware storage persistence readiness audit for SexFiles DiskFS
**Method:** Static code analysis of `servers/sexfiles/src/backends/diskfs.rs`, `servers/sexstore/src/main.rs`, kernel infrastructure, boot chain, PDX ABI
**Status:** ALL PERSISTENCE BLOCKED — in-memory scaffold only

---

## PASS/WARN/FAIL Matrix

| # | Audit Category | Status | Detail |
|---|---------------|--------|--------|
| 1 | Boot device assumptions | FAIL | No persistent boot device. ISO-only boot. No rootfs mount. |
| 2 | Block-device discovery | FAIL | Zero block device enumeration. No NVMe/AHCI probe code. |
| 3 | Sector size assumptions | WARN | 512-byte assumed (hardcoded). No 4Kn detection. OK for first target. |
| 4 | Cache/flush/sync semantics | FAIL | No cache flush, no DMA fence, no write barrier. RwLock with no sync. |
| 5 | Write ordering | FAIL | No ordering guarantees. No barrier between superblock and journal writes. |
| 6 | Journal durability truth | FAIL | In-memory scaffold ONLY. No records ever written to persistent media. |
| 7 | Media corruption behavior | PASS | Checksums at entry, record, and page level. Corrupt detection works in proof. |
| 8 | QEMU vs real hardware gap | FAIL | QEMU: RAM only, `-cdrom` read-only. HW: no driver, no bus, no media. |
| 9 | Destructive-test risk | PASS | NONE. All writes go to RAM. No code path can touch physical storage. |
| 10 | Manual hardware test protocol | BLOCKED | Cannot test until items 1–6 above are resolved. |

**Score: 2/10 PASS or WARN, 8/10 FAIL or BLOCKED**

---

## 1. Boot Device Assumptions — FAIL

### Current State
- SexOS boots from Limine bootloader via ISO image (`-cdrom sexos-v1.0.0.iso` in QEMU)
- All servers and apps are loaded as Limine modules from `boot:///` (in-memory boot protocol)
- **No root filesystem is mounted.** No initrd-based filesystem. No disk partition table parsed.
- `limine.cfg` lists modules, not mount points:
  ```
  MODULE_PATH=boot:///servers/sexfiles
  MODULE_PATH=boot:///servers/sexstore
  ```
- Kernel does not attempt to mount any filesystem at boot
- Kernel `init::init()` spawns PDs from in-memory module list — no disk I/O

### Gap
- On real hardware, ISO boots from USB/CD (read-only, no writable region)
- There is no mechanism to discover, mount, or write to any storage device
- Even if a block driver existed, no boot-time code would call it

### Required
- `limine.cfg` or kernel init must identify and pass a block device
- Kernel must provide or proxy a writable storage region
- SexFiles must be able to call a block server at boot time to `mount()` from persistent media

---

## 2. Block-Device Discovery — FAIL

### Current State
- **Zero block device enumeration code anywhere in the system**
- `apps/sexdrive` is an XHCI MMIO probe + framebuffer pattern writer — NOT a storage driver
- `kernel/src/drivers/pci.rs` scans buses 0..8 but only for display controllers (class 0x03) in `bootstrap_drivers()`
- No PCI class code scanning for:
  - NVMe (PCI class 0x01, subclass 0x08, prog-if 0x02 = `0x010802`)
  - AHCI (PCI class 0x01, subclass 0x06 = `0x0106`)
  - VirtIO block (PCI vendor 0x1AF4, device 0x1001)
- Kernel has no driver subsystem for storage controllers
- No AHCI port enumeration, no NVMe namespace discovery
- No FIS (Frame Information Structure) handling for AHCI
- No NVMe submission/completion queue setup

### Gap
- No code can find a storage controller
- No code can identify attached storage media
- No code can determine device geometry (sector count, sector size)

### Required
- Kernel or userspace driver must enumerate PCI storage controllers
- NVMe driver: set up admin SQ/CQ, identify controller, create namespaces
- AHCI driver: enumerate ports, identify devices via IDENTIFY DEVICE
- Block server must expose bounded `read_sector`/`write_sector` operations

---

## 3. Sector Size Assumptions — WARN

### Current State
- DiskFS hardcodes 512-byte sector alignment:
  ```rust
  const SECTOR_SIZE: u64 = 512;
  // in proof_validate_block_write:
  if block_offset % SECTOR_SIZE != 0 { return Err(...); }
  ```
- `DISKFS_BLOCK_SIZE = 4096` (8 sectors per block)
- This is ONLY used in proof validation functions — never in actual I/O
- No real device sector size is queried

### Risk
- Most NVMe/AHCI devices use 512-byte logical sectors — OK for first target
- **4Kn (4096-byte native) drives** would break this assumption silently
- NAND flash may have erase-block sizes much larger (128K–4M)
- No detection code exists to handle non-512 sector sizes

### Verdict
- WARN, not FAIL, because 512-byte sectors cover the vast majority of first-target hardware
- Must add sector-size detection when driver is written

---

## 4. Cache/Flush/Sync Semantics — FAIL

### Current State
- **Zero cache-management code for storage**
- DiskFS state lives in `static DISKFS_STATE: RwLock<DiskFsState>` — CPU-cached RAM
- All "writes" are plain Rust assignments to struct fields:
  ```rust
  st.superblock = sb;
  st.table[idx] = e;
  st.journal[st.journal_len] = rec;
  ```
- No `core::sync::atomic::fence()` calls
- No CLFLUSH, CLFLUSHOPT, CLWB instructions
- No SFENCE, MFENCE, LFENCE for storage ordering
- No non-temporal stores (MOVNTI, MOVNTDQ)
- No WC (Write-Combining) memory type for storage buffers
- No volatile annotations on storage-mapped structures

### Gap on Real Hardware
- Store buffers: writes may sit in CPU store buffer before reaching device
- Write-combining buffers: PCIe MMIO writes may be reordered/combined
- Device memory: NVMe doorbell writes must be visible before controller acts
- DMA coherency: CPU must flush caches before DMA reads, invalidate after DMA writes

### Required
- `write_volatile` + `compiler_fence` for MMIO register access (doorbell rings)
- SFENCE before NVMe doorbell write to ensure prior writes are visible
- If using DMA from/to RAM buffers:
  - CLFLUSH or CLWB before initiating DMA read (CPU → device)
  - Cache invalidate before reading DMA'd data (device → CPU)
- Write ordering barriers for journal consistency:
  - Metadata record must reach media before commit record
  - Superblock update must be last in an atomic write group

---

## 5. Write Ordering — FAIL

### Current State
- Journal append order is implicit in the in-memory array — ordering "works" because RAM is sequentially consistent for single-threaded writes
- On real hardware, writes to PCIe MMIO (NVMe SQ doorbell) are NOT sequentially consistent with prior memory writes
- No barrier exists between:
  - Object table write → journal write
  - Journal metadata record → journal commit record
  - Journal commit → superblock generation advancement

### DiskFS Write Sequence (current, RAM-only)
```
1. append_journal_record(TxBegin)
2. append_journal_record(ObjectMetadataUpdate)
3. append_journal_record(TxCommit)
4. st.table[idx] = entry
5. st.superblock.fs_generation += 1
```

### On Real Media, This Order MUST Be:
```
1. Write object metadata to journal region          ← barrier
2. Write commit record to journal region            ← barrier
3. Update object table in its block region          ← barrier
4. Update superblock fs_generation                  ← barrier
```

### Required
- Atomic write groups with explicit ordering barriers
- Journal commit must be durable BEFORE object table is updated
- Crash at any point must leave either consistent pre-transaction OR post-transaction state
- Dual-page atomic swap (SexStore pattern) requires ordering: write new page → barrier → update active pointer

---

## 6. Journal Durability Truth — FAIL

### Current State
The DiskFS journal is a **pure in-memory mock scaffold**. This is expressly documented:

```rust
/// BLOCKER: No real block I/O path is wired yet in sexfiles->sexdrive.
/// The system lacks: a block device server (sexdrive is a framebuffer demo),
/// block device PDX opcodes/slots, block device kernel syscalls,
/// and any NVMe/AHCI driver infrastructure.
```

### What Exists (In-Memory)
- `DISKFS_JOURNAL_CAPACITY = 64` fixed-capacity circular journal
- Record types: TxBegin, ObjectMetadataUpdate, TxCommit
- Per-record CRC-32C checksum validation
- Committed/uncommitted transaction filtering during replay
- Replay applies committed transactions in generation order
- Corruption detection via checksum mismatch → `ERR_OVERFLOW`

### What Does NOT Exist (Real Media)
- Journal records are NEVER written to any persistent device
- On system crash or power loss, ALL journal data is lost
- Journal replay logic is PROVEN correct for the algorithm but UNTESTED against real sector writes
- No mechanism to guarantee journal records reached durable media before acknowledging a transaction
- No tear detection for partially written journal records

### Truth Statement
> The journal algorithm is correct. The journal implementation works on in-memory data. The journal provides ZERO durability against power loss or reboot. This is not a bug — it is an unimplemented feature blocked on the storage infrastructure gap.

---

## 7. Media Corruption Behavior — PASS

### Current State
Checksum-based corruption detection exists at multiple layers:

| Layer | Algorithm | Scope | File |
|-------|-----------|-------|------|
| Superblock checksum | XOR-based | 40-byte superblock | `diskfs.rs:checksum_superblock()` |
| Object entry checksum | XOR-based | 32-byte entry | `diskfs.rs:checksum_entry()` |
| Journal record checksum | XOR-based | 24-byte record | `diskfs.rs:checksum_journal_record()` |
| Checkpoint checksum | XOR + per-entry mixing | Full object table | `diskfs.rs:checksum_checkpoint()` |
| Durable page CRC | CRC-32C (Castagnoli) | 512-byte page | `sexstore/main.rs:crc32c()` |
| Durable record CRC | CRC-16-IBM | 24-byte record | `sexstore/main.rs:crc16_ibm()` |

### Validation Behavior (in proofs)
- Corrupted journal record: `ERR_OVERFLOW` on append, replay rejects
- Corrupted entry checksum: `ERR_OVERFLOW` on stat
- Corrupted checkpoint: skipped during `find_latest_valid_checkpoint()`
- Corrupted durable page: sequence number returns 0 (invalid)
- Corrupted durable record: skipped during load, counted as corrupt

### Gap
- No actual media-level corruption handling (bad block remapping, sector retry)
- No ECC or error correction
- XOR-based checksums are fast but weak (single-bit-flip detection only)
- No read-retry on transient media errors
- CRC-32C at page level is solid but 512-byte page granularity means one corrupt byte invalidates entire page

### Verdict
- PASS for detection capabilities — the system correctly identifies corruption
- Future improvement: CRC-32C for all DiskFS structures (replace XOR checksums)

---

## 8. QEMU vs Real Hardware Gap — FAIL

### Current QEMU Environment
- Boots from `-cdrom sexos-v1.0.0.iso` (read-only El Torito ISO)
- **No writable storage attached** — no `-drive`, no `-hda`, no `-device nvme`
- All filesystem state lives in kernel heap (allocated at boot)
- DiskFS runs in server heap (allocated via global allocator)
- SexStore durable region is a `static mut [u8; 1024]` in BSS

### Real Hardware Gap

| Component | QEMU Status | Real HW Requirement |
|-----------|------------|---------------------|
| NVMe driver | NOT PRESENT | Required for NVMe SSD |
| AHCI driver | NOT PRESENT | Required for SATA HDD/SSD |
| USB mass storage | NOT PRESENT | Required for USB flash boot persistence |
| PCI storage enumeration | NOT PRESENT | Required to find storage controllers |
| Sector I/O path | NOT PRESENT | Required to read/write physical media |
| DMA for block transfers | NOT PRESENT | Required for efficient I/O |
| Persistent QEMU media | NOT CONNECTED | Needs `-drive file=disk.qcow2,if=none,id=disk0 -device nvme,drive=disk0` |

### What Would It Take to Test Persistence in QEMU?

```bash
# 1. Create writable disk image
qemu-img create -f qcow2 sexfiles_disk.qcow2 64M

# 2. Attach writable media to QEMU (must be NVMe or AHCI)
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso \
  -drive file=sexfiles_disk.qcow2,if=none,id=disk0 \
  -device nvme,drive=disk0,serial=SEXFILES01 \
  -serial stdio

# 3. SexFiles must: discover NVMe, format, mount, create objects, verify
# 4. Then: QEMU exit, restart with same disk image, verify data persists

# But: NONE of the above works because no NVMe driver exists.
```

---

## 9. Destructive-Test Risk — PASS (SAFE)

### Verdict: ZERO RISK

**No code path can write to physical storage media.**

| Assertion | Evidence |
|-----------|----------|
| No storage driver exists | `apps/sexdrive` = framebuffer only; no NVMe/AHCI/USB-storage code |
| DiskFs uses in-memory RwLock | `static DISKFS_STATE: RwLock<DiskFsState>` — RAM only |
| FsBackend returns ERR_NOT_FOUND | All DiskFs trait methods return `Err(messages::ERR_NOT_FOUND)` |
| No PDX block slot exists | No `SLOT_BLOCK` in `crates/sex-pdx/src/lib.rs` |
| No kernel block syscalls | Zero NVMe/AHCI/SATA references in `kernel/src/` |
| Limine does not mount writable FS | Boot protocol only — no filesystem driver in bootloader |
| ISO is read-only (El Torito) | `-cdrom` in QEMU; USB/CD boot on hardware is also read-only |

**Even intentionally destructive code could NOT write to disk — there is simply no code path from any userspace app to any storage controller.**

---

## 10. Required Manual Hardware Test Protocol — BLOCKED

### Stage A: Driver Implementation (prerequisites — STOP FIRST required)

All of these are blockers requiring infrastructure changes:

| # | Prerequisite | Scope | STOP FIRST? |
|---|-------------|-------|-------------|
| A1 | NVMe or AHCI driver (userspace) | `apps/sexblk` (new) | No — new app |
| A2 | Block device PDX slot + opcodes | `crates/sex-pdx/src/lib.rs` | **YES** — ABI change |
| A3 | DiskFS PDX wiring to block server | `servers/sexfiles/src/backends/diskfs.rs` | No — this mission's scope |
| A4 | Kernel DMA buffer syscall (if needed) | `kernel/src/syscall.rs` | **YES** — kernel edit |
| A5 | QEMU persistent media config | `scripts/master_runtime_gate.sh` | No |

### Stage B: QEMU Persistence Test Protocol

Once A1-A5 exist, the test protocol would be:

```bash
# B1. Create persistent disk image
qemu-img create -f qcow2 /tmp/sexfiles_test.qcow2 64M

# B2. Boot Phase A: Format + Write
GATE_DIR=/tmp/persist_gate_A LOG_PATH=/tmp/persist_gate_A/serial.log \
  SEXFILES_PERSISTENCE_PROOF=write \
  bash scripts/qemu_harness.sh --disk /tmp/sexfiles_test.qcow2

# B3. Verify Phase A markers:
#   [sexfiles.persist.format] ok=1
#   [sexfiles.persist.create] objects_created=N
#   [sexfiles.persist.write.done] fs_generation=M

# B4. Boot Phase B: Mount + Read (separate QEMU invocation)
GATE_DIR=/tmp/persist_gate_B LOG_PATH=/tmp/persist_gate_B/serial.log \
  SEXFILES_PERSISTENCE_PROOF=verify \
  bash scripts/qemu_harness.sh --disk /tmp/sexfiles_test.qcow2

# B5. Verify Phase B markers:
#   [sexfiles.persist.mount] ok=1
#   [sexfiles.persist.read] objects_restored=N
#   [sexfiles.persist.verify.match] ok=1

# B6. Validate: Phase A objects === Phase B objects
```

**All of Stage B is BLOCKED until Stage A is complete.**

### Stage C: Real Hardware Test Protocol (future)

```
DO NOT RUN — ALL STEPS BLOCKED
===============================
1. Build ISO with block driver included
2. Install limine to ISO (if not done): ./limine/limine bios-install sexos-v1.0.0.iso
3. Write ISO to USB (IF DESTRUCTIVE: confirm target device first):
   DO NOT RUN: sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress
4. Boot from USB on test machine with NVMe SSD
5. SexFiles formats NVMe namespace 1
6. Create test objects
7. Power cycle machine
8. Boot again
9. Verify objects survived reboot
```

**Destructive warning:** Step 3 (`dd` to `/dev/sdX`) will IRREVOCABLY DESTROY all data on the target device. Do not run without explicit confirmation.

---

## Exact Blockers Before Claiming Real Hardware Persistence

| # | Blocker | Category | Effort Estimate |
|---|---------|----------|-----------------|
| 1 | **No block device server** | New userspace driver | Large (NVMe: ~2000 lines no_std Rust) |
| 2 | **No block device PDX ABI** | sex-pdx ABI change (STOP FIRST) | Small (2 new opcodes, 1 slot) |
| 3 | **No DiskFS → block server wiring** | DiskFS code change | Medium (refactor RwLock → PDX calls) |
| 4 | **No cache/flush/sync** | Cross-layer | Medium (SFENCE, volatile, write ordering) |
| 5 | **No boot-time disk mount** | Kernel init change (STOP FIRST) | Small (call DiskFS mount on block device) |
| 6 | **No crash consistency** | Cross-layer verification | Large (torn-write tests, power-fail injection) |
| 7 | **No two-boot proof** | Testing infrastructure | Medium (QEMU persistent disk + two-phase harness) |

**All 7 blockers must be resolved before "real hardware persistence" can be claimed.**

---

## Safe Test Commands

These commands are SAFE to run now — they audit, never write:

```bash
# 1. Run the storage preflight (log-only audit)
./scripts/sexfiles_storage_preflight.sh

# 2. Build and run QEMU with block contract validation
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log

# 3. Run reboot persistence proof (single-boot, in-memory only)
SEXOS_SEXFILES_REBOOT_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log

# 4. Run all SexFiles storage proofs at once
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 \
SEXOS_SEXFILES_REBOOT_PROOF=1 \
SEXOS_SEXFILES_JOURNAL_PROOF=1 \
SEXOS_SEXFILES_REPLAY_PROOF=1 \
SEXOS_SEXFILES_EXTENT_PROOF=1 \
SEXOS_SEXFILES_CHECKPOINT_PROOF=1 \
SEXOS_SEXFILES_FAULT_INJECTION_PROOF=1 \
./scripts/master_runtime_gate.sh --probe 25 --keep-log

# 5. Run the real hardware preflight (general boot readiness)
./scripts/real_hardware_preflight.sh

# 6. Verify current build passes
./scripts/entrypoint_build.sh
```

## Unsafe / Destructive Tests — DO NOT RUN

```
DO NOT RUN — Blocked until block device driver exists
======================================================

# DO NOT RUN: dd write to physical device
#   sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress
# Reason: Writing ISO to /dev/sdX destroys all data on that device.
# Only proceed AFTER confirming target device and AFTER backing up data.

# DO NOT RUN: NVMe namespace format
#   (no code exists to do this yet, but when it does:)
#   Format NVM command with secure erase will destroy all namespace data.
#   Only run if the target namespace has been confirmed as test-only.

# DO NOT RUN: AHCI secure erase
#   Same as above — destructive to all data on target drive.
```

---

## Safe Next Patch

### Priority Order (smallest → largest)

**Patch 1: Block Device PDX Slot (SMALL, STOP FIRST)**
- File: `crates/sex-pdx/src/lib.rs`
- Add: `pub const SLOT_BLOCK: u32 = 14;`
- Add: `pub const OP_BLOCK_READ_SECTOR: u64 = 0x40;`
- Add: `pub const OP_BLOCK_WRITE_SECTOR: u64 = 0x41;`
- Update ABI version hash in `sexos_build_spec.toml`
- Impact: Safe if no existing slot 14 in use. Verify slot assignment first.

**Patch 2: QEMU NVMe Emulation for Testing (SMALL)**
- File: `scripts/qemu_harness.sh`
- Add: `-drive file=sexfiles_test.qcow2,if=none,id=disk0 -device nvme,drive=disk0,serial=SEXFILES01`
- Impact: Safe — purely additive QEMU config. No code change.

**Patch 3: NVMe Block Server (LARGE, SAFE — new app)**
- File: `apps/sexblk/src/main.rs` (NEW)
- Scope: PCI probe for NVMe class code, admin queue setup, identify, namespace read/write
- Reference: NVMe 1.4 spec, `apps/sexdrive` for PCI BAR mapping pattern
- Impact: New app — no existing code changed. Safe to implement in parallel.

**Patch 4: DiskFS Block Wiring (MEDIUM)**
- File: `servers/sexfiles/src/backends/diskfs.rs`
- Add: `fn block_read(...)` and `fn block_write(...)` using PDX calls to SLOT_BLOCK
- Wire: `format_init_empty()` writes superblock to LBA 0 via block_write
- Wire: `mount()` reads superblock from LBA 0 via block_read
- Wire: Object table and journal operations read/write their block ranges
- Impact: Core DiskFS change — test throughly in QEMU before hardware.

---

## Files Created / Changed

| File | Type | Purpose |
|------|------|---------|
| `docs/handoff/SEXFILES_REAL_HARDWARE_STORAGE_AUDIT_V1.md` | NEW | This document |
| `scripts/sexfiles_storage_preflight.sh` | NEW | Safe log-only storage hardware audit |

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `kernel/src/` (any file) | STOP FIRST — kernel edits needed for NVMe/AHCI DMA/cache |
| `crates/sex-pdx/src/lib.rs` | STOP FIRST — ABI change needed for block device slot |
| `apps/sexdrive/src/main.rs` | STOP FIRST — would need broad rewrite from framebuffer to block |
| `servers/sexfiles/src/backends/diskfs.rs` | No code changes in this audit (documentation only) |
| `servers/sexstore/src/main.rs` | No code changes in this audit |

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions**: block I/O uses raw sector numbers, not file descriptors
- **No std/libc/threads**: pure no_std Rust, PDX-only message passing
- **MPK/PKU/PKEY isolation preserved**: block server in own PD, DiskFS in own PD, sex-pdx mediated
- **No shared-memory redesign**: block data through PDX registers or bounded static buffers
- **No kernel edits in this scope**: documented as STOP FIRST prerequisites
- **No sex-pdx ABI edits in this scope**: documented as STOP FIRST prerequisites
- **No broad refactor**: audit/documentation only

## Gate Run Command

```bash
# Verify all current proofs pass (in-memory scaffold)
./scripts/entrypoint_build.sh && \
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

## References

- `docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md` — Exact missing contract for block device route
- `docs/handoff/SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1.md` — Single-boot proof, two-boot blocked
- `docs/handoff/SEXFILES_REPLAY_RECOVERY_PROOF_V1.md` — Journal replay proof (in-memory)
- `docs/handoff/SEXFILES_APPEND_ONLY_JOURNAL_IMPL_V1.md` — Journal implementation details
- `docs/handoff/REAL_HARDWARE_BOOT_AUDIT_V1.md` — General real-hardware boot audit
- `docs/handoff/HARDWARE_MATURITY_BOOT_DEVICE_AUDIT_V1.md` — Boot device maturity audit
- `servers/sexfiles/src/backends/diskfs.rs` — DiskFS backend (lines 1–1592)
- `servers/sexstore/src/main.rs` — SexStore durable backend (lines 1–1037)
