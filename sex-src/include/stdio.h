#ifndef _STDIO_H
#define _STDIO_H

#include <stddef.h>
#include <stdint.h>
#include <sexos.h>

/**
 * SexOS Standard I/O Header (stdio.h)
 */

#define STDIN_FILENO  0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

static inline int putchar(int c) {
    char ch = (char)c;
    sexos_write(STDOUT_FILENO, &ch, 1);
    return c;
}

static inline int puts(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    sexos_write(STDOUT_FILENO, s, len);
    putchar('\n');
    return 0;
}

// Minimal printf implementation
static inline int printf(const char *format, ...) {
    // Future: Use va_list to implement real formatting.
    // For now, we just print the format string as a placeholder.
    return puts(format);
}

#endif // _STDIO_H
