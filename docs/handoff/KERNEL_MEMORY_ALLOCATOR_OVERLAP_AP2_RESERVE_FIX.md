# KERNEL_MEMORY_ALLOCATOR_OVERLAP_AP2_RESERVE_FIX

## 1. Files Changed

- `kernel/src/memory/allocator.rs` — +72 lines
  - Added `reserve_frame()` and `reserve_page_range()` methods to `LockFreeBuddyAllocator`
  - Modified `pop_free_local()` and `pop_free_global()` to lazily skip reserved entries (state != 0)

- `kernel/src/memory/manager.rs` — +24/-10 lines
  - Made `BootInfoFrameAllocator.next` `pub(crate)` for accurate cursor read
  - Fixed skip offset: use `frame_allocator.next` (actual consumed frames) instead of `HEAP_SIZE / 4096` estimate
  - Added `GLOBAL_ALLOCATOR.reserve_frame()` call in `BootInfoFrameAllocator::allocate_frame()`
  - Added safety-net `reserve_page_range(0, consumed_total)` at end of `init()`

## 2. Exact Reserve Strategy

### Primary: Accurate Skip Offset (Bug Fix)

The root cause was that `manager.rs::init()` computed `heap_pages = HEAP_SIZE / 4096` to skip frames before feeding regions to the buddy allocator. However, `init_heap()` calls `mapper.map_to()` which may allocate additional page-table frames via the `FrameAllocator`. These extra frames advance `frame_allocator.next` beyond the `heap_pages` estimate, causing them to be included in `GLOBAL_ALLOCATOR`'s pool.

**Fix**: Use `frame_allocator.next` (actual consumed count) instead of `heap_pages`.

**Impact**: Correctly excluded 65,667 frames from GLOBAL_ALLOCATOR (vs ~16,384 with the estimate). Delta: ~49,283 page-table/boot frames no longer leaked into the buddy pool.

### Secondary: Per-Frame Reservation (Post-Init Protection)

Every call to `BootInfoFrameAllocator::allocate_frame()` now calls `GLOBAL_ALLOCATOR.reserve_frame(phys)` immediately after allocation. This sets `PageMetadata.state = 1` for the frame in the buddy allocator's metadata array.

### Tertiary: Lazy Free-List Skip

`pop_free_local()` and `pop_free_global()` now check `PageMetadata.state` before returning a frame. If `state != 0` (reserved or allocated), the entry is atomically unlinked from the free list and skipped. This provides lazy, lock-free removal without requiring O(n) free-list traversal during reservation.

### Quaternary: Safety-Net Bulk Reserve

At the end of `init()`, `reserve_page_range(0, consumed_total)` marks all pages up to `frame_allocator.next` as reserved in buddy metadata. This is a belt-and-suspenders final guard.

## 3. Range/Pages Reserved

**Boot-time reserve at init completion:**
- Start: physical page 0 (0x0)
- End: page 66,172 (0x1027c000)
- Pages: 66,172
- Actually marked: 65,844 (remaining were non-usable or already outside metadata range)

**Skip correction from heap_pages → frame_allocator.next:**
- Old skip: ~16,384 pages (HEAP_SIZE / 4096)
- New skip: 65,667 pages (frame_allocator.next after init_heap)
- Delta excluded from buddy: ~49,283 page-table/boot frames

## 4. Overlap Detection Result

- `[kernel.mem.overlap.detected]` occurrences: **0** (zero) across all 4 test profiles
- Previously: 88/105 global allocations overlapped with boot-frame allocations
- `[kernel.mem.overlap.fix.active] ok=1` — fix active marker present
- `[kernel.mem.global.reserve.boot_frames.done] reserved=65844` — bulk reserve complete

## 5. AP3 Result (DiskFS Multi-Object Proof)

- `sexfiles_diskfs_bridge_multi_object_rw`: **PASS**
- `[sexfiles.diskfs100.ap3.done] ok=1` — all multi-object write/read/match operations verified
- `faults_zero`: **PASS** (0 fault markers)
- No PAGE FAULT, PKU fault, or KERNEL PANIC markers
- FINAL: **PASS** (266 gates proved, 96 skipped, 0 faults)

## 6. AP2 Regression Result (DiskFS Fixed-Object Proof)

- `sexfiles_diskfs_bridge_fixed_object_rw`: **PASS**
- `faults_zero`: **PASS** (0 fault markers)
- FINAL: **PASS** (258 gates proved, 104 skipped, 0 faults)

## 7. SexDrive Regression Result

- `sexdrive_storage_ioq_ready`: **PASS** (NVMe IOQ ready, qid=1 depth=16)
- `sexdrive_storage_single_block_rw`: **PASS** (single-block write/read/match verified)
- `sexdrive_storage_multiblock_rw`: **PASS** (bounded multi-block write/read/match verified)
- `faults_zero`: **PASS** (0 fault markers)
- FINAL: **PASS** (260 gates proved, 102 skipped, 0 faults)

## 8. Default Result

- `faults_zero`: **PASS** (0 fault markers)
- FINAL: **PASS** (257 gates proved, 105 skipped, 0 faults)

## 9. Remaining Blockers

None. All 4 proof profiles pass with zero overlaps and zero faults.
