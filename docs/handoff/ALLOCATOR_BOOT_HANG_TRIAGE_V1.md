# ALLOCATOR_BOOT_HANG_TRIAGE_V1

## Status: RESOLVED (2026-05-03)

## Root Cause
O(n²) bug in `BootInfoFrameAllocator::allocate_frame()` in `kernel/src/memory/manager.rs`.

The original implementation regenerated the full usable-frames iterator on every call and used `.nth(self.next)` to skip forward:

```rust
fn allocate_frame(&mut self) -> Option<PhysFrame> {
    let frame = self.usable_frames().nth(self.next);
    self.next += 1;
    frame
}
```

Where `usable_frames()` created a fresh `FlatMap<StepBy<Range<u64>>>` iterator over ALL usable memory regions, stepping by 4096.

### Why it hung
- HEAP_SIZE = 256 MiB = 65536 pages
- Each `allocate_frame()` call traverses `self.next` elements from the start of the iterator
- Call N traverses N elements → total traversals = Σ(0..65535) ≈ 2.1 billion
- QEMU without KVM executes ~10⁸ operations/sec → ~20+ seconds expected
- But `FlatMap::nth()` does NOT use `StepBy::nth()` efficiently — it calls `next()` in a loop
- Each `next()` requires FlatMap dispatch + StepBy advance + bound check
- Result: ~100+ seconds under QEMU (system killed at 90s before completion)
- **Framebuffer writes and PD spawning** never reached

### Fix
Cursor-based frame allocation that walks memory regions directly in O(num_regions) per call:

```rust
fn allocate_frame(&mut self) -> Option<PhysFrame> {
    let regions = self.memory_map.entries();
    let mut frame_index = 0usize;
    for region in regions.iter().filter(|r| r.type_ == 0) {
        let frames_in_region = (region.length / 4096) as usize;
        if frame_index + frames_in_region > self.next {
            let offset = self.next.saturating_sub(frame_index) as u64;
            let phys_addr = region.base + offset * 4096;
            self.next += 1;
            return Some(PhysFrame::containing_address(PhysAddr::new(phys_addr)));
        }
        frame_index += frames_in_region;
    }
    None
}
```

Total operations: ~30 regions × 65536 calls ≈ 2M (vs 2.1B before). Boot completes in <2 seconds.

### Diagnosis Method
1. Added bounded step markers before/after `init_heap()` call in manager.rs
2. Built fresh ISO via `scripts/entrypoint_build.sh` (run_and_debug.sh uses stale ISO)
3. Observed `[allocator.init.step] name=before_init_heap` but NO `after_init_heap` marker
4. Inspected `allocate_frame()` → identified O(n²) `.nth()` pattern
5. Confirmed: `flat_map().step_by(4096).nth(n)` is O(n) per call due to FlatMap dispatch
6. Applied cursor-based fix → boot completes instantly

### Files Changed
- `kernel/src/memory/manager.rs`:
  - Rewrote `allocate_frame()` to cursor-based region walk
  - Removed `usable_frames()` helper (dead code after fix)
  - Added `#[allocator.init.step]` diagnostic markers (removed after fix confirmed)

### Lessons
- **Always rebuild ISO before testing**: `run_and_debug.sh` and `dev.sh` use `sexos-v1.0.0.iso`
  without rebuilding. Use `scripts/entrypoint_build.sh` first.
- **Reference build**: `./scripts/entrypoint_build.sh` creates fresh ISO at `sexos-v1.0.0.iso`
- **Boot debug**: Add bounded step markers at function boundaries, not inside loops
- **Nightly Rust FlatMap::nth()** is NOT O(1) — avoid for large skip indices
