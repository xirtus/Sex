# Phase 28 Allocator State

## Heap Boundaries
- **HEAP_SIZE**: 128 MiB (128 * 1024 * 1024 bytes)
- **HEAP_START**: 0x_4444_4444_0000 (standard linked-in allocator address)

## The Root Cause of Order 4 Allocation Failure
The `LockFreeBuddyAllocator` (`GLOBAL_ALLOCATOR`) was failing to satisfy `alloc(4)` (16 pages = 64 KiB) during ELF loading, resulting in a `loader: stack OOM` panic.

The investigation revealed that `init_metadata` is **never called** during kernel initialization.

As a result:
- `metadata_base` remains `0`.
- When `add_memory_region` is called in `kernel/src/memory/manager.rs`, it checks for metadata using `get_metadata`. Because `metadata_base` is 0, `get_metadata` returns a null pointer.
- The allocator gracefully but silently aborts adding physical frames to its free lists.
- The `GLOBAL_ALLOCATOR` remains completely empty.
- When `alloc(4)` is called, it correctly returns `None`, triggering the panic.

## Required Patch
`kernel/src/memory/manager.rs` must be patched to:
1. Iterate over the memory map to determine the `total_pages` of physical memory available.
2. Calculate the required memory size for the `PageMetadata` array (`total_pages * size_of::<PageMetadata>()`).
3. Allocate this metadata array from the initial physical frames. Since we already have a bump allocator active before initializing the lock-free buddy allocator, we can allocate the frames using it.
4. Map the physical frames for the metadata array into virtual memory.
5. Call `GLOBAL_ALLOCATOR.init_metadata(vaddr, total_pages)`.
6. Only then proceed with `GLOBAL_ALLOCATOR.add_memory_region(...)`.
