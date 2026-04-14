#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>
#include <sexos.h>

/**
 * SexOS Standard Library Header (stdlib.h)
 */

void* malloc(size_t size);
void free(void *ptr);
void* calloc(size_t nmemb, size_t size);

static inline void exit(int status) {
    sexos_exit(status);
}

static inline void* sbrk(intptr_t increment) {
    void *old_brk = sexos_brk(NULL);
    if (increment == 0) return old_brk;
    void *new_brk = (char*)old_brk + increment;
    if (sexos_brk(new_brk) == (void*)-1) return (void*)-1;
    return old_brk;
}

#endif // _STDLIB_H
