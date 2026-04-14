#include <sexos.h>

/**
 * Sample User-Space Application (The Shell)
 * Demonstrating the SexOS User-Land SDK (libsys).
 */

int main() {
    pd_id_t pid = sexos_getpid();
    
    const char *msg = "--------------------------------------------------\n";
    sexos_write(1, msg, 50);
    
    const char *msg2 = "Hello from Native C Userland! Powered by libsys.\n";
    sexos_write(1, msg2, 49);
    
    const char *msg3 = "--------------------------------------------------\n";
    sexos_write(1, msg3, 50);

    // Try to open a file from our new multi-fs sexvfs
    cap_id_t fd = sexos_open("/disk0/config.json", 0);
    if (fd != (cap_id_t)-1) {
        char buf[64];
        size_t n = sexos_read(fd, buf, sizeof(buf));
        sexos_write(1, "Read config: ", 13);
        sexos_write(1, buf, n);
        sexos_write(1, "\n", 1);
    }

    while (1) {
        sexos_yield();
    }
    
    return 0;
}
