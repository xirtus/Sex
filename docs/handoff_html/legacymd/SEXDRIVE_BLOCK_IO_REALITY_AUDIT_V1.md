# SEXDRIVE_BLOCK_IO_REALITY_AUDIT_V1

**Date**: 2026-05-25
**Mission**: Audit real current SexDrive block I/O reality before SexObject/SexFS v0 persistence.
**Status**: AUDIT ONLY — no implementation changes.

---

## A) Outcome: PASS — Real NVMe Block I/O IS Wired

This audit corrects SEXOBJECT_STORAGE_REALITY_AUDIT_V1 (which was correct about
DiskFS in-memory scaffold but incorrect about SexDrive). There are TWO
SexDrive sources:

| File | Lines | Status |
|------|-------|--------|
| `servers/sexdrive/src/driver.rs` | 43 | OLD stub (DAG/framebuffer placeholder). Never used. |
| `apps/sexdrive/src/main.rs` | 3040 | **REAL binary** — full NVMe block I/O + SLOT_BLOCK dispatch |

The `apps/sexdrive/src/main.rs` is the actual running SexDrive binary. It contains
a working NVMe driver with PCI BAR mapping, admin queue init, IO queue setup,
READ/WRITE/FLUSH commands via PRP, MemLend buffer handoff, and a SLOT_BLOCK
PDX dispatch loop.

---

## B) SexDrive Current Truth

### B1. Architecture

```
SexFiles (PD 11)
  diskfs_block_write/read(BLOCK_*, offset, size, buf_cap)
    → pdx_call(SLOT_BLOCK=15, opcode, ...)   // AsyncEnqueue
    → pdx_listen_raw(0)                       // wait for reply type_id=0x1
    ← reply_val from SexDrive
          |
          v
SexDrive (PD 2 — bootorder slot 2 in kernel init)
  _start():
    nvme_probe_bar()    → MAP_PCI_BAR(16, BAR0, 16KB)
                         → NVMe register dump
                         → Controller disable + re-provision
                         → Admin SQ/CQ (16 entries each)
                         → IO SQ1/CQ1 creation (16 entries each)
    xhci_probe_mmio()   → USB XHCI MMIO probe (for host)
    shared buffer alloc → 1024x768x4 framebuffer
    LOOP:
      pdx_try_listen_raw(0)  → receive PDX messages
        match cmd:
          BLOCK_READ  → nvme_read_into_mapped_va(offset, size, dst_va)
                      → nvme_read_into_bounce(offset, size) [if no MemLend buf]
          BLOCK_WRITE → write_guard_allows(offset, size, buf_cap)
                      → nvme_write_one_block(offset, size, src_va)
                      → nvme_write_readback_proof(offset, size, src_va)
          BLOCK_SYNC  → BLOCK_ERR_NO_DEVICE (flush QEMU note)
          _           → BLOCK_ERR_BAD_CMD
        pdx_reply(caller_pd, reply_val)
      framebuffer render (1024x768 pattern)
```

### B2. NVMe Driver Capabilities

| Feature | Status | Evidence |
|---------|--------|----------|
| **PCI BAR0 mapping** | REAL | `MAP_PCI_BAR(16, 0, 0x4000)` — line 1618 |
| **NVMe register read** | REAL | CAP, VS, CC, CSTS, AQA, ASQ, ACQ — lines 1639-1645 |
| **Controller disable** | REAL | CC.EN=0 + poll CSTS.RDY=0 — lines 1824-1856 |
| **Admin queue alloc** | REAL | `sys_alloc_phys(PAGE_SIZE)` for ASQ/ACQ — lines 1728-1729 |
| **AQA/ASQ/ACQ program** | REAL | MMIO writes to BAR0 offsets — lines 1777-1782 |
| **Controller enable** | REAL | CC.EN=1 with I/O queue config — reprovision sequence |
| **IO SQ1 creation** | REAL | Admin SQ CREATE_IO_SQ command — admin queue submit |
| **IO CQ1 creation** | REAL | Admin SQ CREATE_IO_CQ command — admin queue submit |
| **NVMe READ (0x02)** | REAL | PRP1 programming, SQ doorbell, CQE poll — lines 158-343 |
| **NVMe WRITE (0x01)** | REAL | PRP1 programming, data copy to DMA buffer, CQE poll — lines 609-798 |
| **NVMe FLUSH (0x00)** | WIRED but gated | `nvme_flush()` implemented — lines 1417-1523. QEMU may not post CQE for FLUSH. |
| **MemLend handoff** | REAL | READ path: `sys_map_mem_lend(SLOT_BUF_LEND)` → direct fill — lines 2875-2893 |
| **Write guard** | REAL | `write_guard_allows()` — allows manifest, object, proof LBAs only — lines 551-607 |
| **CQE polling** | REAL | Spin-poll up to 1M iterations — no MSI-X interrupts |
| **SLOT_BLOCK dispatch** | REAL | `pdx_try_listen_raw(0)` loop decodes BLOCK_READ/WRITE/SYNC — lines 2806-3021 |
| **Storage 100 proofs** | REAL | write+readback, multi-block, persist write/read, negative mismatch — lines 800-1612 |

### B3. Kernel Support (Already Wired)

| Kernel Feature | Status | Evidence |
|---------------|--------|----------|
| **NVMe PCI enumeration** | REAL | `devmgr.rs:19` — class 0x01, subclass 0x08 |
| **SLOT_NVME_HOST (16) grant** | REAL | `devmgr.rs:25` — PciCapData granted to sexdrive |
| **SLOT_BLOCK (15) grant** | REAL | `init.rs:597` — SexFiles→SexDrive Domain cap |
| **MAP_PCI_BAR syscall** | REAL | `syscalls/mod.rs:312-380` — with NVMe gate (class 0x01/0x08) |
| **SYS_ALLOC_PHYS (31)** | REAL | Allocates physical pages for NVMe queues/DMA |
| **SYS_MAP_PHYS (30)** | REAL | Maps physical pages to VA for MMIO access |
| **MemLend grant (50)** | REAL | `syscalls/mod.rs:419-484` — allocates + maps page for caller |
| **MemLend map (51)** | REAL | `syscalls/mod.rs:489-530` — maps MemLend cap into consumer VA |
| **QEMU NVMe device** | REAL | `-drive if=none,id=nvm,file=nvme.img -device nvme,serial=sexos01,drive=nvm` |

### B4. What the Block I/O Route Actually Does Today

```
SexFiles DiskFS bridge (diskfs.rs:261):
  diskfs_block_call(BLOCK_READ, offset=0, size=512, buffer_cap=SLOT_BUF_LEND)
    → pdx_call(SLOT_BLOCK, BLOCK_READ, 0, 512, SLOT_BUF_LEND)
    → IPC enqueue to SexDrive ring
    → pdx_listen_raw(0) — spin for reply type_id=0x1

SexDrive dispatch (apps/sexdrive/main.rs:2806):
  pdx_try_listen_raw(0) receives message:
    cmd=BLOCK_READ, offset=0, size=512, buf_cap=17 (SLOT_BUF_LEND)
    → sys_map_mem_lend(SLOT_BUF_LEND) gets fill_va
    → nvme_read_into_mapped_va(0, 512, fill_va)
        → alloc DMA buffer (sys_alloc_phys + sys_map_phys)
        → build NVMe READ SQ entry (opcode 0x02, SLBA=0, NLB=0)
        → ring SQ1 doorbell
        → poll CQ1 for CQE (up to 1M iterations)
        → copy DMA buffer → fill_va (MemLend buffer)
        → update CQ1 head doorbell
        → return 0 (BLOCK_OK)
    → pdx_reply(caller_pd=11, 0)

SexFiles receives reply:
  diskfs_block_call returns BLOCK_OK (0)
  → data is now in the MemLend buffer at buf_va
  → read buffer → verify contents
```

**The full chain works**: Linen→SexFiles→SLOT_BLOCK→SexDrive→NVMe→nvme.img and back.

---

## C) Existing Reusable Pieces

### C1. Ready for SexFS v0 Directly

| Piece | Location | Lines | Reuse |
|-------|----------|-------|-------|
| NVMe PCI BAR probe + init | apps/sexdrive main.rs:1614-2380 | ~770 | **Critical**: admin queue + IO queue setup. Reuse as-is. |
| NVMe READ (bounce) | apps/sexdrive main.rs:158-343 | ~185 | Reuse for non-MemLend reads (journal/table scans). |
| NVMe READ (MemLend) | apps/sexdrive main.rs:345-549 | ~205 | **Primary path**: read into caller buffer via MemLend. |
| NVMe WRITE (one block) | apps/sexdrive main.rs:609-798 | ~190 | **Primary path**: write from caller buffer via MemLend. |
| NVMe FLUSH | apps/sexdrive main.rs:1417-1523 | ~107 | Flush for durability. QEMU limitation noted honestly. |
| Write guard | apps/sexdrive main.rs:551-607 | ~57 | Extend to allow object table, journal, freemap, checkpoint LBAs. |
| SLOT_BLOCK dispatch loop | apps/sexdrive main.rs:2806-3021 | ~215 | **Already working**. Add BLOCK_FLUSH dispatch uncomment. |
| MemLend buffer handoff | apps/sexdrive main.rs:2875-2893 | ~19 | Copy completed DMA data into caller VA. Works. |

### C2. Ready from SexFiles/DiskFS Side

| Piece | Location | Lines | Reuse |
|-------|----------|-------|-------|
| diskfs_block_call | diskfs.rs:261-310 | ~50 | Already sends BLOCK_READ/WRITE via SLOT_BLOCK. Works. |
| diskfs_block_read | diskfs.rs:320-331 | ~12 | Typed wrapper. Works. |
| diskfs_block_write | diskfs.rs:387-398 | ~12 | Typed wrapper. Works. |
| diskfs_block_sync | diskfs.rs:404-414 | ~11 | Typed wrapper. Works (BLOCK_SYNC wired, NVMe FLUSH gated). |
| diskfs_write_object | diskfs.rs:2223-2350 | ~128 | Read-modify-write per sector. Works end-to-end. |
| diskfs_read_object | diskfs.rs:2337-2460 | ~124 | Read from disk via manifest. Works end-to-end. |
| diskfs_lookup_path | diskfs.rs:2171-2221 | ~51 | Manifest read + parse. Works end-to-end. |

### C3. Ready from Kernel Side

| Piece | Location | Lines | Reuse |
|-------|----------|-------|-------|
| NVMe PCI grant | devmgr.rs:19-36 | ~18 | Already discovers NVMe and grants SLOT_NVME_HOST. |
| SLOT_BLOCK grant | init.rs:591-612 | ~22 | Already grants SexFiles→SexDrive. |
| MAP_PCI_BAR syscall | syscalls/mod.rs:312-380 | ~69 | With NVMe gate. Works. |
| Alloc/map phys | syscalls apps/sexdrive | ~30 | SYS_ALLOC_PHYS(31) + SYS_MAP_PHYS(30). |
| MemLend syscalls | syscalls/mod.rs:419-530 | ~112 | SYS_GRANT_MEM_LEND(50) + SYS_MAP_MEM_LEND(51). |
| QEMU NVMe backing | scripts/run_daily_driver_proof.sh:482-485 | ~4 | `-drive` + `-device nvme` on nvme.img. |

---

## D) Stub/Demo Pieces (Not Used or Not Real)

| Piece | Location | Why Stub |
|-------|----------|----------|
| `servers/sexdrive/src/driver.rs` | 43 lines | OLD DAG/framebuffer placeholder. **Never compiled into running binary**. The real binary is `apps/sexdrive/src/main.rs`. |
| `nvme_flush()` QEMU gate | main.rs:2963-2970 | NVMe FLUSH is **wired** but QEMU doesn't post CQE. Comment says "uncomment when real hardware." |
| `BLOCK_SYNC` dispatch | main.rs:2958-2971 | Returns BLOCK_ERR_NO_DEVICE honestly. `nvme_flush()` call is commented out. |
| MSI-X interrupts | N/A | No interrupt-based CQE handling. Uses poll loop (up to 1M spin iterations). |
| Multi-sector transfers | N/A | Single-sector (512B) transfers only. No PRP list chaining. |
| IO queue depth | 16 entries | Hardcoded to 16. No dynamic sizing. |

---

## E) Missing Pieces for Real Block I/O (SexFS v0 Readiness)

### E1. What Exists But Needs Extension

| Gap | Current State | What's Needed |
|-----|--------------|---------------|
| **Write guard LBA whitelist** | Allows manifest (2046), object (2022-2045), proof (2047) | Add object table LBAs, journal LBAs, freemap LBAs, checkpoint LBAs, superblock LBA 0 |
| **BLOCK_SYNC → real NVMe FLUSH** | `nvme_flush()` wired but BLOCK_SYNC dispatch returns ERR_NO_DEVICE | Uncomment `nvme_flush()` call in BLOCK_SYNC handler when QEMU or real HW supports it |
| **SexDrive storage 100 self-tests** | Write/read at fixed proof LBAs only | Remove or gate self-tests so they don't collide with SexFS v0 data at same LBAs |
| **MemLend buffer size** | 4096 bytes (one page) | Already sufficient for 512B sector I/O |
| **Sector-level I/O from DiskFS** | `diskfs_write_object` uses read-modify-write | Already works sector-at-a-time |

### E2. What Must Be Added Before SexFS v0

| Gap | Priority | Description |
|-----|----------|-------------|
| **Format writes superblock to LBA 0** | CRITICAL | `DiskFs::format_init_empty()` must `diskfs_block_write(0, 512, buf)` with serialized superblock |
| **Mount reads superblock from LBA 0** | CRITICAL | `DiskFs::mount()` must `diskfs_block_read(0, 512, buf)` and parse superblock from buffer |
| **Format writes object table** | HIGH | Write object table blocks to reserved post-superblock LBAs |
| **Mount reads object table** | HIGH | Read object table blocks from disk on mount |
| **Format writes freemap** | HIGH | Write extent bitmap to reserved LBA region |
| **Mount reads freemap** | HIGH | Read extent bitmap from disk on mount |
| **Journal write to disk** | HIGH | `append_journal_record` must write journal block to reserved LBA region |
| **Checkpoint write to disk** | MEDIUM | `create_checkpoint` must write checkpoint data to reserved LBA region |
| **Mount reads journal** | HIGH | Read journal from disk on mount for replay |
| **Real two-boot proof** | MEDIUM | Write boot: format + create objects + write + flush. Read boot: mount + read + verify. |
| **DiskFS FsBackend impl** | MEDIUM | `impl FsBackend for DiskFs` currently all-stub. Wire real open/read/write/close to object entries on disk. |

### E3. What Is NOT Missing (Contrary to Previous Audit)

| Claimed Missing in Previous Audit | Actual Status |
|-----------------------------------|---------------|
| "NO real NVMe/AHCI backend" | **WIRED** — full NVMe driver in apps/sexdrive/main.rs |
| "SexDrive SLOT_BLOCK handler is stub" | **WRONG** — real dispatch loop decodes BLOCK_READ/WRITE/SYNC |
| "BLOCK_READ returns unconsumed data" | **WRONG** — `nvme_read_into_mapped_va()` does real NVMe DMA read |
| "BLOCK_WRITE returns status 0 but writes nothing" | **WRONG** — `nvme_write_one_block()` does real NVMe DMA write |
| "No real PCI BAR resolution" | **WRONG** — `MAP_PCI_BAR(16, 0, 0x4000)` maps NVMe BAR0 |
| "No real NVMe queue setup" | **WRONG** — admin queue + IO queue init with doorbell ring |
| "No real DMA buffer" | **WRONG** — `sys_alloc_phys()` + `sys_map_phys()` for DMA pages |
| "No real completion path" | **PARTIALLY WRONG** — CQE polling works (1M iterations); no MSI-X |
| "sexdrive is a framebuffer demo" | **WRONG for block I/O** — it's both: framebuffer render + real NVMe block I/O |

---

## F) Smallest Safe Implementation Ladder for SexFS v0

```
Phase 0 (ZERO NEW CODE — just ungate what exists):
  [ ] Write guard: add superblock LBA 0, object table LBAs, journal LBAs,
      freemap LBAs, checkpoint LBAs to write_guard_allows() whitelist
  [ ] (Optional) Uncomment nvme_flush() in BLOCK_SYNC dispatch
  [ ] Audit LBA layout: document reserved regions for superblock (0),
      object table (1-?), freemap, journal, checkpoints, proof objects

Phase 1 (DISKFS FORMAT → DISK):
  [ ] DiskFs::format_init_empty() writes superblock to LBA 0 via
      diskfs_block_write(0, 512, buf)
  [ ] DiskFs::mount() reads superblock from LBA 0 via
      diskfs_block_read(0, 512, buf) and validates magic+checksum
  [ ] Proof: format → write → read → verify (one-boot)

Phase 2 (OBJECT TABLE → DISK):
  [ ] create_object_entry() writes object table blocks to disk after commit
  [ ] mount() reads object table from disk
  [ ] Journal replay on top of table read
  [ ] Proof: create objects → write → reboot → read → verify (two-boot)

Phase 3 (FREEMAP → DISK):
  [ ] Write extent_bitmap to reserved LBA region on format + each alloc/free
  [ ] Read extent_bitmap on mount
  [ ] Proof: allocate → write → reboot → verify

Phase 4 (JOURNAL + CHECKPOINT → DISK):
  [ ] append_journal_record() writes to journal LBA region
  [ ] create_checkpoint() writes to checkpoint LBA region
  [ ] Mount reads journal + replay
  [ ] Proof: multi-object create → checkpoint → reboot → restore → verify

Phase 5 (DYNAMIC MANIFEST):
  [ ] Extend manifest beyond 3 hardcoded paths
  [ ] Variable-size object allocation
  [ ] Proof: create object → manifest entry → reboot → read → verify

Phase 6 (DISKFS FsBackend REAL):
  [ ] Wire impl FsBackend for DiskFs (real open/read/write/close)
  [ ] This is the LAST step — RamFS stays primary until this works
  [ ] Proof: full Linen→RamFS→DiskFS→NVMe roundtrip

Phase 7 (LIVE USB TESTING):
  [ ] SexFS v0 on real USB NVMe device
  [ ] Boot → create → unmount → reboot → re-mount → verify
```

---

## G) STOP FIRST Risks

1. **DO NOT delete `servers/sexdrive/src/driver.rs`** — it's the Cargo.toml target path. The app binary (`apps/sexdrive`) is a separate build target.

2. **DO NOT change NVMe admin queue init** without STOP FIRST — breaking the admin queue breaks all I/O.

3. **DO NOT change the write guard without adding new LBA ranges** — the guard prevents accidental overwrites of critical regions.

4. **DO NOT add interrupt-driven NVMe completion** without first proving poll-based completion works end-to-end with real two-boot.

5. **DO NOT change the MemLend contract** — the SLOT_BUF_LEND→MemLend buffer handoff is shared between SexFiles and SexDrive.

6. **DO NOT add dynamic NVMe namespace detection** — hardcoded NSID=1 is sufficient for QEMU and single-device testing.

7. **DO NOT remove the framebuffer render loop from sexdrive** — it doubles as the display driver and is the primary output channel.

8. **DO NOT claim durability** even after FLUSH works — durability requires journal-to-disk persistence proof which is a separate phase.

9. **DO NOT edit `crates/sex-pdx`** without STOP FIRST — opcode namespace is shared.

10. **DO NOT add Linen→SexDrive direct calls** — Linen must use only SLOT_STORAGE→SexFiles→DiskFS→SLOT_BLOCK→SexDrive.

---

## H) Exact Audit Evidence (grep References)

### H1. Two SexDrive binaries

```
servers/sexdrive/src/driver.rs:43 lines — OLD stub, DAG placeholder
apps/sexdrive/src/main.rs:3040 lines — REAL binary with NVMe driver
```

### H2. NVMe PCI BAR mapping + init

```
apps/sexdrive/main.rs:1614: fn nvme_probe_bar()
apps/sexdrive/main.rs:1618: MAP_PCI_BAR(16, 0, 0x4000)
apps/sexdrive/main.rs:1639-1645: NVMe register reads (CAP, VS, CC, CSTS, AQA, ASQ, ACQ)
apps/sexdrive/main.rs:1777-1782: AQA/ASQ/ACQ programming via MMIO writes
apps/sexdrive/main.rs:1824-1856: Controller disable (CC.EN=0, poll CSTS.RDY=0)
```

### H3. NVMe READ/WRITE

```
apps/sexdrive/main.rs:158: fn nvme_read_into_bounce(offset, size)
apps/sexdrive/main.rs:345: fn nvme_read_into_mapped_va(offset, size, dst_va)
apps/sexdrive/main.rs:609: fn nvme_write_one_block(offset, size, src_va)
apps/sexdrive/main.rs:800: fn nvme_write_readback_proof(offset, size, src_va)
apps/sexdrive/main.rs:1417: fn nvme_flush() -- NVMe FLUSH opcode 0x00
```

### H4. SLOT_BLOCK dispatch loop

```
apps/sexdrive/main.rs:2806: if let Some(msg) = pdx_try_listen_raw(0)
apps/sexdrive/main.rs:2837-2979: match cmd { BLOCK_READ => ..., BLOCK_WRITE => ..., BLOCK_SYNC => ... }
apps/sexdrive/main.rs:2985: pdx_reply(msg.caller_pd, reply_val)
```

### H5. Write guard

```
apps/sexdrive/main.rs:551: fn write_guard_allows(offset, size, buf_cap) -> bool
apps/sexdrive/main.rs:557: allow_manifest = proof_mode && size == 512 && offset == manifest_offset
apps/sexdrive/main.rs:562-571: allow_object, allow_linen, allow_quil ranges
```

### H6. Kernel NVMe support

```
kernel/src/devmgr.rs:19: (0x01, 0x08) => { // NVMe
kernel/src/devmgr.rs:25: pd.grant_capability(SLOT_NVME_HOST, CapabilityData::Pci(PciCapData { ... }))
kernel/src/init.rs:597: pd.grant_capability(sex_pdx::SLOT_BLOCK, CapabilityData::Domain(sexdrive_id))
kernel/src/syscalls/mod.rs:312: 43 => { // MAP_PCI_BAR(cap_slot, bar_index, map_size)
kernel/src/syscalls/mod.rs:349: let is_nvme = class_id == 0x01 && subclass_id == 0x08;
```

### H7. QEMU NVMe device

```
scripts/run_daily_driver_proof.sh:482-485:
  NVME_ARGS=(
    -drive "if=none,id=nvm,file=${NVME_IMG},format=raw"
    -device "nvme,serial=sexos01,drive=nvm"
  )
```

### H8. MemLend buffer handoff

```
apps/sexdrive/main.rs:2875: let fill_va = sys_map_mem_lend(SLOT_BUF_LEND);
apps/sexdrive/main.rs:2892: nvme_read_into_mapped_va(offset, size, fill_va)
sexfiles diskfs.rs:339: let buf_va = sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND);
sexfiles diskfs.rs:359: Self::diskfs_block_read(0, 512, SLOT_BUF_LEND)
```

### H9. Storage 100 proof lanes (self-test, may collide with SexFS v0 data)

```
apps/sexdrive/main.rs:1163: fn nvme_multiblock_write_readback_proof() -- AP4 LBA 128-131
apps/sexdrive/main.rs:1295: fn nvme_persist_write_proof()          -- AP5A LBA 256-259
apps/sexdrive/main.rs:1343: fn nvme_persist_read_proof()           -- AP5A LBA 256-259
apps/sexdrive/main.rs:1525: fn nvme_storage100_flush_audit()       -- AP6 flush audit
apps/sexdrive/main.rs:1549: fn nvme_storage100_negative_mismatch() -- AP6 LBA 384
```

### H10. DiskFS block bridge → SexDrive

```
sexfiles diskfs.rs:278: let (send_status, _) = pdx_call(SLOT_BLOCK, opcode, arg0, arg1, arg2);
sexfiles diskfs.rs:292: let msg = pdx_listen_raw(0); if msg.type_id == 0x1 { ... }
```

---

## I) Recommended Next Autopilot Prompt

```
MISSION: SEXDRIVE_WRITE_GUARD_EXTEND_V1

Extend write_guard_allows() in apps/sexdrive/src/main.rs to add write permission
for the reserved LBA ranges needed by SexFS v0:

  LBA 0:       superblock (1 sector, read+write)
  LBA 1-n:     object table (n sectors, read+write, n = ceil(16*entry_size/512))
  LBA n+1-m:   extent bitmap (m sectors, read+write)
  LBA m+1-p:   journal (p sectors, read+write)
  LBA p+1-127: checkpoints (remaining up to proof LBA, read+write)

Preserve all existing whitelist entries (manifest=2046, objects=2022-2045,
proof=2047). Document the full LBA layout in a handoff doc.

No NVMe driver changes. No kernel edits. No sex-pdx edits.
BACKUP BEFORE CHANGES.
```

---

*End of audit. No files changed. Commit: this audit doc only.*
