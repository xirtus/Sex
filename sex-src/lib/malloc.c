#include <sexos.h>
#include <stddef.h>

/**
 * SexOS User-Land Memory Allocator
 * A minimal, efficient bump-allocator for the SASOS environment.
 * In a SASOS, we manage a large heap region granted by the 'sext' pager.
 */

static void *heap_start = NULL;
static void *heap_end = NULL;
static void *current_brk = NULL;

void* malloc(size_t size) {
    // 1. Initialize heap if first call
    if (heap_start == NULL) {
        heap_start = sexos_brk(NULL); // Get current break
        current_brk = heap_start;
        // Pre-allocate 1MB for the initial heap
        heap_end = (char*)heap_start + (1024 * 1024);
        sexos_brk(heap_end);
    }

    // 2. Align size to 16 bytes
    size = (size + 15) & ~15;

    // 3. Check for OOM
    if ((char*)current_brk + size > (char*)heap_end) {
        // Expand heap by another 1MB
        heap_end = (char*)heap_end + (1024 * 1024);
        if (sexos_brk(heap_end) == (void*)-1) {
            return NULL; // Real OOM
        }
    }

    // 4. Allocate and advance
    void *ptr = current_brk;
    current_brk = (char*)current_brk + size;
    return ptr;
}

void free(void *ptr) {
    // In a simple bump allocator, free is a no-op.
    // Future expansion: Implement a block-list or slab for real 'free'.
}

void* calloc(size_t nmemb, size_t size) {
    size_t total = nmemb * size;
    void *ptr = malloc(total);
    if (ptr) {
        for (size_t i = 0; i < total; i++) {
            ((char*)ptr)[i] = 0;
        }
    }
    return ptr;
}
