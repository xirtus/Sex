#include <sexos.h>
#include <stdio.h>

/**
 * sext: The Capability/Memory Manager
 */
int main() {
    printf("SEXT: Capability Engine active.\n");
    while(1) sexos_yield();
    return 0;
}
