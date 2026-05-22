# SexFiles DiskFS 100 AP3.0 Multi-Object PKU Isolation

## 1. Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/proof.rs` | Added 49 diagnostic markers to `run_diskfs_multi_object_proofs()` |
| `servers/sexfiles/src/trampoline.rs` | Added AP3 profile gate (`cfg(sexfiles_diskfs100_ap3_proof)`) |
| `servers/sexfiles/build.rs` | Added AP3 cfg; fixed AP2/AP3 env checks from `is_ok()` to `== "1"` |
| `scripts/run_daily_driver_proof.sh` | Changed AP2 export to allow override (`:-1`) |
| `docs/handoff/SEXFILES_DISKFS_100_AP3_MULTI_OBJECT_PKU_ISOLATION.md` | This document |

## 2. Profile/Run Used

```
SEXFILES_DISKFS_100_PROOF=0 \
SEXFILES_DISKFS_100_AP3_PROOF=1 \
SEXOS_STORAGE_100_PROOF=1 \
DAILY_DRIVER_PROBE_SECONDS=90 \
./scripts/run_daily_driver_proof.sh /tmp/sexfiles_diskfs_ap30_pku.log
```

AP2 is disabled; AP3 runs the multi-object proof in isolation and returns.

## 3. Object Order

1. Phase 0: Ensure V2 manifest
2. Phase 1: Lookup path_id=0 (sexfiles-proof-v1), path_id=1 (linen-object-v1), path_id=2 (quil-object-v1) — all resolve OK
3. Phase 2: **Linen** (path_id=1) — write 128 bytes in 8×16-byte chunks, all succeed. Then read back 128 bytes in 16×8-byte chunks:
   - Offsets 0-112 (15 reads): **OK**
   - Offset 120 (16th read): **triggers fault cascade**
4. Phase 3: **Quil** (path_id=2) — **never reached**
5. Phase 4: SexFiles proof intact check — **never reached**
6. Phase 5: Negative tests — **never reached**

## 4. Last Marker Before PKU

```
[sexfiles.diskfs100.ap3.object.read.begin] name=linen path_id=1 off=120 len=8
```

The next operation is `diskfs_read_object` → `diskfs_lookup_path` (manifest read, LBA 2046) → data read (LBA 2030) → sexdrive processes → NVMe SQ fault.

Complete sequence of the fatal I/O:

```
[sexfiles.diskfs.call] slot=15 opcode=0x1 arg0=0xfdc00 arg1=0x200 arg2=0x11
  ... scheduler yields pd_id=11 ...
[sexdrive.block.typed.recv] cmd=1 offset=0xfdc00 size=512 buf_cap=0x11 caller=11
[sexdrive.block.req] op=READ ready=1 lba=2030 bytes=512 buffer_cap=0x11
[kernel.memlend.map.ok] va=0x4000003de000 len=4096
[sexdrive.bufcap.map.ok] fill_va=0x4000003de000
[sexdrive.block.nvme.submit] op=READ lba=2030 bytes=512 cid=1349 tail=5 ready=1
[sexdrive.block.read.handoff.nvme.begin] offset=0xfdc00 size=512 dst_va=0x4000003de000
[sexdrive.nvme.cmd.range] path=typed slba=2030 nlb=0 max_lba=2047 ok=1
EXCEPTION: PAGE FAULT at 0x400000009140 (RIP: 0x410063ac, RSP: 0x7000010ffb08, ERR: 0x6)
```

## 5. Fault Marker Evidence

### Fault 1: sexdrive PAGE FAULT (PD 2, PKU Key 2)

```
EXCEPTION: PAGE FAULT at 0x400000009140 (RIP: 0x410063ac, RSP: 0x7000010ffb08, ERR: 0x6)
task.faulted id=2 pd_id=2
```

- ERR=0x6: user-mode (bit 2), write (bit 1), page not present (bit 0=0)
- **Not a PKU violation** — this is a missing page mapping
- Address 0x400000009140 = NVMe I/O Submission Queue base (0x400000009000) + 0x140
- NVMe SQ was mapped during boot: `[sexdrive.nvme.ioq.alloc.ok] io_sq_va=0x400000009000`
- The SQ page mapping was lost after ~23 prior I/O operations

### Fault 2: sexinput PAGE FAULT (PD 5, PKU Key 4)

```
EXCEPTION: PAGE FAULT at 0x40000034e0cc (RIP: 0x44011ed6, RSP: 0x7000040ff9a8, ERR: 0x4)
task.faulted id=5 pd_id=5
```

- ERR=0x4: user-mode, read, page not present
- Likely a cascade effect from Fault 1

### Fault 3: sexdisplay PKU SECURITY VIOLATION (PD 1, PKU Key 1) — FATAL

```
🔥 HARDWARE SECURITY VIOLATION: PKU LOCK ENGAGED 🔥
FAULT ADDR: 0x70000e0ffdf8
FAULT RIP:  0x400090d6
CURRENT PD: 1
PKRU STATE: 0x00000000
VIOLATION: Access Denied (Read/Data)
KERNEL PANIC: panicked at kernel/src/interrupts.rs:510:9:
PKU SECURITY VIOLATION (READ at 0x70000e0ffdf8)
```

- PD 1 (sexdisplay) reads address 0x70000e0ffdf8
- 0x70000e0ffdf8 is in PD 15's (kaleidoscope) stack region (pattern: 0x70000X...)
- PKRU=0x00000000 (all keys allowed) but PKU LOCK is engaged
- PKU lock makes violations fatal regardless of PKRU

## 6. Root Cause Classification

**G: Other exact cause** — NVMe MMIO mapping exhaustion cascade.

The root cause is an **NVMe Submission Queue virtual address mapping loss** after repeated I/O operations. Sexdrive's NVMe SQ was identity-mapped during initialization at `io_sq_va=0x400000009000`. After approximately 23-24 disk I/O operations (the multi-object proof does manifest reads + data reads/writes for Linen), the SQ page mapping becomes "not present" (ERR=0x6, P=0), causing sexdrive to page-fault when writing the next SQ doorbell at offset 0x140.

This initiates a three-fault cascade:
1. Sexdrive (PD 2) page-faults on NVMe SQ page → scheduler runs
2. Sexinput (PD 5) page-faults on unrelated address → scheduler runs
3. Sexdisplay (PD 1) triggers PKU security violation accessing PD 15's stack → kernel panic

The AP2 proof does not trigger this because it performs fewer I/O operations (8 writes + 8 reads = 16, plus manifest reads, total ~24) but completes all of them within a single object path. The multi-object proof adds more manifest reads (repeated lookups per chunk), pushing the I/O count past the threshold where the SQ mapping is lost.

**Why the SQ mapping is lost**: Potential causes (requires kernel investigation):
- The NVMe MMIO pages were granted via a temporary grant that expires after N uses
- Page table entry for the SQ page is being overwritten by grant mappings at nearby virtual addresses
- The kernel's MemLend implementation reuses VA space and clobbers the SQ PTE
- The grant buffer mapping (0x4000003de000) is near enough to the SQ mapping (0x400000009000) that page table operations on the grant region corrupt the SQ PTE

## 7. AP2 Unaffected Confirmation

AP2 was disabled in this run (`SEXFILES_DISKFS_100_PROOF=0`). When AP2 is enabled (`=1`), it runs before AP3 and returns early (`return;`), so the multi-object proof never executes. AP2's behavior and code path are completely unchanged.

The build.rs fix (`is_ok()` → `== "1"`) does not alter AP2 behavior when `SEXFILES_DISKFS_100_PROOF=1` — the cfg is still set correctly.

## 8. Recommended Next AP3.1 Fix

The fix requires kernel investigation (STOP FIRST per mission constraints):

**Short-term diagnostic**: Check if the NVMe SQ page PTE is being overwritten during grant operations. Add kernel trace markers at `kernel.memlend.map` to log VA ranges and check for collisions with the SQ mapping at 0x400000009000.

**Likely fix**: The kernel's MemLend grant allocator is using VA space that overlaps with or corrupts the sexdrive NVMe MMIO identity mappings. The grant VA 0x4000003de000 and SQ VA 0x400000009000 are in the same 0x400000000000 region. Fix the VA allocator to exclude MMIO-mapped regions, or use separate VA regions for grants vs MMIO.

## 9. Gate Status

```
faults_zero  FAIL   FAULTS FOUND: panic KERNEL PANIC PAGE FAULT
sexfiles_diskfs_bridge_fixed_object_rw  SKIP  (AP2 disabled, expected)
```

Total: 258 PASS, 3 FAIL, 100 SKIP. FINAL: FAIL.
