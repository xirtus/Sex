#ifndef _UNISTD_H
#define _UNISTD_H

#include <stddef.h>
#include <stdint.h>
#include <sexos.h>

/**
 * SexOS Unix Standard Header (unistd.h)
 */

static inline ssize_t read(int fd, void *buf, size_t count) {
    return (ssize_t)sexos_read((cap_id_t)fd, buf, count);
}

static inline ssize_t write(int fd, const void *buf, size_t count) {
    return (ssize_t)sexos_write((cap_id_t)fd, buf, count);
}

static inline int close(int fd) {
    // Future: Implement sys_close
    return 0;
}

static inline uint32_t getpid(void) {
    return sexos_getpid();
}

static inline int isatty(int fd) {
    // For the prototype, we assume fd 0, 1, 2 are always TTYs
    return (fd >= 0 && fd <= 2);
}

#endif // _UNISTD_H
