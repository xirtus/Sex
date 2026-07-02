# KERNEL MEMORY ALLOCATOR OVERLAP — AP1 DIAGNOSTIC

**Date:** 2026-05-23
**Status:** COMPLETE — Overlap Proven
**Classification:** A — Direct overlap observed

---

## 1. Files Changed

- `kernel/src/memory/allocator.rs` — Added 64-entry ring buffer, overlap detector, diagnostic markers
- `kernel/src/memory/manager.rs` — Added boot frame alloc markers, region markers, metadata carve marker

## 2. Allocator Architecture Summary

### BootInfoFrameAllocator
- **Source:** `kernel/src/memory/manager.rs`
- **Regions:** Iterates limine `MemmapResponse`, filter `type_ == 0` (usable) only
- **Allocation:** Cursor-based linear scan, O(regions) per call
- **Consumers:** `init_heap()` (65536 pages for 256 MiB heap), metadata skip (505 pages), then `map_pku_range()` and x86_64 `Mapper::map_to()` for page table intermediate frames
- **Stored in:** `GLOBAL_VAS` (Mutex<Option<GlobalVas>>), used by all PD page table operations

### GLOBAL_ALLOCATOR (LockFreeBuddyAllocator)
- **Source:** `kernel/src/memory/allocator.rs`
- **Regions:** Fed from remaining usable memory after heap + metadata carve-out
- **Allocation:** Lock-free buddy allocator with per-core sharded free lists (MAX_ORDER=18)
- **Consumers:** `syscall 31` (ALLOCATE_MEMORY), `MemLend` grant path (syscall 34), `alloc_frame()` helper
- **Stored in:** Static `GLOBAL_ALLOCATOR: LockFreeBuddyAllocator`

### sys_alloc_phys source
- `kernel/src/syscalls/mod.rs:294` — syscall 31 ALLOCATE_MEMORY: calls `GLOBAL_ALLOCATOR.alloc(order)`
- `kernel/src/syscalls/mod.rs:449` — syscall 34 MemLend grant: calls `GLOBAL_ALLOCATOR.alloc(order)`

## 3. Regions Added to GLOBAL_ALLOCATOR

From runtime diagnostic log:

```
[kernel.mem.global.metadata.carve] phys=0x100b1000 size=2068480 pages=505 totals=129041
[kernel.mem.global.region.add] start=0x102aa000 end=0x1f811000 size=257323008
```

- **Only ONE region** was added: `0x102aa000` to `0x1f811000` (~245 MiB)
- Metadata carved at `0x100b1000`, consuming 505 pages
- Heap consumed all of region 1 (0x50000-0x9f000, 79 pages) + part of region 2 (65457 pages)

## 4. Boot Frame Allocations Observed

- Total boot frame allocations: **67,845**
- First allocation: `phys=0x50000 idx=1`
- Last allocation: `phys=0x109b5000 idx=67845`
- Metadata skip: 505 pages from `0x100b1000` through `0x102a9000`
- After skip, next allocation at `0x102ab000`

## 5. Global Allocations Observed

- Total global allocations: **105**
- First allocation: `phys=0x1f810000 size=4096 order=0 path=global` (top of region)
- Later allocations: from `0x102aa000` upward (bottom of region, via split path)
- Allocation range: `0x102aa000` through `0x1032e000` (from bottom) and `0x1f810000` through `0x1f80c000` (from top)

## 6. Overlap Detector Result

- **Ring buffer:** 64 entries, circular, records every allocate_frame() call
- **Runtime detection:** 0 hits (ring too small — 67,845 allocations in 64-entry buffer; entries overwritten ~1060x)
- **Post-hoc log analysis:** **88 out of 105 (83.8%) global allocations overlap with boot frame allocations**

### Overlap Range

```
0x102aa000 through 0x1032e000 (88 frames)
```

These frames were allocated by BootInfoFrameAllocator during early boot (idx 66041–66174 range, right after metadata skip) for page tables and kernel mappings. The GLOBAL_ALLOCATOR then handed out the SAME physical frames for MemLend and sys_alloc_phys requests.

### Direct Proof

The log shows:

```
[kernel.mem.boot_frame.alloc] phys=0x1032e000 idx=66174    (line 66260 — early boot)
[kernel.mem.global.alloc] phys=0x1032e000 size=4096 order=0 path=split  (line 80346 — just before fault)
```

The global allocation at `0x1032e000` occurred **immediately before** the PAGE FAULT at `0x400000009080`.

## 7. AP3 Fault Reproduced: YES

Fault sequence observed in log (lines 80348–80402):

```
EXCEPTION: PAGE FAULT at 0x400000009080 (RIP: 0x410063ac, RSP: 0x7000010ffb08, ERR: 0x6)
  -> PD 2 (sexdrive): page-not-present write in NVMe SQ memory (0x400000009000 + 0x80)

EXCEPTION: PAGE FAULT at 0x40000034e0cc (RIP: 0x44011ed6, RSP: 0x7000040ff9a8, ERR: 0x4)
  -> PD 5 (sexusb): user page-not-present (cascade)

HARDWARE SECURITY VIOLATION: PKU LOCK ENGAGED
FAULT ADDR: 0x70000e0ffdf8, CURRENT PD: 1 (sexdisplay)
KERNEL PANIC: PKU SECURITY VIOLATION
```

Cascade matches exactly: sexdrive -> sexusb -> sexdisplay PKU -> panic.

Root cause: The page table PTE for sexdrive NVMe SQ VA was stored in physical frame `0x1032e000` (or nearby). When GLOBAL_ALLOCATOR handed out `0x1032e000` for MemLend, the MemLend data write overwrote the PTE, making the SQ page "not present." The next NVMe command submission triggered the page fault.

## 8. Classification: A

**A — Direct overlap observed:** Same physical frames allocated by both BootInfoFrameAllocator and GLOBAL_ALLOCATOR. 88 out of 105 global allocations (83.8%) are duplicate allocations.

## 9. Recommended AP2 Fix Scope

The root cause is that BootInfoFrameAllocator and GLOBAL_ALLOCATOR share the same physical frame pool with no reservation/exclusion mechanism.

**Required fix (AP2):**
1. After initializing GLOBAL_ALLOCATOR, add all remaining usable region pages to GLOBAL_ALLOCATOR as "reserved" (or simply do not add them at all)
2. OR: Replace BootInfoFrameAllocator with GLOBAL_ALLOCATOR for the FrameAllocator impl, so there is a single source of truth
3. OR: Add an exclude_range() mechanism to GLOBAL_ALLOCATOR that marks frames already allocated by BootInfoFrameAllocator as occupied

**Minimal fix approach:**
- Track the BootInfoFrameAllocator cursor position at init time
- Add a reserve_range() to GLOBAL_ALLOCATOR that marks frames from region start to cursor as allocated (state=1)
- This ensures frames already handed out by BootInfoFrameAllocator are not re-issued by GLOBAL_ALLOCATOR

## 10. STOP FIRST Blockers

- The ring buffer (64 entries) is insufficient for the 67k+ boot frame allocations; log-based post-hoc analysis was required
- No semantic allocator changes were made
- No reservation logic was added
- Build and instrumentation only; no behavior changes

## 11. Commit Recommendation

```
git add kernel/src/memory/manager.rs kernel/src/memory/allocator.rs docs/handoff/KERNEL_MEMORY_ALLOCATOR_OVERLAP_AP1_DIAGNOSTIC.md
git commit -m "kernel: instrument allocator frame overlap — prove root cause A"
```
