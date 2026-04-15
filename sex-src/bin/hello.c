#include <sexos.h>
#include <sys/ioctl.h>
#include <stdio.h>

/**
 * Sample User-Space Application (The Shell)
 * Demonstrating the SexOS User-Land SDK (libsys) and TTY support.
 */

int main() {
    pd_id_t pid = sexos_getpid();
    
    printf("--- SexOS Interaction Test ---\n");
    printf("Current PID: %u\n", pid);

    // Test TTY winsize
    struct winsize ws;
    if (_syscall(SYS_IOCTL, 1, TIOCGWINSZ, (uint64_t)&ws) == 0) {
        printf("Terminal Size: %u rows, %u cols\n", ws.ws_row, ws.ws_col);
    }

    // Test ANSI Escape: Clear Screen
    printf("\x1b[2J\x1b[H"); 
    printf("--- SexOS Interactive Terminal ---\n");
    printf("Type something (characters will be echoed):\n");

    while (1) {
        char c;
        if (sexos_read(0, &c, 1) > 0) {
            sexos_write(1, &c, 1); // Echo back
            if (c == 'q') break;   // Press 'q' to exit test
        }
        sexos_yield();
    }

    printf("\nTest complete. Returning to SASOS.\n");
...    if (fd != (cap_id_t)-1) {
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
