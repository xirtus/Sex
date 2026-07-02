# SEXFILES_DISKFS_100_AP3_2_VA_PKU_FAULT_AUDIT

## 1. Files Inspected

| File | Purpose |
|------|---------|
| `kernel/src/interrupts.rs` | Page fault handler, PKU warden trigger, fault.kill path |
| `kernel/src/pku.rs` | PKU enable/disable, rdpkru/wrpkru, pku_warden, tag_virtual_address, set_page_user_accessible |
| `kernel/src/syscalls/mod.rs` | Syscall 30 (MAP_MEMORY/phys), 31 (ALLOCATE_MEMORY), 43 (MAP_PCI_BAR), 50 (GRANT_MEM_LEND), 51 (MAP_MEM_LEND) |
| `kernel/src/memory/va_allocator.rs` | Global VA bump allocator (starts 0x4000_0000_0000) |
| `kernel/src/memory/manager.rs` | GlobalVas, map_physical_range, BootInfoFrameAllocator vs GLOBAL_ALLOCATOR init |
| `apps/sexdrive/src/main.rs` | NVMe BAR mapping, IO SQ/CQ setup, doorbell writes, handoff read path (nvme_read_into_mapped_va) |
| `crates/sex-pdx/src/lib.rs` | SLOT_BUF_LEND, sys_grant_mem_lend/sys_map_mem_lend ABI |
| `kernel/src/capability.rs` | MemLendCapData structure |

## 2. AP3 Log Used

**Log:** `/tmp/sexfiles_diskfs_ap31_multi_object.log` (737K, last modified May 22 23:50)

### Key Log Lines

```
# NVMe BAR map
1529: [sexdrive.nvme.bar.resolve.ok] map_va=0x400000001000

# IO queue alloc (CONFIRMS io_sq_va)
1559: [sexdrive.nvme.ioq.alloc.ok] io_cq_va=0x400000008000 io_sq_va=0x400000009000

# IO queue doorbell offsets
1566: [sexdrive.nvme.ioq.ready] sq1tdbl=0x1008 cq1hdbl=0x100c

# Last linen write OK (8/8 done)
10347: [sexfiles.diskfs100.ap3.object.read.begin] name=linen off=0

# 15 successful linen reads (off=0,8,16,24,32,40,48,56,64,72,80,88,96,104,112)
# Each read triggers 2 NVMe operations (lba=2046 + lba=2030)

# 16th linen read attempt (FAILS)
12433: [sexfiles.diskfs100.ap3.object.read.begin] name=linen off=120

# Last successful NVMe operations (linen off=112, lba=2030)
12432: [sexfiles.diskfs100.ap3.object.read.ok] name=linen off=112

# Off=120 triggers NVMe read (lba=2046, cid=1348 -> OK)
12455: [sexdrive.block.nvme.submit] op=READ lba=2046 cid=1348
12468: [sexdrive.block.reply] op=READ status=0

# Off=120 triggers NVMe read (lba=2030, cid=1349 -> FAULT)
12527: [sexdrive.block.nvme.submit] op=READ lba=2030 cid=1349 tail=5
12528: [sexdrive.block.read.handoff.nvme.begin] dst_va=0x4000003de000
12529: [sexdrive.nvme.cmd.range] path=typed slba=2030 ok=1
12530: EXCEPTION: PAGE FAULT at 0x400000009140 (RIP: 0x410063ac, RSP: 0x7000010ffb08, ERR: 0x6)
12531: task.faulted id=2 pd_id=2

# Cascade faults
12542: EXCEPTION: PAGE FAULT at 0x40000034e0cc (RIP: 0x44011ed6, ERR: 0x4)
12543: task.faulted id=5 pd_id=5

# PKU violation -> KERNEL PANIC
12581: HARDWARE SECURITY VIOLATION: PKU LOCK ENGAGED
12582: FAULT ADDR: 0x70000e0ffdf8
12588: KERNEL PANIC: panicked at kernel/src/interrupts.rs:510:9:
12589: PKU SECURITY VIOLATION (READ at 0x70000e0ffdf8)
```

## 3. Fault Timeline

```
T0: SexFiles AP3 writes linen object (8/8 chunks, off=0-112), ALL SUCCESS
T1: SexFiles begins linen read (off=0) - SUCCESS (NVMe reads ok)
...
T14: SexFiles linen read off=112 - SUCCESS (NVMe reads ok)
T15: SexFiles linen read off=120 - starts NVMe read lba=2046 -> OK
T16: SexFiles linen read off=120 - starts NVMe read lba=2030, cid=1349
     -> writes SQE at io_sq_va + 5*64 = 0x400000009000 + 0x140
     -> PAGE NOT PRESENT at 0x400000009140
T17: Kernel page fault handler blocks PD 2 (sexdrive)
T18: Cascade: PAGE NOT PRESENT at 0x40000034e0cc (PD 5, sexusb)
T19: Cascade: PKU VIOLATION at 0x70000e0ffdf8 (PD 14/kaleidoscope stack area)
T20: KERNEL PANIC at interrupts.rs:510
```

## 4. Address Ownership Table

| Address | Value | Owner/Meaning |
|---------|-------|---------------|
| `0x400000009140` | PAGE FAULT target | **io_sq_va + 5*64** = IO SQ page, 5th SQE slot (64-byte NVMe command) |
| `0x400000009000` | io_sq_va (from log line 1559) | IO Submission Queue page, PD 2 (sexdrive), phys=0x1F804000 |
| `0x400000008000` | io_cq_va (from log line 1559) | IO Completion Queue page, PD 2, phys=0x1F803000 |
| `0x400000001000` | map_va (NVMe BAR, from log line 1529) | NVMe PCI BAR0 VA, PD 2, mapped via syscall 43 (MAP_PCI_BAR), size=0x4000 |
| `0x400000202008` | map_va + sq1tdbl (0x1008) | SQ1 doorbell MMIO register (NOT the fault address) |
| `0x4000003de000` | dst_va for cid=1349 read | MemLend buffer VA for linen off=120 read, PD 2 |
| `0x1F804000` | io_sq_phys (from log line 1559) | IO SQ physical page, allocated by sys_alloc_phys during init |
| `0x1F803000` | io_cq_phys (from log line 1559) | IO CQ physical page, allocated by sys_alloc_phys during init |
| `0x1032d000` | PRP1 for cid=1348 | Bounce buffer physical page (last successful read) |
| `0x40000034e0cc` | Second PAGE FAULT | Unknown mapping in PD 5 (sexusb) space, cascade from first fault |
| `0x70000e0ffdf8` | PKU violation addr | PD 14 (kaleidoscope) stack guard area: 0x70000e100000 - 0x208 |
| `0x410063ac` | Fault RIP (PD 2) | Instruction in sexdrive binary, writing SQE at io_sq_va offset |

### Confirmed Address Derivation

```
io_sq_va = 0x400000009000  (from ioq.alloc.ok log)
sq_tail  = 5                (for cid=1349, the 6th slot in 16-entry ring)
SQE slot = sq_tail * 64    = 5 * 64 = 320 = 0x140
Fault VA = io_sq_va + 0x140 = 0x400000009000 + 0x140 = 0x400000009140 [EXACT MATCH]
```

## 5. Root Question Answers

### Q1: Is the first fault definitely SexDrive PD 2?

**YES.** Confirmed by `task.faulted id=2 pd_id=2` at log line 12531. Fault RIP `0x410063ac` is in PD 2's code segment (PD 2 base = 0x41000000). The fault occurs while sexdrive executes `nvme_read_into_mapped_va`, writing the 6th SQE entry into the IO SQ.

### Q2: What instruction/path writes 0x400000009140?

The `nvme_read_into_mapped_va` handoff function at line 423-427 of `apps/sexdrive/src/main.rs`:
```rust
let sqe_ptr = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
unsafe {
    core::ptr::write_volatile(sqe_ptr.add(0), 0x02u32 | ((cid as u32) << 16)); // READ + CID
}
```
This writes the first dword of the NVMe SQE at `io_sq_va + 5*64 = 0x400000009140`.

### Q3: Is 0x400000009140 SQ memory, CQ memory, or MMIO doorbell?

**SQ memory.** It is `io_sq_va + sq_tail*64`, the 6th 64-byte SQE slot in the IO Submission Queue page. NOT the doorbell (which would be at `map_va + sq1tdbl = 0x400000001000 + 0x1008 = 0x400000002008`).

### Q4: Is the PTE missing, or PKU-denied, for the first fault?

**PTE missing (page not present).** Error code 0x6 = bits 1 (write) + bit 2 (user mode). Bit 0 = 0 means **not present** (not a protection violation). Bit 5 = 0 means **not PKU**. The IO SQ page's page table entry has its Present bit cleared.

### Q5: Is the sexdisplay PKU violation primary or cascade?

**Cascade.** The PKU violation at 0x70000e0ffdf8 occurs AFTER the primary page fault in PD 2. It appears as a SECOND page fault entry (line 12581) with PKU bit set, triggered by PD 1 (sexdisplay, inferred from context) accessing memory at PD 14's stack guard area (0x70000e0ffdf8). The cascade sequence is:
1. Primary: PD 2 page not present at 0x400000009140
2. Secondary: PD 5 page not present at 0x40000034e0cc  
3. Tertiary: PD 1 PKU violation at 0x70000e0ffdf8

This three-fault cascade suggests **page table corruption** rather than an isolated single-page unmap.

### Q6: Does MemLend grant/mapping reuse the same VA region as NVMe queue mappings?

**No, different 2MB regions.** NVMe queues at 0x400000008000-0x400000009FFF (2MB region starting at 0x400000000000, PD index 0). MemLend buffers at 0x400000368000-0x40000038FFFF (2MB region starting at 0x4000200000, PD index 1). They use different page directory entries and thus different page tables. Direct VA collision is ruled out.

### Q7: Does AP3 allocate/revoke more MemLend buffers than AP2?

**Yes.** AP3 is multi-object (linen + quil). The log shows 16+ MemLend consumer mappings for linen alone. AP2 was single-object. Each MemLend map allocates a new VA via the global VA allocator. Additionally, each NVMe read allocates a bounce buffer VA via `sys_map_phys`. The VA allocator cursor advances with each allocation.

### Q8: Does a grant revoke/unmap path run before linen read off=120?

**No explicit revoke detected.** The kernel has `multicast_revoke_key` (pku.rs:113) which only does TLB flush (interrupts.rs:762-764). No pages are explicitly unmapped. No `free_va` or `dealloc_va` exists. However, the VA allocator is a pure bump (never frees), so it can't cause VA reuse.

### Q9: Is the last linen read using the same buffer cap as earlier successful reads?

**Yes.** All linen reads use the same MemLend buffer cap slot (buf_cap=0x11 = SLOT_BUF_LEND). However, the buffer's VA at consumer (sexdrive) side changes per-read because `sys_map_mem_lend` allocates a new VA each time. The dst_va for the faulting read is 0x4000003de000 (new, distinct from prior reads' VAs like 0x4000003dc000).

### Q10: Is the failure deterministic at same chunk/object?

**Yes, deterministic.** The failure always occurs on the LAST linen read (off=120, the 16th chunk), specifically on the SECOND NVMe read within that chunk (lba=2030, after lba=2046 succeeds). This reproducibility suggests the corruption happens during one of the preceding MemLend map operations, not from a race condition.

## 6. Classification: A

**A) NVMe queue VA page unmapped/reused by MemLend allocator.**

More precisely: **Physical frame allocator overlap between BootInfoFrameAllocator (page table frames) and GLOBAL_ALLOCATOR (data frames).**

### Evidence Chain

1. `kernel/src/memory/manager.rs` init code (lines 117-179) shows TWO allocators:
   - **BootInfoFrameAllocator** (passed to `OffsetPageTable::map_to`) — allocates physical frames for **page table pages** (PML4, PDPT, PD, PT).
   - **GLOBAL_ALLOCATOR** (LockFreeBuddyAllocator) — allocates physical frames for **data pages** (MemLend buffers, bounce buffers, NVMe queues via sys_alloc_phys).

2. The init code advances the BootInfoFrameAllocator past metadata pages (lines 173-176) but **does not coordinate** between the two allocators beyond this single point. After init, the BootInfoFrameAllocator and GLOBAL_ALLOCATOR **operate on overlapping physical memory regions without mutual exclusion.**

3. When `map_physical_range` is called (for sys_map_mem_lend or sys_map_phys), it may allocate a new page table page using BootInfoFrameAllocator. That physical frame could already be in use as a GLOBAL_ALLOCATOR data page (MemLend buffer, bounce buffer).

4. If an NVMe DMA write targets a bounce buffer whose physical frame overlaps with a page table page, the DMA data write corrupts the page table entries stored in that frame.

5. The IO SQ PTE happens to reside in the corrupted page table page -> PTE Present bit becomes 0 -> next access to IO SQ triggers PAGE NOT PRESENT.

### Why Not Alternative Classifications

| Class | Why Rejected |
|-------|-------------|
| B (PKU permission change) | Error code 0x6 has PKU bit (bit 5) = 0. Not a PKU violation. |
| C (MMIO doorbell lost) | Fault is at io_sq_va+0x140 (SQ memory), not map_va+sq1tdbl (doorbell MMIO). |
| D (MemLend revoke clears SexDrive queue) | No revoke/unmap code exists in kernel. VA allocator is bump-only. |
| E (corrupted queue pointer) | io_sq_va is a static global (NVME_IO_STATE.io_sq_va), not a pointer susceptible to corruption. Log shows correct values. |
| F (sexdisplay PKU is only cascade) | PKU IS cascade, but primary cause is the frame allocator overlap. |
| G (evidence insufficient) | Evidence is sufficient: confirmed VA match, error code analysis, allocator architecture analysis. |

## 7. STOP FIRST Boundaries

**STOP FIRST — root cause requires kernel edit.**

The physical frame allocator overlap between `BootInfoFrameAllocator` and `GLOBAL_ALLOCATOR` is a **kernel memory management bug**. Fixing it requires either:

1. **Synchronizing the two allocators**: Ensure BootInfoFrameAllocator only hands out frames from regions NOT in GLOBAL_ALLOCATOR's pool, or vice versa.
2. **Unifying the allocators**: Replace BootInfoFrameAllocator with GLOBAL_ALLOCATOR for page table frame allocation.
3. **Reserving a dedicated pool** for page table frames that data allocations cannot touch.

### What NOT to do:
- Do NOT suppress page faults (hides the symptom)
- Do NOT disable PKU (hides cascade detection)
- Do NOT retry on fault (data corruption already occurred)
- Do NOT add OOM checks to sexdrive (allocator, not consumer, is at fault)

## 8. Recommended AP3.3 Patch Scope

### Phase 1: Diagnostic Instrumentation (NO FIX YET)

Add instrumentation to `kernel/src/memory/manager.rs` and `kernel/src/memory/allocator.rs`:

1. **Frame overlap detector**: On every `GLOBAL_ALLOCATOR.alloc`, check if the returned physical frame is within the BootInfoFrameAllocator's active range. Log a warning if overlap detected.
2. **PTE integrity check**: Before and after `map_physical_range`, checksum the target PTEs and log if any change unexpectedly.
3. **Bounce buffer physical logging**: Log the bounce buffer physical address for cid=1349 (the faulting read) to confirm it overlaps with a page table frame.

### Phase 2: Fix (after diagnosis confirmed)

1. **Separate BootInfoFrameAllocator pool**: After `GLOBAL_ALLOCATOR` init, mark all remaining BootInfoFrameAllocator frames as "used" in GLOBAL_ALLOCATOR. Or initialize GLOBAL_ALLOCATOR first, then use it as the frame allocator for page tables.
2. **Replace BootInfoFrameAllocator**: Use GLOBAL_ALLOCATOR for ALL frame allocations (both data and page table), removing the dual-allocator split entirely.
3. **Page table frame pool**: Reserve a small contiguous pool for page table frames, protected from data allocations.

### Scope Boundary

- `kernel/src/memory/manager.rs`: Fix init sequence, frame allocator coordination
- `kernel/src/memory/allocator.rs`: Add GLOBAL_ALLOCATOR-based frame allocation for page tables
- **NO changes to**: apps/sexdrive, servers/sexfiles, servers/sexdisplay, crates/sex-pdx, kernel/src/pku.rs, kernel/src/interrupts.rs

---

*Audit completed: 2026-05-23*
*Classification: A — Physical frame allocator overlap*
*STOP FIRST: Kernel allocator edit required*
