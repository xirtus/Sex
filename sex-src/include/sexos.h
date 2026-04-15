#ifndef _SEXOS_H
#define _SEXOS_H

#include <stddef.h>
#include <stdint.h>

/**
 * SexOS User-Land SDK (libsys)
 * Standard C interface for the Sex Microkernel.
 */

// --- System Call Numbers (Standard x86_64 Linux) ---
#define SYS_READ         0
#define SYS_WRITE        1
#define SYS_OPEN         2
#define SYS_CLOSE        3
#define SYS_STAT         4
#define SYS_FSTAT        5
#define SYS_POLL         7
#define SYS_LSEEK        8
#define SYS_MMAP         9
#define SYS_MPROTECT     10
#define SYS_BRK          12
#define SYS_SIGACTION    13
#define SYS_IOCTL        16
#define SYS_YIELD        24
#define SYS_GETPID       39
#define SYS_EXIT         60
#define SYS_KILL         62
#define SYS_RENAME       82
#define SYS_UNLINK       87
#define SYS_CLOCK_GETTIME 228
#define SYS_SPAWN_PD     400
#define SYS_REGISTER_IRQ 401
#define SYS_SYSINFO      402

// --- Types ---
typedef uint32_t pd_id_t;
typedef uint32_t cap_id_t;

typedef struct {
    uint64_t uptime;
    uint64_t total_ram;
    uint64_t used_ram;
    uint32_t pd_count;
    uint32_t cpu_count;
} SexSysInfo;

// --- Syscall Wrapper (x86_64) ---
static inline uint64_t _syscall(uint64_t num, uint64_t arg0, uint64_t arg1, uint64_t arg2) {
    uint64_t ret;
    __asm__ __volatile__ (
        "syscall"
        : "=a"(ret)
        : "a"(num), "D"(arg0), "S"(arg1), "d"(arg2)
        : "rcx", "r11", "memory"
    );
    return ret;
}

// --- libsys API ---

static inline void sexos_exit(int status) {
    _syscall(SYS_EXIT, (uint64_t)status, 0, 0);
}

static inline cap_id_t sexos_open(const char *path, int flags) {
    return (cap_id_t)_syscall(SYS_OPEN, (uint64_t)path, (uint64_t)flags, 0);
}

static inline size_t sexos_read(cap_id_t fd, void *buf, size_t count) {
    return (size_t)_syscall(SYS_READ, (uint64_t)fd, (uint64_t)buf, (uint64_t)count);
}

static inline size_t sexos_write(cap_id_t fd, const void *buf, size_t count) {
    return (size_t)_syscall(SYS_WRITE, (uint64_t)fd, (uint64_t)buf, (uint64_t)count);
}

static inline pd_id_t sexos_spawn_pd(cap_id_t spawn_cap, const char *path) {
    return (pd_id_t)_syscall(SYS_SPAWN_PD, (uint64_t)spawn_cap, (uint64_t)path, 0);
}

static inline void sexos_yield() {
    _syscall(SYS_YIELD, 0, 0, 0);
}

static inline void* sexos_brk(void *addr) {
    return (void*)_syscall(SYS_BRK, (uint64_t)addr, 0, 0);
}

static inline pd_id_t sexos_getpid() {
    return (pd_id_t)_syscall(SYS_GETPID, 0, 0, 0);
}

#endif // _SEXOS_H
