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

#endif // _UNISTD_H
