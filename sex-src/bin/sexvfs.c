#include <sexos.h>
#include <stdio.h>

/**
 * sexvfs: Virtual File System PD
 */
int main() {
    printf("SEXVFS: VFS Layer active.\n");
    while(1) sexos_yield();
    return 0;
}
