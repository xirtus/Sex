#include <sexos.h>
#include <stdio.h>
#include <stdlib.h>

/**
 * SexOS Init Process
 * The first user-space process. Responsible for spawning system services.
 */

int main() {
    printf("--------------------------------------------------\n");
    printf("SexOS Init: System Booting...\n");
    printf("--------------------------------------------------\n");

    // 1. Initialize POSIX Layer (sexc)
    printf("Init: Initializing POSIX Emulation...\n");

    // 2. Spawn the TTY Server if not already started by kernel
    // In our SASOS, servers are just other PDs.

    // 3. Start the Shell
    printf("Init: Spawning User Shell...\n");
    
    // In a real system, we'd use spawn_pd() here.
    // For the prototype, we might just loop or call a shell function if linked.
    
    printf("Init: System ready. Entering supervisory loop.\n");

    while (1) {
        sexos_yield();
    }

    return 0;
}
