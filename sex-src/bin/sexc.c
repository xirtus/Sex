#include <sexos.h>
#include <stdio.h>

/**
 * sexc: POSIX Emulation Layer
 */
int main() {
    printf("SEXC: POSIX Bridge active.\n");
    while(1) sexos_yield();
    return 0;
}
